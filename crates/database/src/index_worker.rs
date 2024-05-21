use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fmt::Display,
    num::NonZeroU32,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use common::{
    backoff::Backoff,
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexState,
            IndexedFields,
        },
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
        INDEX_TABLE,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    errors::report_error,
    knobs::{
        ENABLE_INDEX_BACKFILL,
        INDEX_BACKFILL_CHUNK_RATE,
        INDEX_BACKFILL_CHUNK_SIZE,
        INDEX_WORKERS_INITIAL_BACKOFF,
    },
    persistence::{
        ConflictStrategy,
        Persistence,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    persistence_helpers::{
        stream_revision_pairs,
        RevisionPair,
    },
    query::{
        IndexRange,
        Order,
        Query,
    },
    runtime::{
        new_rate_limiter,
        RateLimiter,
        Runtime,
        RuntimeInstant,
        SpawnHandle,
    },
    types::{
        DatabaseIndexUpdate,
        IndexId,
        IndexName,
        PersistenceVersion,
        RepeatableTimestamp,
        TabletIndexName,
        Timestamp,
    },
    value::{
        ResolvedDocumentId,
        TableMapping,
        TabletId,
    },
};
use futures::{
    channel::mpsc,
    future,
    pin_mut,
    stream::FusedStream,
    Future,
    SinkExt,
    Stream,
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use indexing::index_registry::IndexRegistry;
use keybroker::Identity;
use maplit::{
    btreemap,
    btreeset,
};
use tracing::log;
use value::{
    InternalDocumentId,
    TableNamespace,
};

use crate::{
    metrics::{
        log_index_backfilled,
        log_num_indexes_to_backfill,
        log_worker_starting,
    },
    retention::LeaderRetentionManager,
    Database,
    ResolvedQuery,
    SystemMetadataModel,
    TableIterator,
};

const MAX_BACKOFF: Duration = Duration::from_secs(30);

static ENTRIES_PER_SECOND: LazyLock<NonZeroU32> = LazyLock::new(|| {
    NonZeroU32::new(
        (*INDEX_BACKFILL_CHUNK_RATE * *INDEX_BACKFILL_CHUNK_SIZE)
            .try_into()
            .unwrap(),
    )
    .unwrap()
});

pub struct IndexWorker<RT: Runtime> {
    database: Database<RT>,
    index_writer: IndexWriter<RT>,
    runtime: RT,
    backoff: Backoff,
    #[cfg(any(test, feature = "testing"))]
    should_terminate: bool,
    persistence_version: PersistenceVersion,
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
}

#[derive(Clone)]
pub enum IndexSelector {
    All(IndexRegistry),
    Index { name: TabletIndexName, id: IndexId },
}

impl Display for IndexSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All(_) => write!(f, "ALL"),
            Self::Index { name, .. } => write!(f, "{}", name),
        }
    }
}

impl IndexSelector {
    fn filter_index_update(&self, index_update: &DatabaseIndexUpdate) -> bool {
        match self {
            Self::All(_) => true,
            Self::Index { id, .. } => id == &index_update.index_id,
        }
    }

    fn iterate_tables(&self) -> impl Iterator<Item = TabletId> {
        match self {
            Self::All(index_registry) => index_registry
                .all_tables_with_indexes()
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter(),
            Self::Index { name, .. } => btreeset!(*name.table()).into_iter(),
        }
    }

    fn filter_id(&self, id: InternalDocumentId) -> bool {
        match self {
            Self::All(_) => true,
            Self::Index { name, .. } => name.table() == id.table(),
        }
    }
}

