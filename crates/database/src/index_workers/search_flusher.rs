use std::{
    collections::{
        BTreeMap,
        Bound,
    },
    future,
    iter,
    marker::PhantomData,
    num::NonZeroU32,
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use common::{
    knobs::{
        DATABASE_WORKERS_MAX_CHECKPOINT_AGE,
        DEFAULT_DOCUMENTS_PAGE_SIZE,
        VECTOR_INDEX_WORKER_PAGE_SIZE,
    },
    persistence::{
        DocumentLogEntry,
        PersistenceReader,
        RepeatablePersistence,
        TimestampRange,
    },
    runtime::{
        new_rate_limiter,
        Runtime,
    },
    types::{
        IndexId,
        RepeatableTimestamp,
        TabletIndexName,
    },
};
use futures::{
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use keybroker::Identity;
use search::metrics::SearchType;
use storage::Storage;
use sync_types::Timestamp;
use tempfile::TempDir;
use tokio::{
    sync::oneshot,
    task,
};
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
};

use crate::{
    bootstrap_model::{
        index_backfills::IndexBackfillModel,
        index_workers::IndexWorkerMetadataModel,
    },
    index_workers::{
        index_meta::{
            SearchIndex,
            SearchIndexConfig,
            SearchOnDiskState,
            SearchSnapshot,
            SegmentStatistics,
            SegmentType,
            SnapshotData,
        },
        writer::{
            SearchIndexMetadataWriter,
            SearchIndexWriteResult,
        },
        BuildReason,
        MultiSegmentBackfillResult,
    },
    metrics::{
        build_one_search_index_timer,
        log_documents_per_new_search_segment,
        log_documents_per_search_index,
        log_documents_per_search_segment,
        log_non_deleted_documents_per_search_index,
        log_non_deleted_documents_per_search_segment,
    },
    Database,
    IndexModel,
    Token,
};

pub(crate) const FLUSH_RUNNING_LABEL: &str = "flush_running";

pub struct SearchFlusher<RT: Runtime, T: SearchIndex> {
    params: Params<RT, T>,
    writer: SearchIndexMetadataWriter<RT, T>,
    _config: PhantomData<T>,
}

impl<RT: Runtime, T: SearchIndex> Deref for SearchFlusher<RT, T> {
    type Target = Params<RT, T>;

    fn deref(&self) -> &Self::Target {
        &self.params
    }
}

#[derive(Clone)]
pub struct Params<RT: Runtime, T: SearchIndex> {
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    limits: SearchIndexLimits,
    build_args: T::BuildIndexArgs,
}

#[derive(Clone)]
pub struct SearchIndexLimits {
    /// The size at which we will start building a new segment. It's a 'soft'
    /// limit because we will allow writes to continue even after this size
    /// is reached. Writes are only blocked after we hit a hard index size
    /// limit not specified here.
    pub index_size_soft_limit: usize,
    /// The number of bytes we'll write to each individual segment when
    /// backfilling a new index.
    ///
    /// For example, if we add an index to a table with 10GiB of documents, we
    /// don't want to build one single 10GiB segment on backend (very costly
    /// in terms of disk/cpu/memory). So instead we build N smaller segments
    /// and allow compaction to merge them (or not) as necessary.
    ///
    /// This number determines the size of each of the N smaller segments based
    /// on an estimate of the size of each document we index.
    pub incremental_multipart_threshold_bytes: usize,
}

impl<RT: Runtime, T: SearchIndex + 'static> SearchFlusher<RT, T> {
    pub(crate) fn new(
        runtime: RT,
        database: Database<RT>,
        reader: Arc<dyn PersistenceReader>,
        storage: Arc<dyn Storage>,
        limits: SearchIndexLimits,
        writer: SearchIndexMetadataWriter<RT, T>,
        build_args: T::BuildIndexArgs,
    ) -> Self {
        Self {
            params: Params {
                runtime,
                database,
                reader,
                storage,
                limits,
                build_args,
            },
            writer,
            _config: PhantomData,
        }
    }

    fn index_type_name(&self) -> &'static str {
        match Self::search_type() {
            SearchType::Vector => "vector",
            SearchType::Text => "text",
        }
    }

    /// Run one step of the flusher's main loop.
    ///
    /// Returns a map of IndexName to number of documents indexed for each
    /// index that was built.
    pub async fn step(&mut self) -> anyhow::Result<(BTreeMap<TabletIndexName, u64>, Token)> {
        let mut metrics = BTreeMap::new();

        let (to_build, token) = self.needs_backfill().await?;
        let num_to_build = to_build.len();
        let index_type = self.index_type_name();
        if num_to_build > 0 {
            tracing::info!("{num_to_build} {index_type} indexes to build");
        }

        let pause_client = self.database.runtime().pause_client();
        pause_client.wait(FLUSH_RUNNING_LABEL).await;

        for job in to_build {
            task::consume_budget().await;

            let index_name = job.index_name.clone();
            let num_documents_indexed = self.build_one(job, self.build_args.clone()).await?;
            metrics.insert(index_name, num_documents_indexed);
        }

        if num_to_build > 0 {
            tracing::info!("SearchIndexFlusher built {num_to_build} {index_type} indexes");
        }

        Ok((metrics, token))
    }

    fn search_type() -> SearchType {
        T::search_type()
    }

    pub(crate) async fn build_one(
        &self,
        job: IndexBuild<T>,
        build_args: T::BuildIndexArgs,
    ) -> anyhow::Result<u64> {
        let timer = build_one_search_index_timer(T::search_type());

        let result = self.build_multipart_segment(&job, build_args).await?;
        tracing::debug!(
            "Built a {} segment for: {result:#?}",
            self.index_type_name()
        );

        let SearchIndexWriteResult {
            index_stats,
            new_segment_stats,
            per_segment_stats,
        } = self.writer.commit_flush(&job, result).await?;

        let new_segment_stats = new_segment_stats.unwrap_or_default();
        log_documents_per_new_search_segment(
            new_segment_stats.num_documents(),
            Self::search_type(),
        );

        per_segment_stats.into_iter().for_each(|stats| {
            log_documents_per_search_segment(stats.num_documents(), Self::search_type());
            log_non_deleted_documents_per_search_segment(
                stats.num_non_deleted_documents(),
                Self::search_type(),
            );
        });

        log_documents_per_search_index(index_stats.num_documents(), Self::search_type());
        log_non_deleted_documents_per_search_index(
            index_stats.num_non_deleted_documents(),
            Self::search_type(),
        );
        timer.finish();

        Ok(new_segment_stats.num_documents())
    }

    /// Compute the set of indexes that need to be backfilled.
    async fn needs_backfill(&self) -> anyhow::Result<(Vec<IndexBuild<T>>, Token)> {
        let mut to_build = vec![];

        let mut tx = self.database.begin(Identity::system()).await?;
        let step_ts = tx.begin_timestamp();

        let snapshot = self.database.snapshot(step_ts)?;

        let ready_index_sizes = T::get_index_sizes(snapshot)?;

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
                    if !T::is_version_current(snapshot) =>
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
                    let too_large = (index_size > self.limits.index_size_soft_limit)
                        .then_some(BuildReason::TooLarge);
                    // Order matters! Too large is more urgent than too old.
                    too_large.or(too_old)
                },
            };
            if let Some(build_reason) = needs_backfill {
                tracing::info!(
                    "Queueing {} index for rebuild: {name:?} ({build_reason:?})",
                    self.index_type_name()
                );
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

    async fn build_multipart_segment(
        &self,
        job: &IndexBuild<T>,
        build_index_args: T::BuildIndexArgs,
    ) -> anyhow::Result<IndexBuildResult<T>> {
        let index_path = TempDir::new()?;
        let mut tx = self.database.begin(Identity::system()).await?;
        let tablet_id = *job.index_name.table();
        let table_number = tx.table_mapping().tablet_number(tablet_id)?;
        let mut new_ts = tx.begin_timestamp();
        let (previous_segments, build_type) = match job.index_config.on_disk_state {
            SearchOnDiskState::Backfilling(ref backfill_state) => {
                let maybe_backfill_snapshot_ts = backfill_state
                    .backfill_snapshot_ts
                    .map(|ts| new_ts.prior_ts(ts))
                    .transpose()?;
                let backfill_snapshot_ts = if let Some(ts) = maybe_backfill_snapshot_ts {
                    ts
                } else {
                    // This is the beginning of a backfill!
                    // We need to initialize the backfill with the size of the table at this
                    // snapshot.
                    let tablet = job.index_name.table();
                    let table_name = tx.table_mapping().tablet_name(*tablet)?;
                    let table_namespace = tx.table_mapping().tablet_namespace(*tablet)?;
                    let total_docs = tx.count(table_namespace, &table_name).await?;
                    let mut index_backfill_model = IndexBackfillModel::new(&mut tx);
                    index_backfill_model
                        .initialize_backfill(job.index_id, total_docs)
                        .await?;
                    new_ts
                };
                // For backfilling indexes, the snapshot timestamp we return is the backfill
                // snapshot timestamp
                new_ts = backfill_snapshot_ts;

                let cursor = backfill_state.cursor;

                (
                    backfill_state.segments.clone(),
                    MultipartBuildType::IncrementalComplete {
                        cursor: cursor.map(|cursor| {
                            ResolvedDocumentId::new(
                                tablet_id,
                                DeveloperDocumentId::new(table_number, cursor),
                            )
                        }),
                        backfill_snapshot_ts,
                    },
                )
            },
            SearchOnDiskState::Backfilled(ref snapshot)
            | SearchOnDiskState::SnapshottedAt(ref snapshot) => {
                match snapshot.data {
                    // We skip rebuilding the index if it is an unknown format because it's very
                    // expensive to compute the whole index in a single segment.
                    SnapshotData::Unknown(_) => {
                        anyhow::bail!("Unknown index format, not rebuilding")
                    },
                    SnapshotData::MultiSegment(ref parts) => {
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
        self.database
            .commit_with_write_source(tx, "search_flusher_initialize_backfill")
            .await?;

        let MultiSegmentBuildResult {
            new_segment,
            updated_previous_segments,
            backfill_result,
        } = self
            .build_multipart_segment_in_dir(
                job,
                &index_path,
                new_ts,
                build_type,
                previous_segments,
                build_index_args,
            )
            .await?;

        let new_segment = if let Some(new_segment) = new_segment {
            Some(self.upload_new_segment(new_segment).await?)
        } else {
            None
        };
        let new_segment_id = new_segment.as_ref().map(|segment| segment.id().to_string());
        let new_segment_stats = new_segment
            .as_ref()
            .map(|segment| segment.statistics())
            .transpose()?;

        let new_and_updated_parts = if let Some(new_segment) = new_segment {
            updated_previous_segments
                .into_iter()
                .chain(iter::once(new_segment))
                .collect()
        } else {
            updated_previous_segments
        };

        let total_stats = new_and_updated_parts
            .iter()
            .map(|segment| segment.statistics())
            .reduce(SegmentStatistics::add)
            .transpose()?
            .unwrap_or_default();
        let data = SnapshotData::MultiSegment(new_and_updated_parts);

        Ok(IndexBuildResult {
            snapshot_ts: new_ts,
            data,
            total_stats,
            new_segment_stats,
            new_segment_id,
            backfill_result,
        })
    }

    async fn build_multipart_segment_in_dir(
        &self,
        job: &IndexBuild<T>,
        index_path: &TempDir,
        snapshot_ts: RepeatableTimestamp,
        build_type: MultipartBuildType,
        previous_segments: Vec<T::Segment>,
        build_index_args: T::BuildIndexArgs,
    ) -> anyhow::Result<MultiSegmentBuildResult<T>> {
        let (tx, rx) = oneshot::channel();
        let index_name = job.index_name.clone();
        let index_path = index_path.path().to_owned();
        let by_id = job.by_id;
        let rate_limit_pages_per_second = job.build_reason.read_max_pages_per_second();
        let developer_config = job.index_config.developer_config.clone();
        let params = self.params.clone();
        let handle = self
            .runtime
            .spawn_thread("build_multipart_segment", move || async move {
                let result = Self::build_multipart_segment_on_thread(
                    params,
                    rate_limit_pages_per_second,
                    index_name,
                    by_id,
                    build_type,
                    snapshot_ts,
                    developer_config,
                    index_path,
                    previous_segments,
                    build_index_args,
                )
                .await;
                let _ = tx.send(result);
            });
        handle.join().await?;
        rx.await?
    }

    async fn build_multipart_segment_on_thread(
        params: Params<RT, T>,
        rate_limit_pages_per_second: NonZeroU32,
        index_name: TabletIndexName,
        by_id: IndexId,
        build_type: MultipartBuildType,
        snapshot_ts: RepeatableTimestamp,
        developer_config: T::DeveloperConfig,
        index_path: PathBuf,
        previous_segments: Vec<T::Segment>,
        build_index_args: T::BuildIndexArgs,
    ) -> anyhow::Result<MultiSegmentBuildResult<T>> {
        let row_rate_limiter = new_rate_limiter(
            params.runtime.clone(),
            Quota::per_second(
                NonZeroU32::new(*DEFAULT_DOCUMENTS_PAGE_SIZE)
                    .and_then(|val| val.checked_mul(rate_limit_pages_per_second))
                    .context("Invalid row rate limit")?,
            ),
        );
        // Cursor and completion state for MultipartBuildType::IncrementalComplete
        let mut new_cursor = None;
        let mut is_backfill_complete = true;
        let mut is_size_exceeded = false;
        let qdrant_schema = T::new_schema(&developer_config);

        let mut lower_bound_ts: Option<Timestamp> = None;
        let (documents, previous_segments) = match build_type {
            MultipartBuildType::Partial(last_ts) => {
                lower_bound_ts = Some(*last_ts);
                (
                    params.database.load_documents_in_table(
                        *index_name.table(),
                        TimestampRange::new((
                            Bound::Excluded(*last_ts),
                            Bound::Included(*snapshot_ts),
                        ))?,
                        T::partial_document_order(),
                        &row_rate_limiter,
                    ),
                    previous_segments,
                )
            },
            MultipartBuildType::IncrementalComplete {
                cursor,
                backfill_snapshot_ts,
            } => {
                let documents = params
                    .database
                    .table_iterator(backfill_snapshot_ts, *VECTOR_INDEX_WORKER_PAGE_SIZE)
                    .stream_documents_in_table(*index_name.table(), by_id, cursor)
                    .boxed()
                    .scan(0_u64, |total_size, res| {
                        if is_size_exceeded {
                            is_backfill_complete = false;
                            return future::ready(None);
                        }
                        let updated_cursor = if let Ok(rev) = &res {
                            let size = T::estimate_document_size(&qdrant_schema, &rev.value);
                            *total_size += size;
                            Some(rev.value.id())
                        } else {
                            None
                        };
                        if *total_size >= params.limits.incremental_multipart_threshold_bytes as u64
                        {
                            // The size is exceeded, but we don't know whether the backfill is
                            // complete until we see if there is another document. So set a boolean
                            // and see if we loop again. If we do, then we know the backfill isn't
                            // finished. If we don't, then this happens to be the last document and
                            // the backfill is done.
                            // This behavior is only really important for very large documents or
                            // very small incremental_multipart_threshold_bytes values where you can
                            // have weird behavior if we returned early here instead (like never
                            // marking the index as backfilled).
                            is_size_exceeded = true;
                        }
                        if let Some(updated_cursor) = updated_cursor {
                            new_cursor = Some(updated_cursor);
                        }
                        future::ready(Some(res))
                    })
                    .map_ok(|rev| DocumentLogEntry {
                        ts: rev.ts,
                        id: rev.value.id_with_table_id(),
                        value: Some(rev.value),
                        prev_ts: rev.prev_ts,
                    })
                    .boxed();
                (documents, previous_segments)
            },
        };

        let mut mutable_previous_segments =
            T::download_previous_segments(params.storage.clone(), previous_segments).await?;

        let persistence = RepeatablePersistence::new(
            params.reader,
            snapshot_ts,
            params.database.retention_validator(),
        );
        let new_segment = T::build_disk_index(
            &qdrant_schema,
            &index_path,
            documents,
            persistence,
            &mut mutable_previous_segments,
            lower_bound_ts,
            build_index_args,
            build_type,
        )
        .await?;

        let updated_previous_segments =
            T::upload_previous_segments(params.storage, mutable_previous_segments).await?;

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

    async fn upload_new_segment(&self, new_segment: T::NewSegment) -> anyhow::Result<T::Segment> {
        let (tx, rx) = oneshot::channel();
        let rt = self.runtime.clone();
        let storage = self.storage.clone();
        let handle = self
            .runtime
            .spawn_thread("upload_new_segment", move || async move {
                let result = T::upload_new_segment(&rt, storage, new_segment).await;
                let _ = tx.send(result);
            });
        handle.join().await?;
        rx.await?
    }
}

pub(crate) struct IndexBuild<T: SearchIndex> {
    pub(crate) index_name: TabletIndexName,
    pub(crate) index_id: IndexId,
    pub(crate) by_id: IndexId,
    pub(crate) metadata_id: ResolvedDocumentId,
    pub(crate) index_config: SearchIndexConfig<T>,
    pub(crate) build_reason: BuildReason,
}

#[derive(Debug)]
pub struct IndexBuildResult<T: SearchIndex> {
    pub snapshot_ts: RepeatableTimestamp,
    pub data: SnapshotData<T::Segment>,
    pub total_stats: T::Statistics,
    pub new_segment_stats: Option<T::Statistics>,
    pub new_segment_id: Option<String>,
    // If this is set, this iteration made progress on backfilling an index
    pub backfill_result: Option<MultiSegmentBackfillResult>,
}

#[derive(Debug)]
pub struct MultiSegmentBuildResult<T: SearchIndex> {
    // This is None only when no new segment was built because all changes were deletes
    new_segment: Option<T::NewSegment>,
    updated_previous_segments: Vec<T::Segment>,
    // This is set only if the build iteration created a segment for a backfilling index
    backfill_result: Option<MultiSegmentBackfillResult>,
}

/// Specifies how documents should be fetched to construct this segment
#[derive(Clone, Copy)]
pub enum MultipartBuildType {
    // Build a part
    Partial(RepeatableTimestamp),
    // Build the whole index in parts
    IncrementalComplete {
        cursor: Option<ResolvedDocumentId>,
        backfill_snapshot_ts: RepeatableTimestamp,
    },
}
