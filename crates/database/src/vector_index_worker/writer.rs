use std::{
    iter,
    num::NonZeroU32,
    ops::Bound,
    sync::Arc,
};

use anyhow::Context;
use common::{
    bootstrap_model::index::{
        vector_index::{
            DeveloperVectorIndexConfig,
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexState,
        },
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
    },
    bounded_thread_pool::BoundedThreadPool,
    document::ParsedDocument,
    knobs::DEFAULT_DOCUMENTS_PAGE_SIZE,
    persistence::TimestampRange,
    query::Order,
    runtime::{
        new_rate_limiter,
        Runtime,
    },
    sync::{
        Mutex,
        MutexGuard,
    },
    types::TabletIndexName,
};
use futures::{
    stream::FuturesUnordered,
    TryStreamExt,
};
use governor::Quota;
use itertools::Itertools;
use keybroker::Identity;
use search::fragmented_segment::MutableFragmentedSegmentMetadata;
use storage::Storage;
use sync_types::Timestamp;
use value::ResolvedDocumentId;
use vector::QdrantExternalId;

use crate::{
    index_workers::{
        search_flusher::IndexBuild,
        MultiSegmentBackfillResult,
    },
    metrics::vector::{
        finish_vector_index_merge_timer,
        vector_compaction_merge_commit_timer,
        vector_flush_merge_commit_timer,
        vector_writer_lock_wait_timer,
        VectorIndexMergeType,
        VectorWriterLockWaiter,
    },
    vector_index_worker::vector_meta::VectorSearchIndex,
    Database,
    IndexModel,
    SystemMetadataModel,
    Transaction,
};

/// Serializes writes to index metadata from the worker and reconciles any
/// conflicting writes that may have happened due to concurrent modifications in
/// the flusher and compactor.
#[derive(Clone)]
pub(crate) struct VectorMetadataWriter<RT: Runtime> {
    inner: Arc<Mutex<Inner<RT>>>,
}

impl<RT: Runtime> VectorMetadataWriter<RT> {
    pub(crate) fn new(runtime: RT, database: Database<RT>, storage: Arc<dyn Storage>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                runtime: runtime.clone(),
                database,
                storage,
                // Use small limits because we should only ever run one job at a time.
                thread_pool: BoundedThreadPool::new(runtime, 2, 1, "vector_writer"),
            })),
        }
    }

    /// Merge results from a compaction with up to N previous writes by the
    /// flusher.
    ///
    /// There are only two writers, the flusher and the compactor. Each run
    /// serially. So we know that the only possibility of contention is from
    /// the flusher because we're writing the result from the compactor.
    ///
    /// The race we're worried about is that the flusher may have written one or
    /// more deletes to to the set of segments we just compacted. We need to
    /// ensure those new deletes end up in our newly compacted segment. To
    /// do so, we'll read the document log from the snapshot timestamp
    /// in the index metadata when compaction started and the current snapshot
    /// timestamp. Each time we see a delete, we'll check to see if that
    /// document is in our new segment and if it is, we'll write the delete.
    pub(crate) async fn commit_compaction(
        &self,
        index_id: ResolvedDocumentId,
        index_name: TabletIndexName,
        start_compaction_ts: Timestamp,
        segments_to_compact: Vec<FragmentedVectorSegment>,
        new_segment: FragmentedVectorSegment,
        rate_limit_pages_per_second: NonZeroU32,
    ) -> anyhow::Result<()> {
        self.inner(VectorWriterLockWaiter::Compactor)
            .await
            .commit_compaction(
                index_id,
                index_name,
                start_compaction_ts,
                segments_to_compact,
                new_segment,
                rate_limit_pages_per_second,
            )
            .await
    }

    /// Merge results from a flush with up to N previous compactions by the
    /// compactor.
    ///
    /// There are only two writers, the flusher and the compactor. Each run
    /// serially. So we know that the only possibility of contention
    /// is from the compactor because we're writing the result from the flusher.
    ///
    /// The race we're worried about is that we may have just written one or
    /// more deletes to segments that were compacted while we were flushing. We
    /// need to ensure those deletes end up in the newly compacted segment. To
    /// do so, we'll read the document log from the current snapshot timestamp
    /// to the new snapshot time we're about to write. If we find any deletes in
    /// the document log, we'll try and write them to all current segments. Then
    /// we can append our new segment (if present) and write the updated result.
    pub(crate) async fn commit_flush(
        &self,
        job: &IndexBuild<VectorSearchIndex>,
        new_ts: Timestamp,
        new_and_modified_segments: Vec<FragmentedVectorSegment>,
        new_segment_id: Option<String>,
        index_backfill_result: Option<MultiSegmentBackfillResult>,
    ) -> anyhow::Result<()> {
        let inner = self.inner(VectorWriterLockWaiter::Flusher).await;

        if let Some(index_backfill_result) = index_backfill_result {
            inner
                .commit_backfill_flush(
                    job,
                    new_ts,
                    new_and_modified_segments,
                    new_segment_id,
                    index_backfill_result,
                )
                .await
        } else {
            inner
                .commit_snapshot_flush(job, new_ts, new_and_modified_segments, new_segment_id)
                .await
        }
    }

    async fn inner(&self, waiter: VectorWriterLockWaiter) -> MutexGuard<Inner<RT>> {
        let lock_timer = vector_writer_lock_wait_timer(waiter);
        let inner = self.inner.lock().await;
        drop(lock_timer);
        inner
    }
}

