//! Authoritative metadata in our system is stored in tables with prefix
//! [`METADATA_PREFIX`]. Each file in this module stores a category of system
//! metadata.
//!
//! Users are not allowed to create table names that start with
//! [`METADATA_PREFIX`]. This choice is similar to to the `_id` field
//! automatically inserted into documents, as the user is not allowed to mutate
//! those either.
//!
//! The core design principle here is that all authoritative system metadata is
//! stored as a document and can be read by UDFs and subscribed to, just like
//! any other document. However, we do *not* allow the user to mutate these
//! through the general purpose `insert`, `update`, `replace` and `delete` APIs,
//! since they have stronger invariants than regular documents. Instead, we
//! provide special purpose APIs for the restricted modifications we'd like to
//! allow. Linux's `procfs` is an inspiration here, where it's useful to present
//! system data as regular files, but most mutations don't make much sense.

#![feature(assert_matches)]
#![feature(coroutines)]
#![feature(result_flattening)]
#![feature(iter_advance_by)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(lazy_cell)]
#![feature(const_option)]
#![feature(is_sorted)]
#![feature(iterator_try_collect)]
#![feature(never_type)]
#![feature(try_blocks)]
#![feature(exclusive_range_pattern)]
#![feature(async_closure)]
#![feature(trait_upcasting)]
#![feature(impl_trait_in_assoc_type)]
#![feature(iter_from_coroutine)]

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::LazyLock,
};

use backend_state::BackendStateTable;
use common::{
    bootstrap_model::index::{
        IndexConfig,
        IndexMetadata,
    },
    document::CREATION_TIME_FIELD_PATH,
    runtime::Runtime,
    types::{
        IndexName,
        TabletIndexName,
    },
    virtual_system_mapping::VirtualSystemMapping,
};
use components::handles::{
    FunctionHandlesTable,
    BY_COMPONENT_PATH_INDEX,
};
use cron_jobs::{
    CRON_JOBS_INDEX_BY_NAME,
    CRON_JOBS_INDEX_BY_NEXT_TS,
    CRON_JOB_LOGS_INDEX_BY_NAME_TS,
};
pub use database::defaults::{
    SystemIndex,
    SystemTable,
};
use database::{
    ComponentDefinitionsTable,
    ComponentsTable,
    Database,
    IndexModel,
    IndexTable,
    IndexWorkerMetadataTable,
    SchemasTable,
    TablesTable,
    Transaction,
    NUM_RESERVED_LEGACY_TABLE_NUMBERS,
};
use environment_variables::ENVIRONMENT_VARIABLES_INDEX_BY_NAME;
use exports::EXPORTS_BY_STATE_AND_TS_INDEX;
use file_storage::FILE_STORAGE_ID_INDEX;
use keybroker::Identity;
use maplit::btreeset;
use modules::{
    MODULE_INDEX_BY_DELETED,
    MODULE_INDEX_BY_PATH,
};
use scheduled_jobs::{
    SCHEDULED_JOBS_INDEX,
    SCHEDULED_JOBS_INDEX_BY_COMPLETED_TS,
    SCHEDULED_JOBS_INDEX_BY_UDF_PATH,
};
use session_requests::SESSION_REQUESTS_INDEX;
use strum::IntoEnumIterator;
pub use value::METADATA_PREFIX;
use value::{
    TableName,
    TableNamespace,
    TableNumber,
};

use crate::{
    auth::AuthTable,
    backend_state::BackendStateModel,
    cron_jobs::{
        CronJobLogsTable,
        CronJobsTable,
    },
    deployment_audit_log::DeploymentAuditLogsTable,
    environment_variables::EnvironmentVariablesTable,
    exports::ExportsTable,
    external_packages::ExternalPackagesTable,
    file_storage::FileStorageTable,
    modules::ModulesTable,
    scheduled_jobs::ScheduledJobsTable,
    session_requests::SessionRequestsTable,
    snapshot_imports::SnapshotImportsTable,
    source_packages::SourcePackagesTable,
    udf_config::UdfConfigTable,
};

