use std::{
    cmp,
    collections::{
        BTreeMap,
        HashSet,
    },
    future::Future,
    hash::{
        Hash,
        Hasher,
    },
    sync::Arc,
};

use common::{
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
        INDEX_BACKFILL_CONCURRENCY,
        INDEX_WORKERS_INITIAL_BACKOFF,
        INDEX_WORKERS_MAX_BACKOFF,
    },
    persistence::{
        Persistence,
        RetentionValidator,
    },
    runtime::Runtime,
    types::{
        IndexId,
        PersistenceVersion,
        RepeatableTimestamp,
        TabletIndexName,
    },
};
use futures::FutureExt;
use hashlink::LinkedHashSet;
use indexing::index_registry::IndexRegistry;
use keybroker::Identity;
use tokio::{
    select,
    sync::{
        OwnedSemaphorePermit,
        Semaphore,
    },
};
use tokio_util::task::JoinMap;
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableMapping,
    TableNamespace,
    TabletId,
};

use crate::{
    bootstrap_model::index_backfills::IndexBackfillModel,
    database_index_workers::index_writer::{
        IndexSelector,
        IndexWriter,
    },
    metrics::{
        log_index_backfilled,
        log_num_indexes_to_backfill,
        tablet_index_backfill_timer,
    },
    system_tables::SystemIndex,
    Database,
    IndexTable,
    SystemMetadataModel,
};

pub mod index_writer;

pub struct IndexWorker<RT: Runtime> {
    // Index IDs that are currently being backfilled.
    in_progress_index_ids: HashSet<IndexId, ahash::RandomState>,
    /// Order-preserving HashMap that represents the order that pending
    /// index backfills will be loaded in.
    pending: LinkedHashSet<IndexBackfill, ahash::RandomState>,
    // Index IDs that are pending backfill.
    pending_index_ids: HashSet<IndexId, ahash::RandomState>,
    concurrency_limiter: Arc<Semaphore>,
    work_pool: JoinMap<IndexBackfill, anyhow::Result<()>>,
    database: Database<RT>,
    persistence_version: PersistenceVersion,
    index_writer: IndexWriter<RT>,
    backoff: Backoff,
    runtime: RT,
    #[cfg(any(test, feature = "testing"))]
    pub should_terminate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexBackfill {
    pub tablet_id: TabletId,
    pub index_ids: Vec<IndexId>,
    pub table_mapping: TableMapping,
    pub index_registry: IndexRegistry,
}

impl Hash for IndexBackfill {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tablet_id.hash(state);
        self.index_ids.hash(state);
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
        let index_writer = IndexWriter::new(
            persistence.clone(),
            reader.clone(),
            retention_validator,
            runtime.clone(),
        );
        let mut worker = IndexWorker {
            in_progress_index_ids: Default::default(),
            pending: Default::default(),
            pending_index_ids: Default::default(),
            concurrency_limiter: Arc::new(Semaphore::new(*INDEX_BACKFILL_CONCURRENCY)),
            work_pool: JoinMap::new(),
            database,
            persistence_version: reader.version(),
            index_writer,
            backoff: Backoff::new(*INDEX_WORKERS_INITIAL_BACKOFF, *INDEX_WORKERS_MAX_BACKOFF),
            runtime,
            #[cfg(any(test, feature = "testing"))]
            should_terminate: false,
        };

