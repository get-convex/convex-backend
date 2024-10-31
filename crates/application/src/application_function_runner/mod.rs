use core::sync::atomic::Ordering;
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::{
        atomic::AtomicUsize,
        Arc,
        LazyLock,
    },
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use authentication::token_to_authorization_header;
use common::{
    backoff::Backoff,
    bootstrap_model::components::{
        definition::ComponentDefinitionMetadata,
        handles::FunctionHandle,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        CanonicalizedComponentModulePath,
        ComponentDefinitionPath,
        ComponentId,
        ComponentName,
        ComponentPath,
        PublicFunctionPath,
        Resource,
    },
    errors::JsError,
    execution_context::ExecutionContext,
    http::fetch::FetchClient,
    knobs::{
        APPLICATION_FUNCTION_RUNNER_SEMAPHORE_TIMEOUT,
        APPLICATION_MAX_CONCURRENT_HTTP_ACTIONS,
        APPLICATION_MAX_CONCURRENT_MUTATIONS,
        APPLICATION_MAX_CONCURRENT_NODE_ACTIONS,
        APPLICATION_MAX_CONCURRENT_QUERIES,
        APPLICATION_MAX_CONCURRENT_V8_ACTIONS,
        BACKEND_ISOLATE_ACTIVE_THREADS_PERCENT,
        ISOLATE_MAX_USER_HEAP_SIZE,
        UDF_EXECUTOR_OCC_INITIAL_BACKOFF,
        UDF_EXECUTOR_OCC_MAX_BACKOFF,
        UDF_EXECUTOR_OCC_MAX_RETRIES,
        UDF_ISOLATE_MAX_EXEC_THREADS,
    },
    log_lines::{
        run_function_and_collect_log_lines,
        LogLine,
    },
    minitrace_helpers::EncodedSpan,
    pause::PauseClient,
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    tokio::sync::{
        Semaphore,
        SemaphorePermit,
    },
    types::{
        AllowedVisibility,
        FunctionCaller,
        ModuleEnvironment,
        NodeDependency,
        Timestamp,
        UdfType,
    },
    value::ConvexArray,
    RequestId,
};
use database::{
    unauthorized_error,
    Database,
    Token,
    Transaction,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use file_storage::TransactionalFileStorage;
use function_runner::{
    server::{
        FunctionMetadata,
        HttpActionMetadata,
    },
    FunctionReads,
    FunctionRunner,
    FunctionWrites,
};
use futures::{
    select_biased,
    FutureExt,
};
use isolate::{
    environment::helpers::validation::{
        ValidatedActionOutcome,
        ValidatedUdfOutcome,
    },
    parse_udf_args,
    validate_schedule_args,
    ActionCallbacks,
    ActionOutcome,
    AuthConfig,
    BackendIsolateWorker,
    ConcurrencyLimiter,
    EvaluateAppDefinitionsResult,
    FunctionOutcome,
    FunctionResult,
    HttpActionOutcome,
    IsolateClient,
    IsolateConfig,
    JsonPackedValue,
    UdfOutcome,
    ValidatedPathAndArgs,
};
use keybroker::{
    Identity,
    InstanceSecret,
    KeyBroker,
};
use model::{
    backend_state::BackendStateModel,
    components::handles::FunctionHandlesModel,
    config::{
        module_loader::ModuleLoader,
        types::ModuleConfig,
    },
    environment_variables::{
        types::{
            EnvVarName,
            EnvVarValue,
        },
        EnvironmentVariablesModel,
    },
    external_packages::{
        types::ExternalDepsPackage,
        ExternalPackagesModel,
    },
    file_storage::{
        types::FileStorageEntry,
        FileStorageId,
    },
    modules::{
        module_versions::{
            AnalyzedModule,
            ModuleSource,
            SourceMap,
        },
        ModuleModel,
    },
    scheduled_jobs::VirtualSchedulerModel,
    session_requests::{
        types::{
            SessionRequestIdentifier,
            SessionRequestOutcome,
            SessionRequestRecord,
        },
        SessionRequestModel,
    },
    source_packages::{
        types::SourcePackage,
        SourcePackageModel,
    },
    udf_config::types::UdfConfig,
};
use node_executor::{
    Actions,
    AnalyzeRequest,
    BuildDepsRequest,
    ExecuteRequest,
};
use serde_json::Value as JsonValue;
use storage::Storage;
use sync_types::CanonicalizedModulePath;
use tokio::sync::mpsc;
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
};
use value::{
    id_v6::DeveloperDocumentId,
    identifier::Identifier,
    TableNamespace,
};
use vector::{
    PublicVectorSearchQueryResult,
    VectorSearch,
};

use self::metrics::{
    function_waiter_timer,
    log_occ_retries,
    log_outstanding_functions,
    log_udf_executor_result,
    mutation_timer,
    OutstandingFunctionState,
    UdfExecutorResult,
};
use crate::{
    application_function_runner::metrics::{
        function_run_timer,
        function_total_timer,
        log_function_wait_timeout,
        log_mutation_already_committed,
    },
    cache::CacheManager,
    function_log::{
        ActionCompletion,
        FunctionExecutionLog,
    },
    ActionError,
    ActionReturn,
    MutationError,
    MutationReturn,
    QueryReturn,
};

mod http_routing;
mod metrics;

static BUILD_DEPS_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| Duration::from_secs(1200));

/// Wrapper for [IsolateClient]s and [FunctionRunner]s that determines where to
/// route requests.
#[derive(Clone)]
pub struct FunctionRouter<RT: Runtime> {
    pub(crate) function_runner: Arc<dyn FunctionRunner<RT>>,
    query_limiter: Arc<Limiter>,
    mutation_limiter: Arc<Limiter>,
    action_limiter: Arc<Limiter>,
    http_action_limiter: Arc<Limiter>,

    rt: RT,
    database: Database<RT>,
    system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
}

impl<RT: Runtime> FunctionRouter<RT> {
    pub fn new(
        function_runner: Arc<dyn FunctionRunner<RT>>,
        rt: RT,
        database: Database<RT>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> Self {
        Self {
            function_runner,
            rt,
            database,
            system_env_vars,
            query_limiter: Arc::new(Limiter::new(
                ModuleEnvironment::Isolate,
                UdfType::Query,
                *APPLICATION_MAX_CONCURRENT_QUERIES,
            )),
            mutation_limiter: Arc::new(Limiter::new(
                ModuleEnvironment::Isolate,
                UdfType::Mutation,
                *APPLICATION_MAX_CONCURRENT_MUTATIONS,
            )),
            action_limiter: Arc::new(Limiter::new(
                ModuleEnvironment::Isolate,
                UdfType::Action,
                *APPLICATION_MAX_CONCURRENT_V8_ACTIONS,
            )),
            http_action_limiter: Arc::new(Limiter::new(
                ModuleEnvironment::Isolate,
                UdfType::HttpAction,
                *APPLICATION_MAX_CONCURRENT_HTTP_ACTIONS,
            )),
        }
    }
}

impl<RT: Runtime> FunctionRouter<RT> {
    #[minitrace::trace]
    pub(crate) async fn execute_query_or_mutation(
        &self,
        tx: Transaction<RT>,
        path_and_args: ValidatedPathAndArgs,
        udf_type: UdfType,
        journal: QueryJournal,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        anyhow::ensure!(udf_type == UdfType::Query || udf_type == UdfType::Mutation);
        // All queries and mutations are run in the isolate environment.
        let timer = function_total_timer(ModuleEnvironment::Isolate, udf_type);
        let (tx, outcome) = self
            .function_runner_execute(
                tx,
                udf_type,
                context,
                None,
                Some(FunctionMetadata {
                    journal,
                    path_and_args,
                }),
                None,
            )
            .await?;
        let tx = tx.with_context(|| format!("Missing transaction in response for {udf_type}"))?;
        timer.finish();
        Ok((tx, outcome))
    }

