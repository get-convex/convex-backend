use std::{
    cmp,
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

use cmd_util::env::env_config;
use common::{
    self,
    backoff::Backoff,
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexState,
            IndexedFields,
        },
        IndexConfig,
        TabletIndexMetadata,
    },
    errors::report_error,
    knobs::{
        ENABLE_INDEX_BACKFILL,
        INDEX_BACKFILL_CHUNK_RATE,
        INDEX_BACKFILL_CHUNK_SIZE,
        INDEX_BACKFILL_WORKERS,
        INDEX_WORKERS_INITIAL_BACKOFF,
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
        stream_revision_pairs,
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
    pin_mut,
    stream::{
        self,
        FusedStream,
    },
    Future,
    Stream,
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use indexing::index_registry::IndexRegistry;
use keybroker::Identity;
use maplit::btreeset;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use value::{
    DeveloperDocumentId,
    TableNamespace,
};

use crate::{
    bootstrap_model::index_backfills::IndexBackfillModel,
    metrics::{
        index_backfill_timer,
        log_index_backfilled,
        log_num_indexes_to_backfill,
        tablet_index_backfill_timer,
    },
    retention::LeaderRetentionManager,
    system_tables::SystemIndex,
    Database,
    IndexTable,
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

static INDEX_WORKER_SLEEP_TIME: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("INDEX_WORKER_SLEEP_TIME_MS", 0)));

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
    Index {
        name: TabletIndexName,
        id: IndexId,
    },
    ManyIndexes {
        tablet_id: TabletId,
        indexes: BTreeMap<IndexId, TabletIndexName>,
    },
}

