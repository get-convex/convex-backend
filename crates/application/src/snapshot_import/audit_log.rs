use std::collections::{
    BTreeMap,
    BTreeSet,
};

use common::{
    components::ComponentPath,
    runtime::Runtime,
};
use database::Database;
use keybroker::Identity;
use model::{
    deployment_audit_log::types::DeploymentAuditLogEvent,
    snapshot_imports::types::SnapshotImport,
};
use value::{
    TableName,
    TableNamespace,
};

use crate::snapshot_import::TableMappingForImport;

pub async fn make_audit_log_event<RT: Runtime>(
    database: &Database<RT>,
    table_mapping_for_import: &TableMappingForImport,
    snapshot_import: &SnapshotImport,
) -> anyhow::Result<DeploymentAuditLogEvent> {
    let (table_count, table_names) =
        audit_log_table_names(database, table_mapping_for_import.tables_imported()).await?;
    let (table_count_deleted, table_names_deleted) =
        audit_log_table_names(database, table_mapping_for_import.tables_deleted()).await?;

    Ok(DeploymentAuditLogEvent::SnapshotImport {
        table_names,
        table_count,
        import_mode: snapshot_import.mode,
        import_format: snapshot_import.format.clone(),
        requestor: snapshot_import.requestor.clone(),
        table_names_deleted,
        table_count_deleted,
    })
}

async fn audit_log_table_names<RT: Runtime>(
    database: &Database<RT>,
    input: BTreeSet<(TableNamespace, TableName)>,
) -> anyhow::Result<(u64, BTreeMap<ComponentPath, Vec<TableName>>)> {
    // Truncate list of table names to avoid hitting the object size limit for the
    // audit log object and failing the import.
    let table_names: BTreeSet<_> = {
        let mut tx = database.begin(Identity::system()).await?;
        input
            .into_iter()
            .map(|(namespace, name)| {
                (
                    tx.get_component_path(namespace.into())
                        .unwrap_or(ComponentPath::root()),
                    name,
                )
            })
            .collect()
    };
    let table_count = table_names.len() as u64;
    Ok((
        table_count,
        table_names
            .into_iter()
            .take(20)
            .fold(BTreeMap::new(), |mut map, (a, b)| {
                map.entry(a).or_default().push(b);
                map
            }),
    ))
}
