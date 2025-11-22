use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fmt::Display,
    num::NonZeroU32,
    sync::Arc,
    time::Duration,
};

use common::{
    self,
    bootstrap_model::index::database_index::IndexedFields,
    knobs::{
        INDEX_BACKFILL_CHUNK_RATE,
        INDEX_BACKFILL_CHUNK_SIZE,
        INDEX_BACKFILL_PROGRESS_INTERVAL,
        INDEX_BACKFILL_READ_SIZE,
        INDEX_BACKFILL_WORKERS,
    },
    persistence::{
        ConflictStrategy,
        LatestDocument,
        Persistence,
        PersistenceIndexEntry,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    persistence_helpers::{
        DocumentRevision,
        RevisionPair,
    },
    query::Order,
    runtime::{
        new_rate_limiter,
        try_join,
        RateLimiter,
        Runtime,
    },
    types::{
        DatabaseIndexUpdate,
        IndexId,
        RepeatableTimestamp,
        TabletIndexName,
        Timestamp,
    },
    value::TabletId,
};
use futures::{
    future::{
        self,
    },
    pin_mut,
    stream::{
        self,
    },
    Stream,
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use indexing::index_registry::IndexRegistry;
use maplit::btreeset;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use value::{
    InternalDocumentId,
    ResolvedDocumentId,
};

use crate::{
    retention::LeaderRetentionWorkers,
    TableIterator,
};

pub const PERFORM_BACKFILL_LABEL: &str = "perform_backfill";
pub const UPDATE_BACKFILL_PROGRESS_LABEL: &str = "update_backfill_progress";

#[derive(Clone)]
pub enum IndexSelector {
    All(IndexRegistry),
    ManyIndexes {
        tablet_id: TabletId,
        indexes: BTreeMap<IndexId, TabletIndexName>,
    },
}

impl Display for IndexSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All(_) => write!(f, "ALL"),
            Self::ManyIndexes { indexes, .. } => {
                write!(f, "ManyIndexes(")?;
                let mut first = true;
                for name in indexes.values() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{name}")?;
                }
                write!(f, ")")
            },
        }
    }
}

impl IndexSelector {
    fn filter_index_update(&self, index_update: &DatabaseIndexUpdate) -> bool {
        match self {
            Self::All(_) => true,
            Self::ManyIndexes { indexes, .. } => indexes.contains_key(&index_update.index_id),
        }
    }

    fn iterate_tables(&self) -> impl Iterator<Item = TabletId> + use<> {
        let tables = match self {
            Self::All(index_registry) => index_registry
                .all_tables_with_indexes()
                .into_iter()
                .collect(),
            Self::ManyIndexes { tablet_id, .. } => btreeset! { *tablet_id },
        };
        tables.into_iter()
    }

    fn index_ids(&self) -> impl Iterator<Item = IndexId> + use<> {
        let indexes: BTreeSet<_> = match self {
            Self::All(index_registry) => index_registry
                .all_database_indexes()
                .into_iter()
                .map(|doc| doc.id().internal_id())
                .collect(),
            Self::ManyIndexes { indexes, .. } => indexes.keys().copied().collect(),
        };
        indexes.into_iter()
    }

    fn tablet_id(&self) -> Option<TabletId> {
        match self {
            Self::All(_) => None,
            Self::ManyIndexes { tablet_id, .. } => Some(*tablet_id),
        }
    }
}

#[derive(Clone)]
pub struct IndexWriter<RT: Runtime> {
    // Persistence target for writing indexes.
    persistence: Arc<dyn Persistence>,
    // Reader must have by_id index fully populated.
    reader: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    rate_limiter: Arc<RateLimiter<RT>>,
    runtime: RT,
    progress_tx: Option<mpsc::Sender<TabletBackfillProgress>>,
}

pub struct TabletBackfillProgress {
    pub tablet_id: TabletId,
    pub index_ids: Vec<IndexId>,
    pub cursor: InternalDocumentId,
    pub num_docs_indexed: u64,
}

