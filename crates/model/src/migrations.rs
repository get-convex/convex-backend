use std::{
    borrow::Cow,
    fmt,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use common::{
    backoff::Backoff,
    document::{
        ParseDocument,
        ParsedDocument,
    },
    errors::report_error,
    persistence::Persistence,
    runtime::Runtime,
    try_chunks::TryChunksExt,
};
use database::{
    defaults::system_index,
    Database,
    IndexModel,
    SystemMetadataModel,
    TableModel,
    Transaction,
};
use futures::{
    StreamExt,
    TryStreamExt,
};
use keybroker::Identity;
use storage::Storage;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    canonical_urls::CANONICAL_URLS_TABLE,
    database_globals::{
        types::DatabaseVersion,
        DatabaseGlobalsModel,
    },
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    exports::{
        types::ExportFormat,
        ExportsModel,
        EXPORTS_TABLE,
    },
    metrics::log_migration_worker_failed,
    snapshot_imports::SnapshotImportModel,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(60);
const MAX_BACKOFF: Duration = Duration::from_secs(3600);

pub enum MigrationCompletionCriterion {
    /// Committing the migration in migrations.rs is sufficient.
    MigrationComplete(DatabaseVersion),
    /// Some other log line printed out, e.g. by a background worker creating
    /// a new index or a backfill.
    LogLine(Cow<'static, str>),
}

impl fmt::Display for MigrationCompletionCriterion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationCompletionCriterion::MigrationComplete(version) => {
                write!(f, "Wait for log line 'Migrated {version}'")
            },
            MigrationCompletionCriterion::LogLine(line) => write!(f, "Wait for log line '{line}'"),
        }
    }
}

// The version for the format of the database. We support all previous
// migrations unless explicitly dropping support.
// Add a user name next to the version when you make a change to highlight merge
// conflicts.
pub const DATABASE_VERSION: DatabaseVersion = 117; // nipunn

pub struct MigrationWorker<RT: Runtime> {
    rt: RT,
    db: Database<RT>,
    _modules_storage: Arc<dyn Storage>,
    _persistence: Arc<dyn Persistence>,
}

impl<RT: Runtime> MigrationWorker<RT> {
    pub fn new(
        rt: RT,
        persistence: Arc<dyn Persistence>,
        db: Database<RT>,
        modules_storage: Arc<dyn Storage>,
    ) -> Self {
        Self {
            rt,
            _modules_storage: modules_storage,
            _persistence: persistence,
            db,
        }
    }

