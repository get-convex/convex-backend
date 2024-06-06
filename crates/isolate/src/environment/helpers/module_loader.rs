use std::{
    collections::HashMap,
    sync::Arc,
};

use anyhow::{
    anyhow,
    Context,
};
use common::{
    components::ComponentDefinitionId,
    document::ParsedDocument,
    knobs::READ_MODULES_FROM_SOURCE_PACKAGE,
    runtime::Runtime,
};
use database::Transaction;
use deno_core::ModuleSpecifier;
use model::{
    modules::{
        module_versions::{
            FullModuleSource,
            ModuleVersion,
        },
        types::ModuleMetadata,
        ModuleModel,
    },
    source_packages::{
        types::SourcePackageId,
        upload_download::download_package,
        SourcePackageModel,
    },
};
use storage::Storage;
use sync_types::CanonicalizedModulePath;
use value::{
    ResolvedDocumentId,
    TableNamespace,
    TabletId,
};

use crate::{
    isolate::CONVEX_SCHEME,
    metrics::module_load_timer,
};

#[minitrace::trace]
pub async fn get_module_and_prefetch<RT: Runtime>(
    tx: &mut Transaction<RT>,
    modules_storage: Arc<dyn Storage>,
    module_metadata: ParsedDocument<ModuleMetadata>,
) -> HashMap<(ResolvedDocumentId, ModuleVersion), anyhow::Result<FullModuleSource>> {
    let all_source_result = if *READ_MODULES_FROM_SOURCE_PACKAGE {
        let _timer = module_load_timer("package");
        download_module_source_from_package(
            tx,
            modules_storage,
            module_metadata.id().table().tablet_id,
            module_metadata.source_package_id,
        )
        .await
    } else {
        let _timer = module_load_timer("db");
        ModuleModel::new(tx)
            .get_source_from_db(module_metadata.id(), module_metadata.latest_version)
            .await
            .map(|source| {
                let mut result = HashMap::new();
                result.insert(
                    (module_metadata.id(), module_metadata.latest_version),
                    source,
                );
                result
            })
    };
    match all_source_result {
        Err(e) => {
            let mut result = HashMap::new();
            result.insert(
                (module_metadata.id(), module_metadata.latest_version),
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
async fn download_module_source_from_package<RT: Runtime>(
    tx: &mut Transaction<RT>,
    modules_storage: Arc<dyn Storage>,
    modules_tablet: TabletId,
    source_package_id: Option<SourcePackageId>,
) -> anyhow::Result<HashMap<(ResolvedDocumentId, ModuleVersion), FullModuleSource>> {
    let mut result = HashMap::new();
    let source_package = SourcePackageModel::new(tx)
        .get(source_package_id.context("source package missing")?)
        .await?;
    let mut package = download_package(
        modules_storage,
        source_package.storage_key.clone(),
        source_package.sha256.clone(),
    )
    .await?;
    let namespace = tx.table_mapping().tablet_namespace(modules_tablet)?;
    let component = match namespace {
        // TODO(lee) global namespace should not have modules, but for existing data this is how
        // it's represented.
        TableNamespace::Global => ComponentDefinitionId::Root,
        TableNamespace::RootComponentDefinition => ComponentDefinitionId::Root,
        TableNamespace::ByComponentDefinition(id) => ComponentDefinitionId::Child(id),
        _ => anyhow::bail!("_modules table namespace {namespace:?} is not a component definition"),
    };
    for module_metadata in ModuleModel::new(tx).get_all_metadata(component).await? {
        let source = package
            .remove(&module_metadata.path)
            .context("module not found in package")?;
        result.insert(
            (module_metadata.id(), module_metadata.latest_version),
            FullModuleSource {
                source: source.source,
                source_map: source.source_map,
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
        .ok_or(anyhow!("module specifier did not start with {}", prefix))
}