pub mod auth;
pub mod backend_state;
pub mod components;
pub mod config;
pub mod cron_jobs;
pub mod deployment_audit_log;
pub mod environment_variables;
pub mod exports;
pub mod external_packages;
pub mod file_storage;
pub mod modules;
pub mod scheduled_jobs;
pub mod session_requests;
pub mod snapshot_imports;
pub mod source_packages;
pub mod udf_config;

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;

/// Default best effort table number when creating the table. If it is taken
/// already, another number is selected. Legacy deployments don't
/// respect this at all. Consistency is mainly nice for making snapshot
/// import/export more likely to work nicely.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, strum::EnumIter)]
enum DefaultTableNumber {
    Tables = 1,
    Index = 2,
    Exports = 4,
    UdfConfig = 6,
    Auth = 7,
    Modules = 9,
    SourcePackages = 12,
    EnvironmentVariables = 13,
    DeploymentAuditLogs = 15,
    SessionRequests = 17,
    CronJobs = 19,
    Schemas = 20,
    CronJobLogs = 21,
    BackendState = 24,
    ExternalPackages = 25,
    ScheduledJobs = 27,
    FileStorage = 28,
    SnapshotImports = 29,
    IndexWorkerMetadata = 30,
    ComponentDefinitionsTable = 31,
    ComponentsTable = 32,
    FunctionHandlesTable = 33,
    // Keep this number and your user name up to date. The number makes it easy to know
    // what to use next. The username on the same line detects merge conflicts
    // Next Number - 34 - sujayakar
}

impl From<DefaultTableNumber> for TableNumber {
    fn from(value: DefaultTableNumber) -> Self {
        (NUM_RESERVED_LEGACY_TABLE_NUMBERS + value as u32)
            .try_into()
            .unwrap()
    }
}

impl From<DefaultTableNumber> for &'static dyn SystemTable {
    fn from(value: DefaultTableNumber) -> Self {
        match value {
            DefaultTableNumber::Tables => &TablesTable,
            DefaultTableNumber::Index => &IndexTable,
            DefaultTableNumber::Exports => &ExportsTable,
            DefaultTableNumber::UdfConfig => &UdfConfigTable,
            DefaultTableNumber::Auth => &AuthTable,
            DefaultTableNumber::Modules => &ModulesTable,
            DefaultTableNumber::SourcePackages => &SourcePackagesTable,
            DefaultTableNumber::EnvironmentVariables => &EnvironmentVariablesTable,
            DefaultTableNumber::DeploymentAuditLogs => &DeploymentAuditLogsTable,
            DefaultTableNumber::SessionRequests => &SessionRequestsTable,
            DefaultTableNumber::CronJobs => &CronJobsTable,
            DefaultTableNumber::Schemas => &SchemasTable,
            DefaultTableNumber::CronJobLogs => &CronJobLogsTable,
            DefaultTableNumber::BackendState => &BackendStateTable,
            DefaultTableNumber::ExternalPackages => &ExternalPackagesTable,
            DefaultTableNumber::ScheduledJobs => &ScheduledJobsTable,
            DefaultTableNumber::FileStorage => &FileStorageTable,
            DefaultTableNumber::SnapshotImports => &SnapshotImportsTable,
            DefaultTableNumber::IndexWorkerMetadata => &IndexWorkerMetadataTable,
            DefaultTableNumber::ComponentDefinitionsTable => &ComponentDefinitionsTable,
            DefaultTableNumber::ComponentsTable => &ComponentsTable,
            DefaultTableNumber::FunctionHandlesTable => &FunctionHandlesTable,
        }
    }
}

pub static DEFAULT_TABLE_NUMBERS: LazyLock<BTreeMap<TableName, TableNumber>> =
    LazyLock::new(|| {
        let mut default_table_numbers = BTreeMap::new();
        for default_table_number in DefaultTableNumber::iter() {
            let system_table: &'static dyn SystemTable = default_table_number.into();
            default_table_numbers.insert(
                system_table.table_name().clone(),
                default_table_number.into(),
            );
            if let Some((virtual_table_name, ..)) = system_table.virtual_table() {
                default_table_numbers
                    .insert(virtual_table_name.clone(), default_table_number.into());
            }
        }
        default_table_numbers
    });

