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
use database::{
    Database,
    Transaction,
};
use futures::FutureExt;
use isolate::ModuleLoader;
use keybroker::Identity;
use model::modules::{
    module_versions::{
        FullModuleSource,
        ModuleVersion,
    },
    types::ModuleMetadata,
    ModuleModel,
    MODULE_VERSIONS_TABLE,
};
use storage::Storage;
use value::ResolvedDocumentId;

mod metrics;

#[derive(Clone)]
pub struct ModuleVersionFetcher<RT: Runtime> {
    database: Database<RT>,
    // TODO(lee) read module source from storage.
    #[allow(unused)]
    modules_storage: Arc<dyn Storage>,
}

impl<RT: Runtime> ModuleVersionFetcher<RT> {
    async fn generate_value(
        self,
        key: (ResolvedDocumentId, ModuleVersion),
    ) -> anyhow::Result<FullModuleSource> {
        let mut tx = self.database.begin(Identity::system()).await?;
        ModuleModel::new(&mut tx).get_source(key.0, key.1).await
    }
}

#[derive(Clone)]
pub struct ModuleCache<RT: Runtime> {
    database: Database<RT>,

    modules_storage: Arc<dyn Storage>,

    cache: AsyncLru<RT, (ResolvedDocumentId, ModuleVersion), FullModuleSource>,
}

impl<RT: Runtime> ModuleCache<RT> {
    pub async fn new(rt: RT, database: Database<RT>, modules_storage: Arc<dyn Storage>) -> Self {
        let cache = AsyncLru::new(
            rt.clone(),
            *MODULE_CACHE_MAX_SIZE_BYTES,
            *MODULE_CACHE_MAX_CONCURRENCY,
            "module_cache",
        );

        Self {
            database,
            modules_storage,
            cache,
        }
    }
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for ModuleCache<RT> {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Option<Arc<FullModuleSource>>> {
        let timer = metrics::module_cache_get_module_timer();

        // If this transaction wrote to module_versions (true for REPLs), we cannot use
        // the cache, load the module directly.
        let module_versions_table_id = tx.table_mapping().id(&MODULE_VERSIONS_TABLE)?;
        if tx.writes().has_written_to(&module_versions_table_id) {
            let source = ModuleModel::new(tx)
                .get_source(module_metadata.id(), module_metadata.latest_version)
                .await?;
            return Ok(Some(Arc::new(source)));
        }

        let key = (module_metadata.id(), module_metadata.latest_version);
        let fetcher = ModuleVersionFetcher {
            database: self.database.clone(),
            modules_storage: self.modules_storage.clone(),
        };
        let result = self
            .cache
            .get(key, fetcher.generate_value(key).boxed())
            .await?;
        // Record read dependency on the module version so the transactions
        // read same is the same regardless if we hit the cache or not.
        // This is not technically needed since the module version is immutable,
        // but better safe and consistent that sorry.
        ModuleModel::new(tx).record_module_version_read_dependency(key.0)?;

        timer.finish();
        Ok(Some(result))
    }
}
