use core::sync::atomic::Ordering;
use std::{
    collections::BTreeMap,
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
        LogLevel,
        LogLine,
        LogLines,
        SystemLogMetadata,
    },
    pause::PauseClient,
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        RuntimeInstant,
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
    FunctionReads,
    FunctionRunner,
    FunctionWrites,
};
use futures::{
    channel::mpsc,
    select_biased,
    try_join,
    FutureExt,
    StreamExt,
};
use http::StatusCode;
use isolate::{
    parse_udf_args,
    validate_schedule_args,
    ActionCallbacks,
    ActionOutcome,
    AuthConfig,
    BackendIsolateWorker,
    ConcurrencyLimiter,
    FunctionOutcome,
    FunctionResult,
    HttpActionOutcome,
    HttpActionRequest,
    HttpActionResponsePart,
    HttpActionResponseStreamer,
    HttpActionResult,
    IsolateClient,
    IsolateConfig,
    IsolateHeapStats,
    JsonPackedValue,
    ModuleLoader,
    UdfOutcome,
    ValidatedHttpPath,
    ValidatedUdfPathAndArgs,
};
use keybroker::{
    Identity,
    InstanceSecret,
    KeyBroker,
};
use minitrace::collector::SpanContext;
use model::{
    config::types::ModuleConfig,
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
use sync_types::{
    CanonicalizedModulePath,
    CanonicalizedUdfPath,
    UdfPath,
};
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
};
use value::{
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
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
        HttpActionStatusCode,
    },
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
    ActionError,
    ActionReturn,
    MutationError,
    MutationReturn,
    QueryReturn,
};

mod metrics;

static BUILD_DEPS_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| Duration::from_secs(1200));

/// Wrapper for [IsolateClient]s and [FunctionRunner]s that determines where to
/// route requests.
#[derive(Clone)]
pub struct FunctionRouter<RT: Runtime> {
    function_runner: Arc<dyn FunctionRunner<RT>>,
    query_limiter: Arc<Limiter>,
    mutation_limiter: Arc<Limiter>,
    action_limiter: Arc<Limiter>,

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
        }
    }
}

impl<RT: Runtime> FunctionRouter<RT> {
    #[minitrace::trace]
    pub(crate) async fn execute_query_or_mutation(
        &self,
        tx: Transaction<RT>,
        path_and_args: ValidatedUdfPathAndArgs,
        udf_type: UdfType,
        journal: QueryJournal,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        anyhow::ensure!(udf_type == UdfType::Query || udf_type == UdfType::Mutation);
        // All queries and mutations are run in the isolate environment.
        let timer = function_total_timer(ModuleEnvironment::Isolate, udf_type);
        let (tx, outcome) = self
            .function_runner_execute(tx, path_and_args, udf_type, journal, context, None)
            .await?;
        let tx = tx.context("Missing transaction in response for {udf_type}")?;
        timer.finish();
        Ok((tx, outcome))
    }