impl<RT: Runtime> IndexWriter<RT> {
    pub fn new(
        persistence: Arc<dyn Persistence>,
        reader: Arc<dyn PersistenceReader>,
        retention_validator: Arc<dyn RetentionValidator>,
        runtime: RT,
        progress_tx: Option<mpsc::Sender<TabletBackfillProgress>>,
    ) -> Self {
        let entries_per_second =
            INDEX_BACKFILL_CHUNK_RATE.saturating_mul(*INDEX_BACKFILL_CHUNK_SIZE);
        debug_assert!(
            entries_per_second >= *INDEX_BACKFILL_CHUNK_SIZE,
            "Entries per second must be at least {}",
            *INDEX_BACKFILL_CHUNK_SIZE
        );
        Self {
            persistence,
            reader,
            retention_validator,
            rate_limiter: Arc::new(new_rate_limiter(
                runtime.clone(),
                Quota::per_second(entries_per_second),
            )),
            runtime,
            progress_tx,
        }
    }

    /// Backfill indexes based on a snapshot at the current time.  After the
    /// current snapshot is backfilled, index snapshot reads at >=ts are
    /// valid.
    ///
    /// The goal of this backfill is to make snapshot reads of `index_name`
    /// valid at or after snapshot_ts.
    /// To support:
    /// 1. Latest documents as of snapshot_ts.
    /// 2. Document changes after `snapshot_ts`. These are handled by active
    ///    writes, assuming `snapshot_ts` is after the index was created. If
    ///    there are no active writes, then `backfill_forwards` must be called
    ///    with a timestamp <= `snapshot_ts`.
    ///
    /// Takes a an optional database to update progress on the index backfill
    pub async fn backfill_from_ts(
        &self,
        snapshot_ts: RepeatableTimestamp,
        index_metadata: &IndexRegistry,
        index_selector: IndexSelector,
        concurrency: usize,
        cursor: Option<ResolvedDocumentId>,
    ) -> anyhow::Result<u64> {
        let pause_client = self.runtime.pause_client();
        pause_client.wait(PERFORM_BACKFILL_LABEL).await;
        let results: Vec<u64> = stream::iter(index_selector.iterate_tables())
            .map(|tablet_id| {
                let index_metadata = index_metadata.clone();
                let index_selector = index_selector.clone();
                let self_ = (*self).clone();
                async move {
                    try_join("index_backfill_table_snapshot", async move {
                        self_
                            .backfill_exact_snapshot_of_table(
                                snapshot_ts,
                                &index_selector,
                                &index_metadata,
                                tablet_id,
                                cursor,
                            )
                            .await
                    })
                    .await
                }
            })
            .buffer_unordered(concurrency)
            .try_collect()
            .await?;
        let docs_indexed: u64 = results.iter().sum();
        Ok(docs_indexed)
    }

    /// Backfills exactly the index entries necessary to represent documents
    /// which were latest at `snapshot`. In particular it does not create any
    /// tombstone index entries. And it only does snapshot reads (of `by_id`) at
    /// `snapshot`, which should remain a valid snapshot for the duration of
    /// walking the index.
    ///
    /// After this function returns, as long as new index entries are written
    /// for document revisions after `snapshot`, then you are allowed to read
    /// `index_name` at any snapshot after `snapshot`.
    async fn backfill_exact_snapshot_of_table(
        &self,
        snapshot_ts: RepeatableTimestamp,
        index_selector: &IndexSelector,
        index_registry: &IndexRegistry,
        tablet_id: TabletId,
        cursor: Option<ResolvedDocumentId>,
    ) -> anyhow::Result<u64> {
        let table_iterator = TableIterator::new(
            self.runtime.clone(),
            snapshot_ts,
            self.reader.clone(),
            self.retention_validator.clone(),
            *INDEX_BACKFILL_READ_SIZE,
        );

        let (index_update_tx, index_update_rx) = mpsc::channel(32);
        // Convert document stream into revision pairs, ignoring previous revisions
        // because we are backfilling at exactly snapshot_ts
        let producer = async {
            let by_id = index_registry.must_get_by_id(tablet_id)?.id();
            let mut stream =
                std::pin::pin!(table_iterator.stream_documents_in_table(tablet_id, by_id, cursor));
            let mut docs_sent = 0;
            while let Some(item) = stream.try_next().await? {
                let LatestDocument {
                    ts,
                    value: document,
                    ..
                } = item;
                docs_sent += 1;
                _ = index_update_tx
                    .send(RevisionPair {
                        id: document.id().into(),
                        rev: DocumentRevision {
                            ts,
                            document: Some(document),
                        },
                        prev_rev: None,
                    })
                    .await;
            }
            drop(index_update_tx);
            Ok(docs_sent)
        };

        let consumer = self.write_index_entries(
            format!("for table {tablet_id} at snapshot {snapshot_ts}"),
            index_registry,
            ReceiverStream::new(index_update_rx),
            index_selector,
        );
        let (docs_indexed, _) = future::try_join(producer, consumer).await?;
        Ok(docs_indexed)
    }

