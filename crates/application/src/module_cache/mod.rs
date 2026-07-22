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
use model::{
    config::module_loader::ModuleLoader,
    modules::{
        hash_module_source,
        module_versions::FullModuleSource,
        types::ModuleMetadata,
    },
    source_packages::{
        types::SourcePackage,
        upload_download::download_package,
    },
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
                    let package = download_package(modules_storage, &source_package).await?;
                    Ok(package
                        .into_iter()
                        .map(|(module_path, module_config)| {
                            (
                                (
                                    module_path,
                                    hash_module_source(
                                        &module_config.source,
                                        module_config.source_map.as_ref(),
                                    ),
                                ),
                                Arc::new(FullModuleSource {
                                    source: module_config.source,
                                    source_map: module_config.source_map,
                                }),
                            )
                        })
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