    #[minitrace::trace]
    pub(crate) async fn execute_action(
        &self,
        tx: Transaction<RT>,
        path_and_args: ValidatedUdfPathAndArgs,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionOutcome> {
        let (_, outcome) = self
            .function_runner_execute(
                tx,
                path_and_args,
                UdfType::Action,
                QueryJournal::new(),
                context,
                Some(log_line_sender),
            )
            .await?;

        let FunctionOutcome::Action(outcome) = outcome else {
            anyhow::bail!(
                "Calling an action returned an invalid outcome: {:?}",
                outcome
            )
        };
        Ok(outcome)
    }

    // Execute using the function runner. Can be used for v8 udfs other than http
    // actions.
    #[minitrace::trace]
    async fn function_runner_execute(
        &self,
        mut tx: Transaction<RT>,
        path_and_args: ValidatedUdfPathAndArgs,
        udf_type: UdfType,
        journal: QueryJournal,
        context: ExecutionContext,
        log_line_sender: Option<mpsc::UnboundedSender<LogLine>>,
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
            UdfType::HttpAction => anyhow::bail!("Function runner does not support http actions"),
        };

        let request_guard = limiter.acquire_permit_with_timeout(&self.rt).await?;

        let timer = function_run_timer(udf_type);
        let (function_tx, outcome, usage_stats) = self
            .function_runner
            .run_function(
                path_and_args,
                udf_type,
                tx.identity().clone(),
                tx.begin_timestamp(),
                tx.writes().clone().into(),
                journal,
                log_line_sender,
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
            let FunctionWrites {
                updates,
                generated_ids,
            } = function_tx.writes;
            tx.apply_function_runner_tx(
                function_tx.begin_timestamp,
                reads,
                num_intervals,
                user_tx_size,
                system_tx_size,
                updates,
                generated_ids,
                function_tx.rows_read,
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
                anyhow::bail!(ErrorMetadata::overloaded(
                    "TooManyConcurrentRequests",
                    "Too many concurrent requests, backoff and try again.",
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

impl<RT: Runtime> HeapSize for ApplicationFunctionRunner<RT> {
    fn heap_size(&self) -> usize {
        self.cache_manager.heap_size()
    }
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
            module_cache.clone(),
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

    pub fn database_heap_size(&self) -> IsolateHeapStats {
        self.analyze_isolate.aggregate_heap_stats()
    }

    pub fn http_actions_heap_size(&self) -> IsolateHeapStats {
        self.http_actions.aggregate_heap_stats()
    }

    // Only used for running queries from REPLs.
    pub async fn run_query_without_caching(
        &self,
        request_id: RequestId,
        mut tx: Transaction<RT>,
        udf_path: CanonicalizedUdfPath,
        arguments: ConvexArray,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
    ) -> anyhow::Result<UdfOutcome> {
        if !(tx.identity().is_admin() || tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("query_without_caching"));
        }

        let identity = tx.inert_identity();
        let start = self.runtime.monotonic_now();
        let validate_result = ValidatedUdfPathAndArgs::new(
            allowed_visibility,
            &mut tx,
            udf_path.clone(),
            arguments.clone(),
            UdfType::Query,
            self.module_cache.clone(),
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
                    udf_path.clone(),
                    arguments.clone(),
                    identity.clone(),
                    self.runtime.clone(),
                    None,
                );
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
        udf_path: UdfPath,
        arguments: Vec<JsonValue>,
        identity: Identity,
        mutation_identifier: Option<SessionRequestIdentifier>,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        pause_client: PauseClient,
        block_logging: bool,
    ) -> anyhow::Result<Result<MutationReturn, MutationError>> {
        let timer = mutation_timer();
        let result = self
            ._retry_mutation(
                request_id,
                udf_path,
                arguments,
                identity,
                mutation_identifier,
                allowed_visibility,
                caller,
                pause_client,
                block_logging,
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
        udf_path: UdfPath,
        arguments: Vec<JsonValue>,
        identity: Identity,
        mutation_identifier: Option<SessionRequestIdentifier>,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        mut pause_client: PauseClient,
        block_logging: bool,
    ) -> anyhow::Result<Result<MutationReturn, MutationError>> {
        if udf_path.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("mutation"));
        }
        let arguments = match parse_udf_args(&udf_path, arguments) {
            Ok(arguments) => arguments,
            Err(error) => {
                return Ok(Err(MutationError {
                    error: RedactedJsError::from_js_error(error, block_logging, request_id),
                    log_lines: RedactedLogLines::empty(),
                }))
            },
        };
        let udf_path = udf_path.canonicalize();
        let udf_path_string = (!udf_path.is_system()).then_some(udf_path.to_string());

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
                .check_mutation_status(&mut tx, &mutation_identifier, block_logging)
                .await?
            {
                return Ok(result);
            }

            let result: Result<(Transaction<RT>, UdfOutcome), anyhow::Error> = self
                .run_mutation_no_udf_log(
                    tx,
                    udf_path.clone(),
                    arguments.clone(),
                    allowed_visibility.clone(),
                    context.clone(),
                )
                .await;
            let (mut tx, mut outcome) = match result {
                Ok(r) => r,
                Err(e) => {
                    self.function_log.log_mutation_system_error(
                        &e,
                        udf_path,
                        arguments,
                        identity,
                        start,
                        caller,
                        context.clone(),
                    );
                    return Err(e);
                },
            };

            // Save a CommittedMutation object so we won't rerun this mutation if
            // successful.
            self.write_mutation_status(&mut tx, &mutation_identifier, &outcome)
                .await?;

            let stats = tx.take_stats();
            let execution_time = start.elapsed();
            let log_lines =
                RedactedLogLines::from_log_lines(outcome.log_lines.clone(), block_logging);
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
                        error: RedactedJsError::from_js_error(
                            error.to_owned(),
                            block_logging,
                            context.request_id,
                        ),
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
                            error: RedactedJsError::from_js_error(
                                js_error,
                                block_logging,
                                context.request_id.clone(),
                            ),
                            log_lines,
                        })
                    } else {
                        if e.is_occ()
                            && (backoff.failures() as usize) < *UDF_EXECUTOR_OCC_MAX_RETRIES
                        {
                            let sleep = self.runtime.with_rng(|rng| backoff.fail(rng));
                            tracing::warn!(
                                "Optimistic concurrency control failed ({e}), retrying \
                                 {udf_path:?} after {sleep:?}",
                            );
                            self.runtime.wait(sleep).await;
                            continue;
                        }
                        outcome.result = Err(JsError::from_error_ref(&e));

                        self.function_log.log_mutation_occ_error(
                            outcome,
                            stats,
                            execution_time,
                            caller,
                            context.clone(),
                        );
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
        udf_path: CanonicalizedUdfPath,
        arguments: ConvexArray,
        allowed_visibility: AllowedVisibility,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, UdfOutcome)> {
        let result = self
            .run_mutation_inner(tx, udf_path, arguments, allowed_visibility, context)
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
        udf_path: CanonicalizedUdfPath,
        arguments: ConvexArray,
        allowed_visibility: AllowedVisibility,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, UdfOutcome)> {
        if udf_path.is_system() && !(tx.identity().is_admin() || tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("mutation"));
        }
        let identity = tx.inert_identity();
        let validate_result = ValidatedUdfPathAndArgs::new(
            allowed_visibility,
            &mut tx,
            udf_path.clone(),
            arguments.clone(),
            UdfType::Mutation,
            self.module_cache.clone(),
        )
        .await?;
        let (tx, outcome) = match validate_result {
            Ok(path_and_args) => {
                self.isolate_functions
                    .execute_query_or_mutation(
                        tx,
                        path_and_args,
                        UdfType::Mutation,
                        QueryJournal::new(),
                        context,
                    )
                    .await?
            },
            Err(js_err) => {
                let mutation_outcome = UdfOutcome::from_error(
                    js_err,
                    udf_path.clone(),
                    arguments.clone(),
                    identity.clone(),
                    self.runtime.clone(),
                    None,
                );
                (tx, FunctionOutcome::Mutation(mutation_outcome))
            },
        };
        let mutation_outcome = match outcome {
            FunctionOutcome::Mutation(o) => o,
            _ => anyhow::bail!("Received non-mutation outcome for mutation"),
        };
        Ok((tx, mutation_outcome))
    }

    #[minitrace::trace]
    pub async fn run_action(
        &self,
        request_id: RequestId,
        name: UdfPath,
        arguments: Vec<JsonValue>,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        block_logging: bool,
    ) -> anyhow::Result<Result<ActionReturn, ActionError>> {
        if name.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("action"));
        }
        let arguments = match parse_udf_args(&name, arguments) {
            Ok(arguments) => arguments,
            Err(error) => {
                return Ok(Err(ActionError {
                    error: RedactedJsError::from_js_error(error, block_logging, request_id),
                    log_lines: RedactedLogLines::empty(),
                }))
            },
        };
        let context = ExecutionContext::new(request_id.clone(), &caller);
        let name = name.canonicalize();
        let usage_tracking = FunctionUsageTracker::new();
        let start = self.runtime.monotonic_now();
        let completion_result = self
            .run_action_no_udf_log(
                name.clone(),
                arguments.clone(),
                identity.clone(),
                allowed_visibility,
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
                    name,
                    arguments,
                    identity.into(),
                    start,
                    caller,
                    vec![].into(),
                    context,
                );
                anyhow::bail!(e)
            },
        };
        let log_lines =
            RedactedLogLines::from_log_lines(completion.log_lines().clone(), block_logging);
        let result = completion.outcome.result.clone();
        self.function_log.log_action(completion, usage_tracking);

        let value = match result {
            Ok(ref value) => value.unpack(),
            // If it's an error inside the UDF, log the failed execution and return the
            // developer error.
            Err(error) => {
                return Ok(Err(ActionError {
                    error: RedactedJsError::from_js_error(error, block_logging, request_id),
                    log_lines,
                }))
            },
        };

        Ok(Ok(ActionReturn { value, log_lines }))
    }

    /// Runs the actions without logging to the UDF log. It is the caller
    /// responsibility to log to the UDF log.
    #[minitrace::trace]
    pub async fn run_action_no_udf_log(
        &self,
        name: CanonicalizedUdfPath,
        arguments: ConvexArray,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        usage_tracking: FunctionUsageTracker,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionCompletion> {
        let result = self
            .run_action_inner(
                name,
                arguments,
                identity,
                allowed_visibility,
                caller,
                usage_tracking,
                context,
            )
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
        name: CanonicalizedUdfPath,
        arguments: ConvexArray,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        usage_tracking: FunctionUsageTracker,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionCompletion> {
        if name.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("action"));
        }
        let unix_timestamp = self.runtime.unix_timestamp();
        let start = self.runtime.monotonic_now();
        let mut tx = self
            .database
            .begin_with_usage(identity.clone(), usage_tracking)
            .await?;
        let validate_result = ValidatedUdfPathAndArgs::new(
            allowed_visibility,
            &mut tx,
            name.clone(),
            arguments.clone(),
            UdfType::Action,
            self.module_cache.clone(),
        )
        .await?;
        let path_and_args = match validate_result {
            Ok(path_and_args) => path_and_args,
            Err(js_error) => {
                return Ok(ActionCompletion {
                    outcome: ActionOutcome::from_error(
                        js_error,
                        name,
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
        let udf_server_version = path_and_args.npm_version().clone();
        // We should not be missing the module given we validated the path above
        // which requires the module to exist.
        let module = ModuleModel::new(&mut tx)
            .get_metadata(name.module().clone())
            .await?
            .context("Missing a valid module")?;
        let (log_line_sender, log_line_receiver) = mpsc::unbounded();

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
                            name.clone(),
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
                timer.finish();
                outcome_result.map(|outcome| ActionCompletion {
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
                let module_version = self
                    .module_cache
                    .get_module(&mut tx, name.module().clone())
                    .await?
                    .context("Missing a valid module_version")?;
                let _request_guard = self
                    .node_action_limiter
                    .acquire_permit_with_timeout(&self.runtime)
                    .await?;
                let mut source_maps = BTreeMap::new();
                if let Some(source_map) = module_version.source_map.clone() {
                    source_maps.insert(name.module().clone(), source_map);
                }

                let source_package_id = module.source_package_id.ok_or_else(|| {
                    anyhow::anyhow!("Source package is required to execute actions")
                })?;
                let source_package = SourcePackageModel::new(&mut tx)
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
                        try_join!(source_uri_future, external_uri_future)?;
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
                    callback_token: self.key_broker.issue_action_token(),
                    context: context.clone(),
                    encoded_parent_trace: SpanContext::current_local_parent()
                        .map(|ctx| ctx.encode_w3c_traceparent()),
                };

                let node_outcome_future = self
                    .node_actions
                    .execute(request, &source_maps, log_line_sender)
                    .boxed();
                let (node_outcome_result, log_lines) = run_function_and_collect_log_lines(
                    node_outcome_future,
                    log_line_receiver,
                    |log_line| {
                        self.function_log.log_action_progress(
                            name.clone(),
                            unix_timestamp,
                            context.clone(),
                            vec![log_line].into(),
                            module.environment,
                        )
                    },
                )
                .await;
                timer.finish();
                node_outcome_result.map(|node_outcome| {
                    let outcome = ActionOutcome {
                        udf_path: name.clone(),
                        arguments: arguments.clone(),
                        identity: tx.inert_identity(),
                        unix_timestamp,
                        result: node_outcome.result.map(JsonPackedValue::pack),
                        syscall_trace: node_outcome.syscall_trace,
                        udf_server_version,
                    };
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
                let outcome = ActionOutcome::from_error(
                    JsError::from_error(e),
                    name,
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
    pub async fn run_http_action(
        &self,
        request_id: RequestId,
        name: UdfPath,
        http_request: HttpActionRequest,
        mut response_streamer: HttpActionResponseStreamer,
        identity: Identity,
        caller: FunctionCaller,
        action_callbacks: Arc<dyn ActionCallbacks>,
    ) -> anyhow::Result<isolate::HttpActionResult> {
        let start = self.runtime.monotonic_now();
        let usage_tracker = FunctionUsageTracker::new();
        let mut tx = self
            .database
            .begin_with_usage(identity.clone(), usage_tracker.clone())
            .await?;

        // Before any developer-visible logging takes place, bail out if it's
        // clear the application has no HTTP actions routed.
        // This should spares developer not using HTTP from the deluge of
        // logspam and other bot traffic.
        if !self.module_cache.has_http(&mut tx).await? {
            drop(tx);
            let response_parts = isolate::HttpActionResponsePart::from_text(
                StatusCode::NOT_FOUND,
                "This Convex deployment does not have HTTP actions enabled.".into(),
            );
            for part in response_parts {
                response_streamer.send_part(part)?;
            }
            return Ok(isolate::HttpActionResult::Streamed);
        }
        let validated_path = match ValidatedHttpPath::new(
            &mut tx,
            name.canonicalize().clone(),
            self.module_cache.as_ref(),
        )
        .await?
        {
            Ok(validated_path) => validated_path,
            Err(e) => return Ok(isolate::HttpActionResult::Error(e)),
        };
        let unix_timestamp = self.runtime.unix_timestamp();
        let context = ExecutionContext::new(request_id, &caller);

        let request_head = http_request.head.clone();
        let route = http_request.head.route_for_failure();
        let (log_line_sender, log_line_receiver) = mpsc::unbounded();
        // We want to intercept the response head so we can log it on function
        // completion, but still stream the response as it comes in, so we
        // create another channel here.
        let (isolate_response_sender, mut isolate_response_receiver) = mpsc::unbounded();
        let outcome_future = self
            .http_actions
            .execute_http_action(
                validated_path,
                http_request,
                identity.clone(),
                action_callbacks,
                self.fetch_client.clone(),
                log_line_sender,
                HttpActionResponseStreamer::new(isolate_response_sender),
                tx,
                context.clone(),
            )
            .boxed();

        let context_ = context.clone();
        let mut outcome_and_log_lines_fut = Box::pin(
            run_function_and_collect_log_lines(outcome_future, log_line_receiver, |log_line| {
                self.function_log.log_http_action_progress(
                    route.clone(),
                    unix_timestamp,
                    context_.clone(),
                    vec![log_line].into(),
                    // http actions are always run in Isolate
                    ModuleEnvironment::Isolate,
                )
            })
            .fuse(),
        );

        let mut result_for_logging = None;
        let (outcome_result, mut log_lines): (anyhow::Result<HttpActionOutcome>, LogLines) = loop {
            select_biased! {
                result = isolate_response_receiver.select_next_some() => {
                    match result {
                        HttpActionResponsePart::Head(h) => {
                            result_for_logging = Some(Ok(HttpActionStatusCode(h.status)));
                            response_streamer.send_part(HttpActionResponsePart::Head(h))?;
                        },
                        HttpActionResponsePart::BodyChunk(bytes) => {
                            response_streamer.send_part(HttpActionResponsePart::BodyChunk(bytes))?;
                        }
                    }
                },
                outcome_and_log_lines = outcome_and_log_lines_fut => {
                    break outcome_and_log_lines
                }
            }
        };

        while let Some(part) = isolate_response_receiver.next().await {
            match part {
                HttpActionResponsePart::Head(h) => {
                    result_for_logging = Some(Ok(HttpActionStatusCode(h.status)));
                    response_streamer.send_part(HttpActionResponsePart::Head(h))?;
                },
                HttpActionResponsePart::BodyChunk(bytes) => {
                    response_streamer.send_part(HttpActionResponsePart::BodyChunk(bytes))?;
                },
            }
        }
        match outcome_result {
            Ok(outcome) => {
                let result = outcome.result.clone();
                let result_for_logging = match &result {
                    HttpActionResult::Error(e) => Err(e.clone()),
                    HttpActionResult::Streamed => result_for_logging.ok_or_else(|| {
                        anyhow::anyhow!(
                            "Result should be populated for successfully completed HTTP action"
                        )
                    })?,
                };
                self.function_log.log_http_action(
                    outcome,
                    result_for_logging,
                    log_lines,
                    start.elapsed(),
                    caller,
                    usage_tracker,
                    context,
                );
                Ok(result)
            },
            Err(e) if e.is_deterministic_user_error() => {
                let js_err = JsError::from_error(e);
                match result_for_logging {
                    Some(r) => {
                        let outcome = HttpActionOutcome::new(
                            None,
                            request_head,
                            identity.into(),
                            unix_timestamp,
                            HttpActionResult::Streamed,
                            None,
                            None,
                        );
                        log_lines.push(LogLine::new_system_log_line(
                            LogLevel::Warn,
                            vec![js_err.to_string()],
                            outcome.unix_timestamp,
                            SystemLogMetadata {
                                code: "error:httpAction".to_string(),
                            },
                        ));
                        self.function_log.log_http_action(
                            outcome.clone(),
                            r,
                            log_lines,
                            start.elapsed(),
                            caller,
                            usage_tracker,
                            context,
                        );
                        Ok(HttpActionResult::Streamed)
                    },
                    None => {
                        let result = isolate::HttpActionResult::Error(js_err.clone());
                        let outcome = HttpActionOutcome::new(
                            None,
                            request_head,
                            identity.into(),
                            unix_timestamp,
                            result.clone(),
                            None,
                            None,
                        );
                        self.function_log.log_http_action(
                            outcome.clone(),
                            Err(js_err),
                            log_lines,
                            start.elapsed(),
                            caller,
                            usage_tracker,
                            context,
                        );
                        Ok(result)
                    },
                }
            },
            Err(e) => {
                self.function_log.log_http_action_system_error(
                    &e,
                    request_head,
                    identity.into(),
                    start,
                    caller,
                    log_lines,
                    context,
                );
                Err(e)
            },
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
    pub async fn analyze(
        &self,
        udf_config: UdfConfig,
        new_modules: Vec<ModuleConfig>,
        source_package: Option<SourcePackage>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        // We use the latest environment variables at the time of the deployment
        // this is not transactional with the rest of the deploy.
        let mut tx = self.database.begin(Identity::system()).await?;
        let mut environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
        // Insert special environment variables if not already provided by user
        environment_variables.extend(self.system_env_vars.clone());

        let (node_modules, isolate_modules) = new_modules
            .into_iter()
            .map(|module| (module.path.clone().canonicalize(), module))
            .partition(|(_, config)| config.environment == ModuleEnvironment::Node);

        let mut result = BTreeMap::new();
        match self
            .analyze_isolate
            .analyze(udf_config, isolate_modules, environment_variables.clone())
            .await?
        {
            Ok(modules) => result.extend(modules),
            Err(e) => return Ok(Err(e)),
        }

        if !node_modules.is_empty() {
            for path_str in ["schema.js", "crons.js", "http.js"] {
                let path: CanonicalizedModulePath = path_str
                    .parse()
                    .expect("Failed to parse static module names");
                // The cli should not do this. Log as system error.
                anyhow::ensure!(
                    !node_modules.contains_key(&path),
                    "{path_str} can't be analyzed in Node.js!"
                );
            }
            let source_maps = node_modules
                .into_iter()
                .filter_map(|(path, module)| module.source_map.map(move |m| (path, m)))
                .collect();
            let source_package = source_package.ok_or_else(|| {
                anyhow::anyhow!("Source package is required to analyze action modules")
            })?;

            // Fetch source and external_deps presigned URI first
            let source_uri_future = self
                .modules_storage
                .signed_url(source_package.storage_key.clone(), Duration::from_secs(60));
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
                        try_join!(source_uri_future, external_uri_future)?;
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
            match self.node_actions.analyze(request, &source_maps).await? {
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
        }
        self.validate_cron_jobs(&result).await??;
        Ok(Ok(result))
    }

    #[minitrace::trace]
    async fn validate_cron_jobs(
        &self,
        modules: &BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    ) -> anyhow::Result<Result<(), JsError>> {
        // Validate that every cron job schedules an action or mutation.
        for module in modules.values() {
            let Some(crons) = module.cron_specs.as_ref() else {
                continue;
            };
            for (identifier, cron_spec) in crons {
                let Some(scheduled_module) = modules.get(cron_spec.udf_path.module()) else {
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
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        ts: Timestamp,
        journal: Option<QueryJournal>,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        block_logging: bool,
    ) -> anyhow::Result<QueryReturn> {
        let result = self
            .run_query_at_ts_inner(
                request_id,
                name,
                args,
                identity,
                ts,
                journal,
                allowed_visibility,
                caller,
                block_logging,
            )
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
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        ts: Timestamp,
        journal: Option<QueryJournal>,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        block_logging: bool,
    ) -> anyhow::Result<QueryReturn> {
        if name.is_system() && !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("query"));
        }
        let args = match parse_udf_args(&name, args) {
            Ok(arguments) => arguments,
            Err(js_error) => {
                return Ok(QueryReturn {
                    result: Err(RedactedJsError::from_js_error(
                        js_error,
                        block_logging,
                        request_id,
                    )),
                    log_lines: RedactedLogLines::empty(),
                    token: Token::empty(ts),
                    ts,
                    journal: QueryJournal::new(),
                });
            },
        };
        let canonicalized_name = name.canonicalize();
        let usage_tracker = FunctionUsageTracker::new();
        let result = self
            .cache_manager
            .get(
                request_id,
                canonicalized_name,
                args,
                identity.clone(),
                ts,
                journal,
                allowed_visibility,
                caller,
                block_logging,
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
        block_logging: bool,
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
                let log_lines = RedactedLogLines::from_log_lines(log_lines, block_logging);
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
        outcome: &UdfOutcome,
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
}

#[async_trait]
impl<RT: Runtime> ActionCallbacks for ApplicationFunctionRunner<RT> {
    #[minitrace::trace]
    async fn execute_query(
        &self,
        identity: Identity,
        name: UdfPath,
        args: Vec<JsonValue>,
        block_logging: bool,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let ts = self.database.now_ts_for_reads();
        let result = self
            .run_query_at_ts(
                context.request_id,
                name,
                args,
                identity,
                *ts,
                None,
                AllowedVisibility::All,
                FunctionCaller::Action {
                    parent_scheduled_job: context.parent_scheduled_job,
                },
                block_logging,
            )
            .await
            .map(|r| r.result.map_err(|e| e.pretend_to_unredact()))?;
        Ok(FunctionResult { result })
    }

    #[minitrace::trace]
    async fn execute_mutation(
        &self,
        identity: Identity,
        name: UdfPath,
        args: Vec<JsonValue>,
        block_logging: bool,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let result = self
            .retry_mutation(
                context.request_id,
                name,
                args,
                identity,
                None,
                AllowedVisibility::All,
                FunctionCaller::Action {
                    parent_scheduled_job: context.parent_scheduled_job,
                },
                PauseClient::new(),
                block_logging,
            )
            .await
            .map(|r| match r {
                Ok(mutation_return) => Ok(mutation_return.value),
                Err(mutation_error) => Err(mutation_error.error.pretend_to_unredact()),
            })?;
        Ok(FunctionResult { result })
    }

    #[minitrace::trace]
    async fn execute_action(
        &self,
        identity: Identity,
        name: UdfPath,
        args: Vec<JsonValue>,
        block_logging: bool,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult> {
        let _tx = self.database.begin(identity.clone()).await?;
        let result = self
            .run_action(
                context.request_id,
                name,
                args,
                identity,
                AllowedVisibility::All,
                FunctionCaller::Action {
                    parent_scheduled_job: context.parent_scheduled_job,
                },
                block_logging,
            )
            .await
            .map(|r| match r {
                Ok(action_return) => Ok(action_return.value),
                Err(action_error) => Err(action_error.error.pretend_to_unredact()),
            })?;
        Ok(FunctionResult { result })
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
        self.database
            .commit_with_write_source(tx, "app_funrun_storage_store_file_entry")
            .await?;
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
        self.database
            .commit_with_write_source(tx, "app_funrun_storage_delete")
            .await?;
        Ok(())
    }

    async fn schedule_job(
        &self,
        identity: Identity,
        udf_path: UdfPath,
        udf_args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
        context: ExecutionContext,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let mut tx = self.database.begin(identity).await?;
        let (udf_path, udf_args) = validate_schedule_args(
            udf_path,
            udf_args,
            scheduled_ts,
            // Scheduling from actions is not transaction and happens at latest
            // timestamp.
            self.database.runtime().unix_timestamp(),
            &mut tx,
        )
        .await?;

        let virtual_id = VirtualSchedulerModel::new(&mut tx)
            .schedule(udf_path, udf_args, scheduled_ts, context)
            .await?;
        self.database
            .commit_with_write_source(tx, "app_funrun_schedule_job")
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
                |tx| async { VirtualSchedulerModel::new(tx).cancel(virtual_id).await }.into(),
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
}
