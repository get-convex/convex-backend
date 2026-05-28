use std::sync::Arc;

use async_lru::async_lru::AsyncLru;
use async_trait::async_trait;
use common::{
    document::ParsedDocument,
    knobs::{
        FUNRUN_CODE_CACHE_SIZE,
        FUNRUN_MODULE_CACHE_SIZE,
        FUNRUN_MODULE_MAX_CONCURRENCY,
    },
    runtime::Runtime,
};
use futures::FutureExt;
use isolate::environment::helpers::module_loader::get_modules_and_prefetch;
use model::{
    config::module_loader::ModuleLoader,
    modules::{
        module_versions::FullModuleSource,
        types::ModuleMetadata,
    },
    source_packages::types::SourcePackage,
};
use moka::sync::Cache;
use storage::Storage;
use sync_types::CanonicalizedModulePath;
use value::sha256::Sha256Digest;

use crate::record_module_sizes;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ModuleCacheKey {
    deployment_name: String,
    module_path: CanonicalizedModulePath,
    sha256: Sha256Digest,
}

#[derive(Clone)]
pub(crate) struct ModuleCache<RT: Runtime>(AsyncLru<RT, ModuleCacheKey, FullModuleSource>);

impl<RT: Runtime> ModuleCache<RT> {
    pub(crate) fn new(rt: RT) -> Self {
        Self(AsyncLru::new(
            rt,
            *FUNRUN_MODULE_CACHE_SIZE,
            *FUNRUN_MODULE_MAX_CONCURRENCY,
            "function_runner_module_cache",
        ))
    }
}

#[derive(Clone)]
pub(crate) struct CodeCache(Arc<Cache<ModuleCacheKey, Arc<[u8]>>>);
impl CodeCache {
    pub(crate) fn new() -> Self {
        Self(Arc::new(
            Cache::builder()
                .max_capacity(*FUNRUN_CODE_CACHE_SIZE)
                .weigher(|_, data: &Arc<[u8]>| u32::try_from(data.len()).unwrap_or(u32::MAX))
                .build(),
        ))
    }
}

pub(crate) struct FunctionRunnerModuleLoader<RT: Runtime> {
    pub cache: ModuleCache<RT>,
    pub code_cache: CodeCache,
    pub deployment_name: String,
    pub modules_storage: Arc<dyn Storage>,
}

impl<RT: Runtime> FunctionRunnerModuleLoader<RT> {
    fn cache_key(&self, module_metadata: &ModuleMetadata) -> ModuleCacheKey {
        ModuleCacheKey {
            deployment_name: self.deployment_name.clone(),
            module_path: module_metadata.path.clone(),
            sha256: module_metadata.sha256.clone(),
        }
    }
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for FunctionRunnerModuleLoader<RT> {
    #[fastrace::trace]
    async fn get_module_with_metadata(
        &self,
        module_metadata: &ParsedDocument<ModuleMetadata>,
        source_package: &ParsedDocument<SourcePackage>,
    ) -> anyhow::Result<Arc<FullModuleSource>> {
        let deployment_name = self.deployment_name.clone();
        let key = self.cache_key(module_metadata);
        let modules_storage = self.modules_storage.clone();
        let source_package = source_package.clone();
        let result = self
            .cache
            .0
            .get_and_prepopulate(
                key.clone(),
                async move {
                    let modules =
                        get_modules_and_prefetch(modules_storage, &source_package).await?;
                    Ok(modules
                        .into_iter()
                        .map(move |((module_path, sha256), source)| {
                            (
                                ModuleCacheKey {
                                    deployment_name: deployment_name.clone(),
                                    module_path,
                                    sha256,
                                },
                                source,
                            )
                        })
                        .collect())
                }
                .boxed(),
            )
            .await?;
        record_module_sizes(
            result.source.len(),
            result.source_map.as_ref().map(|sm| sm.len()),
        );
        Ok(result)
    }
}

impl<RT: Runtime> isolate::module_cache::ModuleCache<RT> for FunctionRunnerModuleLoader<RT> {
    fn put_cached_code(&self, module_metadata: &ModuleMetadata, cached_data: Arc<[u8]>) {
        self.code_cache
            .0
            .insert(self.cache_key(module_metadata), cached_data);
        crate::metrics::record_code_cache_size(self.code_cache.0.weighted_size());
    }

    fn get_cached_code(&self, module_metadata: &ModuleMetadata) -> Option<Arc<[u8]>> {
        self.code_cache.0.get(&self.cache_key(module_metadata))
    }
}
