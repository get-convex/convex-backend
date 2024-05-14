use std::{
    collections::BTreeMap,
    future,
    iter,
    num::NonZeroU32,
    ops::Bound,
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    bootstrap_model::index::vector_index::{
        FragmentedVectorSegment,
        VectorIndexSnapshotData,
    },
    knobs::{
        DATABASE_WORKERS_MAX_CHECKPOINT_AGE,
        DEFAULT_DOCUMENTS_PAGE_SIZE,
        MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        VECTOR_INDEX_SIZE_SOFT_LIMIT,
        VECTOR_INDEX_THREADS,
        VECTOR_INDEX_WORKER_PAGE_SIZE,
    },
    persistence::TimestampRange,
    runtime::{
        new_rate_limiter,
        Runtime,
    },
    types::{
        IndexId,
        RepeatableTimestamp,
        TabletIndexName,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use futures::{
    channel::oneshot,
    stream::FuturesUnordered,
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use keybroker::Identity;
use search::{
    disk_index::upload_segment,
    fragmented_segment::MutableFragmentedSegmentMetadata,
};
use storage::Storage;
use tempfile::TempDir;
use value::TableIdentifier;
use vector::{
    qdrant_segments::DiskSegmentValues,
    QdrantSchema,
};

use super::{
    writer::VectorMetadataWriter,
    IndexBuild,
};
use crate::{
    bootstrap_model::index_workers::IndexWorkerMetadataModel,
    index_workers::{
        index_meta::{
            SearchIndex,
            SearchIndexConfigParser,
            SearchOnDiskState,
            SearchSnapshot,
            VectorIndexConfigParser,
            VectorSearchIndex,
        },
        BuildReason,
        MultiSegmentBackfillResult,
    },
    metrics::{
        self,
        vector::log_documents_per_segment,
    },
    Database,
    IndexModel,
    Token,
};

pub struct VectorIndexFlusher<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    storage: Arc<dyn Storage>,
    index_size_soft_limit: usize,
    full_scan_threshold_kb: usize,
    // Used for constraining the part size of incremental multi segment builds
    incremental_multipart_threshold_bytes: usize,
    writer: VectorMetadataWriter<RT>,

    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    should_terminate: bool,
    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pause_client: Option<PauseClient>,
}

impl<RT: Runtime> VectorIndexFlusher<RT> {
    pub(crate) fn new(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        writer: VectorMetadataWriter<RT>,
    ) -> Self {
        Self {
            runtime,
            database,
            storage,
            index_size_soft_limit: *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            full_scan_threshold_kb: *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            incremental_multipart_threshold_bytes: *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            writer,
            #[cfg(any(test, feature = "testing"))]
            should_terminate: false,
            #[cfg(any(test, feature = "testing"))]
            pause_client: None,
        }
    }

    /// Run one step of the VectorIndexFlusher's main loop.
    ///
    /// Returns a map of IndexName to number of documents indexed for each
    /// index that was built.
    pub(crate) async fn step(&mut self) -> anyhow::Result<(BTreeMap<TabletIndexName, u32>, Token)> {
        let mut metrics = BTreeMap::new();

        let (to_build, token) = self.needs_backfill::<VectorIndexConfigParser>().await?;
        let num_to_build = to_build.len();
        if num_to_build > 0 {
            tracing::info!("{num_to_build} vector indexes to build");
        }

        #[cfg(any(test, feature = "testing"))]
        if let Some(pause_client) = &mut self.pause_client {
            pause_client.wait(FLUSH_RUNNING_LABEL).await;
        }

        for job in to_build {
            let index_name = job.index_name.clone();
            let num_documents_indexed = self.build_one(job).await?;
            metrics.insert(index_name, num_documents_indexed);
        }

        if num_to_build > 0 {
            tracing::info!("built {num_to_build} vector indexes");
        }

        Ok((metrics, token))
    }

    /// Compute the set of indexes that need to be backfilled.
    async fn needs_backfill<T: SearchIndexConfigParser>(
        &self,
    ) -> anyhow::Result<(Vec<IndexBuild<T::IndexType>>, Token)> {
        let mut to_build = vec![];

        let mut tx = self.database.begin(Identity::system()).await?;
        let step_ts = tx.begin_timestamp();

        let snapshot = self.database.snapshot(step_ts)?;

        let ready_index_sizes = T::IndexType::get_index_sizes(snapshot)?;

        for index_doc in IndexModel::new(&mut tx).get_all_indexes().await? {
            let (index_id, index_metadata) = index_doc.into_id_and_value();
            let Some(config) = T::get_config(index_metadata.config) else {
                continue;
            };
            let name = index_metadata.name;

            let needs_backfill = match &config.on_disk_state {
                SearchOnDiskState::Backfilling(_) => Some(BuildReason::Backfilling),
                SearchOnDiskState::SnapshottedAt(snapshot)
                | SearchOnDiskState::Backfilled(snapshot)
                    if !T::IndexType::is_version_current(snapshot) =>
                {
                    Some(BuildReason::VersionMismatch)
                },
                SearchOnDiskState::SnapshottedAt(SearchSnapshot { ts, .. })
                | SearchOnDiskState::Backfilled(SearchSnapshot { ts, .. }) => {
                    let ts = IndexWorkerMetadataModel::new(&mut tx)
                        .get_fast_forward_ts(*ts, index_id.internal_id())
                        .await?;

                    let index_size = ready_index_sizes
                        .get(&index_id.internal_id())
                        .cloned()
                        .unwrap_or(0);

                    anyhow::ensure!(ts <= *step_ts);

                    let index_age = *step_ts - ts;
                    let too_old = (index_age >= *DATABASE_WORKERS_MAX_CHECKPOINT_AGE
                        && index_size > 0)
                        .then_some(BuildReason::TooOld);
                    if too_old.is_some() {
                        tracing::info!(
                            "Non-empty index is too old, age: {:?}, size: {index_size}",
                            index_age
                        );
                    }
                    let too_large =
                        (index_size > self.index_size_soft_limit).then_some(BuildReason::TooLarge);
                    // Order matters! Too large is more urgent than too old.
                    too_large.or(too_old)
                },
            };
            if let Some(build_reason) = needs_backfill {
                tracing::info!("Queueing vector index for rebuild: {name:?} ({build_reason:?})");
                let table_id = name.table();
                let by_id_metadata = IndexModel::new(&mut tx)
                    .by_id_index_metadata(*table_id)
                    .await?;
                let job = IndexBuild {
                    index_name: name.clone(),
                    index_id: index_id.internal_id(),
                    by_id: by_id_metadata.id().internal_id(),
                    index_config: config,
                    metadata_id: index_id,
                    build_reason,
                };
                to_build.push(job);
            }
        }
        Ok((to_build, tx.into_token()?))
    }

    async fn build_one(&self, job: IndexBuild<VectorSearchIndex>) -> anyhow::Result<u32> {
        let timer = metrics::vector::build_one_timer();

        let result = self.build_multipart_segment(&job).await?;
        tracing::debug!("Built a vector segment for: {result:#?}");

        // 3. Update the vector index metadata.
        let IndexBuildResult {
            snapshot_ts,
            data,
            total_vectors,
            vectors_in_new_segment,
            new_segment_id,
            backfill_result,
        } = result;

        match data {
            VectorIndexSnapshotData::Unknown(_) => {
                anyhow::bail!("Created an unknown snapshot data type")
            },
            VectorIndexSnapshotData::MultiSegment(segments) => {
                self.writer
                    .commit_flush(&job, snapshot_ts, segments, new_segment_id, backfill_result)
                    .await?;
            },
        }

        let vectors_in_new_segment = vectors_in_new_segment.unwrap_or(0);
        metrics::vector::log_documents_per_index(total_vectors);
        metrics::vector::log_documents_per_new_segment(vectors_in_new_segment);
        timer.finish();
        Ok(vectors_in_new_segment)
    }

    async fn build_multipart_segment(
        &self,
        job: &IndexBuild<VectorSearchIndex>,
    ) -> anyhow::Result<IndexBuildResult> {
        let index_path = TempDir::new()?;
        let mut tx = self.database.begin(Identity::system()).await?;
        let table_id = tx.table_mapping().inject_table_number()(*job.index_name.table())?;
        let mut new_ts = tx.begin_timestamp();
        let (previous_segments, build_type) = match job.index_config.on_disk_state {
            SearchOnDiskState::Backfilling(ref backfill_state) => {
                let backfill_snapshot_ts = backfill_state
                    .backfill_snapshot_ts
                    .map(|ts| new_ts.prior_ts(ts))
                    .transpose()?
                    .unwrap_or(new_ts);
                // For backfilling indexes, the snapshot timestamp we return is the backfill
                // snapshot timestamp
                new_ts = backfill_snapshot_ts;

                let cursor = backfill_state.cursor;

                (
                    backfill_state.segments.clone(),
                    MultipartBuildType::IncrementalComplete {
                        cursor: cursor.map(|cursor| table_id.id(cursor)),
                        backfill_snapshot_ts,
                    },
                )
            },
            SearchOnDiskState::Backfilled(ref snapshot)
            | SearchOnDiskState::SnapshottedAt(ref snapshot) => {
                match snapshot.data {
                    // If we're on an old or unrecognized version, rebuild everything. The formats
                    // are not compatible.
                    VectorIndexSnapshotData::Unknown(_) => (
                        vec![],
                        MultipartBuildType::IncrementalComplete {
                            cursor: None,
                            backfill_snapshot_ts: new_ts,
                        },
                    ),
                    VectorIndexSnapshotData::MultiSegment(ref parts) => {
                        let ts = IndexWorkerMetadataModel::new(&mut tx)
                            .get_fast_forward_ts(snapshot.ts, job.index_id)
                            .await?;
                        (
                            parts.clone(),
                            MultipartBuildType::Partial(new_ts.prior_ts(ts)?),
                        )
                    },
                }
            },
        };

        let MultiSegmentBuildResult {
            new_segment,
            updated_previous_segments,
            backfill_result,
        } = self
            .build_multipart_segment_in_dir(job, &index_path, new_ts, build_type, previous_segments)
            .await?;

        let new_segment = if let Some(new_segment) = new_segment {
            Some(upload_segment(&self.runtime, self.storage.clone(), new_segment).await?)
        } else {
            None
        };
        let new_segment_id = new_segment
            .as_ref()
            .map(|segment: &FragmentedVectorSegment| segment.id.clone());
        let vectors_in_new_segment = new_segment.as_ref().map(|segment| segment.num_vectors);

        let new_and_updated_parts = if let Some(new_segment) = new_segment {
            updated_previous_segments
                .into_iter()
                .chain(iter::once(new_segment))
                .collect()
        } else {
            updated_previous_segments
        };

        let total_vectors = new_and_updated_parts
            .iter()
            .map(|segment| {
                let total_vectors = segment.non_deleted_vectors()?;
                log_documents_per_segment(total_vectors);
                Ok(total_vectors)
            })
            .sum::<anyhow::Result<_>>()?;
        let data = VectorIndexSnapshotData::MultiSegment(new_and_updated_parts);

        Ok(IndexBuildResult {
            snapshot_ts: *new_ts,
            data,
            total_vectors,
            vectors_in_new_segment,
            new_segment_id,
            backfill_result,
        })
    }

    async fn build_multipart_segment_in_dir(
        &self,
        job: &IndexBuild<VectorSearchIndex>,
        index_path: &TempDir,
        snapshot_ts: RepeatableTimestamp,
        build_type: MultipartBuildType,
        previous_segments: Vec<FragmentedVectorSegment>,
    ) -> anyhow::Result<MultiSegmentBuildResult> {
        let qdrant_schema = QdrantSchema::new(&job.index_config.developer_config);
        let database = self.database.clone();

        let (tx, rx) = oneshot::channel();
        let runtime = self.runtime.clone();
        let index_name = job.index_name.clone();
        let index_path = index_path.path().to_owned();
        let storage = self.storage.clone();
        let full_scan_threshold_kb = self.full_scan_threshold_kb;
        let incremental_multipart_threshold_bytes = self.incremental_multipart_threshold_bytes;
        let by_id = job.by_id;
        let rate_limit_pages_per_second = job.build_reason.read_max_pages_per_second();
        self.runtime.spawn_thread(move || async move {
            let result = Self::build_multipart_segment_on_thread(
                rate_limit_pages_per_second,
                index_name,
                by_id,
                build_type,
                snapshot_ts,
                runtime,
                database,
                index_path,
                storage,
                previous_segments,
                qdrant_schema,
                full_scan_threshold_kb,
                incremental_multipart_threshold_bytes,
            )
            .await;
            _ = tx.send(result);
        });
        rx.await?
    }

    async fn build_multipart_segment_on_thread(
        rate_limit_pages_per_second: NonZeroU32,
        index_name: TabletIndexName,
        by_id: IndexId,
        build_type: MultipartBuildType,
        snapshot_ts: RepeatableTimestamp,
        runtime: RT,
        database: Database<RT>,
        index_path: PathBuf,
        storage: Arc<dyn Storage>,
        previous_segments: Vec<FragmentedVectorSegment>,
        qdrant_schema: QdrantSchema,
        full_scan_threshold_kb: usize,
        incremental_multipart_threshold_bytes: usize,
    ) -> anyhow::Result<MultiSegmentBuildResult> {
        let page_rate_limiter = new_rate_limiter(
            runtime.clone(),
            Quota::per_second(rate_limit_pages_per_second),
        );
        let row_rate_limiter = new_rate_limiter(
            runtime,
            Quota::per_second(
                NonZeroU32::new(*DEFAULT_DOCUMENTS_PAGE_SIZE)
                    .and_then(|val| val.checked_mul(rate_limit_pages_per_second))
                    .context("Invalid row rate limit")?,
            ),
        );
        // Cursor and completion state for MultipartBuildType::IncrementalComplete
        let mut new_cursor = None;
        let mut is_backfill_complete = true;
        let qdrant_vector_size = qdrant_schema.estimate_vector_size() as u64;

        let (documents, previous_segments) = match build_type {
            MultipartBuildType::Partial(last_ts) => (
                database.load_documents_in_table(
                    *index_name.table(),
                    TimestampRange::new((
                        Bound::Excluded(*last_ts),
                        Bound::Included(*snapshot_ts),
                    ))?,
                    &row_rate_limiter,
                ),
                previous_segments,
            ),
            MultipartBuildType::IncrementalComplete {
                cursor,
                backfill_snapshot_ts,
            } => {
                let documents = database
                    .table_iterator(backfill_snapshot_ts, *VECTOR_INDEX_WORKER_PAGE_SIZE, None)
                    .stream_documents_in_table(
                        *index_name.table(),
                        by_id,
                        cursor,
                        &page_rate_limiter,
                    )
                    .boxed()
                    .scan(0_u64, |total_size, res| {
                        let updated_cursor = if let Ok((doc, _)) = &res {
                            *total_size += qdrant_vector_size;
                            Some(doc.id())
                        } else {
                            None
                        };
                        // Conditionally update cursor and proceed with iteration if
                        // we haven't exceeded incremental part size threshold.
                        future::ready(
                            if *total_size <= incremental_multipart_threshold_bytes as u64 {
                                if let Some(updated_cursor) = updated_cursor {
                                    new_cursor = Some(updated_cursor);
                                }
                                Some(res)
                            } else {
                                is_backfill_complete = false;
                                None
                            },
                        )
                    })
                    .map_ok(|(doc, ts)| (ts, doc.id_with_table_id(), Some(doc)))
                    .boxed();
                (documents, previous_segments)
            },
        };

        let mut mutable_previous_segments = previous_segments
            .into_iter()
            .map(|segment| MutableFragmentedSegmentMetadata::download(segment, storage.clone()))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        let new_segment = qdrant_schema
            .build_disk_index(
                &index_path,
                documents,
                *VECTOR_INDEX_THREADS,
                full_scan_threshold_kb,
                &mut mutable_previous_segments.iter_mut().collect::<Vec<_>>(),
            )
            .await?;

        let updated_previous_segments = mutable_previous_segments
            .into_iter()
            .map(|segment| segment.upload_deleted_bitset(storage.clone()))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        let index_backfill_result =
            if let MultipartBuildType::IncrementalComplete { .. } = build_type {
                Some(MultiSegmentBackfillResult {
                    new_cursor,
                    is_backfill_complete,
                })
            } else {
                None
            };

        Ok(MultiSegmentBuildResult {
            new_segment,
            updated_previous_segments,
            backfill_result: index_backfill_result,
        })
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn build_index_in_test(
        index_name: TabletIndexName,
        table_name: value::TableName,
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
    ) -> anyhow::Result<()> {
        use common::types::IndexName;

        let mut tx = database.begin(Identity::system()).await?;
        let index_name_ = IndexName::new(table_name.clone(), index_name.descriptor().clone())?;
        let mut index_model = IndexModel::new(&mut tx);
        let pending_metadata = index_model.pending_index_metadata(&index_name_)?;
        let enabled_metadata = index_model.enabled_index_metadata(&index_name_)?;
        let metadata = pending_metadata.unwrap_or_else(|| enabled_metadata.unwrap());
        let index_config = VectorIndexConfigParser::get_config(metadata.config.clone())
            .context("Not a vector index?")?;
        let by_id = IndexName::by_id(table_name.clone());
        let Some(by_id_metadata) = IndexModel::new(&mut tx).enabled_index_metadata(&by_id)? else {
            anyhow::bail!("Missing by_id index for {index_name:?}");
        };
        let writer = VectorMetadataWriter::new(runtime.clone(), database.clone(), storage.clone());
        let worker = Self::new(runtime, database, storage, writer);
        let job = IndexBuild {
            index_name,
            index_id: metadata.clone().into_id_and_value().0.internal_id(),
            by_id: by_id_metadata.id().internal_id(),
            index_config,
            metadata_id: metadata.clone().id(),
            build_reason: BuildReason::TooLarge,
        };
        worker.build_one(job).await?;
        Ok(())
    }

    /// Backfills all search indexes that are in a "backfilling" state.
    #[cfg(any(test, feature = "testing"))]
    pub async fn backfill_all_in_test(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        index_size_soft_limit: usize,
    ) -> anyhow::Result<()> {
        let mut flusher = Self::new_for_tests(
            runtime,
            database,
            storage,
            index_size_soft_limit,
            *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            None,
        );
        flusher.step().await?;
        Ok(())
    }

    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn new_for_tests(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        index_size_soft_limit: usize,
        full_scan_threshold_kb: usize,
        incremental_multipart_threshold_bytes: usize,
        pause_client: Option<PauseClient>,
    ) -> Self {
        let writer = VectorMetadataWriter::new(runtime.clone(), database.clone(), storage.clone());
        Self {
            runtime,
            database,
            storage,
            index_size_soft_limit,
            full_scan_threshold_kb,
            incremental_multipart_threshold_bytes,
            should_terminate: true,
            writer,
            pause_client,
        }
    }
}

#[derive(Debug)]
struct MultiSegmentBuildResult {
    // This is None only when no new segment was built because all changes were deletes
    new_segment: Option<DiskSegmentValues>,
    updated_previous_segments: Vec<FragmentedVectorSegment>,
    // This is set only if the build iteration created a segment for a backfilling index
    backfill_result: Option<MultiSegmentBackfillResult>,
}

#[cfg(any(test, feature = "testing"))]
pub(crate) const FLUSH_RUNNING_LABEL: &str = "flush_running";

/// Specifies how documents should be fetched to construct this segment
#[derive(Clone, Copy)]
enum MultipartBuildType {
    Partial(RepeatableTimestamp),
    IncrementalComplete {
        cursor: Option<ResolvedDocumentId>,
        backfill_snapshot_ts: RepeatableTimestamp,
    },
}

#[derive(Debug)]
struct IndexBuildResult {
    snapshot_ts: Timestamp,
    data: VectorIndexSnapshotData,
    total_vectors: u64,
    vectors_in_new_segment: Option<u32>,
    new_segment_id: Option<String>,
    // If this is set, this iteration made progress on backfilling an index
    backfill_result: Option<MultiSegmentBackfillResult>,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::{
        bootstrap_model::index::{
            vector_index::{
                FragmentedVectorSegment,
                VectorIndexSnapshot,
                VectorIndexState,
            },
            IndexConfig,
            IndexMetadata,
        },
        knobs::{
            MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            VECTOR_INDEX_SIZE_SOFT_LIMIT,
        },
        runtime::Runtime,
        types::{
            IndexId,
            IndexName,
        },
    };
    use keybroker::Identity;
    use maplit::{
        btreemap,
        btreeset,
    };
    use must_let::must_let;
    use qdrant_segment::vector_storage::VectorStorage;
    use runtime::testing::TestRuntime;
    use storage::LocalDirStorage;
    use value::{
        assert_obj,
        assert_val,
        ConvexValue,
        GenericDocumentId,
        TabletIdAndTableNumber,
    };
    use vector::{
        PublicVectorSearchQueryResult,
        QdrantExternalId,
        VectorSearch,
    };

    use super::VectorIndexFlusher;
    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        test_helpers::new_test_database,
        tests::vector_test_utils::{
            add_document_vec,
            add_document_with_value,
            backfilling_vector_index_with_doc,
            IndexData,
            VectorFixtures,
        },
        vector_index_worker::compactor::CompactionConfig,
        Database,
        IndexModel,
        SystemMetadataModel,
        UserFacingModel,
    };

    fn new_vector_flusher_with_soft_limit(
        rt: &TestRuntime,
        database: &Database<TestRuntime>,
        soft_limit: usize,
    ) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        let storage = LocalDirStorage::new(rt.clone())?;
        Ok(VectorIndexFlusher::new_for_tests(
            rt.clone(),
            database.clone(),
            Arc::new(storage),
            soft_limit,
            *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            None,
        ))
    }

    fn new_vector_flusher(
        rt: &TestRuntime,
        database: &Database<TestRuntime>,
    ) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        new_vector_flusher_with_soft_limit(rt, database, 1000)
    }

    #[convex_macro::test_runtime]
    async fn worker_does_not_crash_on_documents_with_invalid_vector_dimensions(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let database = new_test_database(rt.clone()).await;

        let IndexData { index_name, .. } = backfilling_vector_index_with_doc(&database).await?;

        let mut tx = database.begin_system().await?;
        let vec = [1f64].into_iter().map(ConvexValue::Float64).collect();
        add_document_vec(&mut tx, index_name.table(), vec).await?;
        database.commit(tx).await?;

        let mut worker = new_vector_flusher(&rt, &database)?;
        worker.step().await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_does_not_crash_on_documents_with_non_vector(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let database = new_test_database(rt.clone()).await;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = backfilling_vector_index_with_doc(&database).await?;

        let mut tx = database.begin_system().await?;
        add_document_with_value(
            &mut tx,
            index_name.table(),
            ConvexValue::String(value::ConvexString::try_from("test")?),
        )
        .await?;
        database.commit(tx).await?;

        // Use 0 soft limit so that we always reindex documents
        let mut worker = new_vector_flusher_with_soft_limit(&rt, &database, 0)?;
        let (metrics, _) = worker.step().await?;
        // Make sure we advance past the invalid document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_with_empty_index_does_not_create_empty_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;

        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert!(segments.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_at_or_over_scan_threshold_uses_hnsw(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
        worker.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();

        let segment = fixtures.load_segment(segment).await?;
        assert!(segment.segment_config.is_any_vector_indexed());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn incremental_backfill_exceed_part_threshold_builds_multiple_parts(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        fixtures
            .add_document_vec_array(index_name.table(), [5f64, 6f64])
            .await?;
        let mut worker = fixtures.new_index_flusher_with_incremental_part_threshold(8)?;

        // Should be in backfilling state after step
        worker.step().await?;
        let segments = fixtures
            .get_segments_from_backfilling_index(index_name.clone())
            .await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();
        let segment = fixtures.load_segment(segment).await?;
        assert_eq!(segment.total_point_count(), 1);

        // Should be no longer in backfilling state now after step
        worker.step().await?;
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);
        let segment = segments.get(1).unwrap();
        let segment = fixtures.load_segment(segment).await?;
        assert_eq!(segment.total_point_count(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_under_full_scan_threshold_does_not_use_hnsw(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(1000000)?;
        worker.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();

        let segment = fixtures.load_segment(segment).await?;
        assert!(!segment.segment_config.is_any_vector_indexed());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_with_deleted_vector_does_not_include_deleted_vector_in_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let to_delete = fixtures
            .add_document_vec_array(index_name.table(), [4f64, 5f64])
            .await?;

        let mut tx = fixtures.db.begin_system().await?;
        UserFacingModel::new(&mut tx)
            .delete(to_delete.into())
            .await?;
        fixtures.db.commit(tx).await?;

        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();
        assert_eq!(1, segment.num_vectors);
        assert_eq!(0, segment.num_deleted);

        let segment = fixtures.load_segment(segment).await?;
        assert_eq!(segment.id_tracker.borrow().deleted_point_count(), 0);
        let count = segment
            .vector_data
            .get("default_vector")
            .unwrap()
            .vector_storage
            .borrow()
            .total_vector_count();
        assert_eq!(1, count);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_with_segment_no_new_documents_doesnt_append_empty_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let mut worker = fixtures.new_index_flusher()?;
        worker.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfilled_concurrent_compaction_and_flush(rt: TestRuntime) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        // Mark the index backfilled.
        let index_data = fixtures.backfilling_vector_index().await?;
        fixtures.backfill().await?;

        let IndexData { index_name, .. } = index_data;

        // Create enough segments to trigger compaction.
        let mut deleted_doc_ids = vec![];
        for _ in 0..min_compaction_segments {
            deleted_doc_ids.push(
                fixtures
                    .add_document_vec_array(index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }
        // Queue up deletes for all existing segments, and one new vector that will
        // cause cause the flusher to write a new segment.
        let mut tx = fixtures.db.begin_system().await?;
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new(&mut tx)
                .delete((*doc_id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;
        let non_deleted_id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;

        // Run the compactor / flusher concurrently in a way where the compactor
        // wins the race.
        fixtures.run_compaction_during_flush().await?;

        // Verify we propagate the new deletes to the compacted segment and retain our
        // new segment.
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(2, segments.len());

        let (compacted_segment, new_segment): (Vec<_>, Vec<_>) = segments
            .into_iter()
            .partition(|segment| segment.num_deleted > 0);
        assert_eq!(compacted_segment.len(), 1);

        let compacted_segment = compacted_segment.first().unwrap();
        verify_segment_state(&fixtures, compacted_segment, deleted_doc_ids, vec![]).await?;

        assert_eq!(new_segment.len(), 1);
        let new_segment = new_segment.first().unwrap();
        verify_segment_state(&fixtures, new_segment, vec![], vec![non_deleted_id]).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn incremental_index_backfill_concurrent_compaction_and_flush(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;

        // Add 4 vectors and set part threshold to build 4 segments
        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        for i in 0..(min_compaction_segments + 1) {
            fixtures
                .add_document_vec_array(index_name.table(), [i as f64, (i + 1) as f64])
                .await?;
        }
        // Do every backfill flush step until last one
        let mut worker = fixtures.new_index_flusher_with_incremental_part_threshold(8)?;
        for _ in 0..min_compaction_segments {
            worker.step().await?;
        }

        // For last iteration, run the compactor / flusher concurrently in a way where
        // the compactor wins the race.
        fixtures.run_compaction_during_flush().await?;

        // There should be 2 segments left: the compacted segment and the new segment
        // from flush
        worker.step().await?;
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);

        let compacted_segment = fixtures.load_segment(segments.first().unwrap()).await?;
        assert_eq!(compacted_segment.total_point_count(), 3);
        let compacted_segment = fixtures.load_segment(segments.get(1).unwrap()).await?;
        assert_eq!(compacted_segment.total_point_count(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn concurrent_compaction_and_flush_new_segment_propagates_deletes(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        let IndexData { index_name, .. } = index_data;

        // Create enough segments to trigger compaction.
        let mut deleted_doc_ids = vec![];
        for _ in 0..min_compaction_segments {
            deleted_doc_ids.push(
                fixtures
                    .add_document_vec_array(index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }
        // Queue up deletes for all existing segments, and one new vector that will
        // cause cause the flusher to write a new segment.
        let mut tx = fixtures.db.begin_system().await?;
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new(&mut tx)
                .delete((*doc_id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;
        let non_deleted_id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;

        // Run the compactor / flusher concurrently in a way where the compactor
        // wins the race.
        fixtures.run_compaction_during_flush().await?;

        // Verify we propagate the new deletes to the compacted segment and retain our
        // new segment.
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(2, segments.len());

        let (compacted_segment, new_segment): (Vec<_>, Vec<_>) = segments
            .into_iter()
            .partition(|segment| segment.num_deleted > 0);
        assert_eq!(compacted_segment.len(), 1);

        let compacted_segment = compacted_segment.first().unwrap();
        verify_segment_state(&fixtures, compacted_segment, deleted_doc_ids, vec![]).await?;

        assert_eq!(new_segment.len(), 1);
        let new_segment = new_segment.first().unwrap();
        verify_segment_state(&fixtures, new_segment, vec![], vec![non_deleted_id]).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn concurrent_compaction_and_flush_no_new_segment_propagates_updates_and_deletes(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        let IndexData { index_name, .. } = index_data;

        // Create enough segments to trigger compaction.
        let mut deleted_doc_ids = vec![];
        for _ in 0..min_compaction_segments {
            deleted_doc_ids.push(
                fixtures
                    .add_document_vec_array(index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }
        // Queue up updates and deletes for all existing segments and no new vectors so
        // that compaction will just delete all of the existing segments without
        // adding a new one.
        let mut tx = fixtures.db.begin_system().await?;
        let patched_object = assert_val!([5f64, 6f64]);
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new(&mut tx)
                .patch(
                    (*doc_id).into(),
                    assert_obj!("vector" => patched_object.clone()).into(),
                )
                .await?;
        }
        fixtures.db.commit(tx).await?;

        let mut tx = fixtures.db.begin_system().await?;
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new(&mut tx)
                .delete((*doc_id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;

        fixtures.run_compaction_during_flush().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();

        verify_segment_state(&fixtures, segment, deleted_doc_ids, vec![]).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfilling_and_enabled_version_of_index_writes_to_backfilling(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        fixtures.enabled_vector_index().await?;

        let backfilling_data = fixtures.backfilling_vector_index().await?;
        let IndexData {
            index_id: backfilling_index_id,
            ..
        } = backfilling_data;

        fixtures.new_index_flusher()?.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        let new_metadata = IndexModel::new(&mut tx)
            .require_index_by_id(backfilling_index_id)
            .await?
            .into_value();
        must_let!(let IndexMetadata {
            config: IndexConfig::Vector {
                on_disk_state: VectorIndexState::Backfilled(VectorIndexSnapshot { .. }),
                ..
            },
            ..
        } = new_metadata);

        Ok(())
    }

    async fn verify_segment_state(
        fixtures: &VectorFixtures,
        segment: &FragmentedVectorSegment,
        expected_deletes: Vec<GenericDocumentId<TabletIdAndTableNumber>>,
        expected_non_deleted: Vec<GenericDocumentId<TabletIdAndTableNumber>>,
    ) -> anyhow::Result<()> {
        assert_eq!(segment.num_deleted as usize, expected_deletes.len());
        assert_eq!(
            segment.num_vectors as usize,
            expected_non_deleted.len() + expected_deletes.len()
        );

        let segment = fixtures.load_segment(segment).await?;

        for doc_id in expected_deletes {
            let external_id = QdrantExternalId::try_from(&doc_id)?;
            let internal_id = segment
                .id_tracker
                .borrow()
                .internal_id(*external_id)
                .unwrap();
            assert!(segment.id_tracker.borrow().is_deleted_point(internal_id));
        }
        for doc_id in expected_non_deleted {
            let external_id = QdrantExternalId::try_from(&doc_id)?;
            let internal_id = segment
                .id_tracker
                .borrow()
                .internal_id(*external_id)
                .unwrap();
            assert!(!segment.id_tracker.borrow().is_deleted_point(internal_id));
        }
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_worker_with_segment_document_added_then_removed_no_empty_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut tx = fixtures.db.begin_system().await?;
        UserFacingModel::new(&mut tx).delete(id.into()).await?;
        fixtures.db.commit(tx).await?;

        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 0 });

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_worker_with_segment_updated_then_deleted_after_written_succeeds(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        // Add the document to a segment
        let id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        // Update the document in place
        let mut tx = fixtures.db.begin_system().await?;
        let patched_object = assert_val!([5f64, 6f64]);
        UserFacingModel::new(&mut tx)
            .patch(id.into(), assert_obj!("vector" => patched_object).into())
            .await?;
        fixtures.db.commit(tx).await?;

        // Then delete it
        let mut tx = fixtures.db.begin_system().await?;
        UserFacingModel::new(&mut tx).delete(id.into()).await?;
        fixtures.db.commit(tx).await?;

        // And flush to ensure that we handle the document showing up repeatedly in the
        // document log for the old instance.
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 0 });

        let mut segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.remove(0);
        assert_eq!(segment.num_deleted, 1);
        assert_eq!(segment.num_vectors, 1);

        Ok(())
    }

    // When we're backfilling we read the contents of the table at some snapshot, so
    // we never read deleted documents and our segment should contain exactly
    // the set of vectors in the table at that snapshot timestamp.
    #[convex_macro::test_runtime]
    async fn flush_during_backfill_with_adds_and_deletes_includes_only_non_deleted_in_counts(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        for index in 0..10 {
            let id = fixtures
                .add_document_vec_array(index_name.table(), [3f64, 4f64])
                .await?;
            if index >= 5 {
                let mut tx = fixtures.db.begin_system().await?;
                UserFacingModel::new(&mut tx).delete(id.into()).await?;
                fixtures.db.commit(tx).await?;
            }
        }
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 5});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();
        assert_eq!(5, segment.num_vectors);
        assert_eq!(0, segment.num_deleted);

        Ok(())
    }

    // After backfilling finishes, we read incrementally from the document log. So
    // it's perfectly possible that we'll read a document and a subsequent
    // modification or delete all while writing a single segment. As a result we
    // expect to write some number of deleted vectors.
    #[convex_macro::test_runtime]
    async fn flush_after_backfill_with_adds_and_deletes_includes_deleted_vectors_in_counts(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        for index in 0..10 {
            let id = fixtures
                .add_document_vec_array(index_name.table(), [3f64, 4f64])
                .await?;
            if index >= 5 {
                let mut tx = fixtures.db.begin_system().await?;
                UserFacingModel::new(&mut tx).delete(id.into()).await?;
                fixtures.db.commit(tx).await?;
            }
        }
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 10});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();
        assert_eq!(10, segment.num_vectors);
        assert_eq!(5, segment.num_deleted);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn flush_after_backfill_with_adds_and_updates_includes_updated_vectors_in_count(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        for index in 0..10 {
            let id = fixtures
                .add_document_vec_array(index_name.table(), [3f64, 4f64])
                .await?;
            if index >= 5 {
                let mut tx = fixtures.db.begin_system().await?;
                let patched_object = assert_val!([5f64, 6f64]);
                UserFacingModel::new(&mut tx)
                    .patch(
                        id.into(),
                        assert_obj!("vector" => patched_object.clone()).into(),
                    )
                    .await?;
                fixtures.db.commit(tx).await?;
            }
        }
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 10});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();
        assert_eq!(10, segment.num_vectors);
        assert_eq!(0, segment.num_deleted);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_worker_builds_indexes_incrementally(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.enabled_vector_index().await?;

        for vector in [[3f64, 4f64], [5f64, 6f64], [6f64, 7f64]] {
            let id = fixtures
                .add_document_vec_array(index_name.table(), vector)
                .await?;
            let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
            let (metrics, _) = worker.step().await?;
            assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

            let results = vector_search(&fixtures.db, index_name.clone(), vector).await?;
            assert_eq!(results.first().unwrap().id.internal_id(), id.internal_id());
        }

        Ok(())
    }

    async fn set_fast_forward_time_to_now<RT: Runtime>(
        db: &Database<RT>,
        index_id: IndexId,
    ) -> anyhow::Result<()> {
        let mut tx = db.begin_system().await?;
        let metadata = IndexWorkerMetadataModel::new(&mut tx)
            .get_or_create_vector_search(index_id)
            .await?;
        let (worker_meta_doc_id, mut metadata) = metadata.into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = *tx.begin_timestamp();
        SystemMetadataModel::new(&mut tx)
            .replace(worker_meta_doc_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_with_newer_fast_forward_time_builds_from_fast_forward_time(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            index_id,
            resolved_index_name,
        } = fixtures.enabled_vector_index().await?;

        let vector = [8f64, 9f64];
        fixtures
            .add_document_vec_array(index_name.table(), vector)
            .await?;

        set_fast_forward_time_to_now(&fixtures.db, index_id.internal_id()).await?;

        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        let results = vector_search(&fixtures.db, index_name, vector).await?;

        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_with_older_fast_forward_time_builds_from_index_time(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            index_id,
            resolved_index_name,
        } = fixtures.enabled_vector_index().await?;

        set_fast_forward_time_to_now(&fixtures.db, index_id.internal_id()).await?;

        let vector = [8f64, 9f64];
        let vector_doc_id = fixtures
            .add_document_vec_array(index_name.table(), vector)
            .await?;

        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let results = vector_search(&fixtures.db, index_name, vector).await?;

        assert_eq!(
            results.first().unwrap().id.internal_id(),
            vector_doc_id.internal_id()
        );

        Ok(())
    }

    async fn vector_search<RT: Runtime>(
        db: &Database<RT>,
        index_name: IndexName,
        vector: [f64; 2],
    ) -> anyhow::Result<Vec<PublicVectorSearchQueryResult>> {
        Ok(db
            .vector_search(
                Identity::system(),
                VectorSearch {
                    index_name,
                    vector: vector.into_iter().map(|value| value as f32).collect(),
                    limit: Some(1),
                    expressions: btreeset![],
                },
            )
            .await?
            .0)
    }
}