struct Inner<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    storage: Arc<dyn Storage>,
    thread_pool: BoundedThreadPool<RT>,
}

impl<RT: Runtime> Inner<RT> {
    async fn require_index_metadata(
        tx: &mut Transaction<RT>,
        index_id: ResolvedDocumentId,
    ) -> anyhow::Result<ParsedDocument<TabletIndexMetadata>> {
        let mut index_model = IndexModel::new(tx);
        index_model.require_index_by_id(index_id).await
    }

    fn compaction_vector_metadata(
        metadata: &mut ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(&Timestamp, &mut Vec<FragmentedVectorSegment>)> {
        let on_disk_state = match &mut metadata.config {
            IndexConfig::Database { .. } | IndexConfig::Search { .. } => {
                anyhow::bail!("Index type changed!");
            },
            IndexConfig::Vector {
                ref mut on_disk_state,
                ..
            } => on_disk_state,
        };
        let (segments, snapshot_ts) = match on_disk_state {
            VectorIndexState::Backfilling(VectorIndexBackfillState {
                segments,
                backfill_snapshot_ts,
                ..
            }) => (
                segments,
                backfill_snapshot_ts.as_ref().context(
                    "cannot compact backfilling index without a backfill snapshot set yet",
                )?,
            ),
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                let current_segments = match snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => {
                        anyhow::bail!("Index version changed!")
                    },
                    VectorIndexSnapshotData::MultiSegment(ref mut segments) => segments,
                };
                (current_segments, &snapshot.ts)
            },
        };
        Ok((snapshot_ts, segments))
    }

    fn is_compaction_merge_required(
        segments_to_compact: &Vec<FragmentedVectorSegment>,
        current_segments: &Vec<FragmentedVectorSegment>,
    ) -> anyhow::Result<bool> {
        for original_segment in segments_to_compact {
            let current_version = current_segments
                .iter()
                .find(|segment| segment.id == original_segment.id);
            let Some(current_version) = current_version else {
                // Only the compactor should remove segments, so they should never be removed
                // concurrently.
                anyhow::bail!("Segment unexpectedly removed!")
            };
            // For a given segment id, we can only ever increase the number of deletes. The
            // only way to decrease the number of deletes is by compaction,
            // which creates a new segment with a new id. So if the number of deletes has
            // changed, it's due to an increase from a conflicting write by the
            // flusher.
            if current_version.num_deleted != original_segment.num_deleted {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn commit_compaction(
        &self,
        index_id: ResolvedDocumentId,
        index_name: TabletIndexName,
        start_compaction_ts: Timestamp,
        segments_to_compact: Vec<FragmentedVectorSegment>,
        mut new_segment: FragmentedVectorSegment,
        rate_limit_pages_per_second: NonZeroU32,
    ) -> anyhow::Result<()> {
        let timer = vector_compaction_merge_commit_timer();
        let mut tx: Transaction<RT> = self.database.begin(Identity::system()).await?;
        let mut metadata = Self::require_index_metadata(&mut tx, index_id).await?;
        let (snapshot_ts, mut current_segments) = Self::compaction_vector_metadata(&mut metadata)?;

        let is_merge_required =
            Self::is_compaction_merge_required(&segments_to_compact, current_segments)?;
        if is_merge_required {
            // Drop and then restart the transaction, it could take a while to
            // merge deletes.
            drop(tx);
            let results = self
                .merge_deletes(
                    vec![new_segment],
                    start_compaction_ts,
                    *snapshot_ts,
                    index_name.clone(),
                    rate_limit_pages_per_second,
                )
                .await?;
            anyhow::ensure!(results.len() == 1);
            new_segment = results.into_iter().next().unwrap();
            tx = self.database.begin(Identity::system()).await?;
            metadata = Self::require_index_metadata(&mut tx, index_id).await?;
            (_, current_segments) = Self::compaction_vector_metadata(&mut metadata)?;
        }

        let removed_sement_ids = segments_to_compact
            .into_iter()
            .map(|segment| segment.id)
            .collect_vec();
        let new_segments = current_segments
            .iter()
            .filter(|segment| !removed_sement_ids.contains(&segment.id))
            .cloned()
            .chain(iter::once(new_segment))
            .collect_vec();
        *current_segments = new_segments;

        SystemMetadataModel::new_global(&mut tx)
            .replace(metadata.id(), metadata.into_value().try_into()?)
            .await?;
        self.database
            .commit_with_write_source(tx, "vector_index_worker_commit_compaction")
            .await?;
        finish_vector_index_merge_timer(
            timer,
            if is_merge_required {
                VectorIndexMergeType::Required
            } else {
                VectorIndexMergeType::NotRequired
            },
        );
        Ok(())
    }

    fn is_merge_flush_required(
        new_segments: &Vec<FragmentedVectorSegment>,
        current_segments: &Vec<FragmentedVectorSegment>,
        new_segment_id: &Option<String>,
    ) -> bool {
        // TODO(sam): We could be more efficient if we only counted new segments to
        // which our flush actually added at least one delete.
        let current_segment_ids = current_segments
            .iter()
            .map(|segment| &segment.id)
            .collect_vec();
        // If any of the segments other than the one the flush optionally added is
        // missing, then some conflicting compaction must have happened.
        // Compaction is the only way that segments can be removed.
        new_segments
            .iter()
            // Ignore the new segment id, if we created a new segment
            .filter(|segment| {
                new_segment_id
                    .as_ref()
                    .map(|new_segment_id| *new_segment_id != segment.id)
                    .unwrap_or(true)
            })
            // Check to see if any of our other new segments were removed while we flushed.
            .any(|segment| !current_segment_ids.contains(&&segment.id))
    }

    fn flush_vector_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(
        Option<Timestamp>,
        Vec<FragmentedVectorSegment>,
        bool,
        DeveloperVectorIndexConfig,
    )> {
        let (on_disk_state, developer_config) = match metadata.into_value().config {
            IndexConfig::Database { .. } | IndexConfig::Search { .. } => {
                anyhow::bail!("Index type changed!");
            },
            IndexConfig::Vector {
                on_disk_state,
                developer_config,
            } => (on_disk_state, developer_config),
        };

        let is_snapshotted = match on_disk_state {
            VectorIndexState::Backfilling(_) | VectorIndexState::Backfilled(_) => false,
            VectorIndexState::SnapshottedAt(_) => true,
        };
        let (snapshot_ts, current_segments) = match on_disk_state {
            VectorIndexState::Backfilling(state) => (state.backfill_snapshot_ts, state.segments),
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                let current_segments = match snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => {
                        // We might be migrating to the multi segment format, so we have to be
                        // lenient.
                        vec![]
                    },
                    VectorIndexSnapshotData::MultiSegment(segments) => segments,
                };

                (Some(snapshot.ts), current_segments)
            },
        };

        Ok((
            snapshot_ts,
            current_segments,
            is_snapshotted,
            developer_config,
        ))
    }

    async fn commit_backfill_flush(
        &self,
        job: &IndexBuild<VectorSearchIndex>,
        backfill_complete_ts: Timestamp,
        mut new_and_modified_segments: Vec<FragmentedVectorSegment>,
        new_segment_id: Option<String>,
        backfill_result: MultiSegmentBackfillResult,
    ) -> anyhow::Result<()> {
        let timer = vector_flush_merge_commit_timer();
        let mut tx: Transaction<RT> = self.database.begin(Identity::system()).await?;
        let metadata = Self::require_index_metadata(&mut tx, job.metadata_id).await?;

        // assert index metadata is in backfilling state
        anyhow::ensure!(matches!(
            metadata.config,
            IndexConfig::Vector {
                on_disk_state: VectorIndexState::Backfilling(_),
                ..
            }
        ));

        // Get current segments in database and developer config
        let (_, current_segments, _, developer_config) = Self::flush_vector_metadata(metadata)?;

        // Find new segment and add to current segments to avoid race with compactor
        let new_segment = new_segment_id
            .map(|new_segment_id| {
                new_and_modified_segments
                    .into_iter()
                    .find(|segment| segment.id == new_segment_id)
                    .context("Missing new segment in segments list!")
            })
            .transpose()?;
        new_and_modified_segments = current_segments
            .into_iter()
            .chain(new_segment.into_iter())
            .collect_vec();

        // Build disk state and commit
        let new_on_disk_state = if backfill_result.is_backfill_complete {
            VectorIndexState::Backfilled(VectorIndexSnapshot {
                data: VectorIndexSnapshotData::MultiSegment(new_and_modified_segments),
                ts: backfill_complete_ts,
            })
        } else {
            VectorIndexState::Backfilling(VectorIndexBackfillState {
                segments: new_and_modified_segments,
                cursor: backfill_result
                    .new_cursor
                    .map(|cursor| cursor.internal_id()),
                backfill_snapshot_ts: Some(backfill_complete_ts),
            })
        };

        SystemMetadataModel::new_global(&mut tx)
            .replace(
                job.metadata_id,
                IndexMetadata {
                    name: job.index_name.clone(),
                    config: IndexConfig::Vector {
                        on_disk_state: new_on_disk_state,
                        developer_config: developer_config.clone(),
                    },
                }
                .try_into()?,
            )
            .await?;
        self.database
            .commit_with_write_source(tx, "vector_index_woker_commit_backfill")
            .await?;
        finish_vector_index_merge_timer(timer, VectorIndexMergeType::NotRequired);
        Ok(())
    }

    async fn commit_snapshot_flush(
        &self,
        job: &IndexBuild<VectorSearchIndex>,
        new_ts: Timestamp,
        mut new_and_modified_segments: Vec<FragmentedVectorSegment>,
        new_segment_id: Option<String>,
    ) -> anyhow::Result<()> {
        let timer = vector_flush_merge_commit_timer();
        let mut tx: Transaction<RT> = self.database.begin(Identity::system()).await?;
        let metadata = Self::require_index_metadata(&mut tx, job.metadata_id).await?;

        let (start_snapshot_ts, current_segments, is_snapshotted, developer_config) =
            Self::flush_vector_metadata(metadata.clone())?;
        let is_merge_required = Self::is_merge_flush_required(
            &new_and_modified_segments,
            &current_segments,
            &new_segment_id,
        );
        if is_merge_required {
            // Drop and restart, merging could take a while.
            drop(tx);
            // If we were backfilling and had no timestamp, there should have been no
            // segments for compaction to compact. If we do have segments, then
            // we necesssarily must have a snapshot timestamp for when those
            // segments were valid. So it's an error if we think we need to
            // merge with a compaction but have no snapshot timestamp.
            let start_snapshot_ts =
                start_snapshot_ts.context("Compaction ran before index had a snapshot")?;
            let updated_segments = self
                .merge_deletes(
                    current_segments.clone(),
                    // We're assuming that nothing else can touch the snapshot other than flushes.
                    // Right now this works because the flusher is already serial and its
                    // the only thing that advances the the metadata timestamp. If that were
                    // ever not true, we'd need to pass through a timestamp rather than using the
                    // one in the current metadata.
                    start_snapshot_ts,
                    new_ts,
                    job.index_name.clone(),
                    job.build_reason.read_max_pages_per_second(),
                )
                .await?;
            // If we had a flush that involved only deletes, we may not have a new segment
            // so new_segment / new_segment_id will be None. However if we did
            // have a new segment id, we must find and append the matching
            // segment or something has gone wrong.
            let new_segment = new_segment_id
                .map(|new_segment_id| {
                    new_and_modified_segments
                        .into_iter()
                        .find(|segment| segment.id == new_segment_id)
                        .context("Missing new segment in segments list!")
                })
                .transpose()?;
            new_and_modified_segments = updated_segments
                .into_iter()
                .chain(new_segment.into_iter())
                .collect_vec();
            tx = self.database.begin(Identity::system()).await?;
        }

        // If index_backfill_result is not None, the flusher's build step made progress
        // on a backfilling index. If complete, the new on-disk state uses the
        // `backfill_completed_ts` set in the result, which matches
        // `backfill_snapshot_ts` set on first iteration of the incremental
        // index build.
        let snapshot = VectorIndexSnapshot {
            data: VectorIndexSnapshotData::MultiSegment(new_and_modified_segments),
            ts: new_ts,
        };
        let new_on_disk_state = if is_snapshotted {
            VectorIndexState::SnapshottedAt(snapshot)
        } else {
            VectorIndexState::Backfilled(snapshot)
        };

        SystemMetadataModel::new_global(&mut tx)
            .replace(
                job.metadata_id,
                IndexMetadata {
                    name: job.index_name.clone(),
                    config: IndexConfig::Vector {
                        on_disk_state: new_on_disk_state,
                        developer_config: developer_config.clone(),
                    },
                }
                .try_into()?,
            )
            .await?;
        self.database
            .commit_with_write_source(tx, "vector_index_worker_commit_snapshot")
            .await?;
        finish_vector_index_merge_timer(
            timer,
            if is_merge_required {
                VectorIndexMergeType::Required
            } else {
                VectorIndexMergeType::NotRequired
            },
        );
        Ok(())
    }

    async fn merge_deletes(
        &self,
        segments_to_update: Vec<FragmentedVectorSegment>,
        start_ts: Timestamp,
        current_ts: Timestamp,
        index_name: TabletIndexName,
        rate_limit_pages_per_second: NonZeroU32,
    ) -> anyhow::Result<Vec<FragmentedVectorSegment>> {
        let storage = self.storage.clone();
        let runtime = self.runtime.clone();
        let database = self.database.clone();
        self.thread_pool
            .execute_async(move || async move {
                Self::merge_deletes_on_thread(
                    runtime,
                    database,
                    segments_to_update,
                    start_ts,
                    current_ts,
                    index_name,
                    storage,
                    rate_limit_pages_per_second,
                )
                .await
            })
            .await?
    }

    async fn merge_deletes_on_thread(
        runtime: RT,
        database: Database<RT>,
        segments_to_update: Vec<FragmentedVectorSegment>,
        start_ts: Timestamp,
        current_ts: Timestamp,
        index_name: TabletIndexName,
        storage: Arc<dyn Storage>,
        rate_limit_pages_per_second: NonZeroU32,
    ) -> anyhow::Result<Vec<FragmentedVectorSegment>> {
        let row_rate_limiter = new_rate_limiter(
            runtime,
            Quota::per_second(
                NonZeroU32::new(*DEFAULT_DOCUMENTS_PAGE_SIZE)
                    .and_then(|val| val.checked_mul(rate_limit_pages_per_second))
                    .context("Invalid row rate limit")?,
            ),
        );
        let mut loaded_segments = segments_to_update
            .into_iter()
            .map(|segment| MutableFragmentedSegmentMetadata::download(segment, storage.clone()))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await?;
        let mut documents = database.load_documents_in_table(
            *index_name.table(),
            TimestampRange::new((Bound::Excluded(start_ts), Bound::Included(current_ts)))?,
            Order::Asc,
            &row_rate_limiter,
        );

        while let Some((_, id, document)) = documents.try_next().await? {
            if document.is_none() {
                let point_id = QdrantExternalId::try_from(&id)?;
                for segment in &mut loaded_segments {
                    segment.maybe_delete(*point_id)?;
                }
            }
        }

        loaded_segments
            .into_iter()
            .map(|segment| segment.upload_deleted_bitset(storage.clone()))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await
    }
}
