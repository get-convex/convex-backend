use std::{
    cmp,
    collections::{
        BTreeMap,
        HashSet,
    },
    future::Future,
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
    runtime::{
        JoinMap,
        Runtime,
    },
    types::{
        IndexId,
        RepeatableTimestamp,
        TabletIndexName,
    },
};
use futures::FutureExt;
use hashlink::LinkedHashSet;
use keybroker::Identity;
use tokio::{
    select,
    sync::mpsc,
};
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
    TabletId,
};

use crate::{
    bootstrap_model::index_backfills::{
        types::BackfillCursor,
        IndexBackfillModel,
    },
    database_index_workers::index_writer::{
        IndexSelector,
        IndexWriter,
        TabletBackfillProgress,
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
    Transaction,
};

pub mod index_writer;

pub struct IndexWorker<RT: Runtime> {
    /// Index IDs that are currently being backfilled.
    in_progress_index_ids: HashSet<IndexId, ahash::RandomState>,
    /// The index backfill tasks
    in_progress: JoinMap<Vec<IndexId>, anyhow::Result<u64>>,
    /// Order-preserving HashSet that represents the order that pending index
    /// backfills will be processed. This does not include indexes that are
    /// `in_progress`.
    pending: LinkedHashSet<(IndexId, TabletId, Option<BackfillCursor>), ahash::RandomState>,
    /// Receiver for progress updates from `IndexWriter`'s tablet backfills.
    progress_rx: mpsc::Receiver<TabletBackfillProgress>,
    /// Limit on the size of `in_progress`
    max_concurrency: usize,
    metadata_mutex: Arc<tokio::sync::Mutex<()>>,
    database: Database<RT>,
    index_writer: IndexWriter<RT>,
    backoff: Backoff,
    runtime: RT,
    #[cfg(any(test, feature = "testing"))]
    pub should_terminate: bool,
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
        let (progress_tx, progress_rx) = mpsc::channel(100);
        let index_writer = IndexWriter::new(
            persistence.clone(),
            reader.clone(),
            retention_validator,
            runtime.clone(),
            Some(progress_tx),
        );
        let mut worker = IndexWorker {
            in_progress_index_ids: Default::default(),
            in_progress: JoinMap::new(),
            pending: Default::default(),
            progress_rx,
            max_concurrency: *INDEX_BACKFILL_CONCURRENCY,
            metadata_mutex: Default::default(),
            database,
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
    ) -> impl Future<Output = anyhow::Result<u64>> + Send {
        let mut total_docs_indexed = 0;
        let reader = persistence.reader();
        let (progress_tx, progress_rx) = mpsc::channel(10);
        let index_writer = IndexWriter::new(
            persistence.clone(),
            reader.clone(),
            retention_validator,
            runtime.clone(),
            Some(progress_tx),
        );
        let mut worker = IndexWorker {
            in_progress_index_ids: Default::default(),
            in_progress: JoinMap::new(),
            pending: Default::default(),
            progress_rx,
            max_concurrency: 10,
            metadata_mutex: Default::default(),
            database,
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
                if let Ok(docs_indexed) = r {
                    total_docs_indexed += docs_indexed;
                }
                if worker.in_progress.is_empty()
                    && worker.pending.is_empty()
                    && worker.progress_rx.is_empty()
                {
                    return r.map(|_| total_docs_indexed);
                }
            }
        }
    }

    /// Runs the index worker one loop, either backfilling an index or writing
    /// index backfill progress and returning how many rows were indexed if
    /// backfilling.
    async fn run(&mut self) -> anyhow::Result<u64> {
        // This is a counter for tests
        let mut docs_indexed = 0;
        // Get all the documents from the `_index` table.
        let mut tx = self.database.begin(Identity::system()).await?;
        // _index doesn't have `by_creation_time` index, and thus must use `by_id`.
        let index_documents = tx
            .query_system(TableNamespace::Global, &SystemIndex::<IndexTable>::by_id())?
            .all()
            .await?;
        let mut num_to_backfill = 0;
        let mut model = IndexBackfillModel::new(&mut tx);
        for index_metadata in &index_documents {
            if let IndexConfig::Database { on_disk_state, .. } = &index_metadata.config
                && matches!(on_disk_state, DatabaseIndexState::Backfilling(_))
            {
                let backfill_metadata = model
                    .existing_backfill_metadata(index_metadata.id().developer_id)
                    .await?;
                let backfill_cursor =
                    backfill_metadata.and_then(|metadata| metadata.cursor.clone());
                let index_id = index_metadata.id().internal_id();
                let tablet_id = *index_metadata.name.table();
                if !self.in_progress_index_ids.contains(&index_id)
                    && !self
                        .pending
                        .contains(&(index_id, tablet_id, backfill_cursor.clone()))
                {
                    self.pending
                        .insert((index_id, tablet_id, backfill_cursor.clone()));
                }
                num_to_backfill += 1;
            }
        }
        log_num_indexes_to_backfill(num_to_backfill);
        tracing::info!(
            "{num_to_backfill} database indexes to backfill @ {}",
            tx.begin_timestamp()
        );

        let token = tx.into_token()?;
        let subscription = self.database.subscribe(token).await?;

        #[cfg(any(test, feature = "testing"))]
        if self.should_terminate
            && self.in_progress.is_empty()
            && self.pending.is_empty()
            && self.progress_rx.is_empty()
        {
            return Ok(docs_indexed);
        }

        // Start new work if allowed by the concurrency limit
        while self.in_progress.len() < self.max_concurrency
            && let Some((index_id, tablet_id, backfill_cursor)) = self.pending.pop_front()
        {
            self.queue_index_backfill(index_id, tablet_id, backfill_cursor);
        }
        select! {
            biased;
            // Start by finding indexes that have finished backfilling
            res = self.in_progress.join_next(), if !self.in_progress.is_empty() => {
                let (index_ids, res) = res.expect("join_next cannot return None if nonempty");
                // First, make sure `in_progress_index_ids` is always consistent with `in_progress`
                for &index_id in &index_ids {
                    self.in_progress_index_ids.remove(&index_id);
                }
                // If backfill tasks are failing, return an error here so that we back off
                docs_indexed += res??;
                tracing::info!("Finished backfilling {index_ids:?}");
                // Return so that we possibly queue up more work
            }
            maybe_progress = self.progress_rx.recv() => {
                let Some(
                    TabletBackfillProgress {
                        tablet_id,
                        index_ids,
                        cursor,
                        num_docs_indexed,
                    }) = maybe_progress else {
                    anyhow::bail!("Database index backfill progress channel closed");
                };
                let mut tx = self.database.begin_system().await?;
                let table_number = tx.table_mapping().tablet_number(tablet_id)?;
                let mut model = IndexBackfillModel::new(&mut tx);
                let cursor = ResolvedDocumentId::new(
                    tablet_id,
                    DeveloperDocumentId::new(table_number, cursor.internal_id())
                );
                for index_id in index_ids {
                    model
                        .update_database_index_backfill_progress(
                            index_id,
                            tablet_id,
                            num_docs_indexed,
                            cursor
                        )
                        .await?;
                }
                self.database.commit_with_write_source(tx, "index_worker_backfill_progress")
                    .await?;
                }
            // Alternatively, wait for invalidation
            _ = subscription.wait_for_invalidation().fuse() => {
                self.backoff.reset();
            }
        }

        Ok(docs_indexed)
    }

    /// Spawns a task to process the next index backfill.
    fn queue_index_backfill(
        &mut self,
        index_id: IndexId,
        tablet_id: TabletId,
        backfill_cursor: Option<BackfillCursor>,
    ) {
        let mut index_ids = vec![index_id];
        // Since we're mainly limited by the speed of reading the table, let's
        // grab all the other pending indexes for this table at once
        self.pending.retain(
            |&(other_index_id, other_tablet_id, ref other_backfill_cursor)| {
                if other_tablet_id == tablet_id && other_backfill_cursor == &backfill_cursor {
                    index_ids.push(other_index_id);
                    false
                } else {
                    true
                }
            },
        );

        for &index_id in &index_ids {
            self.in_progress_index_ids.insert(index_id);
        }
        self.in_progress.spawn(
            "backfill_tablet",
            index_ids.clone(),
            Self::backfill_tablet(
                tablet_id,
                index_ids,
                self.database.clone(),
                self.index_writer.clone(),
                self.metadata_mutex.clone(),
                backfill_cursor,
            ),
        );
    }

    async fn backfill_tablet(
        tablet_id: TabletId,
        index_ids: Vec<IndexId>,
        database: Database<RT>,
        index_writer: IndexWriter<RT>,
        metadata_mutex: Arc<tokio::sync::Mutex<()>>,
        backfill_cursor: Option<BackfillCursor>,
    ) -> anyhow::Result<u64> {
        let mut docs_indexed = 0;
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
            let (ts, index_registry, cursor) = if let Some(backfill_cursor) = backfill_cursor
                && let Some(cursor) = backfill_cursor.cursor
            {
                let (latest_ts, snapshot) = database.latest_ts_and_snapshot()?;
                let snapshot_ts = latest_ts.prior_ts(backfill_cursor.snapshot_ts)?;
                let table_mapping = snapshot.table_mapping();
                let table_name = &table_mapping.tablet_to_name()(tablet_id)?;
                let index_registry = snapshot.index_registry;
                let cursor = ResolvedDocumentId::new(tablet_id, cursor);
                tracing::info!(
                    "Resuming backfill of {} indexes for {table_name} at ts {snapshot_ts}: \
                     {needs_backfill:?}",
                    needs_backfill.len(),
                );
                (snapshot_ts, index_registry, Some(cursor))
            } else {
                let mut tx = database.begin_system().await?;
                let ts = tx.begin_timestamp();
                let snapshot = database.snapshot(ts)?;
                let table_mapping = tx.table_mapping();
                let table_name = &table_mapping.tablet_to_name()(tablet_id)?;
                tracing::info!(
                    "Starting backfill of {} indexes for {table_name}: {needs_backfill:?}",
                    needs_backfill.len(),
                );
                let table_summary =
                    snapshot.table_summary(table_mapping.tablet_namespace(tablet_id)?, table_name);
                let total_docs = table_summary.map(|summary| summary.num_values());
                let mut index_backfill_model = IndexBackfillModel::new(&mut tx);
                for index_id in needs_backfill.keys() {
                    index_backfill_model
                        .initialize_database_index_backfill(*index_id, total_docs, *ts)
                        .await?;
                }
                database
                    .commit_with_write_source(tx, "index_worker_backfill_initialization")
                    .await?;
                let index_registry = snapshot.index_registry;
                (ts, index_registry, None)
            };
            let index_selector = IndexSelector::ManyIndexes {
                tablet_id,
                indexes: needs_backfill,
            };
            docs_indexed = index_writer
                .backfill_from_ts(ts, &index_registry, index_selector, 1, cursor)
                .await?;
        }

        let mut min_begin_ts = None;
        let mut retention = BTreeMap::new();
        // The database currently does not allow concurrent writers to the
        // `_index` (or `_tables`) tables; see a TODO in
        // `Writes::record_reads_for_write`.
        // Since we run many `backfill_tablet` tasks concurrently, synchronize
        // here to avoid creating OCC conflicts with ourselves.
        let indexes_lock = metadata_mutex.lock().await;
        let mut tx = database.begin(Identity::system()).await?;
        for index_id in &index_ids {
            let (backfill_begin_ts, index_name, indexed_fields) =
                Self::begin_retention(&mut tx, *index_id).await?;

            min_begin_ts = min_begin_ts
                .map(|t| cmp::min(t, backfill_begin_ts))
                .or(Some(backfill_begin_ts));

            retention.insert(*index_id, (index_name, indexed_fields));
        }
        database
            .commit_with_write_source(tx, "index_worker_start_retention")
            .await?;
        drop(indexes_lock);
        if let Some(min_begin_ts) = min_begin_ts {
            tracing::info!(
                "Started running retention for {} indexes: {retention:?}",
                retention.len()
            );
            index_writer.run_retention(min_begin_ts, retention).await?;
        }

        let indexes_lock = metadata_mutex.lock().await;
        let mut tx = database.begin(Identity::system()).await?;
        for index_id in index_ids {
            Self::finish_backfill(&mut tx, index_id).await?;
        }
        database
            .commit_with_write_source(tx, "index_worker_finish_backfill")
            .await?;
        drop(indexes_lock);

        Ok(docs_indexed)
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
        tx: &mut Transaction<RT>,
        index_id: IndexId,
    ) -> anyhow::Result<(RepeatableTimestamp, TabletIndexName, IndexedFields)> {
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
                spec,
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
                    spec.fields.clone(),
                )
            },
            _ => anyhow::bail!(
                "IndexWorker attempted to backfill an index {index_metadata:?} which wasn't a \
                 database index."
            ),
        };

        let name = index_metadata.name.clone();
        SystemMetadataModel::new_global(tx)
            .replace(index_metadata.id(), index_metadata.into_value().try_into()?)
            .await?;

        Ok((index_ts, name, indexed_fields))
    }

    async fn finish_backfill(tx: &mut Transaction<RT>, index_id: IndexId) -> anyhow::Result<()> {
        // Now that we're done, write that we've finished backfilling the index, sanity
        // checking that it wasn't written concurrently with our backfill.
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

        SystemMetadataModel::new_global(tx)
            .replace(full_index_id, index_metadata.into_value().try_into()?)
            .await?;
        let table_name = tx.table_mapping().tablet_name(*name.table())?;
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
