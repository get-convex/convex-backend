use std::sync::Arc;

use async_lru::async_lru::AsyncLru;
use async_trait::async_trait;
use common::{
    document::ParsedDocument,
    knobs::{
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
use storage::Storage;
use sync_types::CanonicalizedModulePath;

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

pub(crate) struct FunctionRunnerModuleLoader<RT: Runtime> {
    pub cache: ModuleCache<RT>,
    pub instance_name: String,
    pub modules_storage: Arc<dyn Storage>,
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for FunctionRunnerModuleLoader<RT> {
    #[minitrace::trace]
    async fn get_module_with_metadata(
        &self,
        module_metadata: ParsedDocument<ModuleMetadata>,
        source_package: ParsedDocument<SourcePackage>,
    ) -> anyhow::Result<Arc<FullModuleSource>> {
        let instance_name = self.instance_name.clone();
        let key = ModuleCacheKey {
            instance_name: self.instance_name.clone(),
            module_path: module_metadata.path.clone(),
            source_package_id: module_metadata.source_package_id,
        };
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

        Ok(result)
    }
}
