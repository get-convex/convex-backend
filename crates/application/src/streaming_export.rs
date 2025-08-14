use common::{
    knobs::{
        DOCUMENT_DELTAS_LIMIT,
        SNAPSHOT_LIST_LIMIT,
    },
    runtime::Runtime,
};
use database::{
    streaming_export_selection::StreamingExportSelection,
    DocumentDeltas,
    SnapshotPage,
    StreamingExportFilter,
};
use keybroker::Identity;
use model::backend_info::BackendInfoModel;
use sync_types::Timestamp;
use value::ResolvedDocumentId;

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
        selection: StreamingExportSelection,
    ) -> anyhow::Result<DocumentDeltas> {
        self.database
            .document_deltas(
                identity,
                Some(cursor),
                StreamingExportFilter {
                    selection,
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
        selection: StreamingExportSelection,
    ) -> anyhow::Result<SnapshotPage> {
        self.database
            .list_snapshot(
                identity,
                snapshot,
                cursor,
                StreamingExportFilter {
                    selection,
                    ..Default::default()
                },
                *SNAPSHOT_LIST_LIMIT,
                *SNAPSHOT_LIST_LIMIT,
            )
            .await
    }
}
