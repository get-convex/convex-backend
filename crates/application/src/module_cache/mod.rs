use std::sync::Arc;

use async_lru::async_lru::AsyncLru;
use async_trait::async_trait;
use common::{
    document::ParsedDocument,
    knobs::{
        MODULE_CACHE_MAX_CONCURRENCY,
        MODULE_CACHE_MAX_SIZE_BYTES,
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
use storage::Storage;
use sync_types::CanonicalizedModulePath;
use value::sha256::Sha256Digest;

mod metrics;

#[derive(Clone)]
pub struct ModuleCache<RT: Runtime> {
    modules_storage: Arc<dyn Storage>,

    cache: AsyncLru<RT, (CanonicalizedModulePath, Sha256Digest), FullModuleSource, Sha256Digest>,
}

impl<RT: Runtime> ModuleCache<RT> {
    pub async fn new(rt: RT, modules_storage: Arc<dyn Storage>) -> Self {
        let cache = AsyncLru::new(
            rt,
            *MODULE_CACHE_MAX_SIZE_BYTES,
            *MODULE_CACHE_MAX_CONCURRENCY,
            200,
            "module_cache",
        );

        Self {
            modules_storage,
            cache,
        }
    }
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for ModuleCache<RT> {
    #[fastrace::trace]
    async fn get_module_with_metadata(
        &self,
        module_metadata: &ParsedDocument<ModuleMetadata>,
        source_package: &ParsedDocument<SourcePackage>,
    ) -> anyhow::Result<Arc<FullModuleSource>> {
        let timer = metrics::module_cache_get_module_timer();

        let key = (module_metadata.path.clone(), module_metadata.sha256.clone());
        let modules_storage = self.modules_storage.clone();
        let source_package = source_package.clone();
        let result = self
            .cache
            .get_and_prepopulate(
                key,
                source_package.sha256.clone(),
                async move {
                    Ok(get_modules_and_prefetch(modules_storage, &source_package)
                        .await?
                        .map(|(path, sha256, source)| ((path, sha256), Arc::new(source)))
                        .collect())
                }
                .boxed(),
            )
            .await?;

        let source_size = result.source.len();
        let source_map_size = result.source_map.as_ref().map(|sm| sm.len());
        function_runner::record_module_sizes(source_size, source_map_size);
        timer.finish();
        Ok(result)
    }
}
