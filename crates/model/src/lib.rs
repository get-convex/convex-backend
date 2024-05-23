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
#![feature(bound_map)]
#![feature(iterator_try_collect)]
#![feature(never_type)]
#![feature(try_blocks)]
#![feature(exclusive_range_pattern)]
#![feature(async_closure)]
#![feature(trait_upcasting)]
#![feature(impl_trait_in_assoc_type)]

use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use backend_state::BackendStateTable;
use common::{
    bootstrap_model::index::{
        IndexConfig,
        IndexMetadata,
    },
    runtime::Runtime,
    types::TabletIndexName,
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
    VirtualSystemMapping,
    VirtualTablesTable,
    NUM_RESERVED_LEGACY_TABLE_NUMBERS,
};
use file_storage::FILE_STORAGE_VIRTUAL_TABLE;
use keybroker::Identity;
use scheduled_jobs::SCHEDULED_JOBS_VIRTUAL_TABLE;
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
    modules::{
        ModuleVersionsTable,
        ModulesTable,
    },
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
    ModuleVersions = 10,
    SourcePackages = 12,
    EnvironmentVariables = 13,
    DeploymentAuditLogs = 15,
    FileStorage = 16,
    SessionRequests = 17,
    ScheduledJobs = 18,
    CronJobs = 19,
    Schemas = 20,
    CronJobLogs = 21,
    BackendState = 24,
    ExternalPackages = 25,
    VirtualTables = 26,
    ScheduledJobsVirtual = 27,
    FileStorageVirtual = 28,
    SnapshotImports = 29,
    IndexWorkerMetadata = 30,
    ComponentDefinitionsTable = 31,
    ComponentsTable = 32,
    // Keep this number and your user name up to date. The number makes it easy to know
    // what to use next. The username on the same line detects merge conflicts
    // Next Number - 33 - lee
}

impl From<DefaultTableNumber> for TableNumber {
    fn from(value: DefaultTableNumber) -> Self {
        (NUM_RESERVED_LEGACY_TABLE_NUMBERS + value as u32)
            .try_into()
            .unwrap()
    }
}

impl From<DefaultTableNumber> for TableName {
    fn from(value: DefaultTableNumber) -> Self {
        match value {
            DefaultTableNumber::Tables => TablesTable.table_name(),
            DefaultTableNumber::Index => IndexTable.table_name(),
            DefaultTableNumber::Exports => ExportsTable.table_name(),
            DefaultTableNumber::UdfConfig => UdfConfigTable.table_name(),
            DefaultTableNumber::Auth => AuthTable.table_name(),
            DefaultTableNumber::Modules => ModulesTable.table_name(),
            DefaultTableNumber::ModuleVersions => ModuleVersionsTable.table_name(),
            DefaultTableNumber::SourcePackages => SourcePackagesTable.table_name(),
            DefaultTableNumber::EnvironmentVariables => EnvironmentVariablesTable.table_name(),
            DefaultTableNumber::DeploymentAuditLogs => DeploymentAuditLogsTable.table_name(),
            DefaultTableNumber::FileStorage => FileStorageTable.table_name(),
            DefaultTableNumber::SessionRequests => SessionRequestsTable.table_name(),
            DefaultTableNumber::ScheduledJobs => ScheduledJobsTable.table_name(),
            DefaultTableNumber::CronJobs => CronJobsTable.table_name(),
            DefaultTableNumber::Schemas => SchemasTable.table_name(),
            DefaultTableNumber::CronJobLogs => CronJobLogsTable.table_name(),
            DefaultTableNumber::BackendState => BackendStateTable.table_name(),
            DefaultTableNumber::ExternalPackages => ExternalPackagesTable.table_name(),
            DefaultTableNumber::VirtualTables => VirtualTablesTable.table_name(),
            DefaultTableNumber::ScheduledJobsVirtual => &*SCHEDULED_JOBS_VIRTUAL_TABLE,
            DefaultTableNumber::FileStorageVirtual => &*FILE_STORAGE_VIRTUAL_TABLE,
            DefaultTableNumber::SnapshotImports => SnapshotImportsTable.table_name(),
            DefaultTableNumber::IndexWorkerMetadata => IndexWorkerMetadataTable.table_name(),
            DefaultTableNumber::ComponentDefinitionsTable => ComponentDefinitionsTable.table_name(),
            DefaultTableNumber::ComponentsTable => ComponentsTable.table_name(),
        }
        .clone()
    }
}

pub static DEFAULT_TABLE_NUMBERS: LazyLock<BTreeMap<TableName, TableNumber>> =
    LazyLock::new(|| {
        let mut default_table_numbers = BTreeMap::new();
        for default_table_number in DefaultTableNumber::iter() {
            default_table_numbers.insert(default_table_number.into(), default_table_number.into());
        }
        default_table_numbers
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
        // Create new indexes as backfilling.
        let table_id = tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(table.table_name())?
            .tablet_id;
        let mut index_model = IndexModel::new(tx);
        let existing_indexes: BTreeMap<_, _> = index_model
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
        for index in table.indexes() {
            let index_name = TabletIndexName::new(table_id, index.name.descriptor().clone())?;
            match existing_indexes.get(&index_name) {
                Some(existing_fields) => anyhow::ensure!(
                    existing_fields == &index.fields,
                    "{index_name} has the wrong fields: {existing_fields} != {}",
                    index.fields
                ),
                None => {
                    let index_metadata = IndexMetadata::new_backfilling(
                        *tx.begin_timestamp(),
                        index.name,
                        index.fields,
                    );
                    IndexModel::new(tx)
                        .add_system_index(namespace, index_metadata)
                        .await?;
                },
            }
        }
    }

    if let Some((table_name, _indexes, _mapper)) = table.virtual_table() {
        tx.create_virtual_table(table_name, default_table_numbers.get(table_name).cloned())
            .await?;
    }

    Ok(is_new)
}

pub fn app_system_tables() -> Vec<&'static dyn SystemTable> {
    vec![
        &DeploymentAuditLogsTable,
        &EnvironmentVariablesTable,
        &UdfConfigTable,
        &AuthTable,
        &ExternalPackagesTable,
        &ModulesTable,
        &ModuleVersionsTable,
        &SourcePackagesTable,
        &SessionRequestsTable,
        &FileStorageTable,
        &ScheduledJobsTable,
        &CronJobsTable,
        &CronJobLogsTable,
        &BackendStateTable,
        &ExportsTable,
        &SnapshotImportsTable,
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
    use database::defaults::DEFAULT_BOOTSTRAP_TABLE_NUMBERS;

    use crate::{
        app_system_tables,
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
}
