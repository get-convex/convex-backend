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
    errors::is_transient_db_error,
    fmt::format_read_write_balance,
    knobs::{
        INDEX_BACKFILL_CHUNK_RATE,
        INDEX_BACKFILL_CHUNK_SIZE,
        INDEX_BACKFILL_PROGRESS_INTERVAL,
        INDEX_BACKFILL_READ_SIZE,
        INDEX_BACKFILL_WORKERS,
    },
    persistence::{
        ConflictStrategy,
        DocumentLogEntry,
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
    read_write_balance::{
        ReadWriteBalance,
        ReadWriteReporter,
    },
    retry::{
        retry_with_backoff,
        RetryConfig,
    },
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
                .map(|doc| doc.id().internal_id().into())
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

/// What an `IndexWriter` writes per chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexWriterMode {
    /// Default: write only index entries to the destination.
    IndexesOnly,
    /// Also write the document log entries alongside the index entries.
    IndexesAndDocuments,
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
    mode: IndexWriterMode,
    // Optional sink for the read-vs-write timing measured during backfill, so a
    // caller with extra labels (e.g. a db-cluster migration) can emit a metric.
    read_write_reporter: Option<ReadWriteReporter>,
}

pub struct TabletBackfillProgress {
    pub tablet_id: TabletId,
    pub index_ids: Vec<IndexId>,
    pub cursor: InternalDocumentId,
    pub num_docs_indexed: u64,
    pub backfill_bytes_read: u64,
    pub backfill_bytes_written: u64,
}