        tracing::info!("Starting IndexWorker");
        async move {
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("IndexWorkerLoop died")).await;
                    let delay = worker.backoff.fail(&mut worker.runtime.rng());
                    tracing::error!(
                        "IndexIndexWorker died, num_failures: {}. Backing off for {}ms",
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
        let reader = persistence.reader();
        let index_writer = IndexWriter::new(
            persistence.clone(),
            reader.clone(),
            retention_validator,
            runtime.clone(),
        );
        let mut worker = IndexWorker {
            in_progress_index_ids: Default::default(),
            pending: Default::default(),
            pending_index_ids: Default::default(),
            concurrency_limiter: Arc::new(Semaphore::new(10)),
            work_pool: JoinMap::new(),
            database,
            persistence_version: reader.version(),
            index_writer,
            backoff: Backoff::new(*INDEX_WORKERS_INITIAL_BACKOFF, *INDEX_WORKERS_MAX_BACKOFF),
            runtime,
            should_terminate: true,
        };

        async move {
            loop {
                use errors::ErrorMetadataAnyhowExt;

                let r = worker.run().await;

                if let Err(ref e) = r
                    && e.is_occ()
                {
                    tracing::error!("IndexWorker loop failed: {e:?}");
                    continue;
                }
                if worker.work_pool.is_empty() && worker.pending.is_empty() {
                    return r;
                }
            }
        }
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        let mut backfill_queue = Vec::new();
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
        log_num_indexes_to_backfill(num_to_backfill);
        tracing::info!(
            "{num_to_backfill} database indexes to backfill @ {}",
            tx.begin_timestamp()
        );

        let index_registry = IndexRegistry::bootstrap(
            tx.table_mapping(),
            index_documents.into_iter().map(|doc| (*doc).clone()),
            self.persistence_version,
        )?;

        for (tablet_id, index_ids) in to_backfill_by_tablet {
            let mut to_backfill = Vec::new();
            for index_id in index_ids {
                if !self.in_progress_index_ids.contains(&index_id)
                    && !self.pending_index_ids.contains(&index_id)
                {
                    to_backfill.push(index_id);
                }
            }
            backfill_queue.push(IndexBackfill {
                tablet_id,
                index_ids: to_backfill,
                table_mapping: tx.table_mapping().clone(),
                index_registry: index_registry.clone(),
            });
        }

        let token = tx.into_token()?;
        let subscription = self.database.subscribe(token).await?;
        for index_backfill in backfill_queue.clone() {
            self.pending.insert(index_backfill.clone());
            self.pending_index_ids.extend(index_backfill.index_ids);
        }

        #[cfg(any(test, feature = "testing"))]
        if self.should_terminate
            && backfill_queue.is_empty()
            && self.work_pool.is_empty()
            && self.pending.is_empty()
        {
            return Ok(());
        }

        select! {
            biased;
            // Start by finding indexes that have finished backfilling
            (index_backfill, res) = self.work_pool.join_next().map(|o| o.expect("Can't be none if work pool is nonempty")), if !self.work_pool.is_empty() => {
                if let Err(e) = res {
                    tracing::error!("IndexWorker loop failed: {e:?}");
                    report_error(&mut anyhow::anyhow!(format!("{e:?}: IndexWorker loop failed on backfill {index_backfill:?}"))).await;
                }

                for index_id in &index_backfill.index_ids {
                    self.in_progress_index_ids.remove(index_id);
                }
            }
            // Next, queue up new work
            permit = self.concurrency_limiter.clone().acquire_owned(),
                if !self.pending.is_empty() =>
            {
                let permit = permit.expect("Semaphore can't go away");
                self.queue_index_backfill(permit);
            }
            // Finally, wait for invalidation
            _ = subscription.wait_for_invalidation().fuse() => {
                tracing::info!("IndexIndexWorker received invalidation, going to sleep");
                self.backoff.reset();
            }
        }

        Ok(())
    }

