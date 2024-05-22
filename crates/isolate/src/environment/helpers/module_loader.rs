use std::sync::Arc;

use anyhow::anyhow;
use common::{
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

pub async fn get_module<RT: Runtime>(
    tx: &mut Transaction<RT>,
    // TODO(lee) fetch from module storage
    _modules_storage: Arc<dyn Storage>,
    module_metadata: ParsedDocument<ModuleMetadata>,
) -> anyhow::Result<FullModuleSource> {
    let _timer = module_load_timer();
    let source = ModuleModel::new(tx)
        .get_source_from_db(module_metadata.id(), module_metadata.latest_version)
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
