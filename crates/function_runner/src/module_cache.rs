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
use isolate::environment::helpers::module_loader::get_module_and_prefetch;
use model::{
    config::module_loader::ModuleLoader,
    modules::{
        module_versions::FullModuleSource,
        types::ModuleMetadata,
    },
    source_packages::types::{
        SourcePackage,
        SourcePackageId,
    },
};
use moka::sync::Cache;
use storage::Storage;
use sync_types::CanonicalizedModulePath;

use crate::record_module_sizes;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ModuleCacheKey {
    instance_name: String,
    module_path: CanonicalizedModulePath,
    source_package_id: SourcePackageId,
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
    pub instance_name: String,
    pub modules_storage: Arc<dyn Storage>,
}

impl<RT: Runtime> FunctionRunnerModuleLoader<RT> {
    fn cache_key(&self, module_metadata: &ModuleMetadata) -> ModuleCacheKey {
        ModuleCacheKey {
            instance_name: self.instance_name.clone(),
            module_path: module_metadata.path.clone(),
            source_package_id: module_metadata.source_package_id,
        }
    }
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for FunctionRunnerModuleLoader<RT> {
    #[fastrace::trace]
    async fn get_module_with_metadata(
        &self,
        module_metadata: ParsedDocument<ModuleMetadata>,
        source_package: ParsedDocument<SourcePackage>,
    ) -> anyhow::Result<Arc<FullModuleSource>> {
        let instance_name = self.instance_name.clone();
        let key = self.cache_key(&module_metadata);
        let modules_storage = self.modules_storage.clone();
        let result = self
            .cache
            .0
            .get_and_prepopulate(
                key.clone(),
                async move {
                    let modules =
                        get_module_and_prefetch(modules_storage, module_metadata, source_package)
                            .await;
                    modules
                        .into_iter()
                        .map(move |((module_path, source_package_id), source)| {
                            (
                                ModuleCacheKey {
                                    instance_name: instance_name.clone(),
                                    module_path,
                                    source_package_id,
                                },
                                source,
                            )
                        })
                        .collect()
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
