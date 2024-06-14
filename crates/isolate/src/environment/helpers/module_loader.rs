use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    sync::Arc,
};

use anyhow::anyhow;
use common::document::ParsedDocument;
use deno_core::ModuleSpecifier;
use model::{
    modules::{
        module_versions::FullModuleSource,
        types::ModuleMetadata,
    },
    source_packages::{
        types::{
            SourcePackage,
            SourcePackageId,
        },
        upload_download::download_package,
    },
};
use storage::Storage;
use sync_types::CanonicalizedModulePath;
use value::ResolvedDocumentId;

use crate::{
    isolate::CONVEX_SCHEME,
    metrics::module_load_timer,
};

#[minitrace::trace]
pub async fn get_module_and_prefetch(
    modules_storage: Arc<dyn Storage>,
    module_metadata: ParsedDocument<ModuleMetadata>,
    source_package: ParsedDocument<SourcePackage>,
    paths_to_prefetch: BTreeMap<ResolvedDocumentId, CanonicalizedModulePath>,
) -> HashMap<(ResolvedDocumentId, SourcePackageId), anyhow::Result<FullModuleSource>> {
    let _timer = module_load_timer("package");
    let all_source_result =
        download_module_source_from_package(modules_storage, source_package, paths_to_prefetch)
            .await;
    match all_source_result {
        Err(e) => {
            let mut result = HashMap::new();
            result.insert(
                (module_metadata.id(), module_metadata.source_package_id),
                Err(e),
            );
            result
        },
        Ok(all_source) => all_source
            .into_iter()
            .map(|(path, source)| (path, Ok(source)))
            .collect(),
    }
}

#[minitrace::trace]
async fn download_module_source_from_package(
    modules_storage: Arc<dyn Storage>,
    source_package: ParsedDocument<SourcePackage>,
    paths_to_prefetch: BTreeMap<ResolvedDocumentId, CanonicalizedModulePath>,
) -> anyhow::Result<HashMap<(ResolvedDocumentId, SourcePackageId), FullModuleSource>> {
    let mut result = HashMap::new();
    let mut package = download_package(
        modules_storage,
        source_package.storage_key.clone(),
        source_package.sha256.clone(),
    )
    .await?;
    let source_package_id: SourcePackageId = source_package.developer_id().into();
    for (module_id, module_path) in paths_to_prefetch {
        match package.remove(&module_path) {
            None => {
                anyhow::bail!(
                    "module {:?} not found in package {:?}",
                    module_path,
                    source_package_id
                );
            },
            Some(source) => {
                result.insert(
                    (module_id, source_package_id),
                    FullModuleSource {
                        source: source.source,
                        source_map: source.source_map,
                    },
                );
            },
        }
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
        .ok_or(anyhow!("module specifier did not start with {}", prefix))
}
