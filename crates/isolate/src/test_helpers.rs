use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentFunctionPath,
        ComponentPath,
    },
    errors::JsError,
    execution_context::ExecutionContext,
    http::fetch::ProxiedFetchClient,
    log_lines::{
        LogLine,
        LogLines,
    },
    minitrace_helpers::EncodedSpan,
    pause::{
        PauseClient,
        PauseController,
    },
    persistence::Persistence,
    query_journal::QueryJournal,
    runtime::{
        testing::TestRuntime,
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    testing::TestPersistence,
    types::{
        AllowedVisibility,
        ModuleEnvironment,
        UdfType,
    },
    value::ConvexValue,
    version::Version,
};
use database::{
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    vector_index_worker::flusher::backfill_vector_indexes,
    Database,
    FollowerRetentionManager,
    IndexModel,
    IndexWorker,
    TextIndexFlusher,
    Transaction,
};
use file_storage::TransactionalFileStorage;
use futures::{
    channel::{
        mpsc,
        oneshot,
    },
    select,
    Future,
    FutureExt,
    StreamExt,
};
use keybroker::{
    Identity,
    InstanceSecret,
    KeyBroker,
    DEV_INSTANCE_NAME,
    DEV_SECRET,
};
use maplit::btreemap;
use model::{
    config::{
        module_loader::{
            test_module_loader::UncachedModuleLoader,
            ModuleLoader,
        },
        types::{
            ConfigMetadata,
            ModuleConfig,
        },
        ConfigModel,
    },
    file_storage::{
        types::FileStorageEntry,
        FileStorageId,
    },
    scheduled_jobs::VirtualSchedulerModel,
    source_packages::{
        types::SourcePackage,
        upload_download::upload_package,
    },
    test_helpers::DbFixturesWithModel,
    udf_config::{
        types::UdfConfig,
        UdfConfigModel,
    },
    virtual_system_mapping,
};
use rand::Rng;
use search::searcher::InProcessSearcher;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use storage::{
    LocalDirStorage,
    Storage,
};
use sync_types::UdfPath;
use usage_tracking::FunctionUsageStats;
use value::{
    id_v6::DeveloperDocumentId,
    ConvexArray,
    ConvexObject,
    TableName,
    TableNamespace,
};
use vector::{
    PublicVectorSearchQueryResult,
    VectorSearch,
};

