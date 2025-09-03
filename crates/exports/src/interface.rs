use common::{
    runtime::Runtime,
    types::ObjectKey,
};
use futures::future::BoxFuture;
use model::exports::types::{
    ExportFormat,
    ExportRequestor,
};
use usage_tracking::FunctionUsageTracker;
use value::ResolvedDocumentId;

use crate::ExportComponents;

/// An opaque object saved by the export worker. The ExportProvider defines its
/// shape, and uses it to find an ongoing export after a backend restart.
///
/// The object must be a valid Convex object (according to `value::serde`).
pub type ResumptionToken = serde_json::Map<String, serde_json::Value>;

#[async_trait::async_trait]
pub trait ExportProvider<RT: Runtime>: Send + Sync {
    async fn export(
        &self,
        components: &ExportComponents<RT>,
        format: ExportFormat,
        requestor: ExportRequestor,
        export_id: ResolvedDocumentId,
        update_progress: &(dyn Fn(String) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync),
        save_resumption_token: &(dyn Fn(ResumptionToken) -> BoxFuture<'static, anyhow::Result<()>>
              + Send
              + Sync),
    ) -> anyhow::Result<(ObjectKey, FunctionUsageTracker)>;

    /// Attempts to resume receiving status updates of an ongoing export.
    /// Returns None if the export couldn't be resumed.
    async fn resume_export(
        &self,
        instance_name: String,
        resumption_token: ResumptionToken,
        export_id: ResolvedDocumentId,
        update_progress: &(dyn Fn(String) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync),
    ) -> anyhow::Result<Option<(ObjectKey, FunctionUsageTracker)>>;
}

pub struct InProcessExportProvider;

#[async_trait::async_trait]
impl<RT: Runtime> ExportProvider<RT> for InProcessExportProvider {
    async fn export(
        &self,
        components: &ExportComponents<RT>,
        format: ExportFormat,
        requestor: ExportRequestor,
        _export_id: ResolvedDocumentId,
        update_progress: &(dyn Fn(String) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync),
        _save_resumption_token: &(dyn Fn(ResumptionToken) -> BoxFuture<'static, anyhow::Result<()>>
              + Send
              + Sync),
    ) -> anyhow::Result<(ObjectKey, FunctionUsageTracker)> {
        crate::export_inner(components, format, requestor, update_progress).await
    }

    async fn resume_export(
        &self,
        _instance_name: String,
        _resumption_token: ResumptionToken,
        _export_id: ResolvedDocumentId,
        _update_progress: &(dyn Fn(String) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync),
    ) -> anyhow::Result<Option<(ObjectKey, FunctionUsageTracker)>> {
        // Resuming export not supported
        Ok(None)
    }
}