    /// Spawns a task to load the process the next index backfill. The `permit`
    /// is held to ensure we limit our concurrency.
    fn queue_index_backfill(&mut self, permit: OwnedSemaphorePermit) {
        // Try to queue an index backfill
        let Some(index_backfill) = self.pending.pop_front() else {
            return;
        };
        for index_id in &index_backfill.index_ids {
            self.pending_index_ids.remove(index_id);
        }

        if self.work_pool.contains_key(&index_backfill) {
            let IndexBackfill { tablet_id, .. } = index_backfill;
            tracing::warn!("Skipping duplicate index backfill {tablet_id} (already in progress)");
            return;
        }
        if index_backfill
            .index_ids
            .iter()
            .any(|index_id| self.in_progress_index_ids.contains(index_id))
        {
            let IndexBackfill {
                tablet_id,
                index_ids,
                ..
            } = index_backfill;
            tracing::warn!(
                "Skipping backfill of indexes {index_ids:?} on tablet {tablet_id} until current \
                 backfill completes"
            );
            return;
        }

        let IndexBackfill {
            tablet_id,
            index_ids,
            ..
        } = index_backfill.clone();

        let database = self.database.clone();
        let index_writer = self.index_writer.clone();

        for index_id in &index_ids {
            self.in_progress_index_ids.insert(*index_id);
        }

        self.work_pool.spawn(index_backfill, async move {
            let _permit = permit;

            Self::backfill_tablet(tablet_id, index_ids, database, index_writer).await
        });
    }

    async fn backfill_tablet(
        tablet_id: TabletId,
        index_ids: Vec<IndexId>,
        database: Database<RT>,
        index_writer: IndexWriter<RT>,
    ) -> anyhow::Result<()> {
        let _timer = tablet_index_backfill_timer();
        let mut backfills = BTreeMap::new();
        for index_id in &index_ids {
            let (index_name, retention_started) =
                Self::begin_backfill(*index_id, &database).await?;
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
            let ts = database.now_ts_for_reads();
            let snapshot = database.snapshot(ts)?;
            let table_mapping = snapshot.table_mapping();
            let table_name = &table_mapping.tablet_to_name()(tablet_id)?;
            tracing::info!(
                "Starting backfill of {} indexes for {table_name}: {needs_backfill:?}",
                needs_backfill.len()
            );
            let table_summary =
                snapshot.table_summary(table_mapping.tablet_namespace(tablet_id)?, table_name);
            let total_docs = table_summary.map(|summary| summary.num_values());
            let mut tx = database.begin_system().await?;
            let mut index_backfill_model = IndexBackfillModel::new(&mut tx);
            for index_id in needs_backfill.keys() {
                index_backfill_model
                    .initialize_backfill(*index_id, total_docs)
                    .await?;
            }
            database
                .commit_with_write_source(tx, "index_worker_backfill_initialization")
                .await?;
            let index_selector = IndexSelector::ManyIndexes {
                tablet_id,
                indexes: needs_backfill,
            };
            let index_registry = snapshot.index_registry;
            index_writer
                .perform_backfill(
                    ts,
                    &index_registry,
                    index_selector,
                    1,
                    Some(database.clone()),
                )
                .await?;
        }

        let mut min_begin_ts = None;
        let mut retention = BTreeMap::new();
        for index_id in &index_ids {
            let (backfill_begin_ts, index_name, indexed_fields) =
                Self::begin_retention(*index_id, &database).await?;

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
            index_writer.run_retention(min_begin_ts, retention).await?;
        }

        for index_id in index_ids {
            Self::finish_backfill(index_id, &database).await?;
        }

        Ok(())
    }

    async fn begin_backfill(
        index_id: IndexId,
        database: &Database<RT>,
    ) -> anyhow::Result<(TabletIndexName, bool)> {
        let mut tx = database.begin(Identity::system()).await?;
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
        index_id: IndexId,
        database: &Database<RT>,
    ) -> anyhow::Result<(RepeatableTimestamp, TabletIndexName, IndexedFields)> {
        let mut tx = database.begin(Identity::system()).await?;
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
        database
            .commit_with_write_source(tx, "index_worker_start_retention")
            .await?;

        Ok((index_ts, name, indexed_fields))
    }

    async fn finish_backfill(index_id: IndexId, database: &Database<RT>) -> anyhow::Result<()> {
        // Now that we're done, write that we've finished backfilling the index, sanity
        // checking that it wasn't written concurrently with our backfill.
        let mut tx = database.begin(Identity::system()).await?;
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
        database
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