impl<RT: Runtime> IndexWriter<RT> {
    pub fn new(
        persistence: Arc<dyn Persistence>,
        reader: Arc<dyn PersistenceReader>,
        retention_validator: Arc<dyn RetentionValidator>,
        runtime: RT,
        progress_tx: Option<mpsc::Sender<TabletBackfillProgress>>,
        mode: IndexWriterMode,
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
            mode,
            read_write_reporter: None,
        }
    }

    /// Attach a sink that receives the read- and write-side durations of each
    /// backfill reporting window, so callers can report read- vs write-bound.
    pub fn with_read_write_reporter(mut self, reporter: ReadWriteReporter) -> Self {
        self.read_write_reporter = Some(reporter);
        self
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
        retry_config: Option<RetryConfig>,
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
                                retry_config,
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
        retry_config: Option<RetryConfig>,
    ) -> anyhow::Result<u64> {
        let table_iterator = TableIterator::new(
            self.runtime.clone(),
            snapshot_ts,
            self.reader.clone(),
            self.retention_validator.clone(),
            *INDEX_BACKFILL_READ_SIZE,
        );

        let (index_update_tx, index_update_rx) = mpsc::channel(32);
        let balance = ReadWriteBalance::new();
        let producer = async {
            let by_id = index_registry.must_get_by_id(tablet_id)?.id();
            let mut stream =
                std::pin::pin!(table_iterator.stream_documents_in_table(tablet_id, by_id, cursor));
            let mut docs_sent = 0;
            loop {
                let read_start = self.runtime.monotonic_now();
                let next = stream.try_next().await?;
                balance.record_read(self.runtime.monotonic_now() - read_start);
                let Some(item) = next else {
                    break;
                };
                let LatestDocument {
                    ts,
                    value: document,
                    prev_ts,
                } = item;
                docs_sent += 1;
                let write_start = self.runtime.monotonic_now();
                _ = index_update_tx
                    .send(RevisionPair {
                        id: document.id().into(),
                        rev: DocumentRevision {
                            ts,
                            document: Some(document),
                        },
                        // include the prev_ts so we can write it later in IndexesAndDocuments mode
                        prev_rev: prev_ts.map(|ts| DocumentRevision { ts, document: None }),
                    })
                    .await;
                balance.record_write(self.runtime.monotonic_now() - write_start);
            }
            drop(index_update_tx);
            Ok(docs_sent)
        };

        let consumer = self.write_index_entries(
            format!("for table {tablet_id} at snapshot {snapshot_ts}"),
            index_registry,
            ReceiverStream::new(index_update_rx),
            index_selector,
            retry_config,
            &balance,
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
        retry_config: Option<RetryConfig>,
    ) -> anyhow::Result<()> {
        let repeatable_persistence = RepeatablePersistence::new(
            self.reader.clone(),
            end_ts,
            self.retention_validator.clone(),
        );
        let (tx, rx) = mpsc::channel(32);
        let balance = ReadWriteBalance::new();
        let producer = async {
            let revision_stream = repeatable_persistence.load_revision_pairs(
                index_selector.tablet_id(),
                TimestampRange::new(start_ts..=*end_ts),
                Order::Asc,
            );
            futures::pin_mut!(revision_stream);
            loop {
                let read_start = self.runtime.monotonic_now();
                let next = revision_stream.try_next().await?;
                balance.record_read(self.runtime.monotonic_now() - read_start);
                let Some(revision_pair) = next else {
                    break;
                };
                let write_start = self.runtime.monotonic_now();
                tx.send(revision_pair).await?;
                balance.record_write(self.runtime.monotonic_now() - write_start);
            }
            drop(tx);
            Ok(())
        };
        let consumer = self.write_index_entries(
            format!("going forward from {start_ts} to {end_ts}"),
            index_registry,
            ReceiverStream::new(rx),
            index_selector,
            retry_config,
            &balance,
        );

        // Consider ourselves successful if both the producer and consumer exit
        // successfully.
        let ((), ()) = futures::try_join!(producer, consumer)?;
        Ok(())
    }

    /// Chunk writes use `ConflictStrategy::Overwrite`, so re-applying a chunk
    /// after a transient db error is safe for both the index entries and the
    /// (optional) document log entries.
    async fn write_chunk_with_optional_retry(
        &self,
        persistence: &Arc<dyn Persistence>,
        documents: &[DocumentLogEntry],
        index_updates: &[PersistenceIndexEntry],
        retry_config: Option<RetryConfig>,
    ) -> anyhow::Result<()> {
        let write = || persistence.write(documents, index_updates, ConflictStrategy::Overwrite);
        match retry_config {
            None => write().await,
            Some(retry) => {
                retry_with_backoff("index_chunk_write", retry, is_transient_db_error, write).await
            },
        }
    }

    async fn write_index_entries(
        &self,
        phase: String,
        index_registry: &IndexRegistry,
        revision_pairs: impl Stream<Item = RevisionPair>,
        index_selector: &IndexSelector,
        retry_config: Option<RetryConfig>,
        read_write_balance: &ReadWriteBalance,
    ) -> anyhow::Result<()> {
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
                let mut index_updates = Vec::new();
                let mut bytes_written: u64 = 0;
                let mut bytes_read: u64 = 0;
                for revision_pair in chunk.iter() {
                    let doc_size = revision_pair.document().map_or(0, |d| d.size() as u64);
                    let prev_doc_size =
                        revision_pair.prev_document().map_or(0, |d| d.size() as u64);
                    bytes_read += doc_size + prev_doc_size;
                    for update in index_registry
                        .index_updates(revision_pair.prev_document(), revision_pair.document())
                        .into_iter()
                        .filter(|update| index_selector.filter_index_update(update))
                    {
                        bytes_written += update.key.size() as u64;
                        index_updates.push(PersistenceIndexEntry::from_index_update(
                            revision_pair.ts(),
                            &update,
                        ));
                    }
                }
                let docs_in_chunk = chunk.len() as u64;
                let documents: Vec<DocumentLogEntry> = match self.mode {
                    IndexWriterMode::IndexesAndDocuments => {
                        chunk.into_iter().map(|rp| rp.into_log_entry()).collect()
                    },
                    IndexWriterMode::IndexesOnly => Vec::new(),
                };
                let num_entries_written = u32::try_from(index_updates.len() + documents.len())?;
                // N.B: it's possible to end up with no entries if we're
                // backfilling forward through historical documents that have no
                // present indexes in `index_registry`.
                if !index_updates.is_empty() || !documents.is_empty() {
                    if let Some(num_entries_written) = NonZeroU32::new(num_entries_written) {
                        let throttle_start = self.runtime.monotonic_now();
                        while let Err(not_until) = rate_limiter
                            .check_n(num_entries_written)
                            .expect("RateLimiter capacity impossibly small")
                        {
                            let delay =
                                not_until.wait_time_from(self.runtime.monotonic_now().into());
                            self.runtime.wait(delay).await;
                        }
                        read_write_balance
                            .record_throttle(self.runtime.monotonic_now() - throttle_start);
                    }
                    self.write_chunk_with_optional_retry(
                        &persistence,
                        &documents,
                        &index_updates,
                        retry_config,
                    )
                    .await?;
                }
                anyhow::Ok((docs_in_chunk, cursor, bytes_read, bytes_written))
            })
            .buffered(*INDEX_BACKFILL_WORKERS);
        pin_mut!(updates);

        let mut last_logged = self.runtime.system_time();
        let mut last_checkpointed = self.runtime.system_time();
        let mut last_logged_docs_indexed = 0;
        let mut last_logged_balance = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        let mut num_docs_indexed_total = 0;
        let mut num_docs_indexed_since_progress_reported = 0;
        let mut backfill_bytes_read = 0;
        let mut backfill_bytes_written = 0u64;
        let mut last_cursor = None;
        while let Some(result) = updates.next().await {
            let (docs_in_chunk, cursor, bytes_read, bytes_written) = result?;
            if cursor.is_some() {
                last_cursor = cursor;
            }
            num_docs_indexed_since_progress_reported += docs_in_chunk;
            num_docs_indexed_total += docs_in_chunk;
            backfill_bytes_written += bytes_written;
            backfill_bytes_read += bytes_read;
            if let Some(tx) = self.progress_tx.as_ref()
                && last_checkpointed.elapsed()? >= *INDEX_BACKFILL_PROGRESS_INTERVAL
                && let Some(cursor) = cursor
            {
                tx.send(TabletBackfillProgress {
                    tablet_id: cursor.table(),
                    index_ids: index_selector.index_ids().collect(),
                    cursor,
                    num_docs_indexed: num_docs_indexed_since_progress_reported,
                    backfill_bytes_read,
                    backfill_bytes_written,
                })
                .await?;
                num_docs_indexed_since_progress_reported = 0;
                backfill_bytes_written = 0;
                backfill_bytes_read = 0;
                last_checkpointed = self.runtime.system_time();
                self.runtime
                    .pause_client()
                    .wait(UPDATE_BACKFILL_PROGRESS_LABEL)
                    .await;
            }
            if last_logged.elapsed()? >= Duration::from_secs(60) {
                let now = self.runtime.system_time();
                let (read_total, write_total, throttle_total) = read_write_balance.totals();
                let read_delta = read_total - last_logged_balance.0;
                let write_delta = write_total - last_logged_balance.1;
                let throttle_delta = throttle_total - last_logged_balance.2;
                tracing::info!(
                    "Backfilled {num_docs_indexed_total} docs into indexes: {index_selector} \
                     {phase} ({} rows/s) [{}]",
                    (num_docs_indexed_total - last_logged_docs_indexed) as f64
                        / (now.duration_since(last_logged).unwrap_or_default()).as_secs_f64(),
                    format_read_write_balance(read_delta, write_delta, throttle_delta),
                );
                if let Some(reporter) = self.read_write_reporter.as_ref() {
                    reporter(read_delta, write_delta, throttle_delta);
                }
                last_logged = now;
                last_logged_docs_indexed = num_docs_indexed_total;
                last_logged_balance = (read_total, write_total, throttle_total);
            }
        }
        // Flush any remaining accumulated bytes that weren't sent during the loop
        if (backfill_bytes_read > 0 || backfill_bytes_written > 0)
            && let Some(tx) = self.progress_tx.as_ref()
            && let Some(cursor) = last_cursor
        {
            tx.send(TabletBackfillProgress {
                tablet_id: cursor.table(),
                index_ids: index_selector.index_ids().collect(),
                cursor,
                num_docs_indexed: num_docs_indexed_since_progress_reported,
                backfill_bytes_read,
                backfill_bytes_written,
            })
            .await?;
        }

        // Flush read/write time accumulated since the last periodic log so the
        // metric captures the full phase, including a final partial window.
        if let Some(reporter) = self.read_write_reporter.as_ref() {
            let (read_total, write_total, throttle_total) = read_write_balance.totals();
            reporter(
                read_total - last_logged_balance.0,
                write_total - last_logged_balance.1,
                throttle_total - last_logged_balance.2,
            );
        }

        tracing::info!(
            "Done backfilling {num_docs_indexed_total} docs into indexes: {index_selector} {phase}",
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