/// System indexes all end with creation time. Except for these ones which
/// are too large and not worth to backfill.
///
/// New indexes should add creation time as a final tiebreak field.
static SYSTEM_INDEXES_WITHOUT_CREATION_TIME: LazyLock<BTreeSet<IndexName>> = LazyLock::new(|| {
    btreeset! {
        BY_COMPONENT_PATH_INDEX.clone(),
        CRON_JOBS_INDEX_BY_NAME.clone(),
        CRON_JOBS_INDEX_BY_NEXT_TS.clone(),
        CRON_JOB_LOGS_INDEX_BY_NAME_TS.clone(),
        ENVIRONMENT_VARIABLES_INDEX_BY_NAME.clone(),
        EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
        FILE_STORAGE_ID_INDEX.clone(),
        MODULE_INDEX_BY_DELETED.clone(),
        MODULE_INDEX_BY_PATH.clone(),
        SCHEDULED_JOBS_INDEX.clone(),
        SCHEDULED_JOBS_INDEX_BY_COMPLETED_TS.clone(),
        SCHEDULED_JOBS_INDEX_BY_UDF_PATH.clone(),
        SESSION_REQUESTS_INDEX.clone(),
    }
});

/// Idempotently initialize all the tables.
pub async fn initialize_application_system_tables<RT: Runtime>(
    database: &Database<RT>,
) -> anyhow::Result<()> {
    let mut tx = database.begin(Identity::system()).await?;
    for table in app_system_tables() {
        let is_new = initialize_application_system_table(
            &mut tx,
            table,
            TableNamespace::Global,
            &DEFAULT_TABLE_NUMBERS,
        )
        .await?;

        if is_new {
            // This is a bit ugly to put here for initialization, but it's a bit more
            // ergonomic this way instead of having initialize have <RT> generics
            if table.table_name() == BackendStateTable.table_name() {
                BackendStateModel::new(&mut tx).initialize().await?;
            }
        }
    }
    database
        .commit_with_write_source(tx, "init_app_system_tables")
        .await?;
    Ok(())
}

pub async fn initialize_application_system_table<RT: Runtime>(
    tx: &mut Transaction<RT>,
    table: &dyn SystemTable,
    namespace: TableNamespace,
    default_table_numbers: &BTreeMap<TableName, TableNumber>,
) -> anyhow::Result<bool> {
    let is_new = tx
        .create_system_table(
            namespace,
            table.table_name(),
            default_table_numbers.get(table.table_name()).cloned(),
        )
        .await?;
    if is_new {
        for index in table.indexes() {
            let index_metadata = IndexMetadata::new_enabled(index.name, index.fields);
            IndexModel::new(tx)
                .add_system_index(namespace, index_metadata)
                .await?;
        }
    } else {
        let table_id = tx
            .table_mapping()
            .namespace(namespace)
            .id(table.table_name())?
            .tablet_id;
        let existing_indexes: BTreeMap<_, _> = IndexModel::new(tx)
            .all_indexes_on_table(table_id)
            .await?
            .into_iter()
            .filter(|index| !index.name.is_by_id_or_creation_time())
            .map(|index| {
                let IndexConfig::Database {
                    developer_config,
                    on_disk_state: _,
                } = &index.config
                else {
                    // This isn't a strict requirement; it's just not implemented or needed.
                    anyhow::bail!("system tables indexes must be Database");
                };
                anyhow::Ok((index.name.clone(), developer_config.fields.clone()))
            })
            .try_collect()?;

        // Create new indexes as backfilling.
        let defined_indexes = table.indexes();
        for index in defined_indexes.iter() {
            if !SYSTEM_INDEXES_WITHOUT_CREATION_TIME.contains(&index.name) {
                anyhow::ensure!(
                    index.fields.last() == Some(&*CREATION_TIME_FIELD_PATH),
                    "System index {} should end with _creationTime",
                    index.name
                );
            } else {
                anyhow::ensure!(
                    index.fields.last() != Some(&*CREATION_TIME_FIELD_PATH),
                    "System index {} correctly ends with _creationTime. Doesn't need to be in \
                     SYSTEM_INDEXES_WITHOUT_CREATION_TIME list.",
                    index.name
                );
            }
            let index_name = TabletIndexName::new(table_id, index.name.descriptor().clone())?;
            match existing_indexes.get(&index_name) {
                Some(existing_fields) => anyhow::ensure!(
                    existing_fields == &index.fields,
                    "{} has the wrong fields: {existing_fields} != {}",
                    index.name,
                    index.fields,
                ),
                None => {
                    let index_metadata = IndexMetadata::new_backfilling(
                        *tx.begin_timestamp(),
                        index.name.clone(),
                        index.fields.clone(),
                    );
                    IndexModel::new(tx)
                        .add_system_index(namespace, index_metadata)
                        .await?;
                },
            }
        }

        // Remove indexes that are no longer referenced
        for (index, _) in existing_indexes {
            let index_name =
                IndexName::new(table.table_name().clone(), index.descriptor().clone())?;
            if !defined_indexes
                .iter()
                .any(|defined_index| defined_index.name == index_name)
            {
                // Existing index is not referenced any more.
                IndexModel::new(tx)
                    .drop_system_index(namespace, index_name)
                    .await?;
            }
        }
    }

    Ok(is_new)
}