impl<RT: Runtime> IndexWorker<RT> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        runtime: RT,
        persistence: Arc<dyn Persistence>,
        retention_validator: Arc<dyn RetentionValidator>,
        database: Database<RT>,
    ) -> impl Future<Output = ()> + Send {
        let reader = persistence.reader();
        let persistence_version = reader.version();
        let mut worker = IndexWorker {
            database,
            runtime: runtime.clone(),
            backoff: Backoff::new(*INDEX_WORKERS_INITIAL_BACKOFF, MAX_BACKOFF),
            index_writer: IndexWriter::new(persistence, reader, retention_validator, runtime),
            #[cfg(any(test, feature = "testing"))]
            should_terminate: false,
            persistence_version,
        };
        async move {
            if !*ENABLE_INDEX_BACKFILL {
                log::error!("Forcibly disabling index backfill, exiting IndexWorker...");
                return;
            }
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("IndexWorker died"));
                    let delay = worker.runtime.with_rng(|rng| worker.backoff.fail(rng));
                    log::error!(
                        "IndexWorker died, num_failures: {}. Backing off for {}ms",
                        worker.backoff.failures(),
                        delay.as_millis()
                    );
                    worker.runtime.wait(delay).await;
                }
            }
        }
    }

    /// Test-only variant that terminates when there are no more indexes to
    /// backfill.
    #[cfg(any(test, feature = "testing"))]
    pub fn new_terminating(
        runtime: RT,
        persistence: Arc<dyn Persistence>,
        retention_validator: Arc<dyn RetentionValidator>,
        database: Database<RT>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        use errors::ErrorMetadataAnyhowExt;
        use futures::FutureExt;

        let reader = persistence.reader();
        let persistence_version = reader.version();
        let mut worker = IndexWorker {
            database,
            backoff: Backoff::new(*INDEX_WORKERS_INITIAL_BACKOFF, MAX_BACKOFF),
            runtime: runtime.clone(),
            index_writer: IndexWriter::new(persistence, reader, retention_validator, runtime),
            should_terminate: true,
            persistence_version,
        };
        async move {
            loop {
                let r = worker.run().await;
                if let Err(ref e) = r
                    && e.is_occ()
                {
                    let delay = worker.runtime.with_rng(|rng| worker.backoff.fail(rng));
                    log::error!(
                        "IndexWorker died, num_failures: {}. Backing off for {}ms",
                        worker.backoff.failures(),
                        delay.as_millis()
                    );
                    worker.runtime.wait(delay).await;
                    continue;
                }
                return r;
            }
        }
        .boxed()
    }

    /// Returns a future representing the IndexWorker's main loop.
    ///
    /// Note: The return type of this future is misleading. It's really
    /// `anyhow::Result<!>` in production. For ease of writing test code with
    /// `IndexWorker::new_terminating`, the return type is specified as
    /// `anyhow::Result<()>` because `!` unifies with `()`.
    async fn run(&mut self) -> anyhow::Result<()> {
        log::info!("Starting IndexWorker");
        loop {
            let status = log_worker_starting("IndexWorker");
            // Get all the documents from the `_index` table.
            let mut tx = self.database.begin(Identity::system()).await?;
            // Index doesn't have `by_creation_time` index, and thus can't be queried via
            // collect.
            let index_scan = Query::index_range(IndexRange {
                index_name: IndexName::by_id(INDEX_TABLE.clone()),
                range: vec![],
                order: Order::Asc,
            });
            let mut index_documents = BTreeMap::new();
            {
                let mut query = ResolvedQuery::new(&mut tx, TableNamespace::Global, index_scan)?;
                while let Some(document) = query.next(&mut tx, None).await? {
                    index_documents.insert(document.id(), document);
                }
            }
            let mut to_backfill = vec![];
            for (id, doc) in &index_documents {
                let index_metadata: ParsedDocument<IndexMetadata<TabletId>> =
                    doc.clone().try_into()?;
                if let IndexConfig::Database { on_disk_state, .. } = &index_metadata.config {
                    if matches!(*on_disk_state, DatabaseIndexState::Backfilling(_)) {
                        to_backfill.push(id.internal_id());
                    }
                }
            }
            let num_to_backfill = to_backfill.len();
            log::info!(
                "{num_to_backfill} database indexes to backfill @ {}",
                tx.begin_timestamp()
            );
            for (i, index_id) in to_backfill.into_iter().enumerate() {
                log_num_indexes_to_backfill(num_to_backfill - i);
                self.backfill_one(index_id, tx.table_mapping(), index_documents.clone())
                    .await?;
            }
            if num_to_backfill > 0 {
                // We backfilled at least one index during this loop iteration.
                // There's no point in subscribing, as we'd immediately be woken by our own
                // changes.
                self.backoff.reset();
                continue;
            }
            log_num_indexes_to_backfill(0);
            log::info!("IndexWorker loop completed successfully, going to sleep");
            #[cfg(any(test, feature = "testing"))]
            if self.should_terminate {
                return Ok(());
            }
            drop(status);

            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            subscription.wait_for_invalidation().await;
            log::info!("IndexWorker resuming after index subscription notification");
            self.backoff.reset();
        }
    }

    async fn backfill_one(
        &mut self,
        index_id: IndexId,
        table_mapping: &TableMapping,
        index_documents: BTreeMap<ResolvedDocumentId, ResolvedDocument>,
    ) -> anyhow::Result<()> {
        let index_registry = IndexRegistry::bootstrap(
            table_mapping,
            index_documents.values(),
            self.persistence_version,
        )?;

        // If retention is already started, we have already done with the initial
        // step of the backfill.
        let (index_name, retention_started) = self.begin_backfill(index_id).await?;
        if !retention_started {
            log::info!("Starting backfill of index {}", index_name);
            let index_selector = IndexSelector::Index {
                name: index_name,
                id: index_id,
            };
            self.index_writer
                .perform_backfill(
                    self.database.now_ts_for_reads(),
                    &index_registry,
                    index_selector,
                )
                .await?;
        }

        // Run retention.
        let (backfill_begin_ts, index_name, indexed_fields) =
            self.begin_retention(index_id).await?;
        log::info!("Started running retention for index {}", index_name);
        self.index_writer
            .run_retention(index_id, backfill_begin_ts, index_name, indexed_fields)
            .await?;

        self.finish_backfill(index_id).await?;
        Ok(())
    }

    async fn begin_backfill(
        &mut self,
        index_id: IndexId,
    ) -> anyhow::Result<(TabletIndexName, bool)> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let index_table_id = tx.bootstrap_tables().index_id;

        // If we observe an index to be in `Backfilling` state at some `ts`, we
        // know that all documents written after `ts` will already be in the index.
        // The index may contain writes from before `ts` too, but that's okay. We'll
        // just overwrite them.
        let index_doc = tx
            .get(ResolvedDocumentId::new(index_table_id, index_id))
            .await?
            .ok_or_else(|| anyhow::anyhow!("Index {index_id:?} no longer exists"))?;
        let index_metadata = TabletIndexMetadata::from_document(index_doc)?;

        // Assuming that the IndexWorker is the only writer of index state, we expect
        // the state to still be `Backfilling` here. If this assertion fails, we
        // somehow raced with another `IndexWorker`(!) or don't actually have the
        // database lease (!).
        let retention_started = match &index_metadata.config {
            IndexConfig::Database { on_disk_state, .. } => {
                let DatabaseIndexState::Backfilling(state) = on_disk_state else {
                    anyhow::bail!(
                        "IndexWorker started backfilling index {index_metadata:?} not in \
                         Backfilling state"
                    );
                };
                state.retention_started
            },
            _ => anyhow::bail!(
                "IndexWorker attempted to backfill an index {index_metadata:?} which wasn't a \
                 database index."
            ),
        };

        Ok((index_metadata.name.clone(), retention_started))
    }

    async fn begin_retention(
        &mut self,
        index_id: IndexId,
    ) -> anyhow::Result<(Timestamp, TabletIndexName, IndexedFields)> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let index_table_id = tx.bootstrap_tables().index_id;

        let index_doc = tx
            .get(ResolvedDocumentId::new(index_table_id, index_id))
            .await?
            .ok_or_else(|| anyhow::anyhow!("Index {index_id:?} no longer exists"))?;
        let mut index_metadata = TabletIndexMetadata::from_document(index_doc)?;

        // Assuming that the IndexWorker is the only writer of index state, we expect
        // the state to still be `Backfilling` here. If this assertion fails, we
        // somehow raced with another `IndexWorker`(!) or don't actually have the
        // database lease (!).
        let (index_ts, indexed_fields) = match &mut index_metadata.config {
            IndexConfig::Database {
                on_disk_state,
                developer_config,
            } => {
                let DatabaseIndexState::Backfilling(state) = on_disk_state else {
                    anyhow::bail!(
                        "IndexWorker started backfilling index {index_metadata:?} not in \
                         Backfilling state"
                    )
                };

                state.retention_started = true;
                (
                    state.index_created_lower_bound,
                    developer_config.fields.clone(),
                )
            },
            _ => anyhow::bail!(
                "IndexWorker attempted to backfill an index {index_metadata:?} which wasn't a \
                 database index."
            ),
        };

        let name = index_metadata.name.clone();
        SystemMetadataModel::new_global(&mut tx)
            .replace(index_metadata.id(), index_metadata.into_value().try_into()?)
            .await?;
        self.database
            .commit_with_write_source(tx, "index_worker_start_retention")
            .await?;

        Ok((index_ts, name, indexed_fields))
    }

    async fn finish_backfill(&mut self, index_id: IndexId) -> anyhow::Result<()> {
        // Now that we're done, write that we've finished backfilling the index, sanity
        // checking that it wasn't written concurrently with our backfill.
        let mut tx = self.database.begin(Identity::system()).await?;
        let index_table_id = tx.bootstrap_tables().index_id;
        let full_index_id = ResolvedDocumentId::new(index_table_id, index_id);
        let index_doc = tx
            .get(full_index_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Index {index_id:?} no longer exists"))?;
        let mut index_metadata = TabletIndexMetadata::from_document(index_doc)?;
        let is_system_index_on_user_table = index_metadata.name.descriptor().is_reserved();
        let is_index_on_system_table = tx
            .table_mapping()
            .is_system_table_id(*index_metadata.name.table());
        match index_metadata.config {
            IndexConfig::Database {
                ref mut on_disk_state,
                ..
            } => {
                anyhow::ensure!(
                    matches!(*on_disk_state, DatabaseIndexState::Backfilling(_)),
                    "IndexWorker finished backfilling index {index_metadata:?} not in Backfilling \
                     state",
                );

                *on_disk_state = if is_system_index_on_user_table || is_index_on_system_table {
                    DatabaseIndexState::Enabled
                } else {
                    DatabaseIndexState::Backfilled
                };
            },
            _ => anyhow::bail!(
                "IndexWorker finished backfilling index {index_metadata:?} which wasn't a \
                 database index"
            ),
        };

        let name = index_metadata.name.clone();

        SystemMetadataModel::new_global(&mut tx)
            .replace(full_index_id, index_metadata.into_value().try_into()?)
            .await?;
        self.database
            .commit_with_write_source(tx, "index_worker_finish_backfill")
            .await?;
        log::info!("Finished backfill of index {}", name);
        log_index_backfilled();
        Ok(())
    }
}

