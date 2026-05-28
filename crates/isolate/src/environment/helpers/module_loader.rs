use std::{
    collections::HashMap,
    sync::Arc,
};

use anyhow::anyhow;
use common::document::ParsedDocument;
use deno_core::ModuleSpecifier;
use model::{
    modules::{
        hash_module_source,
        module_versions::FullModuleSource,
    },
    source_packages::{
        types::SourcePackage,
        upload_download::download_package,
    },
};
use storage::Storage;
use sync_types::CanonicalizedModulePath;
use value::sha256::Sha256Digest;

use crate::{
    isolate::CONVEX_SCHEME,
    metrics::module_load_timer,
};

#[fastrace::trace]
pub async fn get_modules_and_prefetch(
    modules_storage: Arc<dyn Storage>,
    source_package: &ParsedDocument<SourcePackage>,
) -> anyhow::Result<HashMap<(CanonicalizedModulePath, Sha256Digest), Arc<FullModuleSource>>> {
    let _timer = module_load_timer("package");
    let all_source = download_module_source_from_package(modules_storage, source_package).await?;
    Ok(all_source
        .into_iter()
        .map(|(path, source)| (path, Arc::new(source)))
        .collect())
}

#[fastrace::trace]
async fn download_module_source_from_package(
    modules_storage: Arc<dyn Storage>,
    source_package: &ParsedDocument<SourcePackage>,
) -> anyhow::Result<HashMap<(CanonicalizedModulePath, Sha256Digest), FullModuleSource>> {
    let mut result = HashMap::new();
    let package = download_package(
        modules_storage,
        source_package.storage_key.clone(),
        source_package.sha256.clone(),
    )
    .await?;
    for (module_path, module_config) in package {
        result.insert(
            (
                module_path,
                hash_module_source(&module_config.source, module_config.source_map.as_ref()),
            ),
            FullModuleSource {
                source: module_config.source,
                source_map: module_config.source_map,
            },
        );
    }
    Ok(result)
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
        .ok_or_else(|| anyhow!("module specifier did not start with {}", prefix))
}