    #[minitrace::trace]
    pub(crate) async fn execute_action(
        &self,
        tx: Transaction<RT>,
        path_and_args: ValidatedPathAndArgs,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionOutcome> {
        let (_, outcome) = self
            .function_runner_execute(
                tx,
                UdfType::Action,
                context,
                Some(log_line_sender),
                Some(FunctionMetadata {
                    journal: QueryJournal::new(),
                    path_and_args,
                }),
                None,
            )
            .await?;

        let FunctionOutcome::Action(outcome) = outcome else {
            anyhow::bail!("Calling an action returned an invalid outcome")
        };
        Ok(outcome)
    }

    #[minitrace::trace]
    pub(crate) async fn execute_http_action(
        &self,
        tx: Transaction<RT>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        http_action_metadata: HttpActionMetadata,
        context: ExecutionContext,
    ) -> anyhow::Result<HttpActionOutcome> {
        let (_, outcome) = self
            .function_runner_execute(
                tx,
                UdfType::HttpAction,
                context,
                Some(log_line_sender),
                None,
                Some(http_action_metadata),
            )
            .await?;

        let FunctionOutcome::HttpAction(outcome) = outcome else {
            anyhow::bail!("Calling an http action returned an invalid outcome")
        };
        Ok(outcome)
    }

    // Execute using the function runner. Can be used for v8 udfs other than http
    // actions.
    #[minitrace::trace]
    async fn function_runner_execute(
        &self,
        mut tx: Transaction<RT>,
        udf_type: UdfType,
        context: ExecutionContext,
        log_line_sender: Option<mpsc::UnboundedSender<LogLine>>,
        function_metadata: Option<FunctionMetadata>,
        http_action_metadata: Option<HttpActionMetadata>,
    ) -> anyhow::Result<(Option<Transaction<RT>>, FunctionOutcome)> {
        let in_memory_index_last_modified = self
            .database
            .snapshot(tx.begin_timestamp())?
            .in_memory_indexes
            .in_memory_indexes_last_modified();

        let limiter = match udf_type {
            UdfType::Query => &self.query_limiter,
            UdfType::Mutation => &self.mutation_limiter,
            UdfType::Action => &self.action_limiter,
            UdfType::HttpAction => &self.http_action_limiter,
        };

        let request_guard = limiter.acquire_permit_with_timeout(&self.rt).await?;

        let timer = function_run_timer(udf_type);
        let (function_tx, outcome, usage_stats) = self
            .function_runner
            .run_function(
                udf_type,
                tx.identity().clone(),
                tx.begin_timestamp(),
                tx.writes().as_flat()?.clone().into(),
                log_line_sender,
                function_metadata,
                http_action_metadata,
                self.system_env_vars.clone(),
                in_memory_index_last_modified,
                context,
            )
            .await?;
        timer.finish();
        drop(request_guard);

        // Add the usage stats to the current transaction tracker.
        tx.usage_tracker.add(usage_stats);

        // Apply the reads and writes to the current transaction
        let tx = if let Some(function_tx) = function_tx {
            let FunctionReads {
                reads,
                num_intervals,
                user_tx_size,
                system_tx_size,
            } = function_tx.reads;
            let FunctionWrites { updates } = function_tx.writes;
            tx.apply_function_runner_tx(
                function_tx.begin_timestamp,
                reads,
                num_intervals,
                user_tx_size,
                system_tx_size,
                updates,
                function_tx.rows_read_by_tablet,
            )?;
            Some(tx)
        } else {
            None
        };

        Ok((tx, outcome))
    }
}

// Used to limit upstream concurrency for a given function type. It also tracks
// and log gauges for the number of waiting and currently running functions.
struct Limiter {
    udf_type: UdfType,
    env: ModuleEnvironment,

    // Used to limit running functions.
    semaphore: Semaphore,
    total_permits: usize,

    // Total function requests, including ones still waiting on the semaphore.
    total_outstanding: AtomicUsize,
}

impl Limiter {
    fn new(env: ModuleEnvironment, udf_type: UdfType, total_permits: usize) -> Self {
        let limiter = Self {
            udf_type,
            env,
            semaphore: Semaphore::new(total_permits),
            total_permits,
            total_outstanding: AtomicUsize::new(0),
        };
        // Update the gauges on startup.
        limiter.update_gauges();
        limiter
    }

    async fn acquire_permit_with_timeout<'a, RT: Runtime>(
        &'a self,
        rt: &'a RT,
    ) -> anyhow::Result<RequestGuard<'a>> {
        let mut request_guard = self.start();
        select_biased! {
            _ = request_guard.acquire_permit().fuse() => {},
            _ = rt.wait(*APPLICATION_FUNCTION_RUNNER_SEMAPHORE_TIMEOUT) => {
                log_function_wait_timeout(self.env, self.udf_type);
                anyhow::bail!(ErrorMetadata::rate_limited(
                    "TooManyConcurrentRequests",
                    format!(
                        "Too many concurrent requests. Your backend is limited to {} concurrent {}s. To get more resources, upgrade to Convex Pro. If you are already on Convex Pro, please contact support.",
                        self.total_permits,
                        self.udf_type.to_lowercase_string(),
                    ),
                ));
            },
        }
        Ok(request_guard)
    }

    fn start(&self) -> RequestGuard {
        self.total_outstanding.fetch_add(1, Ordering::SeqCst);
        // Update the gauge to account for the newly waiting request.
        self.update_gauges();
        RequestGuard {
            limiter: self,
            permit: None,
        }
    }

    // Updates the current waiting and running function gauges.
    fn update_gauges(&self) {
        let running = self.total_permits - self.semaphore.available_permits();
        let waiting = self
            .total_outstanding
            .load(Ordering::SeqCst)
            .saturating_sub(running);
        log_outstanding_functions(
            running,
            self.env,
            self.udf_type,
            OutstandingFunctionState::Running,
        );
        log_outstanding_functions(
            waiting,
            self.env,
            self.udf_type,
            OutstandingFunctionState::Waiting,
        );
    }
}

// Wraps a request to guarantee we correctly update the waiting and running
// gauges even if dropped.
struct RequestGuard<'a> {
    limiter: &'a Limiter,
    permit: Option<SemaphorePermit<'a>>,
}

impl<'a> RequestGuard<'a> {
    async fn acquire_permit(&mut self) -> anyhow::Result<()> {
        let timer = function_waiter_timer(self.limiter.udf_type);
        assert!(
            self.permit.is_none(),
            "Called `acquire_permit` more than once"
        );
        self.permit = Some(self.limiter.semaphore.acquire().await?);
        timer.finish();
        // Update the gauge to account for the newly running function.
        self.limiter.update_gauges();
        Ok(())
    }
}