impl<RT: Runtime> IndexWriter<RT> {
    pub fn new(
        persistence: Arc<dyn Persistence>,
        reader: Arc<dyn PersistenceReader>,
        retention_validator: Arc<dyn RetentionValidator>,
        runtime: RT,
    ) -> Self {
        Self {
            persistence,
            reader,
            retention_validator,
            rate_limiter: Arc::new(new_rate_limiter(
                runtime.clone(),
                Quota::per_second(*ENTRIES_PER_SECOND),
            )),
            runtime,
        }
    }

    /// Backfill in two steps: first a snapshot at the current time, and then
    /// walking the log. After the current snapshot is backfilled, index
    /// snapshot reads at >=ts are valid. The subsequent walking of the log
    /// extends the earliest allowed snapshot into the past.
    ///
    /// The goal of this backfill is to make snapshot reads of `index_name`
    /// valid between the range of [retention_cutoff, snapshot_ts].
    /// To support:
    /// 1. Latest documents written before retention_cutoff. They are still
    ///    latest at `ts = snapshot_ts`, so we compute and write index entries
    ///    when we walk the `snapshot_ts` snapshot.
    /// 2. Document changes between retention_cutoff and `snapshot_ts`. These
    ///    are handled by walking the documents log for this range in reverse
    ///    and creating index entries. When walking the documents log we start
    ///    at `snapshot_ts`.
    /// 3. Documents that were latest as of retention_cutoff but were
    ///    overwritten before `snapshot_ts`. These are handled when walking the
    ///    documents log and finding an overwrite.
    /// 4. Document changes after `snapshot_ts`. These are handled by active
    ///    writes, assuming `snapshot_ts` is after the index was created. If
    ///    there are no active writes, then `backfill_forwards` must be called
    ///    with a timestamp <= `snapshot_ts`.
    pub async fn perform_backfill(
        &self,
        snapshot_ts: RepeatableTimestamp,
        index_metadata: &IndexRegistry,
        index_selector: IndexSelector,
    ) -> anyhow::Result<()> {
        // Backfill in two steps: first create index entries for all latest documents,
        // then create index entries for all documents in the retention range.

        let (tx, rx) = mpsc::unbounded();
        let handles: Vec<_> = index_selector
            .iterate_tables()
            .map(|table_id| {
                let index_metadata = index_metadata.clone();
                let index_selector = index_selector.clone();
                let self_ = (*self).clone();
                let tx = tx.clone();
                self.runtime
                    .spawn("index_backfill_table_snapshot", async move {
                        tx.unbounded_send(
                            self_
                                .backfill_exact_snapshot_of_table(
                                    snapshot_ts,
                                    &index_selector,
                                    &index_metadata,
                                    table_id,
                                )
                                .await,
                        )
                        .unwrap();
                    })
            })
            .collect();
        for handle in handles {
            handle.into_join_future().await?;
        }
        tx.close_channel();
        let _: Vec<_> = rx.try_collect().await?;

        let mut min_backfilled_ts = snapshot_ts;

        // Retry until min_snapshot_ts passes min_backfilled_ts, at which point we
        // have backfilled the full range of snapshots within retention.
        loop {
            let min_snapshot_ts = self.retention_validator.min_snapshot_ts().await?;
            if min_snapshot_ts >= *min_backfilled_ts {
                break;
            }
            // NOTE: ordering Desc is important, to keep the range of valid snapshots
            // contiguous. If we backfilled in order Asc, then we might see a
            // document creation before its tombstone, and that document would be
            // visible at snapshots where it should be deleted.
            min_backfilled_ts = self
                .backfill_backwards(
                    min_backfilled_ts,
                    min_snapshot_ts,
                    index_metadata,
                    &index_selector,
                )
                .await?;
        }
        Ok(())
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
    ) -> anyhow::Result<()> {
        let table_iterator = TableIterator::new(
            self.runtime.clone(),
            snapshot_ts,
            self.reader.clone(),
            self.retention_validator.clone(),
            *INDEX_BACKFILL_CHUNK_SIZE,
            None,
        );

        let by_id = index_registry.must_get_by_id(tablet_id)?.id();
        let stream = table_iterator
            .stream_documents_in_table(tablet_id, by_id, None)
            .fuse();
        pin_mut!(stream);
        let mut index_updates_written = 0;
        let mut last_logged = self.runtime.system_time();
        while !stream.is_done() {
            let mut chunk = BTreeSet::new();
            while chunk.len() < *INDEX_BACKFILL_CHUNK_SIZE {
                let (document, ts) = match stream.try_next().await? {
                    Some(d) => d,
                    None => break,
                };
                let index_updates = index_registry.index_updates(None, Some(&document));
                chunk.extend(
                    index_updates
                        .into_iter()
                        .filter(|update| index_selector.filter_index_update(update))
                        .map(|update| (ts, update)),
                );
            }
            if !chunk.is_empty() {
                index_updates_written += chunk.len();
                self.persistence
                    .write(vec![], chunk, ConflictStrategy::Overwrite)
                    .await?;
            }
            if last_logged.elapsed()? >= Duration::from_secs(60) {
                tracing::info!(
                    "backfilled {index_updates_written} index rows for table {tablet_id} at \
                     snapshot {snapshot_ts}",
                );
                last_logged = self.runtime.system_time();
            }
        }
        tracing::info!(
            "backfilled {index_updates_written} index rows for table {tablet_id} at snapshot \
             {snapshot_ts}"
        );
        Ok(())
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
    /// - The selected indexes are fully backfilled for all revisions less than
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
        let (mut tx, rx) = mpsc::channel(32);
        let producer = async {
            let revision_stream = self.stream_revision_pairs(
                &repeatable_persistence,
                TimestampRange::new(start_ts..=*end_ts)?,
                Order::Asc,
                index_selector,
            );
            futures::pin_mut!(revision_stream);
            while let Some(revision_pair) = revision_stream.try_next().await? {
                let index_updates = index_registry
                    .index_updates(revision_pair.prev_document(), revision_pair.document());
                for update in index_updates {
                    tx.send((revision_pair.ts(), update)).await?;
                }
            }
            drop(tx);
            Ok(())
        };
        let consumer = self.write_index_entries(rx, index_selector);

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
        let (mut tx, rx) = mpsc::channel(32);
        let repeatable_persistence = RepeatablePersistence::new(
            self.reader.clone(),
            start_ts,
            self.retention_validator.clone(),
        );
        let producer = async {
            let revision_stream = self.stream_revision_pairs(
                &repeatable_persistence,
                TimestampRange::new(end_ts..*start_ts)?,
                Order::Desc,
                index_selector,
            );
            futures::pin_mut!(revision_stream);
            while let Some(revision_pair) = revision_stream.try_next().await? {
                let ts = revision_pair.ts();
                if ts < self.retention_validator.min_snapshot_ts().await? {
                    // We may not have fully processed the entirety of the transaction at
                    // `min_chunk_ts` (since we paginate by `(ts, id)`), so only consider
                    // ourselves backfilled up to the subsequent timestamp.
                    return ts.succ();
                }

                let rev_updates = index_registry
                    .index_updates(revision_pair.prev_document(), revision_pair.document());
                for update in rev_updates {
                    tx.send((ts, update)).await?;
                }

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
                // process `prev_rev`'s log entry, but setting `ConflictStrategy::Overwrite`
                // in `Persistence::write` makes this a no-op.
                if let Some(ref prev_rev) = revision_pair.prev_rev {
                    if let Some(ref prev_doc) = prev_rev.document {
                        let prev_rev_updates = index_registry.index_updates(None, Some(prev_doc));
                        for update in prev_rev_updates {
                            tx.send((prev_rev.ts, update)).await?;
                        }
                    }
                }
            }
            drop(tx);
            Ok(end_ts)
        };

        let consumer = self.write_index_entries(rx, index_selector);

        // Consider ourselves successful if both the reader and writer exit
        // successfully.
        let (backfilled_ts, ()) = futures::try_join!(producer, consumer)?;
        start_ts.prior_ts(backfilled_ts)
    }