impl Display for IndexSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All(_) => write!(f, "ALL"),
            Self::Index { name, .. } => write!(f, "{}", name),
            Self::ManyIndexes { ref indexes, .. } => {
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
            Self::Index { id, .. } => id == &index_update.index_id,
            Self::ManyIndexes { indexes, .. } => indexes.contains_key(&index_update.index_id),
        }
    }

    fn iterate_tables(&self) -> impl Iterator<Item = TabletId> {
        let tables = match self {
            Self::All(index_registry) => index_registry
                .all_tables_with_indexes()
                .into_iter()
                .collect(),
            Self::Index { name, .. } => btreeset! { *name.table() },
            Self::ManyIndexes { tablet_id, .. } => btreeset! { *tablet_id },
        };
        tables.into_iter()
    }

    fn index_ids(&self) -> impl Iterator<Item = IndexId> {
        let indexes = match self {
            Self::All(index_registry) => index_registry
                .all_indexes()
                .map(|doc| doc.id().internal_id())
                .collect(),

            Self::Index { id, .. } => btreeset! { *id },
            Self::ManyIndexes { indexes, .. } => indexes.keys().copied().collect(),
        };
        indexes.into_iter()
    }

    fn tablet_id(&self) -> Option<TabletId> {
        match self {
            Self::All(_) => None,
            Self::Index { name, .. } => Some(*name.table()),
            Self::ManyIndexes { tablet_id, .. } => Some(*tablet_id),
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
                tracing::error!("Forcibly disabling index backfill, exiting IndexWorker...");
                return;
            }
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("IndexWorker died")).await;
                    let delay = worker.backoff.fail(&mut worker.runtime.rng());
                    tracing::error!(
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
                    let delay = worker.backoff.fail(&mut worker.runtime.rng());
                    tracing::error!(
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
        tracing::info!("Starting IndexWorker");
        loop {
            let timer = index_backfill_timer();
            // Get all the documents from the `_index` table.
            let mut tx = self.database.begin(Identity::system()).await?;
            // _index doesn't have `by_creation_time` index, and thus must use `by_id`.
            let index_documents = tx
                .query_system(TableNamespace::Global, &SystemIndex::<IndexTable>::by_id())?
                .all()
                .await?;
            let mut to_backfill_by_tablet = BTreeMap::new();
            let mut num_to_backfill = 0;
            for index_metadata in &index_documents {
                if let IndexConfig::Database { on_disk_state, .. } = &index_metadata.config {
                    if matches!(on_disk_state, DatabaseIndexState::Backfilling(_)) {
                        to_backfill_by_tablet
                            .entry(*index_metadata.name.table())
                            .or_insert_with(Vec::new)
                            .push(index_metadata.id().internal_id());
                        num_to_backfill += 1;
                    }
                }
            }
            tracing::info!(
                "{num_to_backfill} database indexes to backfill @ {}",
                tx.begin_timestamp()
            );

            let index_registry = IndexRegistry::bootstrap(
                tx.table_mapping(),
                index_documents.into_iter().map(|doc| (*doc).clone()),
                self.persistence_version,
            )?;

            let mut num_backfilled = 0;
            for (tablet_id, index_ids) in to_backfill_by_tablet {
                log_num_indexes_to_backfill(num_to_backfill - num_backfilled);
                num_backfilled += index_ids.len();
                self.backfill_tablet(tablet_id, index_ids, tx.table_mapping(), &index_registry)
                    .await?;
            }
            if num_to_backfill > 0 {
                timer.finish(true);
                // We backfilled at least one index during this loop iteration.
                // There's no point in subscribing, as we'd immediately be woken by our own
                // changes.
                self.backoff.reset();
                continue;
            }
            log_num_indexes_to_backfill(0);
            tracing::info!("IndexWorker loop completed successfully, going to sleep");
            #[cfg(any(test, feature = "testing"))]
            if self.should_terminate {
                return Ok(());
            }

            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            subscription.wait_for_invalidation().await;
            tracing::info!("IndexWorker resuming after index subscription notification");
            self.backoff.reset();
        }
    }

    async fn backfill_tablet(
        &mut self,
        tablet_id: TabletId,
        index_ids: Vec<IndexId>,
        table_mapping: &TableMapping,
        index_registry: &IndexRegistry,
    ) -> anyhow::Result<()> {
        let _timer = tablet_index_backfill_timer();
        let mut backfills = BTreeMap::new();
        for index_id in &index_ids {
            let (index_name, retention_started) = self.begin_backfill(*index_id).await?;
            backfills.insert(*index_id, (index_name, retention_started));
        }

        let needs_backfill = backfills
            .iter()
            // If retention is already started, we're already done with the
            // initial step of the backfill.
            .filter(|(_, (_, retention_started))| !*retention_started)
            .map(|(index_id, (index_name, _))| (*index_id, index_name.clone()))
            .collect::<BTreeMap<_, _>>();

        if !needs_backfill.is_empty() {
            let table_name = table_mapping.tablet_to_name()(tablet_id)?;
            tracing::info!(
                "Starting backfill of {} indexes for {table_name}: {needs_backfill:?}",
                needs_backfill.len()
            );
            let ts = self.database.now_ts_for_reads();
            let table_summary = self
                .database
                .snapshot(ts)?
                .table_summary(table_mapping.tablet_namespace(tablet_id)?, &table_name);
            let total_docs = table_summary.map(|summary| summary.num_values());
            let mut tx = self.database.begin_system().await?;
            let mut index_backfill_model = IndexBackfillModel::new(&mut tx);
            for index_id in needs_backfill.keys() {
                index_backfill_model
                    .initialize_backfill(*index_id, total_docs)
                    .await?;
            }
            self.database
                .commit_with_write_source(tx, "index_worker_backfill_initialization")
                .await?;
            let index_selector = IndexSelector::ManyIndexes {
                tablet_id,
                indexes: needs_backfill,
            };
            self.index_writer
                .perform_backfill(
                    ts,
                    index_registry,
                    index_selector,
                    1,
                    Some(self.database.clone()),
                )
                .await?;
        }

        let mut min_begin_ts = None;
        let mut retention = BTreeMap::new();
        for index_id in &index_ids {
            let (backfill_begin_ts, index_name, indexed_fields) =
                self.begin_retention(*index_id).await?;

            min_begin_ts = min_begin_ts
                .map(|t| cmp::min(t, backfill_begin_ts))
                .or(Some(backfill_begin_ts));

            retention.insert(*index_id, (index_name, indexed_fields));
        }
        if let Some(min_begin_ts) = min_begin_ts {
            tracing::info!(
                "Started running retention for {} indexes: {retention:?}",
                retention.len()
            );
            self.index_writer
                .run_retention(min_begin_ts, retention)
                .await?;
        }

        for index_id in index_ids {
            self.finish_backfill(index_id).await?;
        }

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
            .get(ResolvedDocumentId::new(
                index_table_id.tablet_id,
                DeveloperDocumentId::new(index_table_id.table_number, index_id),
            ))
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
    ) -> anyhow::Result<(RepeatableTimestamp, TabletIndexName, IndexedFields)> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let index_table_id = tx.bootstrap_tables().index_id;

        let index_doc = tx
            .get(ResolvedDocumentId::new(
                index_table_id.tablet_id,
                DeveloperDocumentId::new(index_table_id.table_number, index_id),
            ))
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
                    tx.begin_timestamp()
                        .prior_ts(state.index_created_lower_bound)?,
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
        let full_index_id = ResolvedDocumentId::new(
            index_table_id.tablet_id,
            DeveloperDocumentId::new(index_table_id.table_number, index_id),
        );
        let index_doc = tx
            .get(full_index_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Index {index_id:?} no longer exists"))?;
        let mut index_metadata = TabletIndexMetadata::from_document(index_doc)?;
        let is_system_index_on_user_table = index_metadata.name.descriptor().is_reserved();
        let is_index_on_system_table = tx
            .table_mapping()
            .is_system_tablet(*index_metadata.name.table());
        match index_metadata.config {
            IndexConfig::Database {
                ref mut on_disk_state,
                ..
            } => {
                let DatabaseIndexState::Backfilling(ref backfilling_state) = *on_disk_state else {
                    anyhow::bail!(
                        "IndexWorker finished backfilling index {index_metadata:?} not in \
                         Backfilling state"
                    );
                };
                *on_disk_state = if is_system_index_on_user_table || is_index_on_system_table {
                    DatabaseIndexState::Enabled
                } else {
                    DatabaseIndexState::Backfilled {
                        staged: backfilling_state.staged,
                    }
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
        let table_name = tx.table_mapping().tablet_name(*name.table())?;
        self.database
            .commit_with_write_source(tx, "index_worker_finish_backfill")
            .await?;
        tracing::info!("Finished backfill of index {}", name);
        if is_index_on_system_table || is_system_index_on_user_table {
            tracing::info!(
                "Finished backfill of system index {table_name}.{}",
                name.descriptor()
            );
        }
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
        debug_assert!(
            ENTRIES_PER_SECOND.get() >= *INDEX_BACKFILL_CHUNK_SIZE as u32,
            "Entries per second must be at least {}",
            *INDEX_BACKFILL_CHUNK_SIZE
        );
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
    ///
    /// Takes a an optional database to update progress on the index backfill
    pub async fn perform_backfill(
        &self,
        snapshot_ts: RepeatableTimestamp,
        index_metadata: &IndexRegistry,
        index_selector: IndexSelector,
        concurrency: usize,
        database: Option<Database<RT>>,
    ) -> anyhow::Result<()> {
        // Backfill in two steps: first create index entries for all latest documents,
        // then create index entries for all documents in the retention range.

        stream::iter(index_selector.iterate_tables().map(Ok))
            .try_for_each_concurrent(concurrency, |table_id| {
                let index_metadata = index_metadata.clone();
                let index_selector = index_selector.clone();
                let database = database.clone();
                let self_ = (*self).clone();
                try_join("index_backfill_table_snapshot", async move {
                    self_
                        .backfill_exact_snapshot_of_table(
                            snapshot_ts,
                            &index_selector,
                            &index_metadata,
                            table_id,
                            database,
                        )
                        .await
                })
            })
            .await?;

        let mut min_backfilled_ts = snapshot_ts;

        // Retry until min_snapshot_ts passes min_backfilled_ts, at which point we
        // have backfilled the full range of snapshots within retention.
        loop {
            let min_snapshot_ts = self.retention_validator.min_snapshot_ts().await?;
            if min_snapshot_ts >= min_backfilled_ts {
                break;
            }
            // NOTE: ordering Desc is important, to keep the range of valid snapshots
            // contiguous. If we backfilled in order Asc, then we might see a
            // document creation before its tombstone, and that document would be
            // visible at snapshots where it should be deleted.
            min_backfilled_ts = self
                .backfill_backwards(
                    min_backfilled_ts,
                    *min_snapshot_ts,
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
        database: Option<Database<RT>>,
    ) -> anyhow::Result<()> {
        let table_iterator = TableIterator::new(
            self.runtime.clone(),
            snapshot_ts,
            self.reader.clone(),
            self.retention_validator.clone(),
            *INDEX_BACKFILL_CHUNK_SIZE,
        );

        let by_id = index_registry.must_get_by_id(tablet_id)?.id();
        let stream = table_iterator
            .stream_documents_in_table(tablet_id, by_id, None)
            .fuse();
        pin_mut!(stream);
        let mut index_updates_written = 0;
        let mut last_logged = self.runtime.system_time();
        let mut last_logged_count = 0;
        while !stream.is_done() {
            if !INDEX_WORKER_SLEEP_TIME.is_zero() {
                tokio::time::sleep(*INDEX_WORKER_SLEEP_TIME).await;
            }
            // Number of documents in the table that have been indexed in this iteration
            let mut num_docs_indexed = 0u64;
            let mut chunk = BTreeSet::new();
            while chunk.len() < *INDEX_BACKFILL_CHUNK_SIZE {
                let LatestDocument {
                    ts,
                    value: document,
                    ..
                } = match stream.try_next().await? {
                    Some(d) => d,
                    None => break,
                };
                num_docs_indexed += 1;
                let index_updates = index_registry.index_updates(None, Some(&document));
                chunk.extend(
                    index_updates
                        .into_iter()
                        .filter(|update| index_selector.filter_index_update(update))
                        .map(|update| PersistenceIndexEntry::from_index_update(ts, update)),
                );
            }
            if !chunk.is_empty() {
                index_updates_written += chunk.len();
                self.persistence
                    .write(vec![], chunk, ConflictStrategy::Overwrite)
                    .await?;
                if let Some(db) = &database {
                    let mut tx = db.begin_system().await?;
                    let mut model = IndexBackfillModel::new(&mut tx);
                    for index_id in index_selector.index_ids() {
                        model
                            .update_index_backfill_progress(index_id, tablet_id, num_docs_indexed)
                            .await?;
                    }
                    db.commit_with_write_source(tx, "index_worker_backfill_progress")
                        .await?;
                }
            }
            if last_logged.elapsed()? >= Duration::from_secs(60) {
                tracing::info!(
                    "backfilled {index_updates_written} index rows for table {tablet_id} at \
                     snapshot {snapshot_ts} ({} rows/s)",
                    (index_updates_written - last_logged_count) as f64
                        / last_logged.elapsed()?.as_secs_f64()
                );
                last_logged = self.runtime.system_time();
                last_logged_count = index_updates_written;
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
        let (tx, rx) = mpsc::channel(32);
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
        let consumer = self.write_index_entries(ReceiverStream::new(rx).fuse(), index_selector);

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
            let revision_stream = self.stream_revision_pairs(
                &repeatable_persistence,
                TimestampRange::new(end_ts..*start_ts)?,
                Order::Desc,
                index_selector,
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

        let consumer = self.write_index_entries(ReceiverStream::new(rx).fuse(), index_selector);

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
        let document_stream = if let Some(tablet_id) = index_selector.tablet_id() {
            reader.load_documents_from_table(tablet_id, range, order)
        } else {
            reader.load_documents(range, order)
        };
        stream_revision_pairs(document_stream, reader)
    }

    async fn write_index_entries(
        &self,
        updates: impl FusedStream<Item = (Timestamp, DatabaseIndexUpdate)>,
        index_selector: &IndexSelector,
    ) -> anyhow::Result<()> {
        futures::pin_mut!(updates);

        let mut last_logged = self.runtime.system_time();
        let mut num_entries_written = 0;

        let updates = updates
            .filter_map(|(ts, update)| {
                let result = {
                    if index_selector.filter_index_update(&update) {
                        Some(PersistenceIndexEntry::from_index_update(ts, update))
                    } else {
                        None
                    }
                };
                futures::future::ready(result)
            })
            .chunks(*INDEX_BACKFILL_CHUNK_SIZE)
            .map(|chunk| async {
                let persistence = self.persistence.clone();
                let rate_limiter = self.rate_limiter.clone();
                let size = chunk.len();
                while let Err(not_until) = rate_limiter
                    .check_n(
                        (size as u32)
                            .try_into()
                            .expect("Chunk size must be nonzero"),
                    )
                    .expect("RateLimiter capacity impossibly small")
                {
                    let delay = not_until.wait_time_from(self.runtime.monotonic_now().into());
                    self.runtime.wait(delay).await;
                }
                persistence
                    .write(
                        vec![],
                        chunk.into_iter().collect(),
                        ConflictStrategy::Overwrite,
                    )
                    .await?;
                anyhow::Ok(size)
            })
            .buffer_unordered(*INDEX_BACKFILL_WORKERS);
        pin_mut!(updates);

        while let Some(result) = updates.next().await {
            let entries_written = result?;
            num_entries_written += entries_written;
            if last_logged.elapsed()? >= Duration::from_secs(60) {
                tracing::info!(
                    "Backfilled {} index entries of index {}",
                    num_entries_written,
                    index_selector,
                );
                last_logged = self.runtime.system_time();
            }
        }

        Ok(())
    }

    async fn run_retention(
        &self,
        backfill_begin_ts: RepeatableTimestamp,
        all_indexes: BTreeMap<IndexId, (TabletIndexName, IndexedFields)>,
    ) -> anyhow::Result<()> {
        let min_snapshot_ts = self.retention_validator.min_snapshot_ts().await?;
        // TODO(lee) add checkpointing.
        LeaderRetentionManager::<RT>::delete_all_no_checkpoint(
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