pub fn app_system_tables() -> Vec<&'static dyn SystemTable> {
    let mut system_tables: Vec<&'static dyn SystemTable> = vec![
        &DeploymentAuditLogsTable,
        &EnvironmentVariablesTable,
        &AuthTable,
        &ExternalPackagesTable,
        &SessionRequestsTable,
        &BackendStateTable,
        &ExportsTable,
        &SnapshotImportsTable,
        &FunctionHandlesTable,
    ];
    system_tables.extend(component_system_tables());
    system_tables
}

/// NOTE: Does not include _schemas because that's not an app system table,
/// but it is created for each component.
pub fn component_system_tables() -> Vec<&'static dyn SystemTable> {
    vec![
        &FileStorageTable,
        &ScheduledJobsTable,
        &CronJobsTable,
        &CronJobLogsTable,
        &ModulesTable,
        &UdfConfigTable,
        &SourcePackagesTable,
    ]
}

pub fn virtual_system_mapping() -> VirtualSystemMapping {
    let mut mapping = VirtualSystemMapping::default();
    for table in app_system_tables() {
        if let Some((virtual_table_name, virtual_indexes, mapper)) = table.virtual_table() {
            mapping.add_table(
                virtual_table_name,
                table.table_name(),
                virtual_indexes,
                mapper,
            )
        }
    }
    mapping
}

#[cfg(test)]
mod test_default_table_numbers {
    use std::sync::Arc;

    use common::testing::TestPersistence;
    use database::{
        defaults::DEFAULT_BOOTSTRAP_TABLE_NUMBERS,
        test_helpers::{
            DbFixtures,
            DbFixturesArgs,
        },
    };
    use runtime::testing::TestRuntime;

    use crate::{
        app_system_tables,
        test_helpers::DbFixturesWithModel,
        virtual_system_mapping,
        DEFAULT_TABLE_NUMBERS,
    };

    #[test]
    fn test_ensure_consistent() {
        // Ensure consistent with the bootstrap model defaults
        for (bootstrap_table_name, bootstrap_table_number) in DEFAULT_BOOTSTRAP_TABLE_NUMBERS.iter()
        {
            assert!(
                DEFAULT_TABLE_NUMBERS.contains_key(bootstrap_table_name),
                "{bootstrap_table_name} missing from DEFAULT_TABLE_NUMBERS"
            );
            assert_eq!(
                DEFAULT_TABLE_NUMBERS[bootstrap_table_name],
                *bootstrap_table_number
            );
        }
    }

    #[test]
    fn test_ensure_defaults() {
        for table in app_system_tables() {
            let table_name = table.table_name();
            assert!(
                DEFAULT_TABLE_NUMBERS.contains_key(table_name),
                "{table_name} missing from DEFAULT_TABLE_NUMBERS"
            );
        }
    }

    #[convex_macro::test_runtime]
    async fn test_initialize_model(rt: TestRuntime) -> anyhow::Result<()> {
        let args = DbFixturesArgs {
            tp: Some(Arc::new(TestPersistence::new())),
            virtual_system_mapping: virtual_system_mapping(),
            ..Default::default()
        };
        // Initialize
        DbFixtures::new_with_model_and_args(&rt, args.clone()).await?;
        // Reinitialize (should work a second time - simulating a restart)
        DbFixtures::new_with_model_and_args(&rt, args).await?;
        Ok(())
    }
}
