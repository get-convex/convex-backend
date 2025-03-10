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
use storage::Storage;
use sync_types::CanonicalizedModulePath;

mod metrics;

#[derive(Clone)]
pub struct ModuleCache<RT: Runtime> {
    modules_storage: Arc<dyn Storage>,

    cache: AsyncLru<RT, (CanonicalizedModulePath, SourcePackageId), FullModuleSource>,
}

impl<RT: Runtime> ModuleCache<RT> {
    pub async fn new(rt: RT, modules_storage: Arc<dyn Storage>) -> Self {
        let cache = AsyncLru::new(
            rt.clone(),
            *MODULE_CACHE_MAX_SIZE_BYTES,
            *MODULE_CACHE_MAX_CONCURRENCY,
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
        module_metadata: ParsedDocument<ModuleMetadata>,
        source_package: ParsedDocument<SourcePackage>,
    ) -> anyhow::Result<Arc<FullModuleSource>> {
        let timer = metrics::module_cache_get_module_timer();

        let key = (
            module_metadata.path.clone(),
            module_metadata.source_package_id,
        );
        let modules_storage = self.modules_storage.clone();
        let result = self
            .cache
            .get_and_prepopulate(
                key,
                get_module_and_prefetch(modules_storage, module_metadata, source_package).boxed(),
            )
            .await?;

        let source_size = result.source.len();
        let source_map_size = result.source_map.as_ref().map(|sm| sm.len());
        function_runner::record_module_sizes(source_size, source_map_size);
        timer.finish();
        Ok(result)
    }
}
