use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use common::{
    components::CanonicalizedComponentModulePath,
    document::ParsedDocument,
    runtime::Runtime,
};
use database::Transaction;
use deno_core::ModuleSpecifier;
use model::modules::{
    module_versions::FullModuleSource,
    types::ModuleMetadata,
    ModuleModel,
};
use storage::Storage;
use sync_types::CanonicalizedModulePath;

use crate::{
    isolate::CONVEX_SCHEME,
    metrics::module_load_timer,
};

#[async_trait]
pub trait ModuleLoader<RT: Runtime>: Sync + Send + 'static {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Option<Arc<FullModuleSource>>>;

    async fn get_module(
        &self,
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentModulePath,
    ) -> anyhow::Result<Option<Arc<FullModuleSource>>> {
        let module_metadata = match ModuleModel::new(tx).get_metadata(path).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        self.get_module_with_metadata(tx, module_metadata).await
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
    ) -> anyhow::Result<Option<Arc<FullModuleSource>>> {
        let _timer = module_load_timer();
        let full_source = ModuleModel::new(tx)
            .get_source(module_metadata.id(), module_metadata.latest_version)
            .await?;
        Ok(Some(Arc::new(full_source)))
    }
}

pub async fn get_module<RT: Runtime>(
    mut tx: Transaction<RT>,
    // TODO(lee) fetch from module storage
    _modules_storage: Arc<dyn Storage>,
    module_metadata: ParsedDocument<ModuleMetadata>,
) -> anyhow::Result<FullModuleSource> {
    let _timer = module_load_timer();
    let source = ModuleModel::new(&mut tx)
        .get_source(module_metadata.id(), module_metadata.latest_version)
        .await?;
    Ok(source)
}

pub fn module_specifier_from_path(
    path: &CanonicalizedModulePath,
) -> anyhow::Result<ModuleSpecifier> {
    let url = format!("{CONVEX_SCHEME}:/{}", path.as_str());
    Ok(ModuleSpecifier::parse(&url)?)
}

pub fn module_specifier_from_str(path: &str) -> anyhow::Result<ModuleSpecifier> {
    Ok(ModuleSpecifier::parse(path)?)
}

pub fn path_from_module_specifier(
    spec: &ModuleSpecifier,
) -> anyhow::Result<CanonicalizedModulePath> {
    let spec_str = spec.as_str();
    let prefix = format!("{CONVEX_SCHEME}:/");
    spec_str
        .starts_with(&prefix)
        .then(|| {
            spec_str[prefix.len()..]
                .to_string()
                .parse::<CanonicalizedModulePath>()
        })
        .transpose()?
        .ok_or(anyhow!("module specifier did not start with {}", prefix))
}
