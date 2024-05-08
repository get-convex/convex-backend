use std::{
    collections::{
        BTreeSet,
        HashMap,
    },
    sync::Arc,
    time::Duration,
};

use async_lru::async_lru::AsyncLru;
use async_trait::async_trait;
use common::{
    backoff::Backoff,
    document::ParsedDocument,
    errors::report_error,
    knobs::{
        MODULE_CACHE_MAX_CONCURRENCY,
        MODULE_CACHE_MAX_SIZE_BYTES,
    },
    runtime::{
        Runtime,
        SpawnHandle,
    },
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
        ModuleVersion,
        ModuleVersionMetadata,
    },
    types::ModuleMetadata,
    ModuleModel,
    MODULE_VERSIONS_TABLE,
};
use parking_lot::Mutex;
use storage::Storage;
use value::ResolvedDocumentId;

mod metrics;

const INITIAL_BACKOFF: Duration = Duration::from_millis(10);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

pub struct ModuleCacheWorker<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
    modules_storage: Arc<dyn Storage>,
    cache: AsyncLru<RT, (ResolvedDocumentId, ModuleVersion), ModuleVersionMetadata>,
}

impl<RT: Runtime> ModuleCacheWorker<RT> {
    pub async fn start(
        rt: RT,
        database: Database<RT>,
        modules_storage: Arc<dyn Storage>,
    ) -> ModuleCache<RT> {
        let cache = AsyncLru::new(
            rt.clone(),
            *MODULE_CACHE_MAX_SIZE_BYTES,
            *MODULE_CACHE_MAX_CONCURRENCY,
            "module_cache",
        );
        let worker = Self {
            rt: rt.clone(),
            database: database.clone(),
            modules_storage: modules_storage.clone(),
            cache: cache.clone(),
        };

        let worker_handle = rt.spawn("module_cache_worker", worker.go());
        ModuleCache {
            database,
            modules_storage,
            cache,
            worker: Arc::new(Mutex::new(worker_handle)),
        }
    }

    async fn go(mut self) {
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            match self.run(&mut backoff).await {
                Ok(()) => break,
                Err(mut e) => {
                    let delay = self.rt.with_rng(|rng| backoff.fail(rng));
                    tracing::error!("Module version cache failed, sleeping {delay:?}");
                    report_error(&mut e);
                    self.rt.wait(delay).await;
                },
            }
        }
    }

    async fn run(&mut self, backoff: &mut Backoff) -> anyhow::Result<()> {
        tracing::info!("Starting ModuleCache worker");
        loop {
            let mut tx = self.database.begin(Identity::system()).await?;
            let modules_metadata = ModuleModel::new(&mut tx).get_all_metadata().await?;
            let referenced_versions = modules_metadata
                .into_iter()
                .map(|m| (m.id(), m.latest_version))
                .collect::<BTreeSet<_>>();

            // Eagerly populate the cache with all referenced versions. They may be evicted
            // if the number of modules is high and lots of UDFs are using old
            // versions, but on average they should be populated and remain.
            let num_loaded = referenced_versions.len();
            let fetcher = ModuleVersionFetcher {
                database: self.database.clone(),
                modules_storage: self.modules_storage.clone(),
            };
            if let Some(first_key) = referenced_versions.first().cloned() {
                self.cache
                    .get_and_prepopulate(
                        first_key,
                        fetcher.generate_values(referenced_versions).boxed(),
                    )
                    .await?;
            }

            tracing::info!(
                "Cached module count: {} (Loaded: {})",
                self.cache.size(),
                num_loaded,
            );

            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            subscription.wait_for_invalidation().await;
            tracing::info!("ModuleCache worker resuming after index subscription notification");
            backoff.reset();
        }
    }
}

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
    ) -> anyhow::Result<ModuleVersionMetadata> {
        let mut tx = self.database.begin(Identity::system()).await?;
        Ok(ModuleModel::new(&mut tx)
            .get_version(key.0, key.1)
            .await?
            .into_value())
    }

    async fn generate_values(
        self,
        keys: BTreeSet<(ResolvedDocumentId, ModuleVersion)>,
    ) -> HashMap<(ResolvedDocumentId, ModuleVersion), anyhow::Result<ModuleVersionMetadata>> {
        let mut hashmap = HashMap::new();
        for key in keys {
            hashmap.insert(
                key,
                try {
                    let mut tx = self.database.begin(Identity::system()).await?;
                    ModuleModel::new(&mut tx)
                        .get_version(key.0, key.1)
                        .await?
                        .into_value()
                },
            );
        }
        hashmap
    }
}

pub struct ModuleCache<RT: Runtime> {
    database: Database<RT>,

    modules_storage: Arc<dyn Storage>,

    cache: AsyncLru<RT, (ResolvedDocumentId, ModuleVersion), ModuleVersionMetadata>,

    worker: Arc<Mutex<RT::Handle>>,
}

impl<RT: Runtime> ModuleCache<RT> {
    pub fn shutdown(&self) {
        self.worker.lock().shutdown();
    }
}

impl<RT: Runtime> Clone for ModuleCache<RT> {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            modules_storage: self.modules_storage.clone(),
            cache: self.cache.clone(),
            worker: self.worker.clone(),
        }
    }
}

#[async_trait]
impl<RT: Runtime> ModuleLoader<RT> for ModuleCache<RT> {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Option<Arc<ModuleVersionMetadata>>> {
        let timer = metrics::module_cache_get_module_timer();

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
