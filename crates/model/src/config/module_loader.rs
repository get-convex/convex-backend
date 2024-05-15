use std::sync::Arc;

use async_trait::async_trait;
use common::{
    components::CanonicalizedComponentModulePath,
    document::ParsedDocument,
    runtime::Runtime,
};
use database::Transaction;

use crate::modules::{
    module_versions::FullModuleSource,
    types::ModuleMetadata,
    ModuleModel,
};

#[async_trait]
pub trait ModuleLoader<RT: Runtime>: Sync + Send + 'static {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Arc<FullModuleSource>>;

    async fn get_module(
        &self,
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentModulePath,
    ) -> anyhow::Result<Option<Arc<FullModuleSource>>> {
        let module_metadata = match ModuleModel::new(tx).get_metadata(path).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        self.get_module_with_metadata(tx, module_metadata)
            .await
            .map(Some)
    }
}

// Loads module versions directly from the transaction.
pub struct TransactionModuleLoader;

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for TransactionModuleLoader {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Arc<FullModuleSource>> {
        let full_source = ModuleModel::new(tx)
            .get_source(module_metadata.id(), module_metadata.latest_version)
            .await?;
        Ok(Arc::new(full_source))
    }
}
