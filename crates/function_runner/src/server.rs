use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fmt::Debug,
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    components::{
        ComponentDefinitionPath,
        ComponentName,
        Resource,
    },
    errors::JsError,
    execution_context::ExecutionContext,
    http::{
        fetch::FetchClient,
        RoutedHttpPath,
    },
    log_lines::LogLine,
    persistence::{
        NoopRetentionValidator,
        PersistenceReader,
        RetentionValidator,
    },
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    types::{
        ConvexOrigin,
        IndexId,
        ModuleEnvironment,
        RepeatableTimestamp,
        UdfType,
    },
};
use database::{
    BootstrapMetadata,
    FollowerRetentionManager,
    TableCountSnapshot,
    Transaction,
    TransactionTextSnapshot,
};
use file_storage::TransactionalFileStorage;
use futures::FutureExt;
use isolate::{
    client::EnvironmentData,
    ActionCallbacks,
    AuthConfig,
    EvaluateAppDefinitionsResult,
    FunctionOutcome,
    HttpActionRequest as HttpActionRequestInner,
    HttpActionResponseStreamer,
    IsolateClient,
    UdfCallback,
    ValidatedHttpPath,
    ValidatedPathAndArgs,
};
use keybroker::{
    Identity,
    InstanceSecret,
    KeyBroker,
};
use model::{
    config::types::ModuleConfig,
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    modules::module_versions::{
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    udf_config::types::UdfConfig,
};
use storage::{
    Storage,
    StorageUseCase,
};
use sync_types::{
    CanonicalizedModulePath,
    Timestamp,
};
use tokio::sync::mpsc;
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
};
use value::identifier::Identifier;

use super::in_memory_indexes::InMemoryIndexCache;
use crate::{
    module_cache::{
        FunctionRunnerModuleLoader,
        ModuleCache,
    },
    FunctionFinalTransaction,
    FunctionWrites,
};

const MAX_ISOLATE_WORKERS: usize = 128;

pub struct RunRequestArgs {
    pub instance_name: String,
    pub instance_secret: InstanceSecret,
    pub reader: Arc<dyn PersistenceReader>,
    pub convex_origin: ConvexOrigin,
    pub bootstrap_metadata: BootstrapMetadata,
    pub table_count_snapshot: Arc<dyn TableCountSnapshot>,
    pub text_index_snapshot: Arc<dyn TransactionTextSnapshot>,
    pub action_callbacks: Arc<dyn ActionCallbacks>,
    pub fetch_client: Arc<dyn FetchClient>,
    pub log_line_sender: Option<mpsc::UnboundedSender<LogLine>>,
    pub udf_type: UdfType,
    pub identity: Identity,
    pub ts: RepeatableTimestamp,
    pub existing_writes: FunctionWrites,
    pub system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    pub in_memory_index_last_modified: BTreeMap<IndexId, Timestamp>,
    pub context: ExecutionContext,
}

#[derive(Clone)]
pub struct FunctionMetadata {
    pub path_and_args: ValidatedPathAndArgs,
    pub journal: QueryJournal,
}

pub struct HttpActionMetadata {
    pub http_response_streamer: HttpActionResponseStreamer,
    pub http_module_path: ValidatedHttpPath,
    pub routed_path: RoutedHttpPath,
    pub http_request: HttpActionRequestInner,
}

#[async_trait]
pub trait StorageForInstance<RT: Runtime>: Debug + Clone + Send + Sync + 'static {
    /// Gets a storage impl for a instance. Agnostic to what kind of storage -
    /// local or s3, or how it was loaded (e.g. passed directly within backend,
    /// loaded from a transaction created in Funrun)
    async fn storage_for_instance(
        &self,
        transaction: &mut Transaction<RT>,
        use_case: StorageUseCase,
    ) -> anyhow::Result<Arc<dyn Storage>>;
}

#[derive(Clone, Debug)]
pub struct InstanceStorage {
    pub files_storage: Arc<dyn Storage>,
    pub modules_storage: Arc<dyn Storage>,
}

