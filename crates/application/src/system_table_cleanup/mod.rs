use std::{
    collections::BTreeSet,
    sync::Arc,
    time::Duration,
};

use common::{
    backoff::Backoff,
    bootstrap_model::tables::{
        TableMetadata,
        TableState,
        TABLES_TABLE,
    },
    components::ComponentId,
    document::{
        CreationTime,
        ParseDocument,
        ParsedDocument,
        CREATION_TIME_FIELD_PATH,
        ID_FIELD_PATH,
    },
    errors::report_error,
    knobs::{
        MAX_EXPIRED_SNAPSHOT_AGE,
        MAX_IMPORT_AGE,
        MAX_SESSION_CLEANUP_DURATION,
        SESSION_CLEANUP_DELETE_CONCURRENCY,
        SYSTEM_TABLE_CLEANUP_CHUNK_SIZE,
        SYSTEM_TABLE_CLEANUP_FREQUENCY,
        SYSTEM_TABLE_ROWS_PER_SECOND,
    },
    query::{
        Expression,
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
use futures::{
    Future,
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use keybroker::Identity;
use metrics::{
    log_exports_s3_cleanup,
    log_system_table_cleanup_rows,
    log_system_table_cursor_lag,
    system_table_cleanup_timer,
};
use model::{
    exports::ExportsModel,
    modules::ModuleModel,
    session_requests::SESSION_REQUESTS_TABLE,
    source_packages::{
        types::SourcePackageId,
        SourcePackagesTable,
    },
    SystemIndex,
    SystemTable,
};
use rand::Rng;
use storage::Storage;
use tokio::sync::mpsc::{
    self,
    Receiver,
};
use tokio_stream::wrappers::ReceiverStream;
use value::{
    ConvexValue,
    ResolvedDocumentId,
    TableNamespace,
    TabletId,
};

use crate::system_table_cleanup::metrics::log_tablet_hard_deleted;

mod metrics;

const MAX_ORPHANED_TABLE_NAMESPACE_AGE: Duration = Duration::from_days(2);
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

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
        deleted_tablet_receiver: mpsc::Receiver<TabletId>,
    ) -> impl Future<Output = ()> + Send {
        let mut worker = SystemTableCleanupWorker {
            database: database.clone(),
            runtime: runtime.clone(),
            exports_storage,
        };
        async move {
            if MAX_SESSION_CLEANUP_DURATION.is_none() {
                tracing::error!(
                    "Forcibly disabling system table cleanup, exiting SystemTableCleanupWorker..."
                );
                return;
            }
            let _handle = runtime.clone().spawn(
                "cleanup_deleted_tablets",
                Self::cleanup_deleted_tablets(runtime, database, deleted_tablet_receiver),
            );
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
        let mut session_requests_delete_cursor = None;
        loop {
            // Jitter the wait between deletion runs to even out load.
            let delay = SYSTEM_TABLE_CLEANUP_FREQUENCY.mul_f32(self.runtime.rng().random());
            self.runtime.wait(delay).await;

            self.cleanup_hidden_tables().await?;
            self.cleanup_orphaned_table_namespaces().await?;
            self.cleanup_expired_exports().await?;
            self.cleanup_unused_source_packages().await?;

            // _session_requests are used to make mutations idempotent.
            // We can delete them after they are old enough that the client that
            // created the mutation must be gone.
            let session_requests_cutoff = match *MAX_SESSION_CLEANUP_DURATION {
                Some(duration) => {
                    Some((*self.database.now_ts_for_reads().sub(duration)?).try_into()?)
                },
                None => None,
            };
            // Preserve the deletion cursor between runs. This helps skip index tombstones.
            // Note that we only update the cursor after a successful run.
            (_, session_requests_delete_cursor) = self
                .cleanup_system_table(
                    TableNamespace::Global,
                    &SESSION_REQUESTS_TABLE,
                    session_requests_cutoff
                        .map_or(CreationTimeInterval::None, CreationTimeInterval::Before),
                    &rate_limiter,
                    *SESSION_CLEANUP_DELETE_CONCURRENCY,
                    session_requests_delete_cursor,
                )
                .await?;
        }
    }

    async fn cleanup_deleted_tablets(
        rt: RT,
        database: Database<RT>,
        mut deleted_tablet_receiver: Receiver<TabletId>,
    ) {
        let mut error_backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            let r: anyhow::Result<()> = async {
                while let Some(tablet_id) = deleted_tablet_receiver.recv().await {
                    let mut tx = database.begin_system().await?;
                    TableModel::new(&mut tx)
                        .hard_delete_tablet_document(tablet_id)
                        .await?;
                    database
                        .commit_with_write_source(tx, "cleanup_deleted_tablets")
                        .await?;
                    log_tablet_hard_deleted();
                    error_backoff.reset();
                }
                Ok(())
            }
            .await;
            match r {
                Ok(_) => {
                    tracing::info!(
                        "Deleted tablet channel closed, exiting cleanup_deleted_tablets"
                    );
                    return;
                },
                Err(e) => {
                    report_error(&mut e.context("cleanup_deleted_tablets failed")).await;
                    let delay = error_backoff.fail(&mut rt.rng());
                    rt.wait(delay).await;
                },
            }
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
                let table: ParsedDocument<TableMetadata> = document.parse()?;
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
        const MAX_TABLES_PER_RUN: usize = 1024;
        let mut deleted_tables = 0;
        'cleanup: for (namespace, map) in table_mapping.iter_active_namespaces() {
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
                    // Bypass schema enforcement to avoid inserting writes to
                    // _schemas, which can create a corrupt transaction if we
                    // end up deleting _schemas later.
                    table_model
                        .delete_table_by_id_bypassing_schema_enforcement(*tablet_id)
                        .await?;
                    deleted_tables += 1;
                }
            }

            if deleted_tables >= MAX_TABLES_PER_RUN {
                // Don't create an overly large transaction; we'll get
                // to the remaining tables on the next run.
                tracing::warn!(
                    "Hit the limit of {} tables, stopping early",
                    MAX_TABLES_PER_RUN
                );
                break 'cleanup;
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
        num_deleters: usize,
        mut cursor: Option<(CreationTime, ResolvedDocumentId)>,
    ) -> anyhow::Result<(usize, Option<(CreationTime, ResolvedDocumentId)>)> {
        let _timer = system_table_cleanup_timer();

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let deleter = |chunk: Vec<ResolvedDocumentId>| async {
            let deleted_chunk = self
                .cleanup_system_table_delete_chunk(namespace, table, chunk)
                .await?;

            for _ in 0..deleted_chunk {
                // Don't rate limit within transactions, because that would just increase
                // contention. Rate limit between transactions to limit
                // overall deletion speed.
                while let Err(not_until) = rate_limiter.check() {
                    let delay = not_until.wait_time_from(self.runtime.monotonic_now().into());
                    self.runtime.wait(delay).await;
                }
            }
            Ok(deleted_chunk)
        };
        let deleters = ReceiverStream::new(rx)
            .map(deleter)
            .buffer_unordered(num_deleters)
            .try_fold(0, |acc, x| async move { Ok(acc + x) });

        let reader = async move {
            loop {
                let deleted_chunk = self
                    .cleanup_system_table_read_chunk(namespace, table, to_delete, &mut cursor)
                    .await?;
                if deleted_chunk.is_empty() {
                    return Ok::<_, anyhow::Error>(());
                }
                tx.send(deleted_chunk).await?;
            }
        };

        let ((), deleted) = futures::try_join!(reader, deleters)?;
        Ok((deleted, cursor))
    }

    async fn cleanup_system_table_read_chunk(
        &self,
        namespace: TableNamespace,
        table: &TableName,
        to_delete: CreationTimeInterval,
        cursor: &mut Option<(CreationTime, ResolvedDocumentId)>,
    ) -> anyhow::Result<Vec<ResolvedDocumentId>> {
        let mut tx = self.database.begin(Identity::system()).await?;
        if !TableModel::new(&mut tx).table_exists(namespace, table) {
            return Ok(vec![]);
        }
        if matches!(to_delete, CreationTimeInterval::None) {
            return Ok(vec![]);
        }
        let mut range = match to_delete {
            CreationTimeInterval::None => return Ok(vec![]),
            CreationTimeInterval::All => vec![],
            CreationTimeInterval::Before(cutoff) => vec![IndexRangeExpression::Lt(
                CREATION_TIME_FIELD_PATH.clone(),
                f64::from(cutoff).into(),
            )],
        };
        if let Some((creation_time, _id)) = cursor {
            // The semantics of the cursor mean that all documents <= cursor have been
            // deleted, but retention might not have run yet, so we skip over their
            // tombstones.
            range.push(IndexRangeExpression::Gte(
                CREATION_TIME_FIELD_PATH.clone(),
                f64::from(*creation_time).into(),
            ));
        }
        let mut index_scan = Query::index_range(IndexRange {
            index_name: IndexName::by_creation_time(table.clone()),
            range,
            order: Order::Asc,
        });
        if let Some((creation_time, id)) = cursor {
            index_scan = index_scan.filter(Expression::Or(vec![
                Expression::Neq(
                    Box::new(Expression::Field(CREATION_TIME_FIELD_PATH.clone())),
                    Box::new(Expression::Literal(
                        ConvexValue::from(f64::from(*creation_time)).into(),
                    )),
                ),
                Expression::Gt(
                    Box::new(Expression::Field(ID_FIELD_PATH.clone())),
                    Box::new(Expression::Literal(ConvexValue::from(*id).into())),
                ),
            ]));
        }
        index_scan = index_scan.limit(*SYSTEM_TABLE_CLEANUP_CHUNK_SIZE);
        let mut query = ResolvedQuery::new(&mut tx, namespace, index_scan)?;
        let mut docs = vec![];
        while let Some(document) = query.next(&mut tx, None).await? {
            docs.push(document.id());
            *cursor = Some((document.creation_time(), document.id()));
        }
        if let Some((creation_time, _id)) = cursor {
            log_system_table_cursor_lag(table, *creation_time);
        }
        Ok(docs)
    }

    async fn cleanup_system_table_delete_chunk(
        &self,
        namespace: TableNamespace,
        table: &TableName,
        docs: Vec<ResolvedDocumentId>,
    ) -> anyhow::Result<usize> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let mut deleted_count = 0;
        for doc in docs {
            SystemMetadataModel::new(&mut tx, namespace)
                .delete(doc)
                .await?;
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

    async fn cleanup_unused_source_packages(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let mut num_deleted = 0;
        'deletes: for namespace in tx
            .table_mapping()
            .namespaces_for_name(SourcePackagesTable::table_name())
        {
            let mut source_package_ids: BTreeSet<SourcePackageId> = BTreeSet::new();
            for module in ModuleModel::new(&mut tx)
                .get_all_metadata(namespace.into())
                .await?
            {
                source_package_ids.insert(module.source_package_id);
            }
            for source_package in tx
                .query_system(namespace, &SystemIndex::<SourcePackagesTable>::by_id())?
                .all()
                .await?
            {
                let id = SourcePackageId::from(source_package.id().developer_id);
                if !source_package_ids.contains(&id) {
                    SystemMetadataModel::new(&mut tx, namespace)
                        .delete(source_package.id())
                        .await?;
                    num_deleted += 1;
                    if num_deleted >= 1000 {
                        break 'deletes;
                    }
                }
            }
        }
        if num_deleted > 0 {
            self.database
                .commit_with_write_source(tx, "cleanup_unused_source_packages")
                .await?;
            tracing::info!("Deleted {num_deleted} unreferenced SourcePackages");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum CreationTimeInterval {
    #[allow(dead_code)]
    All,
    None,
    Before(CreationTime),
}

#[cfg(test)]
mod tests {
    use std::{
        num::NonZeroU32,
        sync::Arc,
        time::Duration,
    };

    use common::{
        db_schema,
        document::CreationTime,
        identity::InertIdentity,
        runtime::{
            new_rate_limiter,
            Runtime,
        },
        schemas::DocumentSchema,
    };
    use database::{
        test_helpers::DbFixtures,
        SchemaModel,
    };
    use governor::Quota;
    use keybroker::Identity;
    use model::{
        components::config::ComponentConfigModel,
        session_requests::{
            types::{
                SessionRequestOutcome,
                SessionRequestRecord,
            },
            SessionRequestModel,
            SESSION_REQUESTS_TABLE,
        },
        test_helpers::DbFixturesWithModel,
    };
    use runtime::testing::TestRuntime;
    use storage::LocalDirStorage;
    use sync_types::SessionId;
    use value::{
        ConvexValue,
        JsonPackedValue,
        TableNamespace,
    };

    use crate::system_table_cleanup::{
        CreationTimeInterval,
        SystemTableCleanupWorker,
        MAX_ORPHANED_TABLE_NAMESPACE_AGE,
    };

    async fn test_system_table_cleanup_helper(
        rt: TestRuntime,
        num_deleters: usize,
    ) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let exports_storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let worker = SystemTableCleanupWorker {
            database: db.clone(),
            runtime: rt.clone(),
            exports_storage: exports_storage.clone(),
        };

        let mut creation_times = vec![];
        for _ in 0..10 {
            let mut tx = db.begin_system().await?;
            SessionRequestModel::new(&mut tx)
                .record_session_request(
                    SessionRequestRecord {
                        session_id: SessionId::new(rt.new_uuid_v4()),
                        request_id: 0,
                        outcome: SessionRequestOutcome::Mutation {
                            result: JsonPackedValue::pack(ConvexValue::Null),
                            log_lines: vec![].into(),
                        },
                        identity: InertIdentity::System,
                    },
                    Identity::system(),
                )
                .await?;
            creation_times.push(*tx.begin_timestamp());
            db.commit(tx).await?;
            rt.advance_time(Duration::from_secs(1)).await;
        }

        let cutoff = CreationTime::try_from(creation_times[4])?;
        let rate_limiter =
            new_rate_limiter(rt.clone(), Quota::per_second(NonZeroU32::new(10).unwrap()));

        let (deleted, _cursor) = worker
            .cleanup_system_table(
                TableNamespace::Global,
                &SESSION_REQUESTS_TABLE,
                CreationTimeInterval::Before(cutoff),
                &rate_limiter,
                num_deleters,
                None,
            )
            .await?;
        assert_eq!(deleted, 3);

        let count = db
            .begin_system()
            .await?
            .count(TableNamespace::Global, &SESSION_REQUESTS_TABLE)
            .await?;
        assert_eq!(count, Some(7));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_system_table_cleanup_1(rt: TestRuntime) -> anyhow::Result<()> {
        test_system_table_cleanup_helper(rt, 1).await
    }

    #[convex_macro::test_runtime]
    async fn test_system_table_cleanup_2(rt: TestRuntime) -> anyhow::Result<()> {
        test_system_table_cleanup_helper(rt, 2).await
    }

    #[convex_macro::test_runtime]
    async fn test_system_table_cleanup_8(rt: TestRuntime) -> anyhow::Result<()> {
        test_system_table_cleanup_helper(rt, 8).await
    }

    #[convex_macro::test_runtime]
    async fn test_cleanup_orphaned_table_namespaces_with_schema(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let exports_storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let worker = SystemTableCleanupWorker {
            database: db.clone(),
            runtime: rt.clone(),
            exports_storage: exports_storage.clone(),
        };
        let mut tx = db.begin_system().await?;
        // Create an orphaned table with a schema.
        // In particular, test the case of a table starting with an uppercase
        // letter, because uppercase letters sort after the underscore character
        // in ASCII, and therefore the table name sorts after system tables.
        let component = ComponentConfigModel::new(&mut tx)
            .initialize_component_namespace(false /* is_root */)
            .await?;
        SchemaModel::new(&mut tx, TableNamespace::ByComponent(component))
            .submit_pending(
                db_schema!("UserTable" => DocumentSchema::Any, "user_table" => DocumentSchema::Any),
            )
            .await?;
        db.commit(tx).await?;
        rt.advance_time(MAX_ORPHANED_TABLE_NAMESPACE_AGE * 2).await;
        db.bump_max_repeatable_ts().await?;
        worker.cleanup_orphaned_table_namespaces().await?;
        assert_eq!(
            db.latest_snapshot()?
                .table_mapping()
                .namespace(TableNamespace::ByComponent(component))
                .iter()
                .count(),
            0
        );
        Ok(())
    }
}