    /// Backfill indexes forward for a range of the documents log.
    ///
    /// Arguments:
    /// - `start_ts`: Inclusive lower bound for scanning the documents log.
    /// - `end_ts`: Inclusive upper bound for scanning the documents log.
    /// - `index_registry`: Index registry for backfill, determined externally
    ///   from this backfill. Note that since we're not building up a historical
    ///   view based on the `_index` table, we may be backfilling indexes that
    ///   did not exist at the historical timestamp.
    /// - `index_selector`: Subset of `index_registry` to backfill.
    ///
    /// Preconditions:
    /// - The selected indexes are fully backfilled for all revisions at
    ///   `start_ts`.
    ///
    /// Postconditions:
    /// - The selected indexes will be fully backfilled up to `end_ts`, and they
    ///   will be valid for all timestamps less than or equal to `end_ts`.
    pub async fn backfill_forwards(
        &self,
        start_ts: Timestamp,
        end_ts: RepeatableTimestamp,
        index_registry: &IndexRegistry,
        index_selector: &IndexSelector,
    ) -> anyhow::Result<()> {
        let repeatable_persistence = RepeatablePersistence::new(
            self.reader.clone(),
            end_ts,
            self.retention_validator.clone(),
        );
        let (tx, rx) = mpsc::channel(32);
        let producer = async {
            let revision_stream = repeatable_persistence.load_revision_pairs(
                index_selector.tablet_id(),
                TimestampRange::new(start_ts..=*end_ts),
                Order::Asc,
            );
            futures::pin_mut!(revision_stream);
            while let Some(revision_pair) = revision_stream.try_next().await? {
                tx.send(revision_pair).await?;
            }
            drop(tx);
            Ok(())
        };
        let consumer = self.write_index_entries(
            format!("going forward from {start_ts} to {end_ts}"),
            index_registry,
            ReceiverStream::new(rx),
            index_selector,
        );

        // Consider ourselves successful if both the producer and consumer exit
        // successfully.
        let ((), ()) = futures::try_join!(producer, consumer)?;
        Ok(())
    }