#[async_trait]
impl<RT: Runtime> StorageForInstance<RT> for InstanceStorage {
    async fn storage_for_instance(
        &self,
        _transaction: &mut Transaction<RT>,
        use_case: StorageUseCase,
    ) -> anyhow::Result<Arc<dyn Storage>> {
        match use_case {
            StorageUseCase::Files => Ok(self.files_storage.clone()),
            StorageUseCase::Modules => Ok(self.modules_storage.clone()),
            _ => anyhow::bail!("function runner storage does not support {use_case}"),
        }
    }
}

pub struct FunctionRunnerCore<RT: Runtime, S: StorageForInstance<RT>> {
    rt: RT,
    storage: S,
    index_cache: InMemoryIndexCache<RT>,
    module_cache: ModuleCache<RT>,
    isolate_client: IsolateClient<RT>,
}

impl<RT: Runtime, S: StorageForInstance<RT>> Clone for FunctionRunnerCore<RT, S> {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt.clone(),
            storage: self.storage.clone(),
            index_cache: self.index_cache.clone(),
            module_cache: self.module_cache.clone(),
            isolate_client: self.isolate_client.clone(),
        }
    }
}

#[minitrace::trace]
pub async fn validate_run_function_result(
    udf_type: UdfType,
    ts: Timestamp,
    retention_validator: Arc<dyn RetentionValidator>,
) -> anyhow::Result<()> {
    match udf_type {
        // Since queries and mutations have no side effects, we perform the
        // retention check here, when validating the result.
        UdfType::Query | UdfType::Mutation => retention_validator
            .validate_snapshot(ts)
            .await
            .context("Function runner retention check changed"),
        // Since Actions can have side effects, we have to validate their
        // retention while we run them. We can't perform an additional check
        // here since actions can run longer than the retention.
        UdfType::Action | UdfType::HttpAction => Ok(()),
    }
}

impl<RT: Runtime, S: StorageForInstance<RT>> FunctionRunnerCore<RT, S> {
    pub async fn new(rt: RT, storage: S, max_percent_per_client: usize) -> anyhow::Result<Self> {
        Self::_new(rt, storage, max_percent_per_client, MAX_ISOLATE_WORKERS).await
    }

    async fn _new(
        rt: RT,
        storage: S,
        max_percent_per_client: usize,
        max_isolate_workers: usize,
    ) -> anyhow::Result<Self> {
        let isolate_client = IsolateClient::new(
            rt.clone(),
            max_percent_per_client,
            max_isolate_workers,
            None,
        )?;
        let index_cache = InMemoryIndexCache::new(rt.clone());
        let module_cache = ModuleCache::new(rt.clone());

        Ok(Self {
            rt,
            storage,
            index_cache,
            module_cache,
            isolate_client,
        })
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.isolate_client.shutdown().await
    }