impl<'a> Drop for RequestGuard<'a> {
    fn drop(&mut self) {
        // Drop the semaphore permit before updating gauges.
        drop(self.permit.take());
        // Remove the request from the running ones.
        self.limiter
            .total_outstanding
            .fetch_sub(1, Ordering::SeqCst);
        // Update the gauges to account fo the newly finished request.
        self.limiter.update_gauges();
    }
}

/// Executes UDFs for backends.
///
/// This struct directly executes http and node actions. Queries, Mutations and
/// v8 Actions are instead routed through the FunctionRouter and its
/// FunctionRunner implementation.
pub struct ApplicationFunctionRunner<RT: Runtime> {
    runtime: RT,
    pub(crate) database: Database<RT>,

    key_broker: KeyBroker,

    isolate_functions: FunctionRouter<RT>,
    // Used for analyze, schema, etc.
    analyze_isolate: IsolateClient<RT>,
    http_actions: IsolateClient<RT>,
    node_actions: Actions,

    pub(crate) module_cache: Arc<dyn ModuleLoader<RT>>,
    modules_storage: Arc<dyn Storage>,
    file_storage: TransactionalFileStorage<RT>,

    function_log: FunctionExecutionLog<RT>,

    cache_manager: CacheManager<RT>,
    system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    node_action_limiter: Limiter,
    fetch_client: Arc<dyn FetchClient>,
}

