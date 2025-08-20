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

use crate::ExportComponents;

#[async_trait::async_trait]
pub trait ExportProvider<RT: Runtime>: Send + Sync {
    async fn export(
        &self,
        components: &ExportComponents<RT>,
        format: ExportFormat,
        requestor: ExportRequestor,
        export_id: String,
        update_progress: Box<
            dyn Fn(String) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync,
        >,
    ) -> anyhow::Result<(ObjectKey, FunctionUsageTracker)>;
}

pub struct InProcessExportProvider;

#[async_trait::async_trait]
impl<RT: Runtime> ExportProvider<RT> for InProcessExportProvider {
    async fn export(
        &self,
        components: &ExportComponents<RT>,
        format: ExportFormat,
        requestor: ExportRequestor,
        _export_id: String,
        update_progress: Box<
            dyn Fn(String) -> BoxFuture<'static, anyhow::Result<()>> + Send + Sync,
        >,
    ) -> anyhow::Result<(ObjectKey, FunctionUsageTracker)> {
        crate::export_inner(components, format, requestor, &*update_progress).await
    }
}
