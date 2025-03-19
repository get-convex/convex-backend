use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::{
        Arc,
        Weak,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    auth::AuthConfig,
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    components::{
        ComponentDefinitionPath,
        ComponentName,
        Resource,
    },
    errors::JsError,
    execution_context::ExecutionContext,
    http::fetch::FetchClient,
    log_lines::LogLine,
    persistence::PersistenceReader,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    types::{
        ConvexOrigin,
        IndexId,
        RepeatableTimestamp,
        UdfType,
    },
};
use database::{
    shutdown_error,
    Database,
    TextIndexManagerSnapshot,
};
use errors::ErrorMetadata;
use futures::{
    select_biased,
    FutureExt,
    StreamExt,
};
use isolate::ActionCallbacks;
use keybroker::{
    Identity,
    InstanceSecret,
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
use parking_lot::RwLock;
use sync_types::{
    CanonicalizedModulePath,
    Timestamp,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use udf::{
    EvaluateAppDefinitionsResult,
    FunctionOutcome,
    HttpActionResponseStreamer,
};
use usage_tracking::FunctionUsageStats;
use value::identifier::Identifier;

use super::FunctionRunner;
use crate::{
    server::{
        validate_run_function_result,
        FunctionMetadata,
        FunctionRunnerCore,
        HttpActionMetadata,
        InstanceStorage,
        RunRequestArgs,
    },
    FunctionFinalTransaction,
    FunctionWrites,
};

pub struct InProcessFunctionRunner<RT: Runtime> {
    server: FunctionRunnerCore<RT, InstanceStorage>,
    persistence_reader: Arc<dyn PersistenceReader>,

    // Static information about the backend.
    instance_name: String,
    instance_secret: InstanceSecret,
    convex_origin: ConvexOrigin,
    database: Database<RT>,
    // Use Weak reference to avoid reference cycle between InProcessFunctionRunner
    // and ApplicationFunctionRunner.
    action_callbacks: Arc<RwLock<Option<Weak<dyn ActionCallbacks>>>>,
    fetch_client: Arc<dyn FetchClient>,
}

impl<RT: Runtime> InProcessFunctionRunner<RT> {
    pub async fn new(
        instance_name: String,
        instance_secret: InstanceSecret,
        convex_origin: ConvexOrigin,
        rt: RT,
        persistence_reader: Arc<dyn PersistenceReader>,
        storage: InstanceStorage,
        database: Database<RT>,
        fetch_client: Arc<dyn FetchClient>,
    ) -> anyhow::Result<Self> {
        // InProcessFunrun is single tenant and thus can use the full capacity.
        let max_percent_per_client = 100;
        let server = FunctionRunnerCore::new(rt, storage, max_percent_per_client).await?;
        Ok(Self {
            server,
            persistence_reader,
            instance_name,
            instance_secret,
            convex_origin,
            database,
            action_callbacks: Arc::new(RwLock::new(None)),
            fetch_client,
        })
    }

    async fn run_http_action(
        &self,
        request_metadata: RunRequestArgs,
        mut http_action_metadata: HttpActionMetadata,
    ) -> anyhow::Result<(
        Option<FunctionFinalTransaction>,
        FunctionOutcome,
        FunctionUsageStats,
    )> {
        // Mimic `FunrunClient::process_message_stream` behavior of forwarding
        // the response_streamer, and detecting cancellation.
        let (inner_response_sender, inner_response_receiver) = mpsc::unbounded_channel();
        let inner_response_streamer = HttpActionResponseStreamer::new(inner_response_sender);
        let mut outer_response_streamer = std::mem::replace(
            &mut http_action_metadata.http_response_streamer,
            inner_response_streamer,
        );
        let mut inner_response_stream =
            UnboundedReceiverStream::new(inner_response_receiver).fuse();
        let mut run_function_fut = Box::pin(self.server.run_function_no_retention_check(
            request_metadata,
            None,
            Some(http_action_metadata),
        ))
        .fuse();
        loop {
            select_biased! {
                result = &mut run_function_fut => {
                    // Flush inner_response_stream into outer_response_streamer.
                    while let Some(part) = inner_response_stream.next().await {
                        if outer_response_streamer.send_part(part)?.is_err() {
                            anyhow::bail!(ErrorMetadata::client_disconnect());
                        }
                    }
                    return result;
                },
                _ = outer_response_streamer.sender.closed().fuse() => {
                    // The streamer above us has disconnected, so stop running
                    // the function and throw an error.
                    drop(run_function_fut);
                    anyhow::bail!(ErrorMetadata::client_disconnect());
                },
                // select_next_some waits until there's a new part to send.
                // If inner_response_stream is closed, this branch doesn't run
                // and we continue waiting on the other branches.
                // This behavior (of continuing to allow the function to be
                // cancelled even after its inner_response_stream is closed)
                // isn't very important, since the function has finished running
                // user code. But it's defensive against the isolate changing
                // its behavior in the future, and it matches FunrunClient
                // behavior.
                part = inner_response_stream.select_next_some() => {
                    // Forward a response part.
                    // If outer_response_streamer is disconnected,
                    // continue and the next loop iteration will detect
                    // it is closed.
                    let _ = outer_response_streamer.send_part(part)?;
                },
            }
        }
    }
}

#[async_trait]
impl<RT: Runtime> FunctionRunner<RT> for InProcessFunctionRunner<RT> {
    #[fastrace::trace]
    async fn run_function(
        &self,
        udf_type: UdfType,
        identity: Identity,
        ts: RepeatableTimestamp,
        existing_writes: FunctionWrites,
        log_line_sender: Option<mpsc::UnboundedSender<LogLine>>,
        function_metadata: Option<FunctionMetadata>,
        http_action_metadata: Option<HttpActionMetadata>,
        default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        in_memory_index_last_modified: BTreeMap<IndexId, Timestamp>,
        context: ExecutionContext,
    ) -> anyhow::Result<(
        Option<FunctionFinalTransaction>,
        FunctionOutcome,
        FunctionUsageStats,
    )> {
        let pause_client = self.database.runtime().pause_client();
        pause_client.wait("run_function").await;

        let snapshot = self.database.snapshot(ts)?;
        let table_count_snapshot = Arc::new(snapshot.table_summaries);
        let text_index_snapshot = Arc::new(TextIndexManagerSnapshot::new(
            snapshot.index_registry,
            snapshot.text_indexes,
            self.database.searcher.clone(),
            self.database.search_storage.clone(),
        ));
        let action_callbacks = self
            .action_callbacks
            .read()
            .clone()
            .context("Action callbacks not set")?
            .upgrade()
            .context(shutdown_error())?;

        let request_metadata = RunRequestArgs {
            instance_name: self.instance_name.clone(),
            instance_secret: self.instance_secret,
            reader: self.persistence_reader.clone(),
            convex_origin: self.convex_origin.clone(),
            bootstrap_metadata: self.database.bootstrap_metadata.clone(),
            table_count_snapshot,
            text_index_snapshot,
            action_callbacks,
            fetch_client: self.fetch_client.clone(),
            log_line_sender,
            udf_type,
            identity,
            ts,
            existing_writes,
            default_system_env_vars,
            in_memory_index_last_modified,
            context,
        };

        // NOTE: We run the function without checking retention until after the
        // function execution. It is important that we do not surface any errors
        // or results until after we call `validate_run_function_result` below.
        let result = match udf_type {
            UdfType::Query | UdfType::Mutation | UdfType::Action => {
                self.server
                    .run_function_no_retention_check(request_metadata, function_metadata, None)
                    .await
            },
            UdfType::HttpAction => {
                self.run_http_action(
                    request_metadata,
                    http_action_metadata.context("Http action metadata not set")?,
                )
                .await
            },
        };
        validate_run_function_result(udf_type, *ts, self.database.retention_validator()).await?;
        result
    }

    #[fastrace::trace]
    async fn analyze(
        &self,
        udf_config: UdfConfig,
        modules: BTreeMap<CanonicalizedModulePath, ModuleConfig>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        self.server
            .analyze(
                udf_config,
                modules,
                environment_variables,
                self.instance_name.clone(),
            )
            .await
    }

    #[fastrace::trace]
    async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        self.server
            .evaluate_app_definitions(
                app_definition,
                component_definitions,
                dependency_graph,
                user_environment_variables,
                system_env_vars,
                self.instance_name.clone(),
            )
            .await
    }

    #[fastrace::trace]
    async fn evaluate_component_initializer(
        &self,
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        path: ComponentDefinitionPath,
        definition: ModuleConfig,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>> {
        self.server
            .evaluate_component_initializer(
                evaluated_definitions,
                path,
                definition,
                args,
                name,
                self.instance_name.clone(),
            )
            .await
    }

    #[fastrace::trace]
    async fn evaluate_schema(
        &self,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<DatabaseSchema> {
        self.server
            .evaluate_schema(
                schema_bundle,
                source_map,
                rng_seed,
                unix_timestamp,
                self.instance_name.clone(),
            )
            .await
    }

    #[fastrace::trace]
    async fn evaluate_auth_config(
        &self,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        explanation: &str,
    ) -> anyhow::Result<AuthConfig> {
        self.server
            .evaluate_auth_config(
                auth_config_bundle,
                source_map,
                environment_variables,
                explanation,
                self.instance_name.clone(),
            )
            .await
    }

    /// This fn should be called on startup. All `run_function` calls will fail
    /// if actions callbacks are not set.
    fn set_action_callbacks(&self, action_callbacks: Arc<dyn ActionCallbacks>) {
        *self.action_callbacks.write() = Some(Arc::downgrade(&action_callbacks));
    }
}