impl<RT: Runtime> ApplicationFunctionRunner<RT> {
    pub fn new(
        instance_name: String,
        instance_secret: InstanceSecret,
        runtime: RT,
        database: Database<RT>,
        key_broker: KeyBroker,
        function_runner: Arc<dyn FunctionRunner<RT>>,
        node_actions: Actions,
        file_storage: TransactionalFileStorage<RT>,
        modules_storage: Arc<dyn Storage>,
        module_cache: Arc<dyn ModuleLoader<RT>>,
        function_log: FunctionExecutionLog<RT>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        fetch_client: Arc<dyn FetchClient>,
    ) -> Self {
        // We limit the isolates to only consume fraction of the available
        // cores leaving the rest for tokio. This is still over-provisioning
        // in case there are multiple active backends per server.
        let isolate_concurrency_limit =
            *BACKEND_ISOLATE_ACTIVE_THREADS_PERCENT * num_cpus::get_physical() / 100;
        let limiter = ConcurrencyLimiter::new(isolate_concurrency_limit);
        tracing::info!(
            "Limiting isolate concurrency to {} ({}% out of {} physical cores)",
            isolate_concurrency_limit,
            *BACKEND_ISOLATE_ACTIVE_THREADS_PERCENT,
            num_cpus::get_physical(),
        );

        let http_actions_worker = BackendIsolateWorker::new(
            runtime.clone(),
            IsolateConfig::new("actions", limiter.clone()),
        );
        let http_actions = IsolateClient::new(
            runtime.clone(),
            http_actions_worker,
            *APPLICATION_MAX_CONCURRENT_HTTP_ACTIONS,
            true,
            instance_name.clone(),
            instance_secret,
            file_storage.clone(),
            system_env_vars.clone(),
            module_cache.clone(),
        );

        let analyze_isolate_worker = BackendIsolateWorker::new(
            runtime.clone(),
            IsolateConfig::new("database_executor", limiter),
        );
        let analyze_isolate = IsolateClient::new(
            runtime.clone(),
            analyze_isolate_worker,
            *UDF_ISOLATE_MAX_EXEC_THREADS,
            false,
            instance_name,
            instance_secret,
            file_storage.clone(),
            system_env_vars.clone(),
            module_cache.clone(),
        );

        let isolate_functions = FunctionRouter::new(
            function_runner,
            runtime.clone(),
            database.clone(),
            system_env_vars.clone(),
        );
        let cache_manager = CacheManager::new(
            runtime.clone(),
            database.clone(),
            isolate_functions.clone(),
            function_log.clone(),
        );

        Self {
            runtime,
            database,
            key_broker,
            isolate_functions,
            analyze_isolate,
            http_actions,
            node_actions,
            module_cache,
            modules_storage,
            file_storage,
            function_log,
            cache_manager,
            system_env_vars,
            node_action_limiter: Limiter::new(
                ModuleEnvironment::Node,
                UdfType::Action,
                *APPLICATION_MAX_CONCURRENT_NODE_ACTIONS,
            ),
            fetch_client,
        }
    }

    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        self.analyze_isolate.shutdown().await?;
        self.http_actions.shutdown().await?;
        self.node_actions.shutdown();
        Ok(())
    }

    // Only used for running queries from REPLs.
    pub async fn run_query_without_caching(
        &self,
        request_id: RequestId,
        mut tx: Transaction<RT>,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        caller: FunctionCaller,
    ) -> anyhow::Result<UdfOutcome> {
        if !(tx.identity().is_admin() || tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("query_without_caching"));
        }

        let identity = tx.inert_identity();
        let start = self.runtime.monotonic_now();
        let validate_result = ValidatedPathAndArgs::new(
            caller.allowed_visibility(),
            &mut tx,
            PublicFunctionPath::Component(path.clone()),
            arguments.clone(),
            UdfType::Query,
        )
        .await?;
        let context = ExecutionContext::new(request_id, &caller);
        let (mut tx, outcome) = match validate_result {
            Ok(path_and_args) => {
                self.isolate_functions
                    .execute_query_or_mutation(
                        tx,
                        path_and_args,
                        UdfType::Query,
                        QueryJournal::new(),
                        context.clone(),
                    )
                    .await?
            },
            Err(js_err) => {
                let query_outcome = UdfOutcome::from_error(
                    js_err,
                    path.clone(),
                    arguments.clone(),
                    identity.clone(),
                    self.runtime.clone(),
                    None,
                )?;
                (tx, FunctionOutcome::Query(query_outcome))
            },
        };
        let outcome = match outcome {
            FunctionOutcome::Query(o) => o,
            _ => anyhow::bail!("Received non-query outcome for query"),
        };
        let stats = tx.take_stats();

        self.function_log.log_query(
            outcome.clone(),
            stats,
            false,
            start.elapsed(),
            caller,
            tx.usage_tracker,
            context,
        );

        Ok(outcome)
    }

    /// Runs a mutations and retries on OCC errors.
    #[minitrace::trace]
    pub async fn retry_mutation(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        arguments: Vec<JsonValue>,
        identity: Identity,
        mutation_identifier: Option<SessionRequestIdentifier>,
        caller: FunctionCaller,
        pause_client: PauseClient,
    ) -> anyhow::Result<Result<MutationReturn, MutationError>> {
        let timer = mutation_timer();
        let result = self
            ._retry_mutation(
                request_id,
                path,
                arguments,
                identity,
                mutation_identifier,
                caller,
                pause_client,
            )
            .await;
        match &result {
            Ok(_) => timer.finish(),
            Err(e) => timer.finish_with(e.metric_status_label_value()),
        };
        result
    }

    /// Runs a mutations and retries on OCC errors.
    #[minitrace::trace]
    async fn _retry_mutation(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        arguments: Vec<JsonValue>,
        identity: Identity,
        mutation_identifier: Option<SessionRequestIdentifier>,
        caller: FunctionCaller,
        pause_client: PauseClient,
    ) -> anyhow::Result<Result<MutationReturn, MutationError>> {
        if path.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("mutation"));
        }
        let arguments = match parse_udf_args(path.udf_path(), arguments) {
            Ok(arguments) => arguments,
            Err(error) => {
                return Ok(Err(MutationError {
                    error,
                    log_lines: vec![].into(),
                }))
            },
        };
        let udf_path_string = (!path.is_system()).then_some(path.udf_path().to_string());

        let mut backoff = Backoff::new(
            *UDF_EXECUTOR_OCC_INITIAL_BACKOFF,
            *UDF_EXECUTOR_OCC_MAX_BACKOFF,
        );

        let usage_tracker = FunctionUsageTracker::new();
        loop {
            // Note that we use different context for every mutation attempt.
            // This so every JS function run gets a different executionId.
            let context = ExecutionContext::new(request_id.clone(), &caller);

            let start = self.runtime.monotonic_now();
            let mut tx = self
                .database
                .begin_with_usage(identity.clone(), usage_tracker.clone())
                .await?;
            pause_client.wait("retry_mutation_loop_start").await;
            let identity = tx.inert_identity();

            // Return the previous execution's result if the mutation was committed already.
            if let Some(result) = self
                .check_mutation_status(&mut tx, &mutation_identifier)
                .await?
            {
                return Ok(result);
            }

            let result: Result<(Transaction<RT>, ValidatedUdfOutcome), anyhow::Error> = self
                .run_mutation_no_udf_log(
                    tx,
                    path.clone(),
                    arguments.clone(),
                    caller.allowed_visibility(),
                    context.clone(),
                )
                .await;
            let (mut tx, mut outcome) = match result {
                Ok(r) => r,
                Err(e) => {
                    self.function_log.log_mutation_system_error(
                        &e,
                        path.debug_into_component_path(),
                        arguments,
                        identity,
                        start,
                        caller,
                        context.clone(),
                    )?;
                    return Err(e);
                },
            };

            // Save a CommittedMutation object so we won't rerun this mutation if
            // successful.
            self.write_mutation_status(&mut tx, &mutation_identifier, &outcome)
                .await?;

            let stats = tx.take_stats();
            let execution_time = start.elapsed();
            let log_lines = outcome.log_lines.clone();
            let value = match outcome.result {
                Ok(ref value) => value.clone(),
                // If it's an error inside the UDF, log the failed execution and return the
                // developer error.
                Err(ref error) => {
                    drop(tx);
                    self.function_log.log_mutation(
                        outcome.clone(),
                        stats,
                        execution_time,
                        caller,
                        usage_tracker,
                        context.clone(),
                    );
                    return Ok(Err(MutationError {
                        error: error.to_owned(),
                        log_lines,
                    }));
                },
            };

            let value = value.unpack();

            // Attempt to commit the transaction and log an error if commit failed,
            // even if it was an OCC error. We may decide later to suppress OCC
            // errors from the log.
            let result = match self
                .database
                .commit_with_write_source(tx, udf_path_string.clone())
                .await
            {
                Ok(ts) => Ok(MutationReturn {
                    value,
                    log_lines,
                    ts,
                }),
                Err(e) => {
                    if e.is_deterministic_user_error() {
                        let js_error = JsError::from_error(e);
                        outcome.result = Err(js_error.clone());
                        Err(MutationError {
                            error: js_error,
                            log_lines,
                        })
                    } else {
                        if e.is_occ()
                            && (backoff.failures() as usize) < *UDF_EXECUTOR_OCC_MAX_RETRIES
                        {
                            let sleep = backoff.fail(&mut self.runtime.rng());
                            tracing::warn!(
                                "Optimistic concurrency control failed ({e}), retrying \
                                 {udf_path_string:?} after {sleep:?}",
                            );
                            self.runtime.wait(sleep).await;
                            continue;
                        }
                        outcome.result = Err(JsError::from_error_ref(&e));

                        if e.is_occ() {
                            self.function_log.log_mutation_occ_error(
                                outcome,
                                stats,
                                execution_time,
                                caller,
                                context.clone(),
                            );
                        } else {
                            self.function_log.log_mutation_system_error(
                                &e,
                                path.debug_into_component_path(),
                                arguments,
                                identity,
                                start,
                                caller,
                                context,
                            )?;
                        }
                        log_occ_retries(backoff.failures() as usize);
                        return Err(e);
                    }
                },
            };

            self.function_log.log_mutation(
                outcome.clone(),
                stats,
                execution_time,
                caller,
                usage_tracker,
                context.clone(),
            );
            log_occ_retries(backoff.failures() as usize);
            pause_client.close("retry_mutation_loop_start");
            return Ok(result);
        }
    }

    /// Attempts to run a mutation once using the given transaction.
    /// The method is not idempotent. It is the caller responsibility to
    /// drive retries as we as log in the UDF log.
    pub async fn run_mutation_no_udf_log(
        &self,
        tx: Transaction<RT>,
        path: PublicFunctionPath,
        arguments: ConvexArray,
        allowed_visibility: AllowedVisibility,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, ValidatedUdfOutcome)> {
        let result = self
            .run_mutation_inner(tx, path, arguments, allowed_visibility, context)
            .await;
        match result.as_ref() {
            Ok((_, udf_outcome)) => {
                let result = if udf_outcome.result.is_ok() {
                    UdfExecutorResult::Success
                } else {
                    UdfExecutorResult::UserError
                };
                log_udf_executor_result(UdfType::Mutation, result);
            },
            Err(e) => {
                log_udf_executor_result(
                    UdfType::Mutation,
                    UdfExecutorResult::SystemError(e.metric_status_label_value()),
                );
            },
        };
        result
    }

    /// Runs the mutation once without any logging.
    #[minitrace::trace]
    async fn run_mutation_inner(
        &self,
        mut tx: Transaction<RT>,
        path: PublicFunctionPath,
        arguments: ConvexArray,
        allowed_visibility: AllowedVisibility,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, ValidatedUdfOutcome)> {
        if path.is_system() && !(tx.identity().is_admin() || tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("mutation"));
        }
        let identity = tx.inert_identity();
        let validate_result = ValidatedPathAndArgs::new_with_returns_validator(
            allowed_visibility,
            &mut tx,
            path.clone(),
            arguments.clone(),
            UdfType::Mutation,
        )
        .await?;

        let (path_and_args, returns_validator) = match validate_result {
            Ok(tuple) => tuple,
            Err(js_err) => {
                let mutation_outcome = ValidatedUdfOutcome::from_error(
                    js_err,
                    path.debug_into_component_path(),
                    arguments.clone(),
                    identity.clone(),
                    self.runtime.clone(),
                    None,
                )?;
                return Ok((tx, mutation_outcome));
            },
        };

        let path = path_and_args.path().clone();
        let (mut tx, outcome) = self
            .isolate_functions
            .execute_query_or_mutation(
                tx,
                path_and_args,
                UdfType::Mutation,
                QueryJournal::new(),
                context,
            )
            .await?;
        let mutation_outcome = match outcome {
            FunctionOutcome::Mutation(o) => o,
            _ => anyhow::bail!("Received non-mutation outcome for mutation"),
        };
        let component = path.component;

        let table_mapping = tx.table_mapping().namespace(component.into());

        let outcome = ValidatedUdfOutcome::new(mutation_outcome, returns_validator, &table_mapping);

        Ok((tx, outcome))
    }

    #[minitrace::trace]
    pub async fn run_action(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        arguments: Vec<JsonValue>,
        identity: Identity,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<ActionReturn, ActionError>> {
        if path.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("action"));
        }
        let arguments = match parse_udf_args(path.udf_path(), arguments) {
            Ok(arguments) => arguments,
            Err(error) => {
                return Ok(Err(ActionError {
                    error,
                    log_lines: vec![].into(),
                }))
            },
        };
        let context = ExecutionContext::new(request_id.clone(), &caller);
        let usage_tracking = FunctionUsageTracker::new();
        let start = self.runtime.monotonic_now();
        let completion_result = self
            .run_action_no_udf_log(
                path.clone(),
                arguments.clone(),
                identity.clone(),
                caller.clone(),
                usage_tracking.clone(),
                context.clone(),
            )
            .await;
        let completion = match completion_result {
            Ok(c) => c,
            Err(e) => {
                self.function_log.log_action_system_error(
                    &e,
                    path.debug_into_component_path(),
                    arguments,
                    identity.into(),
                    start,
                    caller,
                    vec![].into(),
                    context,
                )?;
                anyhow::bail!(e)
            },
        };
        let log_lines = completion.log_lines().clone();
        let result = completion.outcome.result.clone();
        self.function_log.log_action(completion, usage_tracking);

        let value = match result {
            Ok(ref value) => value.unpack(),
            // If it's an error inside the UDF, log the failed execution and return the
            // developer error.
            Err(error) => return Ok(Err(ActionError { error, log_lines })),
        };

        Ok(Ok(ActionReturn { value, log_lines }))
    }

    /// Runs the actions without logging to the UDF log. It is the caller
    /// responsibility to log to the UDF log.
    #[minitrace::trace]
    pub async fn run_action_no_udf_log(
        &self,
        path: PublicFunctionPath,
        arguments: ConvexArray,
        identity: Identity,
        caller: FunctionCaller,
        usage_tracking: FunctionUsageTracker,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionCompletion> {
        let result = self
            .run_action_inner(path, arguments, identity, caller, usage_tracking, context)
            .await;
        match result.as_ref() {
            Ok(completion) => {
                let result = if completion.outcome.result.is_ok() {
                    UdfExecutorResult::Success
                } else {
                    UdfExecutorResult::UserError
                };
                log_udf_executor_result(UdfType::Action, result);
            },
            Err(e) => {
                log_udf_executor_result(
                    UdfType::Action,
                    UdfExecutorResult::SystemError(e.metric_status_label_value()),
                );
            },
        };
        result
    }

    /// Runs the action without any logging.
    #[minitrace::trace]
    async fn run_action_inner(
        &self,
        path: PublicFunctionPath,
        arguments: ConvexArray,
        identity: Identity,
        caller: FunctionCaller,
        usage_tracking: FunctionUsageTracker,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionCompletion> {
        if path.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("action"));
        }
        let unix_timestamp = self.runtime.unix_timestamp();
        let start = self.runtime.monotonic_now();
        let mut tx = self
            .database
            .begin_with_usage(identity.clone(), usage_tracking)
            .await?;
        let validate_result = ValidatedPathAndArgs::new_with_returns_validator(
            caller.allowed_visibility(),
            &mut tx,
            path.clone(),
            arguments.clone(),
            UdfType::Action,
        )
        .await?;

        // Fetch the returns_validator now to be used at a later ts.
        let (path_and_args, returns_validator) = match validate_result {
            Ok((path_and_args, returns_validator)) => (path_and_args, returns_validator),
            Err(js_error) => {
                return Ok(ActionCompletion {
                    outcome: ValidatedActionOutcome::from_error(
                        js_error,
                        path.debug_into_component_path(),
                        arguments,
                        identity.into(),
                        self.runtime.clone(),
                        None,
                    ),
                    environment: ModuleEnvironment::Invalid,
                    memory_in_mb: 0,
                    execution_time: Duration::from_secs(0),
                    context,
                    unix_timestamp,
                    caller,
                    log_lines: vec![].into(),
                });
            },
        };

        let component = path_and_args.path().component;

        // We should use table mappings from the same transaction as the output
        // validator was retrieved.
        let table_mapping = tx.table_mapping().namespace(component.into());
        let virtual_system_mapping = tx.virtual_system_mapping().clone();
        let udf_server_version = path_and_args.npm_version().clone();
        // We should not be missing the module given we validated the path above
        // which requires the module to exist.
        let path = path_and_args.path().clone();
        let module = ModuleModel::new(&mut tx)
            .get_metadata_for_function_by_id(&path)
            .await?
            .context("Missing a valid module")?;
        let (log_line_sender, log_line_receiver) = mpsc::unbounded_channel();

        let inert_identity = tx.inert_identity();
        let timer = function_total_timer(module.environment, UdfType::Action);
        let completion_result = match module.environment {
            ModuleEnvironment::Isolate => {
                // TODO: This is the only use case of clone. We should get rid of clone,
                // when we deprecate that codepath.
                let outcome_future = self
                    .isolate_functions
                    .execute_action(tx, path_and_args, log_line_sender, context.clone())
                    .boxed();
                let (outcome_result, log_lines) = run_function_and_collect_log_lines(
                    outcome_future,
                    log_line_receiver,
                    |log_line| {
                        self.function_log.log_action_progress(
                            path.clone().for_logging(),
                            unix_timestamp,
                            context.clone(),
                            vec![log_line].into(),
                            module.environment,
                        )
                    },
                )
                .await;

                let memory_in_mb: u64 = (*ISOLATE_MAX_USER_HEAP_SIZE / (1 << 20))
                    .try_into()
                    .unwrap();

                let validated_outcome_result = outcome_result.map(|outcome| {
                    ValidatedActionOutcome::new(outcome, returns_validator, &table_mapping)
                });

                timer.finish();
                validated_outcome_result.map(|outcome| ActionCompletion {
                    outcome,
                    execution_time: start.elapsed(),
                    environment: ModuleEnvironment::Isolate,
                    memory_in_mb,
                    context: context.clone(),
                    unix_timestamp,
                    caller: caller.clone(),
                    log_lines,
                })
            },
            ModuleEnvironment::Node => {
                // We should not be missing the module given we validated the path above
                // which requires the module to exist.
                let module_path = CanonicalizedComponentModulePath {
                    component: path.component,
                    module_path: path.udf_path.module().clone(),
                };
                let module_version = self
                    .module_cache
                    .get_module(&mut tx, module_path.clone())
                    .await?
                    .context("Missing a valid module_version")?;
                let _request_guard = self
                    .node_action_limiter
                    .acquire_permit_with_timeout(&self.runtime)
                    .await?;
                let mut source_maps = BTreeMap::new();
                if let Some(source_map) = module_version.source_map.clone() {
                    source_maps.insert(module_path.module_path.clone(), source_map);
                }

                let source_package_id = module.source_package_id;
                let source_package = SourcePackageModel::new(&mut tx, component.into())
                    .get(source_package_id)
                    .await?
                    .into_value();
                let mut environment_variables =
                    EnvironmentVariablesModel::new(&mut tx).get_all().await?;
                // Insert special environment variables if not already provided by user
                environment_variables.extend(self.system_env_vars.clone());

                // Fetch source and external_deps presigned URI first
                let source_uri_future = self
                    .modules_storage
                    .signed_url(source_package.storage_key.clone(), Duration::from_secs(60));
                let (source_uri, external_deps_package) = if let Some(external_deps_package_id) =
                    source_package.external_deps_package_id
                {
                    let pkg = ExternalPackagesModel::new(&mut tx)
                        .get(external_deps_package_id)
                        .await?
                        .into_value();
                    let external_uri_future = self
                        .modules_storage
                        .signed_url(pkg.storage_key.clone(), Duration::from_secs(60));

                    let (source_uri, external_deps_uri) =
                        tokio::try_join!(source_uri_future, external_uri_future)?;
                    (
                        source_uri,
                        Some(node_executor::Package {
                            uri: external_deps_uri,
                            key: pkg.storage_key,
                            sha256: pkg.sha256,
                        }),
                    )
                } else {
                    (source_uri_future.await?, None)
                };

                let udf_server_version = path_and_args.npm_version().clone();
                let request = ExecuteRequest {
                    path_and_args,
                    source_package: node_executor::SourcePackage {
                        bundled_source: node_executor::Package {
                            uri: source_uri,
                            key: source_package.storage_key,
                            sha256: source_package.sha256,
                        },
                        external_deps: external_deps_package,
                    },
                    source_package_id,
                    user_identity: tx.user_identity(),
                    auth_header: token_to_authorization_header(tx.authentication_token())?,
                    environment_variables,
                    callback_token: self.key_broker.issue_action_token(path.component),
                    context: context.clone(),
                    encoded_parent_trace: EncodedSpan::from_parent().0,
                };

                let node_outcome_future = self
                    .node_actions
                    .execute(request, &source_maps, log_line_sender)
                    .boxed();
                let (mut node_outcome_result, log_lines) = run_function_and_collect_log_lines(
                    node_outcome_future,
                    log_line_receiver,
                    |log_line| {
                        self.function_log.log_action_progress(
                            path.clone().for_logging(),
                            unix_timestamp,
                            context.clone(),
                            vec![log_line].into(),
                            module.environment,
                        )
                    },
                )
                .await;

                timer.finish();

                if let Ok(ref mut node_outcome) = node_outcome_result {
                    if let Ok(ref output) = node_outcome.result {
                        if let Some(js_err) = returns_validator.check_output(
                            output,
                            &table_mapping,
                            &virtual_system_mapping,
                        ) {
                            node_outcome.result = Err(js_err);
                        }
                    }
                }

                node_outcome_result.map(|node_outcome| {
                    let outcome = ActionOutcome {
                        path: path.clone().for_logging(),
                        arguments: arguments.clone(),
                        identity: tx.inert_identity(),
                        unix_timestamp,
                        result: node_outcome.result.map(JsonPackedValue::pack),
                        syscall_trace: node_outcome.syscall_trace,
                        udf_server_version,
                    };
                    let outcome =
                        ValidatedActionOutcome::new(outcome, returns_validator, &table_mapping);
                    ActionCompletion {
                        outcome,
                        execution_time: start.elapsed(),
                        environment: ModuleEnvironment::Node,
                        memory_in_mb: node_outcome.memory_used_in_mb,
                        context: context.clone(),
                        unix_timestamp,
                        caller: caller.clone(),
                        log_lines,
                    }
                })
            },
            ModuleEnvironment::Invalid => {
                Err(anyhow::anyhow!("Attempting to run an invalid function"))
            },
        };
        match completion_result {
            Ok(c) => Ok(c),
            Err(e) if e.is_deterministic_user_error() => {
                let outcome = ValidatedActionOutcome::from_error(
                    JsError::from_error(e),
                    path.for_logging(),
                    arguments,
                    inert_identity,
                    self.runtime.clone(),
                    udf_server_version,
                );
                Ok(ActionCompletion {
                    outcome,
                    execution_time: start.elapsed(),
                    environment: module.environment,
                    memory_in_mb: match module.environment {
                        ModuleEnvironment::Isolate => (*ISOLATE_MAX_USER_HEAP_SIZE / (1 << 20))
                            .try_into()
                            .unwrap(),
                        // This isn't correct but we don't have a value to use here.
                        ModuleEnvironment::Node => 0,
                        ModuleEnvironment::Invalid => 0,
                    },
                    context,
                    unix_timestamp,
                    caller,
                    log_lines: vec![].into(),
                })
            },
            Err(e) => Err(e),
        }
    }

    #[minitrace::trace]
    pub async fn build_deps(
        &self,
        deps: Vec<NodeDependency>,
    ) -> anyhow::Result<Result<ExternalDepsPackage, JsError>> {
        let (object_key, upload_uri) = self
            .modules_storage
            .presigned_upload_url(*BUILD_DEPS_TIMEOUT)
            .await?;
        let request = BuildDepsRequest {
            deps: deps.clone(),
            upload_url: upload_uri,
        };
        let build_deps_res = self.node_actions.build_deps(request).await?;
        Ok(
            build_deps_res.map(move |(digest, package_size)| ExternalDepsPackage {
                storage_key: object_key,
                sha256: digest,
                deps,
                package_size,
            }),
        )
    }

    #[minitrace::trace]
    pub async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        self.analyze_isolate
            .evaluate_app_definitions(
                app_definition,
                component_definitions,
                dependency_graph,
                environment_variables,
                self.system_env_vars.clone(),
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
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>> {
        self.analyze_isolate
            .evaluate_component_initializer(evaluated_definitions, path, definition, args, name)
            .await
    }

    #[minitrace::trace]
    pub async fn analyze(
        &self,
        udf_config: UdfConfig,
        new_modules: Vec<ModuleConfig>,
        source_package: SourcePackage,
        mut environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        // Insert special environment variables if not already provided by user
        environment_variables.extend(self.system_env_vars.clone());

        let (node_modules, isolate_modules) = new_modules
            .into_iter()
            .map(|module| (module.path.clone().canonicalize(), module))
            .partition(|(_, config)| config.environment == ModuleEnvironment::Node);

        let mut result = BTreeMap::new();

        let isolate_future = self.isolate_functions.function_runner.analyze(
            udf_config,
            isolate_modules,
            environment_variables.clone(),
        );

        let node_future = async {
            if node_modules.is_empty() {
                return Ok(Ok(BTreeMap::new()));
            }
            for path_str in ["schema.js", "crons.js", "http.js"] {
                let path = path_str
                    .parse()
                    .expect("Failed to parse static module names");
                // The cli should not do this. Log as system error.
                anyhow::ensure!(
                    !node_modules.contains_key(&path),
                    "{path_str} can't be analyzed in Node.js!"
                );
            }
            let mut source_maps = BTreeMap::new();
            for (path, module) in node_modules.iter() {
                if let Some(source_map) = module.source_map.clone() {
                    source_maps.insert(path.clone(), source_map);
                }
            }
            // Fetch source and external_deps presigned URI first
            let source_uri_future = self
                .modules_storage
                .signed_url(source_package.storage_key.clone(), Duration::from_secs(60));
            let mut tx = self.database.begin_system().await?;
            let (source_uri, external_deps_package) =
                if let Some(external_deps_package_id) = source_package.external_deps_package_id {
                    let pkg = ExternalPackagesModel::new(&mut tx)
                        .get(external_deps_package_id)
                        .await?
                        .into_value();
                    let external_uri_future = self
                        .modules_storage
                        .signed_url(pkg.storage_key.clone(), Duration::from_secs(60));

                    let (source_uri, external_deps_uri) =
                        tokio::try_join!(source_uri_future, external_uri_future)?;
                    (
                        source_uri,
                        Some(node_executor::Package {
                            uri: external_deps_uri,
                            key: pkg.storage_key,
                            sha256: pkg.sha256,
                        }),
                    )
                } else {
                    (source_uri_future.await?, None)
                };

            let request = AnalyzeRequest {
                source_package: node_executor::SourcePackage {
                    bundled_source: node_executor::Package {
                        uri: source_uri,
                        key: source_package.storage_key,
                        sha256: source_package.sha256,
                    },
                    external_deps: external_deps_package,
                },
                environment_variables,
            };
            self.node_actions.analyze(request, &source_maps).await
        };

        let (isolate_result, node_result) = tokio::try_join!(isolate_future, node_future)?;
        match isolate_result {
            Ok(modules) => result.extend(modules),
            Err(e) => return Ok(Err(e)),
        }
        match node_result {
            Ok(modules) => {
                for (path, analyzed_module) in modules {
                    let exists = result.insert(path, analyzed_module).is_some();
                    // Note that although we send all modules to actions.analyze, it
                    // currently ignores isolate modules.
                    anyhow::ensure!(!exists, "actions.analyze returned isolate modules");
                }
            },
            Err(e) => return Ok(Err(e)),
        }

        self.validate_cron_jobs(&result)??;
        Ok(Ok(result))
    }

    #[minitrace::trace]
    fn validate_cron_jobs(
        &self,
        modules: &BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    ) -> anyhow::Result<Result<(), JsError>> {
        // Validate that every cron job schedules an action or mutation.
        for module in modules.values() {
            let Some(crons) = module.cron_specs.as_ref() else {
                continue;
            };
            for (identifier, cron_spec) in crons {
                let path = cron_spec.udf_path.module().clone();
                let Some(scheduled_module) = modules.get(&path) else {
                    return Ok(Err(JsError::from_message(format!(
                        "The cron job '{identifier}' schedules a function that does not exist: {}",
                        cron_spec.udf_path
                    ))));
                };
                let name = cron_spec.udf_path.function_name();
                let Some(scheduled_function) =
                    scheduled_module.functions.iter().find(|f| &f.name == name)
                else {
                    return Ok(Err(JsError::from_message(format!(
                        "The cron job '{identifier}' schedules a function that does not exist: {}",
                        cron_spec.udf_path
                    ))));
                };
                match scheduled_function.udf_type {
                    UdfType::Query => {
                        return Ok(Err(JsError::from_message(format!(
                            "The cron job '{identifier}' schedules a query function, only actions \
                             and mutations can be scheduled: {}",
                            cron_spec.udf_path
                        ))));
                    },
                    UdfType::HttpAction => {
                        return Ok(Err(JsError::from_message(format!(
                            "The cron job '{identifier}' schedules an HTTP action, only actions \
                             and mutations can be scheduled: {}",
                            cron_spec.udf_path
                        ))));
                    },
                    UdfType::Mutation => {},
                    UdfType::Action => {},
                }
            }
        }
        Ok(Ok(()))
    }

    pub async fn evaluate_schema(
        &self,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<DatabaseSchema> {
        self.analyze_isolate
            .evaluate_schema(schema_bundle, source_map, rng_seed, unix_timestamp)
            .await
    }

    pub async fn evaluate_auth_config(
        &self,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        mut environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<AuthConfig> {
        environment_variables.extend(self.system_env_vars.clone());
        self.analyze_isolate
            .evaluate_auth_config(auth_config_bundle, source_map, environment_variables)
            .await
    }

    pub fn enable_actions(&self) -> anyhow::Result<()> {
        self.node_actions.enable()
    }

    #[minitrace::trace]
    pub async fn run_query_at_ts(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        ts: Timestamp,
        journal: Option<QueryJournal>,
        caller: FunctionCaller,
    ) -> anyhow::Result<QueryReturn> {
        let result = self
            .run_query_at_ts_inner(request_id, path, args, identity, ts, journal, caller)
            .await;
        match result.as_ref() {
            Ok(udf_outcome) => {
                let result = if udf_outcome.result.is_ok() {
                    UdfExecutorResult::Success
                } else {
                    UdfExecutorResult::UserError
                };
                log_udf_executor_result(UdfType::Query, result);
            },
            Err(e) => {
                log_udf_executor_result(
                    UdfType::Query,
                    UdfExecutorResult::SystemError(e.metric_status_label_value()),
                );
            },
        };
        result
    }

    #[minitrace::trace]
    async fn run_query_at_ts_inner(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        ts: Timestamp,
        journal: Option<QueryJournal>,
        caller: FunctionCaller,
    ) -> anyhow::Result<QueryReturn> {
        if path.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("query"));
        }
        let args = match parse_udf_args(path.udf_path(), args) {
            Ok(arguments) => arguments,
            Err(js_error) => {
                return Ok(QueryReturn {
                    result: Err(js_error),
                    log_lines: vec![].into(),
                    token: Token::empty(ts),
                    journal: QueryJournal::new(),
                });
            },
        };
        let usage_tracker = FunctionUsageTracker::new();
        let result = self
            .cache_manager
            .get(
                request_id,
                path,
                args,
                identity.clone(),
                ts,
                journal,
                caller,
                usage_tracker.clone(),
            )
            .await?;

        Ok(result)
    }

    #[minitrace::trace]
    async fn check_mutation_status(
        &self,
        tx: &mut Transaction<RT>,
        mutation_identifier: &Option<SessionRequestIdentifier>,
    ) -> anyhow::Result<Option<Result<MutationReturn, MutationError>>> {
        let Some(ref identifier) = mutation_identifier else {
            return Ok(None);
        };
        let mutation_status = SessionRequestModel::new(tx)
            .get_session_request_record(identifier, Identity::system())
            .await?;
        let result = match mutation_status {
            Some((ts, SessionRequestOutcome::Mutation { result, log_lines })) => {
                tracing::info!("Mutation already executed so skipping {:?}", identifier);
                log_mutation_already_committed();
                Ok(MutationReturn {
                    value: result,
                    log_lines,
                    ts,
                })
            },
            None => return Ok(None),
        };
        Ok(Some(result))
    }

    #[minitrace::trace]
    async fn write_mutation_status(
        &self,
        tx: &mut Transaction<RT>,
        mutation_identifier: &Option<SessionRequestIdentifier>,
        outcome: &ValidatedUdfOutcome,
    ) -> anyhow::Result<()> {
        let Some(ref identifier) = mutation_identifier else {
            return Ok(());
        };
        if let Ok(ref value) = outcome.result {
            let record = SessionRequestRecord {
                session_id: identifier.session_id,
                request_id: identifier.request_id,
                outcome: SessionRequestOutcome::Mutation {
                    result: value.unpack(),
                    log_lines: outcome.log_lines.clone(),
                },
                identity: outcome.identity.clone(),
            };
            SessionRequestModel::new(tx)
                .record_session_request(record, Identity::system())
                .await?;
        }
        Ok(())
    }

    async fn bail_if_backend_not_running(&self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        let backend_state = BackendStateModel::new(tx).get_backend_state().await?;
        if backend_state.is_stopped() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "BackendIsNotRunning",
                "Cannot perform this operation when the backend is not running"
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl<RT: Runtime> ActionCallbacks for ApplicationFunctionRunner<RT> {
    #[minitrace::trace]
    async fn execute_query(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let ts = self.database.now_ts_for_reads();
        let result = self
            .run_query_at_ts(
                context.request_id,
                PublicFunctionPath::Component(path),
                args,
                identity,
                *ts,
                None,
                FunctionCaller::Action {
                    parent_scheduled_job: context.parent_scheduled_job,
                },
            )
            .await?
            .result;
        Ok(FunctionResult { result })
    }

    #[minitrace::trace]
    async fn execute_mutation(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let result = self
            .retry_mutation(
                context.request_id,
                PublicFunctionPath::Component(path),
                args,
                identity,
                None,
                FunctionCaller::Action {
                    parent_scheduled_job: context.parent_scheduled_job,
                },
                PauseClient::new(),
            )
            .await
            .map(|r| match r {
                Ok(mutation_return) => Ok(mutation_return.value),
                Err(mutation_error) => Err(mutation_error.error),
            })?;
        Ok(FunctionResult { result })
    }

    #[minitrace::trace]
    async fn execute_action(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let _tx = self.database.begin(identity.clone()).await?;
        let result = self
            .run_action(
                context.request_id,
                PublicFunctionPath::Component(path),
                args,
                identity,
                FunctionCaller::Action {
                    parent_scheduled_job: context.parent_scheduled_job,
                },
            )
            .await
            .map(|r| match r {
                Ok(action_return) => Ok(action_return.value),
                Err(action_error) => Err(action_error.error),
            })?;
        Ok(FunctionResult { result })
    }

    async fn storage_get_url(
        &self,
        identity: Identity,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>> {
        let mut tx = self.database.begin(identity).await?;
        self.bail_if_backend_not_running(&mut tx).await?;
        self.file_storage
            .get_url(&mut tx, component, storage_id)
            .await
    }

    async fn storage_get_file_entry(
        &self,
        identity: Identity,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<(ComponentPath, FileStorageEntry)>> {
        let mut tx = self.database.begin(identity).await?;
        self.bail_if_backend_not_running(&mut tx).await?;
        let Some(component_path) = tx.get_component_path(component) else {
            return Ok(None);
        };
        let entry = self
            .file_storage
            .get_file_entry(&mut tx, component.into(), storage_id)
            .await?;
        Ok(entry.map(|e| (component_path, e)))
    }

    async fn storage_store_file_entry(
        &self,
        identity: Identity,
        component: ComponentId,
        entry: FileStorageEntry,
    ) -> anyhow::Result<(ComponentPath, DeveloperDocumentId)> {
        let mut tx = self.database.begin(identity.clone()).await?;
        self.bail_if_backend_not_running(&mut tx).await?;
        let (_ts, r, _stats) = self
            .database
            .execute_with_occ_retries(
                identity,
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "app_funrun_storage_store_file_entry",
                |tx| {
                    async {
                        let component_path = tx
                            .get_component_path(component)
                            .context(format!("Component {component:?} not found"))?;
                        let id = self
                            .file_storage
                            .store_file_entry(tx, component.into(), entry.clone())
                            .await?;
                        Ok((component_path, id))
                    }
                    .into()
                },
            )
            .await?;
        Ok(r)
    }

    async fn storage_delete(
        &self,
        identity: Identity,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<()> {
        let mut tx = self.database.begin(identity.clone()).await?;
        self.bail_if_backend_not_running(&mut tx).await?;
        self.database
            .execute_with_occ_retries(
                identity,
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "app_funrun_storage_delete",
                |tx| {
                    async {
                        self.file_storage
                            .delete(tx, component.into(), storage_id.clone())
                            .await?;
                        Ok(())
                    }
                    .into()
                },
            )
            .await?;

        Ok(())
    }

    async fn schedule_job(
        &self,
        identity: Identity,
        scheduling_component: ComponentId,
        scheduled_path: CanonicalizedComponentFunctionPath,
        udf_args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
        context: ExecutionContext,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let (_ts, virtual_id, _stats) = self
            .database
            .execute_with_occ_retries(
                identity,
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "app_funrun_schedule_job",
                |tx| {
                    let path = scheduled_path.clone();
                    let args = udf_args.clone();
                    let context = context.clone();
                    async move {
                        let (path, udf_args) = validate_schedule_args(
                            path,
                            args,
                            scheduled_ts,
                            // Scheduling from actions is not transaction and happens at latest
                            // timestamp.
                            self.database.runtime().unix_timestamp(),
                            tx,
                        )
                        .await?;
                        let virtual_id =
                            VirtualSchedulerModel::new(tx, scheduling_component.into())
                                .schedule(path, udf_args, scheduled_ts, context)
                                .await?;
                        Ok(virtual_id)
                    }
                    .into()
                },
            )
            .await?;
        Ok(virtual_id)
    }

    async fn cancel_job(
        &self,
        identity: Identity,
        virtual_id: DeveloperDocumentId,
    ) -> anyhow::Result<()> {
        self.database
            .execute_with_occ_retries(
                identity,
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "app_funrun_cancel_job",
                |tx| {
                    async {
                        VirtualSchedulerModel::new(tx, TableNamespace::by_component_TODO())
                            .cancel(virtual_id)
                            .await
                    }
                    .into()
                },
            )
            .await?;
        Ok(())
    }

    async fn vector_search(
        &self,
        identity: Identity,
        query: JsonValue,
    ) -> anyhow::Result<(Vec<PublicVectorSearchQueryResult>, FunctionUsageStats)> {
        let query = VectorSearch::try_from(query).map_err(|e| {
            let message = e.to_string();
            e.context(ErrorMetadata::bad_request("InvalidVectorQuery", message))
        })?;
        self.database.vector_search(identity, query).await
    }

    async fn lookup_function_handle(
        &self,
        identity: Identity,
        handle: FunctionHandle,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath> {
        let mut tx = self.database.begin(identity).await?;
        FunctionHandlesModel::new(&mut tx).lookup(handle).await
    }

    async fn create_function_handle(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<FunctionHandle> {
        let mut tx = self.database.begin(identity).await?;
        FunctionHandlesModel::new(&mut tx)
            .get_with_component_path(path)
            .await
    }
}