    pub async fn go(self) {
        let mut backoff: Backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            tracing::info!("Attempting migration");
            match self.attempt_migrations().await {
                Ok(()) => break,
                Err(mut e) => {
                    log_migration_worker_failed();
                    let delay = backoff.fail(&mut self.rt.rng());
                    tracing::error!("Migration worker failed, sleeping {delay:?}");
                    report_error(&mut e).await;
                    self.rt.wait(delay).await;
                },
            }
        }
        tracing::info!("Migration complete");
    }

    async fn attempt_migrations(&self) -> anyhow::Result<()> {
        loop {
            let mut tx: Transaction<_> = self.db.begin(Identity::system()).await?;

            let database_globals = DatabaseGlobalsModel::new(&mut tx)
                .database_globals()
                .await?;
            let persisted_version = database_globals.version;

            // Since the migration might take a long time, we will create a new
            // transaction at the end instead of trying to commit this one.
            drop(tx);

            match persisted_version {
                1..DATABASE_VERSION => {
                    tracing::info!("Migrating to {}", persisted_version + 1);
                    self.perform_migration(persisted_version + 1).await?;

                    // Update database globals in a new transaction.
                    let mut tx: Transaction<_> = self.db.begin(Identity::system()).await?;
                    let mut globals_model = DatabaseGlobalsModel::new(&mut tx);
                    let mut database_globals = globals_model.database_globals().await?;
                    anyhow::ensure!(
                        persisted_version == database_globals.version,
                        "Persisted version changed while performing a migration: Expected {}. Got \
                         {}",
                        persisted_version,
                        database_globals.version
                    );
                    let new_version = persisted_version + 1;
                    database_globals.version = new_version;
                    globals_model
                        .replace_database_globals(database_globals)
                        .await?;
                    self.db
                        .commit_with_write_source(tx, "migrate_persisted_version")
                        .await?;
                    tracing::info!("Migrated {}", new_version);
                },
                DATABASE_VERSION => {
                    tracing::info!("db metadata version up to date at {}", DATABASE_VERSION);
                    break;
                },
                _ => {
                    // TODO(presley): Do we want to limit how far we go back to
                    // avoid accidentally pushing very old binary?
                    tracing::warn!(
                        "persisted db metadata version is ahead at {}, this binary is at {}",
                        persisted_version,
                        DATABASE_VERSION
                    );
                    break;
                },
            };
        }
        Ok(())
    }

    async fn perform_migration(&self, to_version: DatabaseVersion) -> anyhow::Result<()> {
        let completion_criterion = match to_version {
            1..=104 => panic!("Transition too old!"),
            105 => {
                // Delete all exports in non-zip formats (CleanJsonl and InternalJson)
                let mut tx = self.db.begin_system().await?;
                let mut exports_model = ExportsModel::new(&mut tx);
                let exports = exports_model.list().await?;
                for export in exports {
                    if !matches!(export.format(), ExportFormat::Zip { .. }) {
                        SystemMetadataModel::new_global(&mut tx)
                            .delete(export.id())
                            .await?;
                    }
                }

                let exports_by_state_index = system_index(&EXPORTS_TABLE, "by_state");
                IndexModel::new(&mut tx)
                    .drop_system_index(TableNamespace::Global, exports_by_state_index)
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            106 => {
                // Ugh - try 105 again but actually commit the transaction
                // Delete all exports in non-zip formats (CleanJsonl and InternalJson)
                let mut tx = self.db.begin_system().await?;
                let mut exports_model = ExportsModel::new(&mut tx);
                let exports = exports_model.list().await?;
                for export in exports {
                    let mut system_model = SystemMetadataModel::new_global(&mut tx);
                    if !matches!(export.format(), ExportFormat::Zip { .. }) {
                        system_model.delete(export.id()).await?;
                    } else {
                        // rewrite out things in the zip format to convert from
                        // raw string -> object
                        system_model
                            .replace(export.id(), export.into_value().try_into()?)
                            .await?;
                    }
                }

                let exports_by_state_index = system_index(&EXPORTS_TABLE, "by_state");
                IndexModel::new(&mut tx)
                    .drop_system_index(TableNamespace::Global, exports_by_state_index)
                    .await?;
                self.db
                    .commit_with_write_source(tx, "migration_106")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            107 => {
                let mut tx = self.db.begin_system().await?;
                let mut exports_model = ExportsModel::new(&mut tx);
                let exports = exports_model.list().await?;
                for export in exports {
                    let mut system_model = SystemMetadataModel::new_global(&mut tx);
                    // rewrite out things to add new requestor column
                    system_model
                        .replace(export.id(), export.into_value().try_into()?)
                        .await?;
                }
                self.db
                    .commit_with_write_source(tx, "migration_107")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            108 => {
                // Drop the exports_by_requestor index to prepare to recreate it
                // with different fields.
                let mut tx = self.db.begin_system().await?;
                let exports_by_requestor = system_index(&EXPORTS_TABLE, "by_requestor");
                IndexModel::new(&mut tx)
                    .drop_system_index(TableNamespace::Global, exports_by_requestor)
                    .await?;
                self.db
                    .commit_with_write_source(tx, "migration_108")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            109 => {
                // Drop the exports_by_requestor index AGAIN
                let mut tx = self.db.begin_system().await?;
                let exports_by_requestor = system_index(&EXPORTS_TABLE, "by_requestor");
                IndexModel::new(&mut tx)
                    .drop_system_index(TableNamespace::Global, exports_by_requestor)
                    .await?;
                self.db
                    .commit_with_write_source(tx, "migration_109")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            110 => {
                // Empty migration corresponding to _exports.by_requestor
                // creation
                MigrationCompletionCriterion::LogLine(
                    "Finished backfill of system index _exports.by_requestor".into(),
                )
            },
            111 => {
                let virtual_tables_table: TableName = "_virtual_tables"
                    .parse()
                    .expect("Invalid built-in virtual_tables table");
                let mut tx = self.db.begin_system().await?;
                TableModel::new(&mut tx)
                    .delete_active_table(TableNamespace::Global, virtual_tables_table)
                    .await?;
                self.db
                    .commit_with_write_source(tx, "migration_111")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            112 => {
                // Read in / write out exports table to process new expiration_ts
                // column.
                let mut tx = self.db.begin_system().await?;
                let mut exports_model = ExportsModel::new(&mut tx);
                let exports = exports_model.list().await?;
                for export in exports {
                    let mut system_model = SystemMetadataModel::new_global(&mut tx);
                    system_model
                        .replace(export.id(), export.into_value().try_into()?)
                        .await?;
                }
                self.db
                    .commit_with_write_source(tx, "migration_112")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            113 => {
                // Read in / write out imports table to process new requestor column.
                let mut tx = self.db.begin_system().await?;
                let mut imports_model = SnapshotImportModel::new(&mut tx);
                let imports = imports_model.list().await?;
                for import in imports {
                    let mut system_model = SystemMetadataModel::new_global(&mut tx);
                    system_model
                        .replace(import.id(), import.into_value().try_into()?)
                        .await?;
                }
                self.db
                    .commit_with_write_source(tx, "migration_113")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            114 => {
                // Read in / write out deployment audit logs table to process new requestor, and
                // table_names_deleted columns.
                let mut tx = self.db.begin_system().await?;
                let mut audit_log_model = DeploymentAuditLogModel::new(&mut tx);
                let mut chunked_stream = audit_log_model.list().try_chunks2(50);
                while let Some(entries) = chunked_stream.next().await {
                    let entries = entries?;
                    let mut tx_inner = self.db.begin_system().await?;
                    let mut system_model = SystemMetadataModel::new_global(&mut tx_inner);
                    for entry in entries {
                        system_model
                            .replace(entry.id(), entry.into_value().try_into()?)
                            .await?;
                    }
                    self.db
                        .commit_with_write_source(tx_inner, "migration_114")
                        .await?;
                }
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            115 => {
                // Read in / write out deployment audit logs table to process new
                // component-awareness in the table_names and table_names_deleted columns.
                let mut tx = self.db.begin_system().await?;
                let mut audit_log_model = DeploymentAuditLogModel::new(&mut tx);
                let all_entries: Vec<_> = audit_log_model
                    .list()
                    .map_ok(|entry| entry.id())
                    .try_collect()
                    .await?;
                drop(tx);
                for chunk in all_entries.chunks(20) {
                    let mut tx_inner = self.db.begin_system().await?;
                    let mut system_model = SystemMetadataModel::new_global(&mut tx_inner);
                    for id in chunk {
                        let entry: ParsedDocument<DeploymentAuditLogEvent> = system_model
                            .get(*id)
                            .await?
                            .context("Id missing?")?
                            .parse()?;
                        system_model
                            .replace(entry.id(), entry.clone().into_value().try_into()?)
                            .await?;
                    }
                    self.db
                        .commit_with_write_source(tx_inner, "migration_115")
                        .await?;
                }
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            116 => MigrationCompletionCriterion::LogLine(
                format!("Created system table: {}", *CANONICAL_URLS_TABLE).into(),
            ),
            117 => {
                let backend_serving_record_table: TableName = "_backend_serving_record"
                    .parse()
                    .expect("Invalid built-in backend_serving_record table");
                let mut tx = self.db.begin_system().await?;
                TableModel::new(&mut tx)
                    .delete_active_table(TableNamespace::Global, backend_serving_record_table)
                    .await?;
                self.db
                    .commit_with_write_source(tx, "migration_117")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            // NOTE: Make sure to increase DATABASE_VERSION when adding new migrations.
            _ => anyhow::bail!("Version did not define a migration! {}", to_version),
        };
        tracing::warn!(
            "Executing Migration {}/{}. {}",
            to_version,
            DATABASE_VERSION,
            completion_criterion
        );
        Ok(())
    }
}
