use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::Transaction;
use deno_core::ModuleSpecifier;
use errors::ErrorMetadata;
use model::modules::{
    module_versions::{
        AnalyzedFunction,
        ModuleVersionMetadata,
    },
    types::ModuleMetadata,
    ModuleModel,
};
use storage::Storage;
use sync_types::{
    CanonicalizedModulePath,
    CanonicalizedUdfPath,
};

use crate::{
    isolate::CONVEX_SCHEME,
    metrics::module_load_timer,
    FunctionNotFoundError,
    ModuleNotFoundError,
};

#[async_trait]
pub trait ModuleLoader<RT: Runtime>: Sync + Send + 'static {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Option<Arc<ModuleVersionMetadata>>>;

    async fn get_module(
        &self,
        tx: &mut Transaction<RT>,
        path: CanonicalizedModulePath,
    ) -> anyhow::Result<Option<Arc<ModuleVersionMetadata>>> {
        let module_metadata = match ModuleModel::new(tx).get_metadata(path).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        self.get_module_with_metadata(tx, module_metadata).await
    }

    // Helper method that returns the AnalyzedFunction for the specified path.
    // It returns a user error if the module or function does not exist.
    // Note that using this method will error if AnalyzedResult is not backfilled,
    async fn get_analyzed_function(
        &self,
        tx: &mut Transaction<RT>,
        udf_path: &CanonicalizedUdfPath,
    ) -> anyhow::Result<anyhow::Result<AnalyzedFunction>> {
        let Some(module) = self.get_module(tx, udf_path.module().clone()).await? else {
            return Ok(Err(ErrorMetadata::bad_request(
                "ModuleNotFound",
                ModuleNotFoundError::new(udf_path.module().as_str()).to_string(),
            )
            .into()));
        };

        // Dependency modules don't have AnalyzedModule.
        if !udf_path.module().is_deps() {
            let analyzed_module = module
                .as_ref()
                .analyze_result
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Expected analyze result for {udf_path:?}"))?;

            for function in &analyzed_module.functions {
                if &function.name == udf_path.function_name() {
                    return Ok(Ok(function.clone()));
                }
            }
        }

        Ok(Err(ErrorMetadata::bad_request(
            "FunctionNotFound",
            FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
                .to_string(),
        )
        .into()))
    }

    async fn has_http(&self, tx: &mut Transaction<RT>) -> anyhow::Result<bool> {
        Ok(self.get_module(tx, "http.js".parse()?).await?.is_some())
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
    ) -> anyhow::Result<Option<Arc<ModuleVersionMetadata>>> {
        let _timer = module_load_timer();
        let module_version = ModuleModel::new(tx)
            .get_version(module_metadata.id(), module_metadata.latest_version)
            .await?
            .into_value();
        Ok(Some(Arc::new(module_version)))
    }
}

pub async fn get_module<RT: Runtime>(
    mut tx: Transaction<RT>,
    // TODO(lee) fetch from module storage
    _modules_storage: Arc<dyn Storage>,
    module_metadata: ParsedDocument<ModuleMetadata>,
) -> anyhow::Result<ModuleVersionMetadata> {
    let _timer = module_load_timer();
    let module_version = ModuleModel::new(&mut tx)
        .get_version(module_metadata.id(), module_metadata.latest_version)
        .await?
        .into_value();
    Ok(module_version)
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