    pub async fn begin_tx(
        &self,
        identity: Identity,
        ts: RepeatableTimestamp,
        existing_writes: FunctionWrites,
        reader: Arc<dyn PersistenceReader>,
        instance_name: String,
        in_memory_index_versions: BTreeMap<IndexId, Timestamp>,
        bootstrap_metadata: BootstrapMetadata,
        table_count_snapshot: Arc<dyn TableCountSnapshot>,
        text_index_snapshot: Arc<dyn TransactionTextSnapshot>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<Transaction<RT>> {
        let usage_tracker = FunctionUsageTracker::new();
        let transaction = self
            .index_cache
            .begin_tx(
                identity.clone(),
                ts,
                existing_writes,
                reader,
                instance_name.clone(),
                in_memory_index_versions,
                bootstrap_metadata,
                table_count_snapshot,
                text_index_snapshot,
                usage_tracker.clone(),
                retention_validator,
            )
            .await?;
        Ok(transaction)
    }

    // Runs a function given the information for the backend as well as arguments
    // to the function itself.
    // NOTE: The caller of this is responsible of checking retention by calling
    // `validate_function_runner_result`. If the retention check fails, we should
    // ignore any results or errors returned by this method.
    #[minitrace::trace]
    pub async fn run_function_no_retention_check(
        &self,
        run_request_args: RunRequestArgs,
        function_metadata: Option<FunctionMetadata>,
        http_action_metadata: Option<HttpActionMetadata>,
    ) -> anyhow::Result<(
        Option<FunctionFinalTransaction>,
        FunctionOutcome,
        FunctionUsageStats,
    )> {
        self.run_function_no_retention_check_inner(
            run_request_args,
            function_metadata,
            http_action_metadata,
        )
        .boxed()
        .await
    }

    #[minitrace::trace]
    pub async fn run_function_no_retention_check_inner(
        &self,
        RunRequestArgs {
            instance_name,
            instance_secret,
            reader,
            convex_origin,
            bootstrap_metadata,
            table_count_snapshot,
            text_index_snapshot,
            action_callbacks,
            fetch_client,
            log_line_sender,
            udf_type,
            identity,
            ts,
            existing_writes,
            system_env_vars,
            in_memory_index_last_modified,
            context,
        }: RunRequestArgs,
        function_metadata: Option<FunctionMetadata>,
        http_action_metadata: Option<HttpActionMetadata>,
    ) -> anyhow::Result<(
        Option<FunctionFinalTransaction>,
        FunctionOutcome,
        FunctionUsageStats,
    )> {
        let usage_tracker = FunctionUsageTracker::new();
        let retention_validator: Arc<dyn RetentionValidator> = match udf_type {
            // Since queries and mutations are ready only, we can check the retention
            // in at end in `validate_function_runner_result`.
            UdfType::Query | UdfType::Mutation => Arc::new(NoopRetentionValidator {}),
            // For actions, we have to do it inline since they have side effects.
            UdfType::Action | UdfType::HttpAction => Arc::new(
                FollowerRetentionManager::new_with_repeatable_ts(
                    self.rt.clone(),
                    reader.clone(),
                    ts,
                )
                .await?,
            ),
        };
        let mut transaction = self
            .index_cache
            .begin_tx(
                identity.clone(),
                ts,
                existing_writes,
                reader,
                instance_name.clone(),
                in_memory_index_last_modified,
                bootstrap_metadata,
                table_count_snapshot,
                text_index_snapshot,
                usage_tracker.clone(),
                retention_validator,
            )
            .await?;
        let storage = self
            .storage
            .storage_for_instance(&mut transaction, StorageUseCase::Files)
            .await?;
        let file_storage = TransactionalFileStorage::new(self.rt.clone(), storage, convex_origin);
        let modules_storage = self
            .storage
            .storage_for_instance(&mut transaction, StorageUseCase::Modules)
            .await?;

        let key_broker = KeyBroker::new(&instance_name, instance_secret)?;
        let environment_data = EnvironmentData {
            key_broker,
            system_env_vars,
            file_storage,
            module_loader: Arc::new(FunctionRunnerModuleLoader {
                instance_name: instance_name.clone(),
                cache: self.module_cache.clone(),
                modules_storage,
            }),
        };

        match udf_type {
            UdfType::Query | UdfType::Mutation => {
                let FunctionMetadata {
                    path_and_args,
                    journal,
                } = function_metadata.context("Missing function metadata for query or mutation")?;
                let (tx, outcome) = self
                    .isolate_client
                    .execute_udf(
                        udf_type,
                        path_and_args,
                        transaction,
                        journal,
                        context,
                        environment_data,
                        0,
                        instance_name,
                    )
                    .await?;
                Ok((
                    Some(tx.try_into()?),
                    outcome,
                    usage_tracker.gather_user_stats(),
                ))
            },
            UdfType::Action => {
                let FunctionMetadata { path_and_args, .. } =
                    function_metadata.context("Missing function metadata for action")?;
                let log_line_sender =
                    log_line_sender.context("Missing log line sender for action")?;
                let outcome = self
                    .isolate_client
                    .execute_action(
                        path_and_args,
                        transaction,
                        action_callbacks,
                        fetch_client,
                        log_line_sender,
                        context,
                        environment_data,
                        instance_name,
                    )
                    .await?;
                Ok((
                    None,
                    FunctionOutcome::Action(outcome),
                    usage_tracker.gather_user_stats(),
                ))
            },
            UdfType::HttpAction => {
                let HttpActionMetadata {
                    http_response_streamer,
                    http_module_path,
                    routed_path,
                    http_request,
                } = http_action_metadata.context("Missing http action metadata")?;
                let log_line_sender =
                    log_line_sender.context("Missing log line sender for http action")?;
                let outcome = self
                    .isolate_client
                    .execute_http_action(
                        http_module_path,
                        routed_path,
                        http_request,
                        identity,
                        action_callbacks,
                        fetch_client,
                        log_line_sender,
                        http_response_streamer,
                        transaction,
                        context,
                        environment_data,
                        instance_name,
                    )
                    .await?;
                Ok((
                    None,
                    FunctionOutcome::HttpAction(outcome),
                    usage_tracker.gather_user_stats(),
                ))
            },
        }
    }

    pub async fn analyze(
        &self,
        udf_config: UdfConfig,
        modules: BTreeMap<CanonicalizedModulePath, ModuleConfig>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        instance_name: String,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        anyhow::ensure!(
            modules
                .values()
                .all(|m| m.environment == ModuleEnvironment::Isolate),
            "Can only analyze Isolate modules"
        );

        self.isolate_client
            .analyze(udf_config, modules, environment_variables, instance_name)
            .await
    }

    #[minitrace::trace]
    pub async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        instance_name: String,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        anyhow::ensure!(
            app_definition.environment == ModuleEnvironment::Isolate,
            "Can only evaluate Isolate modules"
        );
        anyhow::ensure!(
            component_definitions
                .values()
                .all(|m| m.environment == ModuleEnvironment::Isolate),
            "Can only evaluate Isolate modules"
        );

        self.isolate_client
            .evaluate_app_definitions(
                app_definition,
                component_definitions,
                dependency_graph,
                environment_variables,
                system_env_vars,
                instance_name,
            )
            .await
    }

