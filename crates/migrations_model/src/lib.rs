//! This crate exists to duplicate logic from the model crate
//! so that we can evolve model over time, while freezing the old version of the
//! model code in time at the time of the migration. This allows migrations to
//! continue to work long after the model has been updated.

#![feature(coroutines)]
#![feature(try_blocks)]
#![feature(impl_trait_in_assoc_type)]

use std::{
    borrow::Cow,
    fmt,
};

use common::runtime::Runtime;
use database::{
    Database,
    TableModel,
};
use value::{
    TableName,
    TableNamespace,
};

pub mod migr_119;
pub mod migr_121;

pub type DatabaseVersion = i64;
// The version for the format of the database. We support all previous
// migrations unless explicitly dropping support.
// Add a user name next to the version when you make a change to highlight merge
// conflicts.
pub const DATABASE_VERSION: DatabaseVersion = 122; // emma

pub struct MigrationExecutor<RT: Runtime> {
    pub db: Database<RT>,
}

impl<RT: Runtime> MigrationExecutor<RT> {
    pub async fn perform_migration(&self, to_version: DatabaseVersion) -> anyhow::Result<()> {
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
            120 => {
                // This is an empty migration because we added a new system
                // table, _index_backfills
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            121 => {
                let mut tx = self.db.begin_system().await?;
                migr_121::run_migration(&mut tx).await?;
                self.db
                    .commit_with_write_source(tx, "migration_121")
                    .await?;
                MigrationCompletionCriterion::MigrationComplete(to_version)
            },
            122 => {
                // This is an empty migration because we added a new system
                // table for each component, _schema_validation_progress
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