use super::FunctionResult;
use crate::{
    bundled_js::UDF_TEST_BUNDLE_PATH,
    client::{
        initialize_v8,
        EnvironmentData,
        IsolateWorker,
        Request,
        RequestType,
        SharedIsolateHeapStats,
        UdfCallback,
        UdfRequest,
        PAUSE_RECREATE_CLIENT,
    },
    concurrency_limiter::ConcurrencyLimiter,
    environment::{
        action::outcome::ActionOutcome,
        helpers::{
            validation::ValidatedHttpPath,
            FunctionOutcome,
        },
        udf::outcome::UdfOutcome,
    },
    http_action::{
        HttpActionRequest,
        HttpActionResponsePart,
        HttpActionResponseStreamer,
    },
    isolate2::runner::{
        run_isolate_v2_udf,
        SeedData,
    },
    metrics::queue_timer,
    parse_udf_args,
    validate_schedule_args,
    ActionCallbacks,
    BackendIsolateWorker,
    HttpActionOutcome,
    HttpActionResponse,
    HttpActionResult,
    IsolateClient,
    IsolateConfig,
    ValidatedPathAndArgs,
    CONVEX_ORIGIN,
    CONVEX_SITE,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bundle {
    path: String,
    source: String,
    source_map: Option<String>,
    environment: ModuleEnvironment,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushOutput {
    modules: Vec<Bundle>,
}

fn load_test_source() -> anyhow::Result<Vec<ModuleConfig>> {
    let mut file = File::open(UDF_TEST_BUNDLE_PATH)?;
    let mut contents = vec![];
    file.read_to_end(&mut contents)?;
    let output: PushOutput = serde_json::from_slice(&contents)?;
    let mut modules = BTreeMap::new();
    for module in output.modules {
        let config = ModuleConfig {
            path: module.path.parse()?,
            source: module.source,
            source_map: module.source_map,
            environment: module.environment,
        };
        assert!(modules.insert(module.path, config).is_none());
    }
    Ok(modules.into_values().collect())
}

pub static TEST_SOURCE: LazyLock<Vec<ModuleConfig>> = LazyLock::new(|| load_test_source().unwrap());

pub static TEST_SOURCE_ISOLATE_ONLY: LazyLock<Vec<ModuleConfig>> = LazyLock::new(|| {
    TEST_SOURCE
        .clone()
        .into_iter()
        .filter(|m| m.environment == ModuleEnvironment::Isolate)
        .collect()
});

pub fn test_environment_data<RT: Runtime>(rt: RT) -> anyhow::Result<EnvironmentData<RT>> {
    let key_broker = KeyBroker::new(DEV_INSTANCE_NAME, InstanceSecret::try_from(DEV_SECRET)?)?;
    let modules_storage = Arc::new(LocalDirStorage::new(rt.clone())?);
    let module_loader = Arc::new(UncachedModuleLoader { modules_storage });
    let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
    let convex_origin = "http://127.0.0.1:8000".into();
    let file_storage = TransactionalFileStorage::new(rt.clone(), storage.clone(), convex_origin);

    let system_env_vars = btreemap! {
        CONVEX_ORIGIN.clone() => "https://carnitas.convex.cloud".parse()?,
        CONVEX_SITE.clone() => "https://carnitas.convex.site".parse()?
    };
    Ok(EnvironmentData {
        key_broker,
        system_env_vars,
        file_storage,
        module_loader,
    })
}

#[derive(Clone)]
pub struct UdfTest<RT: Runtime, P: Persistence + Clone> {
    pub database: Database<RT>,
    pub isolate: IsolateClient<RT>,
    pub persistence: Arc<P>,
    pub rt: RT,
    pub key_broker: KeyBroker,
    pub module_loader: Arc<dyn ModuleLoader<RT>>,
    search_storage: Arc<dyn Storage>,
    file_storage: TransactionalFileStorage<RT>,

    isolate_v2_enabled: bool,
}

impl<RT: Runtime, P: Persistence + Clone> UdfTest<RT, P> {
    async fn new(
        modules: Vec<ModuleConfig>,
        rt: RT,
        persistence: Arc<P>,
        config: UdfTestConfig,
        max_isolate_workers: usize,
    ) -> anyhow::Result<Result<Self, JsError>> {
        let DbFixtures {
            db: database,
            search_storage,
            ..
        } = DbFixtures::new_with_args(
            &rt,
            DbFixturesArgs {
                tp: Some(persistence.clone()),
                searcher: Some(Arc::new(InProcessSearcher::new(rt.clone()).await?)),
                virtual_system_mapping: virtual_system_mapping(),
                ..Default::default()
            },
        )
        .await?
        .with_model()
        .await?;
        database
            .start_search_and_vector_bootstrap(PauseClient::new())
            .into_join_future()
            .await?;
        let key_broker = KeyBroker::new(DEV_INSTANCE_NAME, InstanceSecret::try_from(DEV_SECRET)?)?;
        let modules_storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let module_loader = Arc::new(UncachedModuleLoader {
            modules_storage: modules_storage.clone(),
        });
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let convex_origin = "http://127.0.0.1:8000".into();
        let file_storage =
            TransactionalFileStorage::new(rt.clone(), storage.clone(), convex_origin);

        let system_env_vars = btreemap! {
            CONVEX_ORIGIN.clone() => "https://carnitas.convex.cloud".parse()?,
            CONVEX_SITE.clone() => "https://carnitas.convex.site".parse()?
        };
        let isolate_worker = BackendIsolateWorker::new(rt.clone(), config.isolate_config);
        let isolate = IsolateClient::new(
            rt.clone(),
            isolate_worker,
            max_isolate_workers,
            true,
            DEV_INSTANCE_NAME.to_string(),
            DEV_SECRET.try_into()?,
            file_storage.clone(),
            system_env_vars,
            module_loader.clone(),
        );

        anyhow::ensure!(
            modules
                .iter()
                .all(|m| m.environment == ModuleEnvironment::Isolate),
            "Only isolate modules allowed in isolate tests. Please use application level tests to \
             test Node modules"
        );

        let udf_config = UdfConfig::new_for_test(&rt, config.udf_server_version);
        let modules_by_path: BTreeMap<_, _> = modules
            .iter()
            .map(|c| (c.path.clone().canonicalize(), c.clone()))
            .collect();
        let analyze_results = match isolate
            .analyze(udf_config.clone(), modules_by_path.clone(), BTreeMap::new())
            .await?
        {
            Ok(analyze_results) => analyze_results,
            Err(err) => return Ok(Err(err)),
        };

        let (storage_key, sha256, package_size) = upload_package(
            modules_by_path
                .iter()
                .map(|(path, m)| (path.clone(), m))
                .collect(),
            modules_storage,
            None,
        )
        .await?;
        let mut tx = database.begin(Identity::system()).await?;
        ConfigModel::new(&mut tx)
            .apply(
                ConfigMetadata::new(),
                modules,
                udf_config,
                Some(SourcePackage {
                    storage_key,
                    sha256,
                    package_size,
                    external_deps_package_id: None,
                }),
                analyze_results,
                None,
            )
            .await?;
        database.commit(tx).await?;

        Ok(Ok(Self {
            database,
            isolate,
            persistence,
            rt,
            key_broker,
            search_storage,
            module_loader,
            file_storage,
            isolate_v2_enabled: false,
        }))
    }

    pub async fn with_persistence(
        rt: RT,
        persistence: Arc<P>,
        config: UdfTestConfig,
        max_isolate_workers: usize,
    ) -> anyhow::Result<Self> {
        let result = Self::new(
            TEST_SOURCE_ISOLATE_ONLY.clone(),
            rt,
            persistence,
            config,
            max_isolate_workers,
        )
        .await?
        .expect("Unexpected JSError");
        Ok(result)
    }

    pub fn enable_isolate_v2(&mut self) {
        self.isolate_v2_enabled = true;
    }

    pub async fn create_index(&self, name: &str, field: &str) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let index_name = name.parse()?;
        let index = IndexMetadata::new_backfilling(
            *tx.begin_timestamp(),
            index_name,
            IndexedFields::try_from(vec![field.parse()?])?,
        );
        IndexModel::new(&mut tx)
            .add_application_index(index)
            .await?;
        self.database.commit(tx).await?;

        let retention_manager =
            FollowerRetentionManager::new(self.rt.clone(), self.persistence.reader()).await?;
        IndexWorker::new_terminating(
            self.rt.clone(),
            self.persistence.clone(),
            Arc::new(retention_manager),
            self.database.clone(),
        )
        .await
    }

    pub async fn mutation(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<ConvexValue> {
        self.mutation_with_identity(udf_path, args, Identity::system())
            .await
    }

    pub async fn mutation_js_error(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<JsError> {
        let outcome: UdfOutcome = self
            .raw_mutation(
                udf_path,
                vec![ConvexValue::Object(args)],
                Identity::system(),
            )
            .await?;
        Ok(outcome.result.unwrap_err())
    }

    pub async fn mutation_log_lines(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<LogLines> {
        let (_, outcome) = self
            .mutation_outcome(udf_path, args, Identity::system())
            .await?;
        Ok(outcome.log_lines)
    }

    pub async fn mutation_with_identity(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<ConvexValue> {
        let (v, _) = self.mutation_outcome(udf_path, args, identity).await?;
        Ok(v)
    }

    pub async fn mutation_outcome(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<(ConvexValue, UdfOutcome)> {
        let outcome = self
            .raw_mutation(udf_path, vec![ConvexValue::Object(args)], identity)
            .await?;
        let value = outcome
            .result
            .as_ref()
            .map(|v| v.unpack())
            .map_err(|e| {
                anyhow::anyhow!(
                    "mutation failed with user error. If that is intended, call mutation_js_error \
                     or raw_mutation instead. {e:?}"
                )
            })
            .unwrap();
        Ok((value, outcome))
    }

    pub async fn raw_mutation(
        &self,
        udf_path: &str,
        args: Vec<ConvexValue>,
        identity: Identity,
    ) -> anyhow::Result<UdfOutcome> {
        // TODO: This will panic if used within a prod_rt test.
        // Bump time before running a mutation so we have a higher creation time than
        // previous mutations.
        tokio::time::advance(Duration::from_secs(1)).await;

        let mut tx = self.database.begin(identity.clone()).await?;
        let path = ComponentFunctionPath {
            component: ComponentPath::root(),
            udf_path: udf_path.parse()?,
        };
        let canonicalized_path = path.canonicalize();

        let args_array = ConvexArray::try_from(args)?;

        let validated_path_or_err = ValidatedPathAndArgs::new(
            AllowedVisibility::PublicOnly,
            &mut tx,
            canonicalized_path.clone(),
            args_array.clone(),
            UdfType::Mutation,
        )
        .await?;

        let path_and_args = match validated_path_or_err {
            Err(js_error) => {
                return UdfOutcome::from_error(
                    js_error,
                    canonicalized_path,
                    args_array,
                    identity.into(),
                    self.rt.clone(),
                    None,
                );
            },
            Ok(path_and_args) => path_and_args,
        };

        if self.isolate_v2_enabled {
            let (tx, outcome) = run_isolate_v2_udf(
                self.rt.clone(),
                tx,
                self.module_loader.clone(),
                SeedData {
                    rng_seed: self.rt.with_rng(|rng| rng.gen()),
                    unix_timestamp: self.rt.unix_timestamp(),
                },
                UdfType::Mutation,
                path_and_args,
                self.key_broker.clone(),
                ExecutionContext::new_for_test(),
                QueryJournal::new(),
            )
            .await?;
            let path: UdfPath = udf_path.parse()?;
            let canonicalized_path = path.canonicalize();
            self.database
                .commit_with_write_source(tx, Some(canonicalized_path.into()))
                .await?;
            Ok(outcome)
        } else {
            let (tx, outcome) = self
                .isolate
                .execute_udf(
                    UdfType::Mutation,
                    path_and_args,
                    tx,
                    QueryJournal::new(),
                    ExecutionContext::new_for_test(),
                )
                .await?;
            let FunctionOutcome::Mutation(outcome) = outcome else {
                anyhow::bail!("Called raw_mutation on a non-mutation");
            };

            self.database
                .commit_with_write_source(tx, Some(canonicalized_path.into_root_udf_path()?.into()))
                .await?;
            Ok(outcome)
        }
    }

    pub async fn query(&self, udf_path: &str, args: ConvexObject) -> anyhow::Result<ConvexValue> {
        self.query_with_identity(udf_path, args, Identity::system())
            .await
    }

    pub async fn query_js_error(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<JsError> {
        let outcome = self
            .raw_query(
                udf_path,
                vec![ConvexValue::Object(args)],
                Identity::system(),
                None,
            )
            .await?;
        Ok(outcome.result.unwrap_err())
    }

    pub async fn query_log_lines(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<LogLines> {
        let (_, outcome) = self
            .query_outcome(udf_path, args, Identity::system())
            .await?;
        Ok(outcome.log_lines)
    }

    pub async fn query_with_identity(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<ConvexValue> {
        let (v, _) = self.query_outcome(udf_path, args, identity).await?;
        Ok(v)
    }

    /// Execute the query and also return the corresponding UdfOutcome struct.
    pub async fn query_outcome(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<(ConvexValue, UdfOutcome)> {
        let outcome = self
            .raw_query(udf_path, vec![ConvexValue::Object(args)], identity, None)
            .await?;
        let value = outcome
            .result
            .as_ref()
            .map(|v| v.unpack())
            .map_err(|e| {
                anyhow::anyhow!(
                    "query failed with user error. If that is intended, call query_js_error or \
                     raw_query instead. {e:?}"
                )
            })
            .unwrap();
        Ok((value, outcome))
    }

    pub async fn raw_query(
        &self,
        udf_path: &str,
        args: Vec<ConvexValue>,
        identity: Identity,
        journal: Option<QueryJournal>,
    ) -> anyhow::Result<UdfOutcome> {
        let mut tx = self.database.begin(identity.clone()).await?;
        let path = ComponentFunctionPath {
            component: ComponentPath::root(),
            udf_path: udf_path.parse()?,
        };
        let canonicalized_path = path.canonicalize();
        let args_array = ConvexArray::try_from(args)?;
        let validated_path_or_err = ValidatedPathAndArgs::new(
            AllowedVisibility::PublicOnly,
            &mut tx,
            canonicalized_path.clone(),
            args_array.clone(),
            UdfType::Query,
        )
        .await?;

        let path_and_args = match validated_path_or_err {
            Err(js_error) => {
                return UdfOutcome::from_error(
                    js_error,
                    canonicalized_path,
                    args_array,
                    identity.into(),
                    self.rt.clone(),
                    None,
                );
            },
            Ok(path_and_args) => path_and_args,
        };

        if self.isolate_v2_enabled {
            let (tx, outcome) = run_isolate_v2_udf(
                self.rt.clone(),
                tx,
                self.module_loader.clone(),
                SeedData {
                    rng_seed: self.rt.with_rng(|rng| rng.gen()),
                    unix_timestamp: self.rt.unix_timestamp(),
                },
                UdfType::Query,
                path_and_args,
                self.key_broker.clone(),
                ExecutionContext::new_for_test(),
                journal.unwrap_or_else(QueryJournal::new),
            )
            .await?;
            // Ensure the transaction is readonly by turning it into a subscription token.
            let _ = tx.into_token()?;
            Ok(outcome)
        } else {
            let (tx, outcome) = self
                .isolate
                .execute_udf(
                    UdfType::Query,
                    path_and_args,
                    tx,
                    journal.unwrap_or_else(QueryJournal::new),
                    ExecutionContext::new_for_test(),
                )
                .await?;
            // Ensure the transaction is readonly by turning it into a subscription token.
            let _ = tx.into_token()?;
            let FunctionOutcome::Query(query_outcome) = outcome else {
                anyhow::bail!("Called raw_query on a non-query");
            };
            Ok(query_outcome)
        }
    }

    /// Run a query, bypassing the validation done in `ValidatedUdfPathAndArgs`,
    /// and retrieve the JS error it produces.
    ///
    /// This can be useful for testing errors from lower layers.
    pub async fn query_js_error_no_validation(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<JsError> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let udf_config = UdfConfigModel::new(&mut tx).get().await?;
        let npm_version = udf_config
            .context("Missing udf_config")?
            .server_version
            .clone();

        let path: UdfPath = udf_path.parse()?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: path.canonicalize(),
            },
            ConvexArray::try_from(vec![ConvexValue::Object(args)])?,
            Some(npm_version),
        );

        if self.isolate_v2_enabled {
            let (_, outcome) = run_isolate_v2_udf(
                self.rt.clone(),
                tx,
                self.module_loader.clone(),
                SeedData {
                    rng_seed: self.rt.with_rng(|rng| rng.gen()),
                    unix_timestamp: self.rt.unix_timestamp(),
                },
                UdfType::Query,
                path_and_args,
                self.key_broker.clone(),
                ExecutionContext::new_for_test(),
                QueryJournal::new(),
            )
            .await?;
            Ok(outcome.result.unwrap_err())
        } else {
            let (_, outcome) = self
                .isolate
                .execute_udf(
                    UdfType::Query,
                    path_and_args,
                    tx,
                    QueryJournal::new(),
                    ExecutionContext::new_for_test(),
                )
                .await?;
            match outcome {
                FunctionOutcome::Query(query_outcome) => Ok(query_outcome.result.unwrap_err()),
                _ => Err(anyhow::anyhow!(
                    "Called query_js_error_no_validation on a non-query"
                )),
            }
        }
    }

    pub async fn add_index(&self, index: IndexMetadata<TableName>) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        IndexModel::new(&mut tx)
            .add_application_index(index)
            .await?;
        self.database.commit(tx).await?;
        Ok(())
    }

    pub async fn backfill_indexes(&self) -> anyhow::Result<()> {
        let retention_validator = Arc::new(
            FollowerRetentionManager::new(self.rt.clone(), self.persistence.reader()).await?,
        );
        IndexWorker::new_terminating(
            self.rt.clone(),
            self.persistence.clone(),
            retention_validator,
            self.database.clone(),
        )
        .await?;

        self.enable_backfilled_indexes().await?;

        Ok(())
    }

    pub async fn backfill_search_indexes(&self) -> anyhow::Result<()> {
        TextIndexFlusher::backfill_all_in_test(
            self.rt.clone(),
            self.database.clone(),
            self.search_storage.clone(),
        )
        .await?;
        self.enable_backfilled_indexes().await
    }

    pub async fn backfill_vector_indexes(&self) -> anyhow::Result<()> {
        backfill_vector_indexes(
            self.rt.clone(),
            self.database.clone(),
            self.persistence.reader(),
            self.search_storage.clone(),
        )
        .await?;
        self.enable_backfilled_indexes().await
    }

    async fn enable_backfilled_indexes(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin_system().await?;
        let indexes: Vec<IndexMetadata<TableName>> = IndexModel::new(&mut tx)
            .get_application_indexes()
            .await?
            .into_iter()
            .map(|doc| doc.into_value())
            .filter(|index| !index.config.is_enabled())
            .collect();

        for index in indexes {
            IndexModel::new(&mut tx)
                .enable_index_for_testing(TableNamespace::Global, &index.name)
                .await?
        }

        self.database.commit(tx).await?;
        Ok(())
    }

    pub async fn http_action(
        &self,
        udf_path: &str,
        http_request: HttpActionRequest,
        identity: Identity,
    ) -> anyhow::Result<HttpActionResponse> {
        let (result, _log_lines) = self._http_action(udf_path, http_request, identity).await?;
        match result {
            Ok(r) => Ok(r),
            Err(e) => anyhow::bail!(
                "action failed with user error. If that is intended, call http_action_js_error or \
                 raw_http_action instead. {e:?}"
            ),
        }
    }

    pub async fn http_action_with_log_lines(
        &self,
        udf_path: &str,
        http_request: HttpActionRequest,
        identity: Identity,
    ) -> anyhow::Result<(HttpActionResponse, LogLines)> {
        let (result, log_lines) = self._http_action(udf_path, http_request, identity).await?;
        match result {
            Ok(r) => Ok((r, log_lines)),
            Err(e) => anyhow::bail!(
                "action failed with user error. If that is intended, call http_action_js_error or \
                 raw_http_action instead. {e:?}"
            ),
        }
    }

    pub async fn http_action_js_error(
        &self,
        udf_path: &str,
        http_request: HttpActionRequest,
        identity: Identity,
    ) -> anyhow::Result<JsError> {
        let (result, _log_lines) = self._http_action(udf_path, http_request, identity).await?;
        Ok(result.unwrap_err())
    }

    async fn _http_action(
        &self,
        udf_path: &str,
        http_request: HttpActionRequest,
        identity: Identity,
    ) -> anyhow::Result<(Result<HttpActionResponse, JsError>, LogLines)> {
        let (response_sender, mut response_receiver) = mpsc::unbounded();
        let http_response_streamer = HttpActionResponseStreamer::new(response_sender);
        let (outcome, log_lines) = self
            .raw_http_action(udf_path, http_request, identity, http_response_streamer)
            .await?;
        let mut response_head = None;
        let mut body = vec![];
        while let Some(part) = response_receiver.next().await {
            match part {
                HttpActionResponsePart::BodyChunk(bytes) => body.extend(bytes),
                HttpActionResponsePart::Head(head) => response_head = Some(head),
            }
        }
        let response = match outcome.result {
            HttpActionResult::Error(e) => Err(e),
            HttpActionResult::Streamed => {
                let response_head = response_head.unwrap();
                Ok(HttpActionResponse {
                    body: Some(body),
                    status: response_head.status,
                    headers: response_head.headers,
                })
            },
        };
        Ok((response, log_lines))
    }

    pub async fn raw_http_action(
        &self,
        udf_path: &str,
        http_request: HttpActionRequest,
        identity: Identity,
        http_response_streamer: HttpActionResponseStreamer,
    ) -> anyhow::Result<(HttpActionOutcome, LogLines)> {
        let app = Arc::new(self.clone());
        let mut tx = self.database.begin(identity.clone()).await?;
        let path: UdfPath = udf_path.parse()?;

        let fetch_client = Arc::new(ProxiedFetchClient::new(None, DEV_INSTANCE_NAME.to_owned()));
        let (log_line_sender, log_line_receiver) = mpsc::unbounded();
        let outcome = self
            .isolate
            .execute_http_action(
                ValidatedHttpPath::new_for_tests(&mut tx, path.canonicalize(), None).await?,
                http_request,
                identity,
                app.clone(),
                fetch_client,
                log_line_sender,
                http_response_streamer,
                tx,
                ExecutionContext::new_for_test(),
            )
            .await?;
        let log_lines: Vec<LogLine> = log_line_receiver.collect().await;
        Ok((outcome, log_lines.into()))
    }

    pub async fn action(&self, udf_path: &str, args: ConvexObject) -> anyhow::Result<ConvexValue> {
        self.action_with_identity(udf_path, args, Identity::system())
            .await
    }

    pub async fn action_js_error(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<JsError> {
        let (outcome, _log_lines) = self
            .raw_action(
                udf_path,
                vec![ConvexValue::Object(args)],
                Identity::system(),
            )
            .await?;
        Ok(outcome.result.unwrap_err())
    }

    pub async fn action_log_lines(
        &self,
        udf_path: &str,
        args: ConvexObject,
    ) -> anyhow::Result<LogLines> {
        let (_value, _outcome, log_lines) = self
            .action_outcome_and_log_lines(udf_path, args, Identity::system())
            .await?;
        Ok(log_lines)
    }

    pub async fn action_with_identity(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<ConvexValue> {
        let (v, _) = self.action_outcome(udf_path, args, identity).await?;
        Ok(v)
    }

    pub async fn action_outcome(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<(ConvexValue, ActionOutcome)> {
        let (value, outcome, _) = self
            .action_outcome_and_log_lines(udf_path, args, identity)
            .await?;
        Ok((value, outcome))
    }

    pub async fn action_outcome_and_log_lines(
        &self,
        udf_path: &str,
        args: ConvexObject,
        identity: Identity,
    ) -> anyhow::Result<(ConvexValue, ActionOutcome, LogLines)> {
        let (outcome, log_lines) = self
            .raw_action(udf_path, vec![ConvexValue::Object(args)], identity)
            .await?;
        let value = outcome
            .result
            .as_ref()
            .map(|v| v.unpack())
            .map_err(|e| {
                anyhow::anyhow!(
                    "action failed with user error. If that is intended, call action_js_error or \
                     raw_action instead. {e:?}"
                )
            })
            .unwrap();
        Ok((value, outcome, log_lines))
    }

    pub async fn raw_action(
        &self,
        udf_path: &str,
        args: Vec<ConvexValue>,
        identity: Identity,
    ) -> anyhow::Result<(ActionOutcome, LogLines)> {
        let mut tx = self.database.begin(identity.clone()).await?;
        let path = ComponentFunctionPath {
            component: ComponentPath::root(),
            udf_path: udf_path.parse()?,
        };
        let canonicalized_path = path.canonicalize();
        let args_array = ConvexArray::try_from(args)?;
        let validated_path_or_err = ValidatedPathAndArgs::new(
            AllowedVisibility::PublicOnly,
            &mut tx,
            canonicalized_path.clone(),
            args_array.clone(),
            UdfType::Action,
        )
        .await?;
        let path_and_args = match validated_path_or_err {
            Err(js_error) => {
                return Ok((
                    ActionOutcome::from_error(
                        js_error,
                        canonicalized_path,
                        args_array,
                        identity.into(),
                        self.rt.clone(),
                        None,
                    ),
                    vec![].into(),
                ))
            },
            Ok(path_and_args) => path_and_args,
        };
        let fetch_client = Arc::new(ProxiedFetchClient::new(None, DEV_INSTANCE_NAME.to_owned()));
        let (log_line_sender, log_line_receiver) = mpsc::unbounded();

        // TODO(presley): Make this also be able to use local executor.
        let outcome = self
            .isolate
            .execute_action(
                path_and_args,
                tx,
                Arc::new(self.clone()),
                fetch_client,
                log_line_sender,
                ExecutionContext::new_for_test(),
            )
            .await?;
        let log_lines: Vec<LogLine> = log_line_receiver.collect().await;
        Ok((outcome, log_lines.into()))
    }
}

static DEFAULT_CONFIG: LazyLock<UdfTestConfig> = LazyLock::new(|| UdfTestConfig {
    isolate_config: IsolateConfig::default(),
    udf_server_version: Version::parse("1000.0.0").unwrap(),
});

static DEFAULT_MAX_ISOLATE_WORKERS: usize = 1;

#[derive(Clone)]
pub struct UdfTestConfig {
    pub isolate_config: IsolateConfig,
    pub udf_server_version: Version,
}

/// Rust can't seem to determine the type of the function argument in these
/// tests, so we specify it explicitly.
pub type UdfTestType = UdfTest<TestRuntime, TestPersistence>;

impl<RT: Runtime> UdfTest<RT, TestPersistence> {
    pub async fn run_test_with_isolate<F, Fut>(rt: RT, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(Self) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        let test = Self::default(rt.clone()).await?;
        f(test).await?;

        Ok(())
    }

    pub async fn run_test_with_isolate2<F, Fut>(rt: RT, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(Self) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        let test = Self::default(rt.clone()).await?;
        f(test).await.context("test failed on isolate1")?;

        let mut test = Self::default(rt.clone()).await?;
        test.enable_isolate_v2();
        f(test).await.context("test failed on isolate2")?;

        Ok(())
    }

    pub async fn default(rt: RT) -> anyhow::Result<Self> {
        Self::default_with_config(DEFAULT_CONFIG.clone(), DEFAULT_MAX_ISOLATE_WORKERS, rt).await
    }

    pub async fn default_with_config(
        config: UdfTestConfig,
        max_isolate_workers: usize,
        rt: RT,
    ) -> anyhow::Result<Self> {
        let result = Self::new(
            TEST_SOURCE_ISOLATE_ONLY.clone(),
            rt,
            Arc::new(TestPersistence::new()),
            config,
            max_isolate_workers,
        )
        .await?
        .expect("Unexpected JSError");
        Ok(result)
    }

    pub async fn default_with_modules(
        modules: Vec<ModuleConfig>,
        rt: RT,
    ) -> anyhow::Result<Result<Self, JsError>> {
        Self::new(
            modules,
            rt,
            Arc::new(TestPersistence::new()),
            DEFAULT_CONFIG.clone(),
            DEFAULT_MAX_ISOLATE_WORKERS,
        )
        .await
    }

    pub async fn with_timeout(rt: RT, timeout: Option<Duration>) -> anyhow::Result<Self> {
        let result = Self::new(
            TEST_SOURCE_ISOLATE_ONLY.clone(),
            rt,
            Arc::new(TestPersistence::new()),
            UdfTestConfig {
                isolate_config: IsolateConfig::new_with_max_user_timeout(
                    "test",
                    timeout,
                    ConcurrencyLimiter::unlimited(),
                ),
                udf_server_version: "1000.0.0".parse().unwrap(),
            },
            DEFAULT_MAX_ISOLATE_WORKERS,
        )
        .await?
        .expect("Unexpected JSError");
        Ok(result)
    }
}

#[async_trait]
impl<RT: Runtime, P: Persistence + Clone> UdfCallback<RT> for UdfTest<RT, P> {
    async fn execute_udf(
        &self,
        _client_id: String,
        _identity: Identity,
        _udf_type: UdfType,
        _path_and_args: ValidatedPathAndArgs,
        _environment_data: EnvironmentData<RT>,
        _transaction: Transaction<RT>,
        _journal: QueryJournal,
        _context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        anyhow::bail!("Component calls not implemented in tests yet")
    }
}

#[async_trait]
impl<RT: Runtime, P: Persistence + Clone> ActionCallbacks for UdfTest<RT, P> {
    async fn execute_query(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        _context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let arguments = parse_udf_args(&path, args)?;
        let str_name = String::from(path.udf_path);
        let outcome = self
            .raw_query(&str_name, arguments.into(), identity, None)
            .await?;

        let r = match outcome.result {
            Ok(packed_value) => Ok(packed_value.unpack()),
            Err(e) => Err(e),
        };
        Ok(FunctionResult { result: r })
    }

    async fn execute_mutation(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        _context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let arguments = parse_udf_args(&path, args)?;
        let str_name = String::from(path.udf_path);
        let outcome = self
            .raw_mutation(&str_name, arguments.into(), identity)
            .await?;

        let r = match outcome.result {
            Ok(packed_value) => Ok(packed_value.unpack()),
            Err(e) => Err(e),
        };
        Ok(FunctionResult { result: r })
    }

    async fn execute_action(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        _context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let arguments = parse_udf_args(&path, args)?;
        let str_name = String::from(path.udf_path);
        let (outcome, _) = self
            .raw_action(&str_name, arguments.into(), identity)
            .await?;

        let r = match outcome.result {
            Ok(packed_value) => Ok(packed_value.unpack()),
            Err(e) => Err(e),
        };
        Ok(FunctionResult { result: r })
    }

    async fn storage_get_url(
        &self,
        identity: Identity,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>> {
        let mut tx = self.database.begin(identity).await?;
        self.file_storage.get_url(&mut tx, storage_id).await
    }

    async fn storage_get_file_entry(
        &self,
        identity: Identity,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>> {
        let mut tx = self.database.begin(identity).await?;
        self.file_storage.get_file_entry(&mut tx, storage_id).await
    }

    async fn storage_store_file_entry(
        &self,
        identity: Identity,
        entry: FileStorageEntry,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let mut tx = self.database.begin(identity).await?;
        let id = self.file_storage.store_file_entry(&mut tx, entry).await?;
        self.database.commit(tx).await?;
        Ok(id)
    }

    async fn storage_delete(
        &self,
        identity: Identity,
        storage_id: FileStorageId,
    ) -> anyhow::Result<()> {
        let mut tx = self.database.begin(identity).await?;
        self.file_storage
            .delete(&mut tx, storage_id.clone())
            .await?;
        self.database.commit(tx).await?;
        Ok(())
    }

    async fn schedule_job(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        udf_args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
        context: ExecutionContext,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let mut tx: database::Transaction<RT> = self.database.begin(identity).await?;
        let (path, udf_args) = validate_schedule_args(
            path,
            udf_args,
            scheduled_ts,
            // Scheduling from actions is not transaction and happens at latest
            // timestamp.
            self.database.runtime().unix_timestamp(),
            &mut tx,
        )
        .await?;

        let virtual_id = VirtualSchedulerModel::new(&mut tx)
            .schedule(path, udf_args, scheduled_ts, context)
            .await?;
        self.database.commit(tx).await?;

        Ok(virtual_id)
    }

    async fn cancel_job(
        &self,
        identity: Identity,
        virtual_id: DeveloperDocumentId,
    ) -> anyhow::Result<()> {
        let mut tx = self.database.begin(identity).await?;
        VirtualSchedulerModel::new(&mut tx)
            .cancel(virtual_id)
            .await?;
        self.database.commit(tx).await?;
        Ok(())
    }

    async fn vector_search(
        &self,
        identity: Identity,
        query: JsonValue,
    ) -> anyhow::Result<(Vec<PublicVectorSearchQueryResult>, FunctionUsageStats)> {
        let query = VectorSearch::try_from(query)?;
        self.database.vector_search(identity, query).await
    }
}

/// Create a bogus UDF request for testing. Should only be used for tests
/// that don't depend on UDF execution succeeding, like scheduler tests.
pub async fn bogus_udf_request<RT: Runtime>(
    db: &Database<RT>,
    client_id: &str,
    pause_client: Option<PauseClient>,
    sender: oneshot::Sender<anyhow::Result<(Transaction<RT>, FunctionOutcome)>>,
) -> anyhow::Result<Request<RT>> {
    let tx = db.begin_system().await?;
    // let (sender, _rx) = oneshot::channel();
    let request = UdfRequest {
        path_and_args: ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "path.js:default".parse()?,
            },
            ConvexArray::empty(),
            None,
        ),
        udf_type: UdfType::Query,
        identity: Identity::system().into(),
        transaction: tx,
        journal: QueryJournal::new(),
        context: ExecutionContext::new_for_test(),
    };
    let inner = RequestType::Udf {
        request,
        environment_data: test_environment_data(db.runtime().clone())?,
        response: sender,
        queue_timer: queue_timer(),
        udf_callback: Box::new(BogusUdfCallback),
    };
    Ok(Request {
        client_id: client_id.to_string(),
        inner,
        pause_client: pause_client.unwrap_or_default(),
        parent_trace: EncodedSpan::empty(),
    })
}

struct BogusUdfCallback;

#[async_trait]
impl<RT: Runtime> UdfCallback<RT> for BogusUdfCallback {
    async fn execute_udf(
        &self,
        _client_id: String,
        _identity: Identity,
        _udf_type: UdfType,
        _path_and_args: ValidatedPathAndArgs,
        _environment_data: EnvironmentData<RT>,
        _transaction: Transaction<RT>,
        _journal: QueryJournal,
        _context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        anyhow::bail!("BogusUdfCallback called")
    }
}

pub async fn test_isolate_recreated_with_client_change<RT: Runtime, W: IsolateWorker<RT>>(
    rt: RT,
    worker: W,
    mut pause: PauseController,
) -> anyhow::Result<()> {
    initialize_v8();
    let mut wait_for_blocked = pause.wait_for_blocked(PAUSE_RECREATE_CLIENT).boxed();
    let heap_stats = SharedIsolateHeapStats::new();
    let (mut work_sender, work_receiver) = mpsc::channel(1);
    let _handle = rt
        .spawn_thread(move || worker.service_requests::<Option<usize>>(work_receiver, heap_stats));
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let (done_sender, done_receiver) = oneshot::channel();
    let (sender, _rx) = oneshot::channel();
    let request = bogus_udf_request(&db, "carnitas", None, sender).await?;
    work_sender.try_send((request, done_sender, None)).unwrap();
    let mut done_receiver = done_receiver.boxed();
    // First request should not recreate isolate.
    select! {
        _ = done_receiver.as_mut().fuse() => {
            Ok(())
        },
        _ = wait_for_blocked.as_mut().fuse() => {
            Err(anyhow::anyhow!("recreated isolate on the first request"))
        }
    }?;
    // Second request with different client_id should recreate isolate.
    let (done_sender, done_receiver) = oneshot::channel();
    let (sender, _rx) = oneshot::channel();
    let request = bogus_udf_request(&db, "alpastor", None, sender).await?;
    work_sender.try_send((request, done_sender, None)).unwrap();
    let mut done_receiver = done_receiver.boxed();
    loop {
        select! {
                _ = done_receiver.as_mut().fuse() => {
                    anyhow::bail!("Should have recreated isolate on the second
            request");
        },
                pause_guard = wait_for_blocked.as_mut().fuse() => {
                    if let Some(mut pause_guard) = pause_guard {
                        drop(done_receiver);
                        pause_guard.unpause();
                        drop(wait_for_blocked);
                        break;
                    }
                }
            }
    }
    Ok(())
}

pub async fn test_isolate_not_recreated_with_same_client<RT: Runtime, W: IsolateWorker<RT>>(
    rt: RT,
    worker: W,
    mut pause: PauseController,
) -> anyhow::Result<()> {
    initialize_v8();
    let mut wait_for_blocked = pause.wait_for_blocked(PAUSE_RECREATE_CLIENT).boxed();
    let heap_stats = SharedIsolateHeapStats::new();
    let (mut work_sender, work_receiver) = mpsc::channel(1);
    let _handle = rt
        .spawn_thread(move || worker.service_requests::<Option<usize>>(work_receiver, heap_stats));
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let (done_sender, done_receiver) = oneshot::channel();
    let (sender, _rx) = oneshot::channel();
    let request = bogus_udf_request(&db, "carnitas", None, sender).await?;
    work_sender.try_send((request, done_sender, None)).unwrap();
    let mut done_receiver = done_receiver.boxed();
    // First request should not recreate isolate.
    select! {
        _ = done_receiver.as_mut().fuse() => {
            Ok(())
        },
        _ = wait_for_blocked.as_mut().fuse() => {
            Err(anyhow::anyhow!("recreated isolate on the first request"))
        }
    }?;
    // Second request with the same client_id should not recreate isolate.
    let (done_sender, done_receiver) = oneshot::channel();
    let (sender, _rx) = oneshot::channel();
    let request = bogus_udf_request(&db, "carnitas", None, sender).await?;
    work_sender.try_send((request, done_sender, None)).unwrap();
    let mut done_receiver = done_receiver.boxed();
    select! {
        _ = done_receiver.as_mut().fuse() => {
            Ok(())
        },
        _ = wait_for_blocked.as_mut().fuse() => {
            Err(anyhow::anyhow!("recreated isolate on the second request with the same client id"))
        }
    }?;
    Ok(())
}