    #[minitrace::trace]
    pub async fn evaluate_component_initializer(
        &self,
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        path: ComponentDefinitionPath,
        definition: ModuleConfig,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
        instance_name: String,
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>> {
        self.isolate_client
            .evaluate_component_initializer(
                evaluated_definitions,
                path,
                definition,
                args,
                name,
                instance_name,
            )
            .await
    }

    #[minitrace::trace]
    pub async fn evaluate_schema(
        &self,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
        instance_name: String,
    ) -> anyhow::Result<DatabaseSchema> {
        self.isolate_client
            .evaluate_schema(
                schema_bundle,
                source_map,
                rng_seed,
                unix_timestamp,
                instance_name,
            )
            .await
    }

    #[minitrace::trace]
    pub async fn evaluate_auth_config(
        &self,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        explanation: &str,
        instance_name: String,
    ) -> anyhow::Result<AuthConfig> {
        self.isolate_client
            .evaluate_auth_config(
                auth_config_bundle,
                source_map,
                environment_variables,
                explanation,
                instance_name,
            )
            .await
    }
}

#[async_trait]
impl<RT: Runtime, S: StorageForInstance<RT>> UdfCallback<RT> for FunctionRunnerCore<RT, S> {
    async fn execute_udf(
        &self,
        client_id: String,
        udf_type: UdfType,
        path_and_args: ValidatedPathAndArgs,
        environment_data: EnvironmentData<RT>,
        transaction: Transaction<RT>,
        journal: QueryJournal,
        context: ExecutionContext,
        reactor_depth: usize,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        self.isolate_client
            .execute_udf(
                udf_type,
                path_and_args,
                transaction,
                journal,
                context,
                environment_data,
                reactor_depth,
                client_id,
            )
            .await
    }
}