    /// Backfill indexes backwards through the documents log, stopping early if
    /// we hit the retention window's minimum snapshot timestamp.
    ///
    /// Arguments:
    /// - `start_ts`: Non-inclusive upper bound for scanning the documents log.
    /// - `end_ts`: Inclusive lower bound for scanning the documents log. Note
    ///   that we may not reach this timestamp if we stop early due to hitting
    ///   end of the retention window.
    /// - `index_registry` Index metadata to backfill.
    /// - `index_selector`: Subset of `index_registry` to backfill.
    ///
    /// Returns:
    /// - The minimum log revision we successfully processed.
    ///
    /// Preconditions:
    /// - The selected indexes are fully backfilled at `start_ts`.
    /// - `start_ts > end_ts`.
    ///
    /// Postconditions:
    /// - The selected indexes will be fully backfilled for all revisions `ts`
    ///   where `end_ts <= ts <= start_ts`.
    pub async fn backfill_backwards(
        &self,
        start_ts: RepeatableTimestamp,
        end_ts: Timestamp,
        index_registry: &IndexRegistry,
        index_selector: &IndexSelector,
    ) -> anyhow::Result<RepeatableTimestamp> {
        anyhow::ensure!(*start_ts > end_ts);
        let (tx, rx) = mpsc::channel(32);
        let repeatable_persistence = RepeatablePersistence::new(
            self.reader.clone(),
            start_ts,
            self.retention_validator.clone(),
        );
        let producer = async {
            let revision_stream = repeatable_persistence.load_revision_pairs(
                index_selector.tablet_id(),
                TimestampRange::new(end_ts..*start_ts),
                Order::Desc,
            );
            futures::pin_mut!(revision_stream);
            while let Some(revision_pair) = revision_stream.try_next().await? {
                let ts = revision_pair.ts();
                if ts < *self.retention_validator.min_snapshot_ts().await? {
                    // We may not have fully processed the entirety of the transaction at
                    // `min_chunk_ts` (since we paginate by `(ts, id)`), so only consider
                    // ourselves backfilled up to the subsequent timestamp.
                    return ts.succ();
                }

                let prev_doc_and_ts = if let Some(ref prev_rev) = revision_pair.prev_rev {
                    if let Some(prev_doc) = prev_rev.document.clone() {
                        Some((prev_doc, prev_rev.ts))
                    } else {
                        None
                    }
                } else {
                    None
                };
                tx.send(revision_pair).await?;

                // Let's say we're backfilling backwards and processing a revision for `id`
                // at `ts`:
                //
                //                  end_ts          |<------start_ts
                // timestamps: --------|------------------------|----->
                // id:            o                 o
                //                ^ prev_ts         ^ ts
                //
                // Processing the log entry for `ts` will generate at most two index entries:
                // one for deleting `prev_ts`'s value from the index and one for inserting
                // `ts`'s value.
                //
                // However, since we're backfilling backwards, we need to inductively guarantee
                // that all timestamps past our current timestamp are valid for the index. If
                // we just wrote our two entries, a historical read between `prev_ts` and `ts`
                // wouldn't see the add for `prev_ts`'s entry. Therefore, we need to write
                // three entries for `ts`: its add, `prev_rev`'s delete, and `prev_ts`'s add.
                //
                // This does mean that we'll potentially write `prev_rev`'s add again when we
                // process `prev_rev`'s log entry, but setting `ConflictStrategy::Overwrite` and
                // deduplicating using `BTreeSet` in `write_index_entries`
                // in `Persistence::write` makes this a no-op.
                if let Some((prev_doc, prev_ts)) = prev_doc_and_ts {
                    tx.send(RevisionPair {
                        id: prev_doc.id().into(),
                        rev: DocumentRevision {
                            ts: prev_ts,
                            document: Some(prev_doc),
                        },
                        prev_rev: None,
                    })
                    .await?;
                }
            }
            drop(tx);
            Ok(end_ts)
        };

        let consumer = self.write_index_entries(
            format!("going backward from {start_ts} to {end_ts}"),
            index_registry,
            ReceiverStream::new(rx),
            index_selector,
        );

        // Consider ourselves successful if both the reader and writer exit
        // successfully.
        let (backfilled_ts, ()) = futures::try_join!(producer, consumer)?;
        start_ts.prior_ts(backfilled_ts)
    }

