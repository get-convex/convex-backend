use std::collections::{
    BTreeMap,
    BTreeSet,
};

use common::{
    components::ComponentPath,
    runtime::Runtime,
};
use database::Transaction;
use model::{
    deployment_audit_log::types::DeploymentAuditLogEvent,
    snapshot_imports::types::{
        ImportFormat,
        ImportMode,
        ImportRequestor,
    },
};
use value::{
    TableName,
    TableNamespace,
};

use crate::snapshot_import::TableMappingForImport;

pub async fn make_audit_log_event<RT: Runtime>(
    tx: &mut Transaction<RT>,
    table_mapping_for_import: &TableMappingForImport,
    import_mode: ImportMode,
    import_format: ImportFormat,
    requestor: ImportRequestor,
) -> anyhow::Result<DeploymentAuditLogEvent> {
    let (table_count, table_names) =
        audit_log_table_names(tx, table_mapping_for_import.tables_imported()).await?;
    let (table_count_deleted, table_names_deleted) =
        audit_log_table_names(tx, table_mapping_for_import.tables_deleted()).await?;

    Ok(DeploymentAuditLogEvent::SnapshotImport {
        table_names,
        table_count,
        import_mode,
        import_format,
        requestor,
        table_names_deleted,
        table_count_deleted,
    })
}

async fn audit_log_table_names<RT: Runtime>(
    tx: &mut Transaction<RT>,
    input: BTreeSet<(TableNamespace, TableName)>,
) -> anyhow::Result<(u64, BTreeMap<ComponentPath, Vec<TableName>>)> {
    // Truncate list of table names to avoid hitting the object size limit for the
    // audit log object and failing the import.
    let table_names: BTreeSet<_> = {
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
