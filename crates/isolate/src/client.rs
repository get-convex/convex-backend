use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap,
        VecDeque,
    },
    sync::{
        Arc,
        Once,
    },
    time::Duration,
};

use ::metrics::Timer;
use async_trait::async_trait;
use common::{
    auth::AuthConfig,
    bootstrap_model::components::{
        definition::ComponentDefinitionMetadata,
        handles::FunctionHandle,
    },
    codel_queue::{
        new_codel_queue_async,
        CoDelQueueReceiver,
        CoDelQueueSender,
        ExpiredInQueue,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentDefinitionPath,
        ComponentId,
        ComponentName,
        ComponentPath,
        Resource,
    },
    errors::{
        recapture_stacktrace,
        JsError,
    },
    execution_context::ExecutionContext,
    fastrace_helpers::{
        initialize_root_from_parent,
        EncodedSpan,
    },
    http::{
        fetch::FetchClient,
        RoutedHttpPath,
    },
    knobs::{
        FUNRUN_ISOLATE_ACTIVE_THREADS,
        HEAP_WORKER_REPORT_INTERVAL_SECONDS,
        ISOLATE_IDLE_TIMEOUT,
        ISOLATE_MAX_LIFETIME,
        ISOLATE_QUEUE_SIZE,
        REUSE_ISOLATES,
        V8_THREADS,
    },
    log_lines::LogLine,
    query_journal::QueryJournal,
    runtime::{
        shutdown_and_join,
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    static_span,
    types::{
        ModuleEnvironment,
        UdfType,
    },
    utils::ensure_utc,
};
use database::{
    shutdown_error,
    Transaction,
};
use deno_core::{
    v8,
    v8::V8,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use fastrace::{
    func_path,
    future::FutureExt as _,
};
use file_storage::TransactionalFileStorage;
use futures::{
    select,
    select_biased,
    stream::{
        FuturesUnordered,
        StreamExt,
    },
    FutureExt,
};
use keybroker::{
    Identity,
    KeyBroker,
};
use model::{
    config::{
        module_loader::ModuleLoader,
        types::ModuleConfig,
    },
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    file_storage::{
        types::FileStorageEntry,
        FileStorageId,
    },
    modules::module_versions::{
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    udf_config::types::UdfConfig,
};
use parking_lot::Mutex;
use prometheus::VMHistogram;
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedModulePath;
use tokio::sync::{
    mpsc,
    oneshot,
};
use udf::{
    validation::{
        ValidatedHttpPath,
        ValidatedPathAndArgs,
    },
    ActionOutcome,
    EvaluateAppDefinitionsResult,
    FunctionOutcome,
    FunctionResult,
    HttpActionOutcome,
    HttpActionResponseStreamer,
};
use usage_tracking::FunctionUsageStats;
use value::{
    id_v6::DeveloperDocumentId,
    identifier::Identifier,
};
use vector::PublicVectorSearchQueryResult;

use crate::{
    concurrency_limiter::ConcurrencyLimiter,
    isolate::{
        Isolate,
        IsolateHeapStats,
    },
    isolate_worker::FunctionRunnerIsolateWorker,
    metrics::{
        self,
        log_aggregated_heap_stats,
        log_pool_max,
        log_pool_running_count,
        log_worker_stolen,
        queue_timer,
    },
};

// We gather prometheus stats every 30 seconds, so we should make sure we log
// active permits more frequently than that.
const ACTIVE_CONCURRENCY_PERMITS_LOG_FREQUENCY: Duration = Duration::from_secs(10);

pub const PAUSE_RECREATE_CLIENT: &str = "recreate_client";
pub const PAUSE_REQUEST: &str = "pause_request";
pub const NO_AVAILABLE_WORKERS: &str = "There are no available workers to process the request";

#[derive(Clone)]
pub struct IsolateConfig {
    // Name of isolate pool, used in metrics.
    pub name: &'static str,

    // Typically, the user timeout is configured based on environment. This
    // allows us to set an upper bound to it that we use for tests.
    max_user_timeout: Option<Duration>,

    limiter: ConcurrencyLimiter,
}

impl IsolateConfig {
    pub fn new(name: &'static str, limiter: ConcurrencyLimiter) -> Self {
        Self {
            name,
            max_user_timeout: None,
            limiter,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_with_max_user_timeout(
        name: &'static str,
        max_user_timeout: Option<Duration>,
        limiter: ConcurrencyLimiter,
    ) -> Self {
        Self {
            name,
            max_user_timeout,
            limiter,
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for IsolateConfig {
    fn default() -> Self {
        Self {
            name: "test",
            max_user_timeout: None,
            limiter: ConcurrencyLimiter::unlimited(),
        }
    }
}

#[async_trait]
pub trait ActionCallbacks: Send + Sync {
    // Executing UDFs
    async fn execute_query(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult>;

    async fn execute_mutation(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult>;

    async fn execute_action(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult>;

    // Storage
    async fn storage_get_url(
        &self,
        identity: Identity,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>>;

    async fn storage_delete(
        &self,
        identity: Identity,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<()>;

    // Used to get a file content from an action running in v8.
    async fn storage_get_file_entry(
        &self,
        identity: Identity,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<(ComponentPath, FileStorageEntry)>>;

    // Used to store an already uploaded file from an action running in v8.
    async fn storage_store_file_entry(
        &self,
        identity: Identity,
        component: ComponentId,
        entry: FileStorageEntry,
    ) -> anyhow::Result<(ComponentPath, DeveloperDocumentId)>;

    // Scheduler
    async fn schedule_job(
        &self,
        identity: Identity,
        scheduling_component: ComponentId,
        scheduled_path: CanonicalizedComponentFunctionPath,
        udf_args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
        context: ExecutionContext,
    ) -> anyhow::Result<DeveloperDocumentId>;

    async fn cancel_job(
        &self,
        identity: Identity,
        virtual_id: DeveloperDocumentId,
    ) -> anyhow::Result<()>;

    // Vector Search
    async fn vector_search(
        &self,
        identity: Identity,
        query: JsonValue,
    ) -> anyhow::Result<(Vec<PublicVectorSearchQueryResult>, FunctionUsageStats)>;

    // Components
    async fn lookup_function_handle(
        &self,
        identity: Identity,
        handle: FunctionHandle,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath>;
    async fn create_function_handle(
        &self,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<FunctionHandle>;
}

pub struct UdfRequest<RT: Runtime> {
    pub path_and_args: ValidatedPathAndArgs,
    pub udf_type: UdfType,
    pub transaction: Transaction<RT>,
    pub journal: QueryJournal,
    pub context: ExecutionContext,
}

pub struct HttpActionRequest<RT: Runtime> {
    pub http_module_path: ValidatedHttpPath,
    pub routed_path: RoutedHttpPath,
    pub http_request: udf::HttpActionRequest,
    pub transaction: Transaction<RT>,
    pub identity: Identity,
    pub context: ExecutionContext,
}

pub struct ActionRequest<RT: Runtime> {
    pub params: ActionRequestParams,
    pub transaction: Transaction<RT>,
    pub identity: Identity,
    pub context: ExecutionContext,
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ActionRequestParams {
    pub path_and_args: ValidatedPathAndArgs,
}

#[derive(Clone)]
pub struct EnvironmentData<RT: Runtime> {
    pub key_broker: KeyBroker,
    pub default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    pub file_storage: TransactionalFileStorage<RT>,
    pub module_loader: Arc<dyn ModuleLoader<RT>>,
}

pub struct Request<RT: Runtime> {
    pub client_id: String,
    pub inner: RequestType<RT>,
    pub parent_trace: EncodedSpan,
}

impl<RT: Runtime> Request<RT> {
    pub fn new(client_id: String, inner: RequestType<RT>, parent_trace: EncodedSpan) -> Self {
        Self {
            client_id,
            inner,
            parent_trace,
        }
    }
}

pub enum RequestType<RT: Runtime> {
    Udf {
        request: UdfRequest<RT>,
        environment_data: EnvironmentData<RT>,
        response: oneshot::Sender<anyhow::Result<(Transaction<RT>, FunctionOutcome)>>,
        queue_timer: Timer<VMHistogram>,
        reactor_depth: usize,
        udf_callback: Box<dyn UdfCallback<RT>>,
        function_started_sender: Option<oneshot::Sender<()>>,
    },
    Action {
        request: ActionRequest<RT>,
        environment_data: EnvironmentData<RT>,
        response: oneshot::Sender<anyhow::Result<ActionOutcome>>,
        queue_timer: Timer<VMHistogram>,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        function_started_sender: Option<oneshot::Sender<()>>,
    },
    HttpAction {
        request: HttpActionRequest<RT>,
        environment_data: EnvironmentData<RT>,
        response: oneshot::Sender<anyhow::Result<HttpActionOutcome>>,
        queue_timer: Timer<VMHistogram>,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        http_response_streamer: HttpActionResponseStreamer,
        function_started_sender: Option<oneshot::Sender<()>>,
    },
    Analyze {
        udf_config: UdfConfig,
        modules: BTreeMap<CanonicalizedModulePath, ModuleConfig>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        response: oneshot::Sender<
            anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>>,
        >,
    },
    EvaluateSchema {
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
        response: oneshot::Sender<anyhow::Result<DatabaseSchema>>,
    },
    EvaluateAuthConfig {
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        response: oneshot::Sender<anyhow::Result<AuthConfig>>,
    },
    EvaluateAppDefinitions {
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        response: oneshot::Sender<anyhow::Result<EvaluateAppDefinitionsResult>>,
    },
    EvaluateComponentInitializer {
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        path: ComponentDefinitionPath,
        definition: ModuleConfig,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
        response: oneshot::Sender<anyhow::Result<BTreeMap<Identifier, Resource>>>,
    },
}

#[async_trait]
pub trait UdfCallback<RT: Runtime>: Send + Sync {
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
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)>;
}

impl<RT: Runtime> Request<RT> {
    fn expire(self, error: ExpiredInQueue) {
        let error = anyhow::anyhow!(error).context(ErrorMetadata::overloaded(
            "ExpiredInQueue",
            "Too many concurrent requests, backoff and try again.",
        ));
        match self.inner {
            RequestType::Udf { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::Action { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::HttpAction { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::Analyze { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateSchema { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateAuthConfig { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateAppDefinitions { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateComponentInitializer { response, .. } => {
                let _ = response.send(Err(error));
            },
        }
    }

    fn reject(self) {
        let error =
            ErrorMetadata::rejected_before_execution("WorkerOverloaded", NO_AVAILABLE_WORKERS)
                .into();
        match self.inner {
            RequestType::Udf { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::Action { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::HttpAction { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::Analyze { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateSchema { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateAuthConfig { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateAppDefinitions { response, .. } => {
                let _ = response.send(Err(error));
            },
            RequestType::EvaluateComponentInitializer { response, .. } => {
                let _ = response.send(Err(error));
            },
        }
    }
}

impl<RT: Runtime> Clone for IsolateClient<RT> {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt.clone(),
            handles: self.handles.clone(),
            scheduler: self.scheduler.clone(),
            sender: self.sender.clone(),
            concurrency_logger: self.concurrency_logger.clone(),
        }
    }
}

pub fn initialize_v8() {
    ensure_utc().expect("Failed to setup timezone");
    static V8_INIT: Once = Once::new();
    V8_INIT.call_once(|| {
        let _s = static_span!("initialize_v8");

        // `deno_core_icudata` internally loads this with proper 16-byte alignment.
        assert!(v8::icu::set_common_data_74(deno_core_icudata::ICU_DATA).is_ok());

        // Calls into `v8::platform::v8__Platform__NewUnprotectedDefaultPlatform`
        // Can configure with...
        // - thread_pool_size (default: zero): number of worker threads for background
        //   jobs, picks a reasonable default based on number of cores if set to zero
        // - idle_task_support (default: false): platform will except idle tasks and
        //   will rely on embedder calling `v8::platform::RunIdleTasks`. Idle tasks are
        //   low-priority tasks that are run with a deadline indicating how long the
        //   scheduler expects to be idle (e.g. unused remainder of a frame budget)
        // - in_process_stack_dumping (default: false)
        // - tracing_controller (default: null): if null, the platform creates a
        //   `v8::platform::TracingController` instance and uses it
        // Why "unprotected"? The "protected" default platform utilizes Memory
        // Protection Keys (PKU), which requires that all threads utilizing V8 are
        // descendents of the thread that initialized V8. Unfortunately, this is
        // not compatible with how Rust tests run and additionally, the version of V8
        // used at the time of this comment has a bug with PKU on certain Intel CPUs.
        // See https://github.com/denoland/rusty_v8/issues/1381
        let platform = v8::new_unprotected_default_platform(*V8_THREADS, false).make_shared();

        // Calls into `v8::V8::InitializePlatform`, sets global platform.
        V8::initialize_platform(platform);

        // TODO: Figure out what V8 uses entropy for and set it here.
        // V8::set_entropy_source(...);

        // Set V8 command line flags.
        // https://github.com/v8/v8/blob/master/src/flags/flag-definitions.h
        let argv = vec![
            "".to_owned(), // first arg is ignored
            "--harmony-import-assertions".to_owned(),
            // See https://github.com/denoland/deno/issues/2544
            "--no-wasm-async-compilation".to_string(),
            // Disable `eval` or `new Function()`.
            "--disallow-code-generation-from-strings".to_string(),
            // We ensure 4MiB of stack space on all of our threads, so
            // tell V8 it can use up to 2MiB of stack space itself. The
            // default is 1MiB. Note that the flag is in KiB (https://github.com/v8/v8/blob/master/src/flags/flag-definitions.h#L1594).
            "--stack-size=2048".to_string(),
        ];
        // v8 returns the args that were misunderstood
        let misunderstood = V8::set_flags_from_command_line(argv);
        assert_eq!(misunderstood, vec![""]);

        // Calls into `v8::V8::Initialize`
        V8::initialize();
    });
}

/// The V8 code all expects to run on a single thread, which makes it ineligible
/// for Tokio's scheduler, which wants the ability to move work across scheduler
/// threads. Instead, we'll manage our V8 threads ourselves.
///
/// [`IsolateClient`] is the "client" entry point to our V8 threads.
pub struct IsolateClient<RT: Runtime> {
    rt: RT,
    handles: Arc<Mutex<Vec<IsolateWorkerHandle>>>,
    scheduler: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
    sender: CoDelQueueSender<RT, Request<RT>>,
    concurrency_logger: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
}

impl<RT: Runtime> IsolateClient<RT> {
    pub fn new(
        rt: RT,
        max_percent_per_client: usize,
        max_isolate_workers: usize,
        isolate_config: Option<IsolateConfig>,
    ) -> anyhow::Result<Self> {
        let concurrency_limit = if *FUNRUN_ISOLATE_ACTIVE_THREADS > 0 {
            ConcurrencyLimiter::new(*FUNRUN_ISOLATE_ACTIVE_THREADS)
        } else {
            ConcurrencyLimiter::unlimited()
        };
        let concurrency_logger = rt.spawn(
            "concurrency_logger",
            concurrency_limit.go_log(rt.clone(), ACTIVE_CONCURRENCY_PERMITS_LOG_FREQUENCY),
        );
        let isolate_config =
            isolate_config.unwrap_or(IsolateConfig::new("funrun", concurrency_limit));

        initialize_v8();
        // NB: We don't call V8::Dispose or V8::ShutdownPlatform since we just assume a
        // single V8 instance per process and don't need to clean up its
        // resources.
        let (sender, receiver) =
            new_codel_queue_async::<_, Request<_>>(rt.clone(), *ISOLATE_QUEUE_SIZE);
        let handles = Arc::new(Mutex::new(Vec::new()));
        let handles_clone = handles.clone();
        let rt_clone = rt.clone();
        let scheduler = rt.spawn("shared_isolate_scheduler", async move {
            // The scheduler thread pops a worker from available_workers and
            // pops a request from the CoDelQueueReceiver. Then it sends the request
            // to the worker.
            let isolate_worker = FunctionRunnerIsolateWorker::new(rt_clone.clone(), isolate_config);
            let scheduler = SharedIsolateScheduler::new(
                rt_clone,
                isolate_worker,
                max_isolate_workers,
                handles_clone,
                max_percent_per_client,
            );
            scheduler.run(receiver).await
        });
        Ok(Self {
            rt,
            sender,
            scheduler: Arc::new(Mutex::new(Some(scheduler))),
            concurrency_logger: Arc::new(Mutex::new(Some(concurrency_logger))),
            handles,
        })
    }

    pub fn aggregate_heap_stats(&self) -> IsolateHeapStats {
        let mut total = IsolateHeapStats::default();
        for handle in self.handles.lock().iter() {
            total += handle.heap_stats.get();
        }
        total
    }

    #[fastrace::trace]
    pub async fn execute_udf(
        &self,
        udf_type: UdfType,
        path_and_args: ValidatedPathAndArgs,
        transaction: Transaction<RT>,
        journal: QueryJournal,
        context: ExecutionContext,
        environment_data: EnvironmentData<RT>,
        reactor_depth: usize,
        instance_name: String,
        function_started_sender: Option<oneshot::Sender<()>>,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::Udf {
            request: UdfRequest {
                path_and_args,
                udf_type,
                transaction,
                journal,
                context,
            },
            environment_data,
            response: tx,
            queue_timer: queue_timer(),
            reactor_depth,
            udf_callback: Box::new(self.clone()),
            function_started_sender,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        let (tx, outcome) = Self::receive_response(rx).await??;

        Ok((tx, outcome))
    }

    #[fastrace::trace]
    pub async fn execute_action(
        &self,
        path_and_args: ValidatedPathAndArgs,
        transaction: Transaction<RT>,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        context: ExecutionContext,
        environment_data: EnvironmentData<RT>,
        instance_name: String,
        function_started_sender: Option<oneshot::Sender<()>>,
    ) -> anyhow::Result<ActionOutcome> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::Action {
            request: ActionRequest {
                params: ActionRequestParams { path_and_args },
                identity: transaction.identity().clone(),
                transaction,
                context,
            },
            response: tx,
            queue_timer: queue_timer(),
            action_callbacks,
            fetch_client,
            log_line_sender,
            environment_data,
            function_started_sender,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        let outcome = Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })?;

        Ok(outcome)
    }

    /// Execute an HTTP action.
    /// HTTP actions can run other UDFs, so they take in a ActionCallbacks from
    /// the application layer. This creates a transient reference cycle.
    #[fastrace::trace]
    pub async fn execute_http_action(
        &self,
        http_module_path: ValidatedHttpPath,
        routed_path: RoutedHttpPath,
        http_request: udf::HttpActionRequest,
        identity: Identity,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        http_response_streamer: HttpActionResponseStreamer,
        transaction: Transaction<RT>,
        context: ExecutionContext,
        environment_data: EnvironmentData<RT>,
        instance_name: String,
        function_started_sender: Option<oneshot::Sender<()>>,
    ) -> anyhow::Result<HttpActionOutcome> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::HttpAction {
            request: HttpActionRequest {
                http_module_path,
                routed_path,
                http_request,
                identity,
                transaction,
                context,
            },
            environment_data,
            response: tx,
            queue_timer: queue_timer(),
            action_callbacks,
            fetch_client,
            log_line_sender,
            http_response_streamer,
            function_started_sender,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        let outcome = Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })?;

        Ok(outcome)
    }

    /// Analyze a set of user-defined modules.
    #[fastrace::trace]
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
        let (tx, rx) = oneshot::channel();
        let request = RequestType::Analyze {
            modules,
            response: tx,
            udf_config,
            environment_variables,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        IsolateClient::<RT>::receive_response(rx)
            .await?
            .map_err(|e| {
                if e.is_overloaded() {
                    recapture_stacktrace(e)
                } else {
                    e
                }
            })
    }

    #[fastrace::trace]
    pub async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
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
        let (tx, rx) = oneshot::channel();
        let request = RequestType::EvaluateAppDefinitions {
            app_definition,
            component_definitions,
            dependency_graph,
            user_environment_variables,
            system_env_vars,
            response: tx,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        IsolateClient::<RT>::receive_response(rx)
            .await?
            .map_err(|e| {
                if e.is_overloaded() {
                    recapture_stacktrace(e)
                } else {
                    e
                }
            })
    }

    #[fastrace::trace]
    pub async fn evaluate_component_initializer(
        &self,
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        path: ComponentDefinitionPath,
        definition: ModuleConfig,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
        instance_name: String,
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::EvaluateComponentInitializer {
            evaluated_definitions,
            path,
            definition,
            args,
            name,
            response: tx,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        IsolateClient::<RT>::receive_response(rx)
            .await?
            .map_err(|e| {
                if e.is_overloaded() {
                    recapture_stacktrace(e)
                } else {
                    e
                }
            })
    }

    #[fastrace::trace]
    pub async fn evaluate_schema(
        &self,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
        instance_name: String,
    ) -> anyhow::Result<DatabaseSchema> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::EvaluateSchema {
            schema_bundle,
            source_map,
            rng_seed,
            unix_timestamp,
            response: tx,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        IsolateClient::<RT>::receive_response(rx)
            .await?
            .map_err(|e| {
                if e.is_overloaded() {
                    recapture_stacktrace(e)
                } else {
                    e
                }
            })
    }

    #[fastrace::trace]
    pub async fn evaluate_auth_config(
        &self,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        explanation: &str,
        instance_name: String,
    ) -> anyhow::Result<AuthConfig> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::EvaluateAuthConfig {
            auth_config_bundle,
            source_map,
            environment_variables,
            response: tx,
        };
        self.send_request(Request::new(
            instance_name,
            request,
            EncodedSpan::from_parent(),
        ))?;
        let auth_config = IsolateClient::<RT>::receive_response(rx)
            .await?
            .map_err(|e| {
                let err = if e.is_overloaded() {
                    recapture_stacktrace(e)
                } else {
                    e
                };
                let error = err.to_string();
                if error.starts_with("Uncaught Error: Environment variable") {
                    // Reformatting the underlying message to be nicer
                    // here. Since we lost the underlying ErrorMetadata into the JSError,
                    // we do some string matching instead. CX-4531
                    ErrorMetadata::bad_request(
                        "AuthConfigMissingEnvironmentVariable",
                        error.trim_start_matches("Uncaught Error: ").to_string(),
                    )
                } else {
                    ErrorMetadata::bad_request(
                        "InvalidAuthConfig",
                        format!("{explanation}: {error}"),
                    )
                }
            })?;

        Ok(auth_config)
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        {
            let handles: Vec<_> = {
                let mut handles = self.handles.lock();
                for handle in &mut *handles {
                    handle.handle.shutdown();
                }
                handles.drain(..).collect()
            };
            for handle in handles.into_iter() {
                shutdown_and_join(handle.handle).await?;
            }
        }
        if let Some(mut scheduler) = self.scheduler.lock().take() {
            scheduler.shutdown();
        }
        if let Some(mut concurrency_logger) = self.concurrency_logger.lock().take() {
            concurrency_logger.shutdown();
        }

        Ok(())
    }

    fn send_request(&self, request: Request<RT>) -> anyhow::Result<()> {
        self.sender
            .try_send(request)
            .map_err(|_| metrics::execute_full_error())?;
        Ok(())
    }

    async fn receive_response<T>(rx: oneshot::Receiver<T>) -> anyhow::Result<T> {
        // The only reason a oneshot response channel wil be dropped prematurely if the
        // isolate worker is shutting down.
        rx.await.map_err(|_| shutdown_error())
    }
}

#[async_trait]
impl<RT: Runtime> UdfCallback<RT> for IsolateClient<RT> {
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
        self.execute_udf(
            udf_type,
            path_and_args,
            transaction,
            journal,
            context,
            environment_data,
            reactor_depth,
            client_id,
            None, /* function_started_sender */
        )
        .await
    }
}

pub struct SharedIsolateScheduler<RT: Runtime, W: IsolateWorker<RT>> {
    rt: RT,
    worker: W,
    /// Vec of channels for sending work to individual workers.
    worker_senders: Vec<
        mpsc::Sender<(
            Request<RT>,
            oneshot::Sender<ActiveWorkerState>,
            ActiveWorkerState,
        )>,
    >,
    /// Map from client_id to stack of workers (implemented with a deque). The
    /// most recently used worker for a given client is at the front of the
    /// deque. These workers were previously used by this client, but may
    /// safely be "stolen" for use by another client. A worker with a
    /// `last_used_ts` older than `ISOLATE_IDLE_TIMEOUT` has already been
    /// recreated and there will be no penalty for reassigning this worker to a
    /// new client.
    available_workers: HashMap<String, VecDeque<IdleWorkerState>>,
    /// Set of futures awaiting a response from an active worker.
    in_progress_workers: FuturesUnordered<oneshot::Receiver<ActiveWorkerState>>,
    /// Counts the number of active workers per client. Should only contain a
    /// key if the value is greater than 0.
    in_progress_count: HashMap<String, usize>,
    /// The max number of workers this scheduler is permitted to create.
    max_workers: usize,
    handles: Arc<Mutex<Vec<IsolateWorkerHandle>>>,
    max_percent_per_client: usize,
}

struct IdleWorkerState {
    worker_id: usize,
    last_used_ts: tokio::time::Instant,
}
struct ActiveWorkerState {
    worker_id: usize,
    client_id: String,
}

impl<RT: Runtime, W: IsolateWorker<RT>> SharedIsolateScheduler<RT, W> {
    pub fn new(
        rt: RT,
        worker: W,
        max_workers: usize,
        handles: Arc<Mutex<Vec<IsolateWorkerHandle>>>,
        max_percent_per_client: usize,
    ) -> Self {
        Self {
            rt,
            worker,
            worker_senders: Vec::new(),
            in_progress_workers: FuturesUnordered::new(),
            in_progress_count: HashMap::new(),
            available_workers: HashMap::new(),
            max_workers,
            handles,
            max_percent_per_client,
        }
    }

    fn handle_completed_worker(&mut self, completed_worker: ActiveWorkerState) {
        let new_count = match self
            .in_progress_count
            .remove_entry(&completed_worker.client_id)
        {
            Some((client_id, count)) if count > 1 => {
                self.in_progress_count.insert(client_id, count - 1);
                count - 1
            },
            Some((_, 1)) => {
                // Nothing to do; we've already removed the entry above.
                0
            },
            _ => panic!(
                "Inconsistent state in `in_progress_count` map; the count of active workers for \
                 client {} must be >= 1",
                completed_worker.client_id
            ),
        };
        log_pool_running_count(
            self.worker.config().name,
            new_count,
            &completed_worker.client_id,
        );

        self.available_workers
            .entry(completed_worker.client_id)
            .or_default()
            .push_front(IdleWorkerState {
                worker_id: completed_worker.worker_id,
                last_used_ts: self.rt.monotonic_now(),
            });
    }

    pub async fn run(mut self, receiver: CoDelQueueReceiver<RT, Request<RT>>) {
        log_pool_max(self.worker.config().name, self.max_workers);
        let mut receiver = receiver.fuse();
        let mut report_stats = self.rt.wait(*HEAP_WORKER_REPORT_INTERVAL_SECONDS);
        loop {
            select_biased! {
                completed_worker = self.in_progress_workers.select_next_some() => {
                    let Ok(completed_worker): Result<ActiveWorkerState, _> = completed_worker else {
                        tracing::warn!("Worker has shut down uncleanly. Shutting down {} scheduler.", self.worker.config().name);
                        return;
                    };
                    self.handle_completed_worker(completed_worker);
                }
                request = receiver.next() => {
                    let Some((request, expired)) = request else {
                        tracing::warn!("Request sender went away; {} scheduler shutting down", self.worker.config().name);
                        return
                    };
                    if let Some(expired) = expired {
                        request.expire(expired);
                        continue;
                    }
                    let Some(worker_id) = self.get_worker(&request.client_id) else {
                        request.reject();
                        continue;
                    };
                    let (done_sender, done_receiver) = oneshot::channel();
                    self.in_progress_workers.push(done_receiver);
                    let entry = self
                        .in_progress_count
                        .entry(request.client_id.clone())
                        .or_default();
                    *entry += 1;
                    log_pool_running_count(
                        self.worker.config().name,
                        *entry,
                        &request.client_id,
                    );
                    let client_id = request.client_id.clone();
                    if self.worker_senders[worker_id]
                        .try_send((
                            request,
                            done_sender,
                            ActiveWorkerState {
                                client_id,
                                worker_id,
                            },
                        ))
                        .is_err()
                    {
                        // Available worker should have an empty channel, so if we fail
                        // here it must be shut down. We should shut down too.
                        tracing::warn!(
                            "Worker died or dropped channel. Shutting down {} scheduler.",
                            self.worker.config().name
                        );
                        return;
                    }
                },
                _ = report_stats => {
                    let heap_stats = self.aggregate_heap_stats();
                    log_aggregated_heap_stats(&heap_stats);
                    report_stats = self.rt.wait(*HEAP_WORKER_REPORT_INTERVAL_SECONDS);
                },
            }
        }
    }

    /// Find a worker for the given `client_id`.`
    /// Returns `None` if no worker could be allocated for this client (i.e.
    /// this client has reached it's capacity with the scheduler).
    ///
    /// Note that the returned worker id is removed from the
    /// `self.available_workers` state, so the caller is responsible for using
    /// the worker and returning it back to `self.available_workers` after it is
    /// done.
    fn get_worker(&mut self, client_id: &str) -> Option<usize> {
        // Make sure this client isn't overloading the scheduler.
        let active_worker_count = self
            .in_progress_count
            .get(client_id)
            .copied()
            .unwrap_or_default();
        if (active_worker_count * 100) / (self.max_workers) >= self.max_percent_per_client {
            tracing::warn!(
                "Client {} is using >= {}% of scheduler capacity; rejecting new request",
                client_id,
                self.max_percent_per_client,
            );
            return None;
        }
        // Try to find an existing worker for this client.
        if let Some((client_id, mut workers)) = self.available_workers.remove_entry(client_id) {
            let worker = workers
                .pop_front()
                .expect("Available worker map should never contain an empty list");
            if !workers.is_empty() {
                self.available_workers.insert(client_id, workers);
            }
            return Some(worker.worker_id);
        }
        // If we've recently started up and haven't yet created `max_workers` threads,
        // create a new worker instead of "stealing" some other client's worker.
        if self.worker_senders.len() < self.max_workers {
            let new_worker = self.worker.clone();
            let heap_stats = SharedIsolateHeapStats::new();
            let heap_stats_ = heap_stats.clone();
            let (work_sender, work_receiver) = mpsc::channel(1);
            let handle = self.rt.spawn_thread("isolate", move || {
                new_worker.service_requests(work_receiver, heap_stats_)
            });
            self.worker_senders.push(work_sender);
            self.handles
                .lock()
                .push(IsolateWorkerHandle { handle, heap_stats });
            tracing::info!(
                "Created {} isolate worker {}",
                self.worker.config().name,
                self.worker_senders.len() - 1
            );
            return Some(self.worker_senders.len() - 1);
        }
        // No existing worker for this client and we've already started the max number
        // of workers -- just grab the least recently used worker. This worker is least
        // likely to be reused by its' previous client.
        let Some((key, workers)) =
            self.available_workers
                .iter_mut()
                .min_by(|(_, workers1), (_, workers2)| {
                    workers1
                        .back()
                        .expect("Available worker map should never contain an empty list")
                        .last_used_ts
                        .cmp(
                            &workers2
                                .back()
                                .expect("Available worker map should never contain an empty list")
                                .last_used_ts,
                        )
                })
        else {
            // No available workers.
            return None;
        };
        log_worker_stolen(
            workers
                .back()
                .expect("Available worker map should never contain an empty list")
                .last_used_ts
                .elapsed(),
        );
        let worker_id = workers
            .pop_back()
            .expect("Available worker map should never contain an empty list");
        if workers.is_empty() {
            // This variable shadowing drops the mutable reference to
            // `self.available_workers`.
            let key = key.clone();
            self.available_workers.remove(&key);
        }
        Some(worker_id.worker_id)
    }

    fn aggregate_heap_stats(&self) -> IsolateHeapStats {
        let mut total = IsolateHeapStats::default();
        for handle in self.handles.lock().iter() {
            total += handle.heap_stats.get();
        }
        total
    }
}

pub struct IsolateWorkerHandle {
    pub handle: Box<dyn SpawnHandle>,
    heap_stats: SharedIsolateHeapStats,
}

#[derive(Clone)]
pub struct SharedIsolateHeapStats(Arc<Mutex<IsolateHeapStats>>);

impl SharedIsolateHeapStats {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(IsolateHeapStats::default())))
    }

    pub(crate) fn get(&self) -> IsolateHeapStats {
        *self.0.lock()
    }

    pub fn store(&self, stats: IsolateHeapStats) {
        *self.0.lock() = stats;
    }
}

#[async_trait(?Send)]
pub trait IsolateWorker<RT: Runtime>: Clone + Send + 'static {
    async fn service_requests<T>(
        self,
        mut reqs: mpsc::Receiver<(Request<RT>, oneshot::Sender<T>, T)>,
        heap_stats: SharedIsolateHeapStats,
    ) {
        let IsolateConfig {
            max_user_timeout,
            limiter,
            ..
        } = self.config();
        let mut isolate = Isolate::new(self.rt(), *max_user_timeout, limiter.clone());
        heap_stats.store(isolate.heap_stats());
        let mut last_client_id: Option<String> = None;
        loop {
            select! {
                _ = if last_client_id.is_some() {
                        self.rt().wait(*ISOLATE_IDLE_TIMEOUT).boxed_local().fuse()
                    }
                    else {
                        // If the isolate isn't "tainted", no need to wait for the idle timeout.
                        futures::future::pending().boxed_local().fuse()
                    } => {
                    drop(isolate);
                    isolate = Isolate::new(self.rt().clone(), *max_user_timeout, limiter.clone());
                    tracing::debug!("Restarting isolate for {last_client_id:?} due to idle timeout");
                    last_client_id = None;
                    metrics::log_recreate_isolate("idle_timeout");
                    continue;
                },
                req = reqs.recv().fuse() => {
                    let Some((req, done, done_token)) = req else {
                        return;
                    };
                    let root = initialize_root_from_parent(func_path!(),req.parent_trace.clone());
                    // If we receive a request from a different client (i.e. a different backend),
                    // recreate the isolate. We don't allow an isolate to be reused
                    // across clients for security isolation.
                    if last_client_id.get_or_insert_with(|| {
                        req.client_id.clone()
                    }) != &req.client_id {
                        let pause_client = self.rt().pause_client();
                        pause_client.wait(PAUSE_RECREATE_CLIENT).await;
                        tracing::debug!("Restarting isolate due to client change, previous: {:?}, new: {:?}", last_client_id, req.client_id);
                        metrics::log_recreate_isolate("client_id_changed");
                        drop(isolate);
                        isolate = Isolate::new(
                            self.rt().clone(),
                            *max_user_timeout,
                            limiter.clone(),
                        );
                        last_client_id = Some(req.client_id.clone());
                    } else if last_client_id.is_some() {
                        tracing::debug!("Reusing isolate for client {}", req.client_id);
                    }
                    // Require the layer below to opt into isolate reuse by setting `isolate_clean`.
                    let mut isolate_clean = false;
                    let debug_str = self
                        .handle_request(&mut isolate, &mut isolate_clean, req, heap_stats.clone())
                        .in_span(root)
                        .await;
                    let _ = done.send(done_token);
                    if !isolate_clean || should_recreate_isolate(&mut isolate, debug_str) {
                        // Clean up current isolate before creating another.
                        // If we just overwrite `isolate`, the `Isolate::new` runs before
                        // dropping the old isolate. And v8 stores the current isolate in a
                        // thread local, so the drop handler sets the current isolate to null.
                        // Therefore without this drop(isolate), we get segfaults.
                        drop(isolate);
                        isolate = Isolate::new(
                            self.rt().clone(),
                            *max_user_timeout,
                            limiter.clone(),
                        );
                        last_client_id = None;
                    }
                    heap_stats.store(isolate.heap_stats());
                }
            }
        }
    }

    async fn handle_request(
        &self,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        req: Request<RT>,
        heap_stats: SharedIsolateHeapStats,
    ) -> String;

    fn config(&self) -> &IsolateConfig;
    fn rt(&self) -> RT;
}

pub(crate) fn should_recreate_isolate<RT: Runtime>(
    isolate: &mut Isolate<RT>,
    last_executed: String,
) -> bool {
    if !*REUSE_ISOLATES {
        metrics::log_recreate_isolate("env_disabled");
        return true;
    }
    if let Err(e) = isolate.check_isolate_clean() {
        tracing::error!(
            "Restarting Isolate {}: {e:?}, last request: {last_executed:?}",
            e.reason()
        );
        metrics::log_recreate_isolate(e.reason());
        return true;
    }

    if isolate.created().elapsed() > *ISOLATE_MAX_LIFETIME {
        metrics::log_recreate_isolate("max_lifetime");
        return true;
    }

    false
}

#[cfg(test)]
mod tests {

    use cmd_util::env::env_config;
    use common::pause::PauseController;
    use database::test_helpers::DbFixtures;
    use errors::ErrorMetadataAnyhowExt;
    use model::test_helpers::DbFixturesWithModel;
    use pb::common::FunctionResult as FunctionResultProto;
    use proptest::prelude::*;
    use runtime::testing::TestRuntime;
    use sync_types::testing::assert_roundtrips;
    use tokio::sync::oneshot;

    use super::FunctionResult;
    use crate::{
        client::{
            initialize_v8,
            NO_AVAILABLE_WORKERS,
            PAUSE_REQUEST,
        },
        test_helpers::bogus_udf_request,
        IsolateClient,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_function_result_proto_roundtrips(left in any::<FunctionResult>()) {
            assert_roundtrips::<FunctionResult, FunctionResultProto>(left);
        }
    }

    #[convex_macro::test_runtime]
    async fn test_scheduler_workers_limit_requests(
        rt: TestRuntime,
        pause1: PauseController,
    ) -> anyhow::Result<()> {
        initialize_v8();
        let function_runner_core = IsolateClient::new(rt.clone(), 100, 1, None)?;
        let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
        let client1 = "client1";
        let hold_guard = pause1.hold(PAUSE_REQUEST);
        let (sender, _rx1) = oneshot::channel();
        let request = bogus_udf_request(&db, client1, sender).await?;
        function_runner_core.send_request(request)?;
        // Pausing a request while being executed should make the next request be
        // rejected because there are no available workers.
        let _guard = hold_guard.wait_for_blocked().await.unwrap();
        let (sender, rx2) = oneshot::channel();
        let request2 = bogus_udf_request(&db, client1, sender).await?;
        function_runner_core.send_request(request2)?;
        let response = IsolateClient::<TestRuntime>::receive_response(rx2).await?;
        let err = response.unwrap_err();
        assert!(err.is_rejected_before_execution(), "{err:?}");
        assert!(err.to_string().contains(NO_AVAILABLE_WORKERS));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_scheduler_does_not_throttle_different_clients(
        rt: TestRuntime,
        pause1: PauseController,
    ) -> anyhow::Result<()> {
        initialize_v8();
        let function_runner_core = IsolateClient::new(rt.clone(), 50, 2, None)?;
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let client1 = "client1";
        let hold_guard = pause1.hold(PAUSE_REQUEST);
        let (sender, _rx1) = oneshot::channel();
        let request = bogus_udf_request(&db, client1, sender).await?;
        function_runner_core.send_request(request)?;
        // Pausing a request should not affect the next one because we have 2 workers
        // and 2 requests from different clients.
        let _guard = hold_guard.wait_for_blocked().await.unwrap();
        let (sender, rx2) = oneshot::channel();
        let client2 = "client2";
        let request2 = bogus_udf_request(&db, client2, sender).await?;
        function_runner_core.send_request(request2)?;
        IsolateClient::<TestRuntime>::receive_response(rx2).await??;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_scheduler_throttles_same_client(
        rt: TestRuntime,
        pause1: PauseController,
    ) -> anyhow::Result<()> {
        initialize_v8();
        let function_runner_core = IsolateClient::new(rt.clone(), 50, 2, None)?;
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let client = "client";
        let hold_guard = pause1.hold(PAUSE_REQUEST);
        let (sender, _rx1) = oneshot::channel();
        let request = bogus_udf_request(&db, client, sender).await?;
        function_runner_core.send_request(request)?;
        // Pausing the first request and sending a second should make the second fail
        // because there's only one worker left and it is reserved for other clients.
        let _guard = hold_guard.wait_for_blocked().await.unwrap();
        let (sender, rx2) = oneshot::channel();
        let request2 = bogus_udf_request(&db, client, sender).await?;
        function_runner_core.send_request(request2)?;
        let response = IsolateClient::<TestRuntime>::receive_response(rx2).await?;
        let err = response.unwrap_err();
        assert!(err.is_rejected_before_execution());
        assert!(err.to_string().contains(NO_AVAILABLE_WORKERS));
        Ok(())
    }
}
