use std::sync::Arc;

use common::runtime::Runtime;
use model::{
    config::module_loader::ModuleLoader,
    modules::types::ModuleMetadata,
};

use crate::environment::ModuleCodeCacheResult;

/// A `ModuleLoader` that also has the ability to store V8 code caches.
pub trait ModuleCache<RT: Runtime>: ModuleLoader<RT> {
    fn put_cached_code(&self, module_metadata: &ModuleMetadata, cached_data: Arc<[u8]>);
    fn get_cached_code(&self, module_metadata: &ModuleMetadata) -> Option<Arc<[u8]>>;
}

impl<RT: Runtime> dyn ModuleCache<RT> {
    pub fn code_cache_result(
        self: Arc<Self>,
        module_metadata: ModuleMetadata,
    ) -> ModuleCodeCacheResult {
        if let Some(cached_data) = self.get_cached_code(&module_metadata) {
            ModuleCodeCacheResult::Cached(cached_data)
        } else {
            ModuleCodeCacheResult::Uncached(Box::new(move |cached_data| {
                self.put_cached_code(&module_metadata, cached_data);
            }))
        }
    }
}

#[cfg(any(test, feature = "testing"))]
mod test_helpers {
    use model::config::module_loader::test_module_loader::UncachedModuleLoader;

    use super::*;
    impl<RT: Runtime> ModuleCache<RT> for UncachedModuleLoader {
        fn put_cached_code(&self, _module_metadata: &ModuleMetadata, _cached_data: Arc<[u8]>) {}

        fn get_cached_code(&self, _module_metadata: &ModuleMetadata) -> Option<Arc<[u8]>> {
            None
        }
    }
}
