use std::{
    sync::Arc,
    time::Duration,
};

use common::{
    bootstrap_model::tables::{
        TableMetadata,
        TableState,
        TABLES_TABLE,
    },
    components::ComponentId,
    document::{
        CreationTime,
        ParsedDocument,
        CREATION_TIME_FIELD_PATH,
    },
    errors::report_error,
    knobs::{
        MAX_EXPIRED_SNAPSHOT_AGE,
        MAX_IMPORT_AGE,
        MAX_SESSION_CLEANUP_DURATION,
        SYSTEM_TABLE_CLEANUP_CHUNK_SIZE,
        SYSTEM_TABLE_CLEANUP_FREQUENCY,
        SYSTEM_TABLE_ROWS_PER_SECOND,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::{
        new_rate_limiter,
        RateLimiter,
        Runtime,
    },
    types::{
        IndexName,
        TableName,
    },
};
use database::{
    query::PaginationOptions,
    BootstrapComponentsModel,
    Database,
    ResolvedQuery,
    SystemMetadataModel,
    TableModel,
};
use futures::Future;
use governor::Quota;
use keybroker::Identity;
use metrics::{
    log_exports_s3_cleanup,
    log_system_table_cleanup_rows,
    system_table_cleanup_timer,
};
use model::{
    exports::ExportsModel,
    session_requests::SESSION_REQUESTS_TABLE,
};
use rand::Rng;
use storage::Storage;
use value::{
    TableNamespace,
    TabletId,
};

mod metrics;

static MAX_ORPHANED_TABLE_NAMESPACE_AGE: Duration = Duration::from_days(2);

pub struct SystemTableCleanupWorker<RT: Runtime> {
    database: Database<RT>,
    runtime: RT,
    exports_storage: Arc<dyn Storage>,
}

impl<RT: Runtime> SystemTableCleanupWorker<RT> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        runtime: RT,
        database: Database<RT>,
        exports_storage: Arc<dyn Storage>,
    ) -> impl Future<Output = ()> + Send {
        let mut worker = SystemTableCleanupWorker {
            database,
            runtime,
            exports_storage,
        };
        async move {
            if MAX_SESSION_CLEANUP_DURATION.is_none() {
                tracing::error!(
                    "Forcibly disabling system table cleanup, exiting SystemTableCleanupWorker..."
                );
                return;
            }
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("SystemTableCleanupWorker died")).await;
                }
            }
        }
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting SystemTableCleanupWorker");
        let rate_limiter = new_rate_limiter(
            self.runtime.clone(),
            Quota::per_second(*SYSTEM_TABLE_ROWS_PER_SECOND),
        );
        loop {
            // Jitter the wait between deletion runs to even out load.
            let delay = SYSTEM_TABLE_CLEANUP_FREQUENCY.mul_f32(self.runtime.rng().random());
            self.runtime.wait(delay).await;

            self.cleanup_hidden_tables().await?;
            self.cleanup_orphaned_table_namespaces().await?;
            self.cleanup_expired_exports().await?;

            // _session_requests are used to make mutations idempotent.
            // We can delete them after they are old enough that the client that
            // created the mutation must be gone.
            let session_requests_cutoff = match *MAX_SESSION_CLEANUP_DURATION {
                Some(duration) => {
                    Some((*self.database.now_ts_for_reads().sub(duration)?).try_into()?)
                },
                None => None,
            };
            self.cleanup_system_table(
                TableNamespace::Global,
                &SESSION_REQUESTS_TABLE,
                session_requests_cutoff
                    .map_or(CreationTimeInterval::None, CreationTimeInterval::Before),
                &rate_limiter,
            )
            .await?;
        }
    }

    async fn cleanup_hidden_tables(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;

        let mut num_deleted = 0;
        let query = Query::full_table_scan(TABLES_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(&mut tx, TableNamespace::Global, query.clone())?;
        {
            while let Some(document) = query_stream.next(&mut tx, None).await? {
                // Limit rows read and rows deleted to avoid hitting transaction limits size.
                if query_stream.is_approaching_data_limit() || num_deleted > 1000 {
                    let cursor = query_stream.cursor();
                    self.database
                        .commit_with_write_source(tx, "system_table_cleanup")
                        .await?;
                    tracing::info!("Deleted {num_deleted} hidden tables");
                    num_deleted = 0;
                    tx = self.database.begin(Identity::system()).await?;
                    query_stream = ResolvedQuery::new_bounded(
                        &mut tx,
                        TableNamespace::Global,
                        query.clone(),
                        PaginationOptions::ManualPagination {
                            start_cursor: cursor,
                            maximum_rows_read: None,
                            maximum_bytes_read: None,
                        },
                        None,
                        database::query::TableFilter::IncludePrivateSystemTables,
                    )?;
                }
                let table: ParsedDocument<TableMetadata> = document.try_into()?;
                match table.state {
                    TableState::Active | TableState::Deleting => {},
                    TableState::Hidden => {
                        let now = CreationTime::try_from(*self.database.now_ts_for_reads())?;
                        let creation_time = table.creation_time();
                        let age = Duration::from_millis(
                            (f64::from(now) - f64::from(creation_time)) as u64,
                        );
                        // Mark as deleting if hidden for more than twice the max import age.
                        if age > 2 * (*MAX_IMPORT_AGE) {
                            let table_id = TabletId(table.id().internal_id());
                            tracing::info!("Deleting hidden table: {table_id:?}");
                            TableModel::new(&mut tx)
                                .delete_hidden_table(table_id)
                                .await?;
                            num_deleted += 1;
                        }
                    },
                };
            }
        }

        if num_deleted > 0 {
            self.database
                .commit_with_write_source(tx, "system_table_cleanup")
                .await?;
            tracing::info!("Deleted {num_deleted} hidden tables");
        }

        Ok(())
    }

    /// Delete table namespaces that are not associated with any component.
    /// This can occur when a push does not complete successfully, where
    /// `start_push` initializes component system tables in a new namespace
    /// but `finish_push` never commits the component.
    async fn cleanup_orphaned_table_namespaces(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let ts = tx.begin_timestamp();
        let table_mapping = tx.table_mapping().clone();
        let component_paths = BootstrapComponentsModel::new(&mut tx).all_component_paths();
        let mut table_model = TableModel::new(&mut tx);
        for (namespace, map) in table_mapping.iter_active_namespaces() {
            let component_id = ComponentId::from(*namespace);
            if component_paths.contains_key(&component_id) {
                continue;
            }
            for (table_name, tablet_id) in map.iter() {
                // Ensure user tables are empty before deleting.
                if !table_name.is_system() {
                    let count = table_model.must_count(*namespace, table_name).await?;
                    anyhow::ensure!(
                        count == 0,
                        "Non-system table {table_name} found with {count} documents in orphaned \
                         table namespace component id: {component_id:?}"
                    );
                }
                let table_metadata = table_model.get_table_metadata(*tablet_id).await?;
                let now = CreationTime::try_from(*ts)?;
                let creation_time = table_metadata.creation_time();
                let age = Duration::from_millis((f64::from(now) - f64::from(creation_time)) as u64);
                if age > MAX_ORPHANED_TABLE_NAMESPACE_AGE {
                    tracing::info!(
                        "Deleting orphaned table {table_name:?} in non-existent component \
                         {component_id:?}"
                    );
                    table_model
                        .delete_table(*namespace, table_name.clone())
                        .await?;
                }
            }
        }
        self.database
            .commit_with_write_source(tx, "system_table_cleanup")
            .await?;
        Ok(())
    }

    async fn cleanup_system_table(
        &self,
        namespace: TableNamespace,
        table: &TableName,
        to_delete: CreationTimeInterval,
        rate_limiter: &RateLimiter<RT>,
    ) -> anyhow::Result<usize> {
        let mut cursor = None;

        let mut deleted = 0;
        loop {
            let _timer = system_table_cleanup_timer();
            let deleted_chunk = self
                .cleanup_system_table_chunk(namespace, table, to_delete, &mut cursor)
                .await?;
            deleted += deleted_chunk;
            if deleted_chunk == 0 {
                break Ok(deleted);
            }
            for _ in 0..deleted_chunk {
                // Don't rate limit within transactions, because that would just increase
                // contention. Rate limit between transactions to limit
                // overall deletion speed.
                while let Err(not_until) = rate_limiter.check() {
                    let delay = not_until.wait_time_from(self.runtime.monotonic_now().into());
                    self.runtime.wait(delay).await;
                }
            }
        }
    }

    async fn cleanup_system_table_chunk(
        &self,
        namespace: TableNamespace,
        table: &TableName,
        to_delete: CreationTimeInterval,
        cursor: &mut Option<CreationTime>,
    ) -> anyhow::Result<usize> {
        let mut tx = self.database.begin(Identity::system()).await?;
        if !TableModel::new(&mut tx).table_exists(namespace, table) {
            return Ok(0);
        }
        if matches!(to_delete, CreationTimeInterval::None) {
            return Ok(0);
        }
        let mut range = match to_delete {
            CreationTimeInterval::None => return Ok(0),
            CreationTimeInterval::All => vec![],
            CreationTimeInterval::Before(cutoff) => vec![IndexRangeExpression::Lt(
                CREATION_TIME_FIELD_PATH.clone(),
                f64::from(cutoff).into(),
            )],
        };
        if let Some(cursor) = cursor {
            // The semantics of the cursor mean that all documents <= cursor have been
            // deleted, but retention might not have run yet, so we skip over their
            // tombstones.
            range.push(IndexRangeExpression::Gt(
                CREATION_TIME_FIELD_PATH.clone(),
                f64::from(*cursor).into(),
            ));
        }
        let index_scan = Query::index_range(IndexRange {
            index_name: IndexName::by_creation_time(table.clone()),
            range,
            order: Order::Asc,
        })
        .limit(*SYSTEM_TABLE_CLEANUP_CHUNK_SIZE);
        let mut query = ResolvedQuery::new(&mut tx, namespace, index_scan)?;
        let mut deleted_count = 0;
        while let Some(document) = query.next(&mut tx, None).await? {
            SystemMetadataModel::new(&mut tx, namespace)
                .delete(document.id())
                .await?;
            *cursor = Some(document.creation_time());
            deleted_count += 1;
        }
        if deleted_count == 0 {
            return Ok(0);
        }
        self.database
            .commit_with_write_source(tx, "system_table_cleanup")
            .await?;
        tracing::info!("deleted {deleted_count} documents from {table}");
        log_system_table_cleanup_rows(table, deleted_count);
        Ok(deleted_count)
    }

    async fn cleanup_expired_exports(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let object_keys_to_del = ExportsModel::new(&mut tx)
            .cleanup_expired(*MAX_EXPIRED_SNAPSHOT_AGE)
            .await?;
        let num_deleted = object_keys_to_del.len();
        for object_key in object_keys_to_del {
            self.exports_storage.delete_object(&object_key).await?;
            log_exports_s3_cleanup();
        }
        self.database
            .commit_with_write_source(tx, "system_table_cleanup")
            .await?;
        if num_deleted > 0 {
            tracing::info!("Deleted {num_deleted} expired snapshots");
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum CreationTimeInterval {
    #[allow(dead_code)]
    All,
    None,
    Before(CreationTime),
}