    async fn write_index_entries(
        &self,
        phase: String,
        index_registry: &IndexRegistry,
        revision_pairs: impl Stream<Item = RevisionPair>,
        index_selector: &IndexSelector,
    ) -> anyhow::Result<()> {
        let mut last_logged = self.runtime.system_time();
        let mut last_checkpointed = self.runtime.system_time();
        let mut last_logged_entries_written = 0;
        let mut num_entries_written = 0;
        let should_send_progress = self.progress_tx.is_some();
        let approx_num_indexes = match index_selector {
            // We choose an arbitrary number of indexes because the revision stream can include
            // pairs from different tables which may have different numbers of indexes.
            IndexSelector::All(_) => 8,
            IndexSelector::ManyIndexes { indexes, .. } => indexes.len(),
        };

        let updates = revision_pairs
            .chunks((INDEX_BACKFILL_CHUNK_SIZE.get() as usize).div_ceil(approx_num_indexes))
            .map(|chunk| async move {
                let persistence = self.persistence.clone();
                let rate_limiter = self.rate_limiter.clone();
                // ID of last document written in this chunk
                let cursor = should_send_progress
                    .then(|| chunk.last().map(|revision_pair| revision_pair.id))
                    .flatten();
                let index_updates: Vec<PersistenceIndexEntry> = chunk
                    .iter()
                    .flat_map(|revision_pair| {
                        index_registry
                            .index_updates(revision_pair.prev_document(), revision_pair.document())
                            .into_iter()
                            .filter(|update| index_selector.filter_index_update(update))
                            .map(|update| {
                                PersistenceIndexEntry::from_index_update(
                                    revision_pair.ts(),
                                    &update,
                                )
                            })
                    })
                    .collect();
                let size = u32::try_from(index_updates.len())?;
                // N.B: it's possible to end up with no entries if we're
                // backfilling forward through historical documents that have no
                // present indexes in `index_registry`.
                if let Some(size) = NonZeroU32::new(size) {
                    while let Err(not_until) = rate_limiter
                        .check_n(size)
                        .expect("RateLimiter capacity impossibly small")
                    {
                        let delay = not_until.wait_time_from(self.runtime.monotonic_now().into());
                        self.runtime.wait(delay).await;
                    }
                    persistence
                        .write(&[], &index_updates, ConflictStrategy::Overwrite)
                        .await?;
                }
                anyhow::Ok((u64::from(size), cursor))
            })
            .buffered(*INDEX_BACKFILL_WORKERS);
        pin_mut!(updates);

        let mut num_docs_indexed = 0;
        while let Some(result) = updates.next().await {
            let (entries_written, cursor) = result?;
            num_docs_indexed += entries_written;
            num_entries_written += entries_written;
            if let Some(tx) = self.progress_tx.as_ref()
                && last_checkpointed.elapsed()? >= *INDEX_BACKFILL_PROGRESS_INTERVAL
                && let Some(cursor) = cursor
            {
                tx.send(TabletBackfillProgress {
                    tablet_id: cursor.table(),
                    index_ids: index_selector.index_ids().collect(),
                    cursor,
                    num_docs_indexed,
                })
                .await?;
                num_docs_indexed = 0;
                last_checkpointed = self.runtime.system_time();
                self.runtime
                    .pause_client()
                    .wait(UPDATE_BACKFILL_PROGRESS_LABEL)
                    .await;
            }
            if last_logged.elapsed()? >= Duration::from_secs(60) {
                let now = self.runtime.system_time();
                tracing::info!(
                    "Backfilled {num_entries_written} rows of index {index_selector} {phase} ({} \
                     rows/s)",
                    (num_entries_written - last_logged_entries_written) as f64
                        / (now.duration_since(last_logged).unwrap_or_default()).as_secs_f64(),
                );
                last_logged = now;
                last_logged_entries_written = num_entries_written;
            }
        }

        tracing::info!(
            "Done backfilling {num_entries_written} rows of index {index_selector} {phase}",
        );
        Ok(())
    }

    pub async fn run_retention(
        &self,
        backfill_begin_ts: RepeatableTimestamp,
        all_indexes: BTreeMap<IndexId, (TabletIndexName, IndexedFields)>,
    ) -> anyhow::Result<()> {
        let min_snapshot_ts = self.retention_validator.min_snapshot_ts().await?;
        // TODO(lee) add checkpointing.
        LeaderRetentionWorkers::delete_all_no_checkpoint(
            backfill_begin_ts,
            min_snapshot_ts,
            self.persistence.clone(),
            &all_indexes,
            self.retention_validator.clone(),
        )
        .await?;
        Ok(())
    }
}
