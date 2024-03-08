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
use database::Transaction;
use futures::FutureExt;
use isolate::{
    environment::helpers::module_loader::get_module,
    ModuleLoader,
};
use model::modules::{
    module_versions::{
        ModuleVersion,
        ModuleVersionMetadata,
    },
    types::ModuleMetadata,
    ModuleModel,
    MODULE_VERSIONS_TABLE,
};
use value::ResolvedDocumentId;

use crate::in_memory_indexes::TransactionIngredients;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ModuleCacheKey {
    instance_name: String,
    module_id: ResolvedDocumentId,
    module_version: ModuleVersion,
}

#[derive(Clone)]
pub(crate) struct ModuleCache<RT: Runtime>(AsyncLru<RT, ModuleCacheKey, ModuleVersionMetadata>);

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
    pub transaction_ingredients: TransactionIngredients<RT>,
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for FunctionRunnerModuleLoader<RT> {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Option<Arc<ModuleVersionMetadata>>> {
        // The transaction we're getting modules for should be from the same ts as when
        // this module loader was created.
        assert_eq!(tx.begin_timestamp(), self.transaction_ingredients.ts);

        // If this transaction wrote to module_versions (true for REPLs), we cannot use
        // the cache, load the module directly.
        let module_versions_table_id = tx.table_mapping().id(&MODULE_VERSIONS_TABLE)?;
        if tx.writes().has_written_to(&module_versions_table_id) {
            let module_version = ModuleModel::new(tx)
                .get_version(module_metadata.id(), module_metadata.latest_version)
                .await?
                .into_value();
            return Ok(Some(Arc::new(module_version)));
        }

        let key = ModuleCacheKey {
            instance_name: self.instance_name.clone(),
            module_id: module_metadata.id(),
            module_version: module_metadata.latest_version,
        };
        let transaction = self.transaction_ingredients.clone().try_into()?;
        let result = self
            .cache
            .0
            .get(
                key.clone(),
                get_module(transaction, module_metadata).boxed(),
            )
            .await?;
        // Record read dependency on the module version so the transactions
        // read same is the same regardless if we hit the cache or not.
        // This is not technically needed since the module version is immutable,
        // but better safe and consistent that sorry.
        ModuleModel::new(tx)
            .record_module_version_read_dependency(key.module_id, key.module_version)?;

        Ok(Some(result))
    }
}
