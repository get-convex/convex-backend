use std::{
    cmp,
    collections::BTreeMap,
    sync::Arc,
    time::Duration,
};

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
        INDEX_WORKERS_INITIAL_BACKOFF,
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
    value::{
        ResolvedDocumentId,
        TableMapping,
        TabletId,
    },
};
use futures::Future;
use indexing::index_registry::IndexRegistry;
use keybroker::Identity;
use value::{
    DeveloperDocumentId,
    TableNamespace,
};

use crate::{
    bootstrap_model::index_backfills::IndexBackfillModel,
    database_index_workers::index_writer::{
        IndexSelector,
        IndexWriter,
    },
    metrics::{
        index_backfill_timer,
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

const MAX_BACKOFF: Duration = Duration::from_secs(30);

pub struct IndexWorker<RT: Runtime> {
    database: Database<RT>,
    index_writer: IndexWriter<RT>,
    runtime: RT,
    backoff: Backoff,
    #[cfg(any(test, feature = "testing"))]
    should_terminate: bool,
    persistence_version: PersistenceVersion,
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

        use crate::database_index_workers::index_writer::IndexWriter;

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
