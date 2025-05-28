use std::{
    borrow::Cow,
    fmt,
    sync::Arc,
    time::Duration,
};

use common::{
    self,
    backoff::Backoff,
    errors::report_error,
    persistence::Persistence,
    runtime::Runtime,
};
use database::{
    Database,
    TableModel,
    Transaction,
};
use keybroker::Identity;
use migrations_model::migr_119;
use storage::Storage;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    database_globals::{
        types::DatabaseVersion,
        DatabaseGlobalsModel,
    },
    metrics::log_migration_worker_failed,
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
pub const DATABASE_VERSION: DatabaseVersion = 119; // nipunn

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
            1..=116 => panic!("Transition too old!"),
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
            // Empty migration for 118 - represents creation of CronNextRun table
            118 => MigrationCompletionCriterion::MigrationComplete(to_version),
            119 => {
                let mut tx = self.db.begin_system().await?;
                migr_119::run_migration(&mut tx).await?;
                self.db
                    .commit_with_write_source(tx, "migration_119")
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