    fn stream_revision_pairs<'a>(
        &'a self,
        reader: &'a RepeatablePersistence,
        range: TimestampRange,
        order: Order,
        index_selector: &'a IndexSelector,
    ) -> impl Stream<Item = anyhow::Result<RevisionPair>> + 'a {
        let document_stream = reader
            .load_documents(range, order)
            .try_filter(|(_, id, _)| future::ready(index_selector.filter_id(*id)));
        stream_revision_pairs(document_stream, reader)
    }

    async fn write_index_entries(
        &self,
        updates: impl Stream<Item = (Timestamp, DatabaseIndexUpdate)> + FusedStream,
        index_selector: &IndexSelector,
    ) -> anyhow::Result<()> {
        futures::pin_mut!(updates);

        let mut last_logged = self.runtime.system_time();
        let mut num_entries_written = 0;

        while !updates.is_terminated() {
            // There are potentially more document revisions, so start a new chunk. First,
            // check with the rate limiter upfront to ensure we're allowed to
            // continue.
            while let Err(not_until) = self.rate_limiter.check() {
                // NB: We can't use `RateLimiter`'s async API since it internally relies on
                // `futures-timer`. These timers will never get satisfied under our test
                // runtime.
                let delay = not_until.wait_time_from(self.runtime.monotonic_now().as_nanos());
                self.runtime.wait(delay).await;
            }

            // Try to fill up a full chunk until we exhaust the stream or fill the chunk.
            let mut chunk = BTreeSet::new();
            while chunk.len() < *INDEX_BACKFILL_CHUNK_SIZE {
                let (ts, update) = match updates.next().await {
                    Some(r) => r,
                    None => break,
                };
                if !index_selector.filter_index_update(&update) {
                    continue;
                }
                chunk.insert((ts, update));
            }
            if !chunk.is_empty() {
                num_entries_written += chunk.len();
                self.persistence
                    .write(vec![], chunk, ConflictStrategy::Overwrite)
                    .await?;
                if last_logged.elapsed()? >= Duration::from_secs(60) {
                    log::info!(
                        "Backfilled {} index entries of index {}",
                        num_entries_written,
                        index_selector,
                    );
                    last_logged = self.runtime.system_time();
                }
            }
        }
        Ok(())
    }

    async fn run_retention(
        &self,
        index_id: IndexId,
        backfill_begin_ts: Timestamp,
        index_name: TabletIndexName,
        indexed_fields: IndexedFields,
    ) -> anyhow::Result<()> {
        let min_snapshot_ts = self.retention_validator.min_snapshot_ts().await?;
        let all_indexes = btreemap! { index_id => (index_name, indexed_fields) };
        // TODO(lee) add checkpointing.
        LeaderRetentionManager::delete_all_no_checkpoint(
            backfill_begin_ts,
            min_snapshot_ts,
            self.persistence.clone(),
            &self.runtime,
            &all_indexes,
            self.retention_validator.clone(),
        )
        .await?;
        Ok(())
    }
}
