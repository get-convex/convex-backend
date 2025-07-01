use common::{
    components::ComponentPath,
    knobs::{
        DOCUMENT_DELTAS_LIMIT,
        SNAPSHOT_LIST_LIMIT,
    },
    runtime::Runtime,
};
use database::{
    DocumentDeltas,
    SnapshotPage,
    StreamingExportTableFilter,
};
use keybroker::Identity;
use model::backend_info::BackendInfoModel;
use sync_types::Timestamp;
use value::{
    ResolvedDocumentId,
    TableName,
};

use crate::Application;

impl<RT: Runtime> Application<RT> {
    pub async fn ensure_streaming_export_enabled(&self, identity: Identity) -> anyhow::Result<()> {
        let mut tx = self.begin(identity).await?;
        BackendInfoModel::new(&mut tx)
            .ensure_streaming_export_enabled()
            .await
    }

    #[fastrace::trace]
    pub async fn document_deltas(
        &self,
        identity: Identity,
        cursor: Timestamp,
        table_filter: Option<TableName>,
        component_filter: Option<ComponentPath>,
    ) -> anyhow::Result<DocumentDeltas> {
        if let Some(ref component_filter) = component_filter {
            if !component_filter.is_root() {
                anyhow::bail!(
                    "Components are currently unsupported in streaming export: {}",
                    String::from(component_filter.clone())
                );
            }
        }
        self.database
            .document_deltas(
                identity,
                Some(cursor),
                StreamingExportTableFilter {
                    table_name: table_filter,
                    component_path: component_filter,
                    ..Default::default()
                },
                *DOCUMENT_DELTAS_LIMIT,
                *DOCUMENT_DELTAS_LIMIT,
            )
            .await
    }

    #[fastrace::trace]
    pub async fn list_snapshot(
        &self,
        identity: Identity,
        snapshot: Option<Timestamp>,
        cursor: Option<ResolvedDocumentId>,
        table_filter: Option<TableName>,
        component_filter: Option<ComponentPath>,
    ) -> anyhow::Result<SnapshotPage> {
        self.database
            .list_snapshot(
                identity,
                snapshot,
                cursor,
                StreamingExportTableFilter {
                    table_name: table_filter,
                    component_path: component_filter,
                    ..Default::default()
                },
                *SNAPSHOT_LIST_LIMIT,
                *SNAPSHOT_LIST_LIMIT,
            )
            .await
    }
}
