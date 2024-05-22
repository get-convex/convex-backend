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
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    codel_queue::{
        new_codel_queue_async,
        CoDelQueueReceiver,
        CoDelQueueSender,
        ExpiredInQueue,
    },
    components::{
        ComponentDefinitionPath,
        ComponentFunctionPath,
        ComponentPath,
    },
    errors::{
        recapture_stacktrace,
        JsError,
    },
    execution_context::ExecutionContext,
    http::fetch::FetchClient,
    identity::InertIdentity,
    knobs::{
        HEAP_WORKER_REPORT_INTERVAL_SECONDS,
        ISOLATE_IDLE_TIMEOUT,
        ISOLATE_MAX_LIFETIME,
        ISOLATE_QUEUE_SIZE,
        REUSE_ISOLATES,
        V8_THREADS,
    },
    log_lines::LogLine,
    minitrace_helpers::{
        initialize_root_from_parent,
        EncodedSpan,
    },
    pause::PauseClient,
    query_journal::QueryJournal,
    runtime::{
        shutdown_and_join,
        Runtime,
        RuntimeInstant,
        SpawnHandle,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    static_span,
    types::{
        ModuleEnvironment,
        UdfType,
    },
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
use file_storage::TransactionalFileStorage;
use futures::{
    channel::{
        mpsc,
        oneshot,
    },
    future,
    pin_mut,
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
    InstanceSecret,
    KeyBroker,
};
use minitrace::{
    collector::SpanContext,
    full_name,
    future::FutureExt as _,
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
use pb::common::{
    function_result::Result as FunctionResultTypeProto,
    FunctionResult as FunctionResultProto,
};
use prometheus::VMHistogram;
use serde_json::Value as JsonValue;
use sync_types::{
    CanonicalizedModulePath,
    CanonicalizedUdfPath,
};
use usage_tracking::FunctionUsageStats;
use value::{
    id_v6::DeveloperDocumentId,
    ConvexValue,
};
use vector::PublicVectorSearchQueryResult;

use crate::{
    concurrency_limiter::ConcurrencyLimiter,
    environment::{
        action::{
            ActionEnvironment,
            HttpActionResult,
        },
        analyze::AnalyzeEnvironment,
        app_definitions::AppDefinitionEvaluator,
        auth_config::{
            AuthConfig,
            AuthConfigEnvironment,
        },
        helpers::validation::{
            ValidatedHttpPath,
            ValidatedPathAndArgs,
        },
        schema::SchemaEnvironment,
        udf::DatabaseUdfEnvironment,
    },
    http_action::{
        self,
        HttpActionResponseStreamer,
    },
    isolate::{
        Isolate,
        IsolateHeapStats,
    },
    metrics::{
        self,
        is_developer_ok,
        log_aggregated_heap_stats,
        log_pool_allocated_count,
        log_pool_running_count,
        log_worker_stolen,
        queue_timer,
        RequestStatus,
    },
    ActionOutcome,
    FunctionOutcome,
    HttpActionOutcome,
};

#[cfg(any(test, feature = "testing"))]
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

#[derive(Clone, Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq,)
)]
pub struct FunctionResult {
    pub result: Result<ConvexValue, JsError>,
}

impl TryFrom<FunctionResultProto> for FunctionResult {
    type Error = anyhow::Error;

    fn try_from(result: FunctionResultProto) -> anyhow::Result<Self> {
        let result = match result.result {
            Some(FunctionResultTypeProto::JsonPackedValue(value)) => {
                let json: JsonValue = serde_json::from_str(&value)?;
                let value = ConvexValue::try_from(json)?;
                Ok(value)
            },
            Some(FunctionResultTypeProto::JsError(js_error)) => Err(js_error.try_into()?),
            None => anyhow::bail!("Missing result"),
        };
        Ok(FunctionResult { result })
    }
}

impl TryFrom<FunctionResult> for FunctionResultProto {
    type Error = anyhow::Error;

    fn try_from(result: FunctionResult) -> anyhow::Result<Self> {
        let result = match result.result {
            Ok(value) => {
                let json = JsonValue::from(value);
                FunctionResultTypeProto::JsonPackedValue(serde_json::to_string(&json)?)
            },
            Err(js_error) => FunctionResultTypeProto::JsError(js_error.try_into()?),
        };
        Ok(FunctionResultProto {
            result: Some(result),
        })
    }
}

#[async_trait]
pub trait ActionCallbacks: Send + Sync {
    // Executing UDFs
    async fn execute_query(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult>;

    async fn execute_mutation(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult>;

    async fn execute_action(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        context: ExecutionContext,
    ) -> anyhow::Result<FunctionResult>;

    // Storage
    async fn storage_get_url(
        &self,
        identity: Identity,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>>;

    async fn storage_delete(
        &self,
        identity: Identity,
        storage_id: FileStorageId,
    ) -> anyhow::Result<()>;

    // Used to get a file content from an action running in v8.
    async fn storage_get_file_entry(
        &self,
        identity: Identity,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>>;

    // Used to store an already uploaded file from an action running in v8.
    async fn storage_store_file_entry(
        &self,
        identity: Identity,
        entry: FileStorageEntry,
    ) -> anyhow::Result<DeveloperDocumentId>;

    // Scheduler
    async fn schedule_job(
        &self,
        identity: Identity,
        path: ComponentFunctionPath,
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
}

pub struct UdfRequest<RT: Runtime> {
    pub path_and_args: ValidatedPathAndArgs,
    pub udf_type: UdfType,
    pub identity: InertIdentity,
    pub transaction: Transaction<RT>,
    pub journal: QueryJournal,
    pub context: ExecutionContext,
}

pub struct HttpActionRequest<RT: Runtime> {
    router_path: ValidatedHttpPath,
    http_request: http_action::HttpActionRequest,
    transaction: Transaction<RT>,
    identity: Identity,
    context: ExecutionContext,
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

pub struct EnvironmentData<RT: Runtime> {
    pub key_broker: KeyBroker,
    pub system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    pub file_storage: TransactionalFileStorage<RT>,
    pub module_loader: Arc<dyn ModuleLoader<RT>>,
}

pub struct Request<RT: Runtime> {
    pub client_id: String,
    pub inner: RequestType<RT>,
    pub pause_client: PauseClient,
    pub parent_trace: EncodedSpan,
}

impl<RT: Runtime> Request<RT> {
    pub fn new(client_id: String, inner: RequestType<RT>, parent_trace: EncodedSpan) -> Self {
        Self {
            client_id,
            inner,
            pause_client: PauseClient::new(),
            parent_trace,
        }
    }
}

pub type EvaluateAppDefinitionsResult =
    BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>;

pub enum RequestType<RT: Runtime> {
    Udf {
        request: UdfRequest<RT>,
        environment_data: EnvironmentData<RT>,
        response: oneshot::Sender<anyhow::Result<(Transaction<RT>, FunctionOutcome)>>,
        queue_timer: Timer<VMHistogram>,
    },
    Action {
        request: ActionRequest<RT>,
        environment_data: EnvironmentData<RT>,
        response: oneshot::Sender<anyhow::Result<ActionOutcome>>,
        queue_timer: Timer<VMHistogram>,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
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
        response: oneshot::Sender<anyhow::Result<EvaluateAppDefinitionsResult>>,
    },
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
        }
    }
}

/// The V8 code all expects to run on a single thread, which makes it ineligible
/// for Tokio's scheduler, which wants the ability to move work across scheduler
/// threads. Instead, we'll manage our V8 threads ourselves.
///
/// [`IsolateClient`] is the "client" entry point to our V8 threads.
pub struct IsolateClient<RT: Runtime> {
    rt: RT,
    handles: Arc<Mutex<Vec<IsolateWorkerHandle<RT>>>>,
    scheduler: Arc<Mutex<Option<RT::Handle>>>,
    sender: CoDelQueueSender<RT, Request<RT>>,
    allow_actions: bool,
    system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    instance_name: String,
    instance_secret: InstanceSecret,
    file_storage: TransactionalFileStorage<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
}

impl<RT: Runtime> Clone for IsolateClient<RT> {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt.clone(),
            handles: self.handles.clone(),
            scheduler: self.scheduler.clone(),
            sender: self.sender.clone(),
            allow_actions: self.allow_actions,
            system_env_vars: self.system_env_vars.clone(),
            instance_name: self.instance_name.clone(),
            instance_secret: self.instance_secret,
            file_storage: self.file_storage.clone(),
            module_loader: self.module_loader.clone(),
        }
    }
}

pub fn initialize_v8() {
    static V8_INIT: Once = Once::new();
    V8_INIT.call_once(|| {
        let _s = static_span!("initialize_v8");

        // `deno_core_icudata` internally loads this with proper 16-byte alignment.
        assert!(v8::icu::set_common_data_73(deno_core_icudata::ICU_DATA).is_ok());

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

impl<RT: Runtime> IsolateClient<RT> {
    pub fn new(
        rt: RT,
        isolate_worker: BackendIsolateWorker<RT>,
        max_workers: usize,
        allow_actions: bool,
        instance_name: String,
        instance_secret: InstanceSecret,
        file_storage: TransactionalFileStorage<RT>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        module_loader: Arc<dyn ModuleLoader<RT>>,
    ) -> Self {
        initialize_v8();

        // NB: We don't call V8::Dispose or V8::ShutdownPlatform since we just assume a
        // single V8 instance per process and don't need to clean up its
        // resources.
        let (sender, receiver) =
            new_codel_queue_async::<_, Request<_>>(rt.clone(), *ISOLATE_QUEUE_SIZE);

        let handles = Arc::new(Mutex::new(Vec::new()));
        let _handles = handles.clone();
        let _rt = rt.clone();
        let scheduler = rt.spawn("isolate_scheduler", async move {
            // The scheduler thread pops a worker from available_workers and
            // pops a request from the CoDelQueueReceiver. Then it sends the request
            // to the worker.
            let isolate_worker = isolate_worker.clone();
            let scheduler = IsolateScheduler::new(_rt, isolate_worker, max_workers, _handles);
            scheduler.run(receiver).await
        });
        Self {
            rt,
            handles,
            scheduler: Arc::new(Mutex::new(Some(scheduler))),
            sender,
            allow_actions,
            system_env_vars,
            instance_name,
            instance_secret,
            file_storage,
            module_loader,
        }
    }

    pub fn aggregate_heap_stats(&self) -> IsolateHeapStats {
        let mut total = IsolateHeapStats::default();
        for handle in self.handles.lock().iter() {
            total += handle.heap_stats.get();
        }
        total
    }

    /// Execute a UDF within a transaction.
    #[minitrace::trace]
    pub async fn execute_udf(
        &self,
        udf_type: UdfType,
        path_and_args: ValidatedPathAndArgs,
        transaction: Transaction<RT>,
        journal: QueryJournal,
        context: ExecutionContext,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        let timer = metrics::execute_timer(&udf_type, path_and_args.npm_version());
        let (tx, rx) = oneshot::channel();
        let key_broker = KeyBroker::new(&self.instance_name, self.instance_secret)?;
        let request = RequestType::Udf {
            request: UdfRequest {
                path_and_args,
                udf_type,
                identity: transaction.inert_identity(),
                transaction,
                journal,
                context,
            },
            environment_data: EnvironmentData {
                key_broker,
                system_env_vars: self.system_env_vars.clone(),
                file_storage: self.file_storage.clone(),
                module_loader: self.module_loader.clone(),
            },
            response: tx,
            queue_timer: queue_timer(),
        };
        self.send_request(Request::new(
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        let (tx, outcome) = Self::receive_response(rx).await??;
        metrics::finish_execute_timer(timer, &outcome);
        Ok((tx, outcome))
    }

    /// Execute an HTTP action.
    /// HTTP actions can run other UDFs, so they take in a ActionCallbacks from
    /// the application layer. This creates a transient reference cycle.
    #[minitrace::trace]
    pub async fn execute_http_action(
        &self,
        router_path: ValidatedHttpPath,
        http_request: http_action::HttpActionRequest,
        identity: Identity,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        http_response_streamer: HttpActionResponseStreamer,
        transaction: Transaction<RT>,
        context: ExecutionContext,
    ) -> anyhow::Result<HttpActionOutcome> {
        // In production, we have two isolate clients, one for DB UDFs (queries,
        // mutations) and one for actions (including HTTP actions).
        // This check should prevent us from mixing them up
        if !self.allow_actions {
            anyhow::bail!("Requested an action from an Isolate client that does not allow actions")
        }
        let timer = metrics::execute_timer(&UdfType::HttpAction, router_path.npm_version());
        let (tx, rx) = oneshot::channel();
        let key_broker = KeyBroker::new(&self.instance_name, self.instance_secret)?;
        let request = RequestType::HttpAction {
            request: HttpActionRequest {
                router_path,
                http_request,
                identity,
                transaction,
                context,
            },
            response: tx,
            queue_timer: queue_timer(),
            action_callbacks,
            fetch_client,
            log_line_sender,
            http_response_streamer,
            environment_data: EnvironmentData {
                key_broker,
                system_env_vars: self.system_env_vars.clone(),
                file_storage: self.file_storage.clone(),
                module_loader: self.module_loader.clone(),
            },
        };
        self.send_request(Request::new(
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        let outcome = Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })?;
        metrics::finish_execute_timer(timer, &FunctionOutcome::HttpAction(outcome.clone()));
        Ok(outcome)
    }

    #[minitrace::trace]
    pub async fn execute_action(
        &self,
        path_and_args: ValidatedPathAndArgs,
        transaction: Transaction<RT>,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        context: ExecutionContext,
    ) -> anyhow::Result<ActionOutcome> {
        // In production, we have two isolate clients, one for DB UDFs (queries,
        // mutations) and one for actions (including HTTP actions).
        // This check should prevent us from mixing them up
        if !self.allow_actions {
            anyhow::bail!("Requested an action from an Isolate client that does not allow actions")
        }
        let timer = metrics::execute_timer(&UdfType::Action, path_and_args.npm_version());
        let (tx, rx) = oneshot::channel();
        let key_broker = KeyBroker::new(&self.instance_name, self.instance_secret)?;
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
            environment_data: EnvironmentData {
                key_broker,
                system_env_vars: self.system_env_vars.clone(),
                file_storage: self.file_storage.clone(),
                module_loader: self.module_loader.clone(),
            },
        };
        self.send_request(Request::new(
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        let outcome = Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })?;
        metrics::finish_execute_timer(timer, &FunctionOutcome::Action(outcome.clone()));
        Ok(outcome)
    }

    /// Analyze a set of user-defined modules.
    #[minitrace::trace]
    pub async fn analyze(
        &self,
        udf_config: UdfConfig,
        modules: BTreeMap<CanonicalizedModulePath, ModuleConfig>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
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
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })
    }

    #[minitrace::trace]
    pub async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
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
            response: tx,
        };
        self.send_request(Request::new(
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })
    }

    /// Evaluate a (bundled) schema module.
    #[minitrace::trace]
    pub async fn evaluate_schema(
        &self,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
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
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })
    }

    /// Evaluate a (bundled) auth config module.
    #[minitrace::trace]
    pub async fn evaluate_auth_config(
        &self,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<AuthConfig> {
        let (tx, rx) = oneshot::channel();
        let request = RequestType::EvaluateAuthConfig {
            auth_config_bundle,
            source_map,
            environment_variables,
            response: tx,
        };
        // TODO(jordan): this is an incomplete state. eventually we will expand to trace
        // other requests besides udfs
        self.send_request(Request::new(
            self.instance_name.clone(),
            request,
            EncodedSpan::from_parent(SpanContext::current_local_parent()),
        ))?;
        Self::receive_response(rx).await?.map_err(|e| {
            if e.is_overloaded() {
                recapture_stacktrace(e)
            } else {
                e
            }
        })
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

pub struct IsolateScheduler<RT: Runtime, W: IsolateWorker<RT>> {
    rt: RT,
    worker: W,
    max_workers: usize,

    // Vec of channels for sending work to individual workers.
    worker_senders: Vec<mpsc::Sender<(Request<RT>, oneshot::Sender<usize>, usize)>>,
    // Stack of indexes into worker_senders, including exactly the workers
    // that are not running any request.
    // Very important that it's a LIFO stack because workers keep memory
    // around after running UDFs, making it more efficient to reuse a worker
    // that was recently used.
    available_workers: Vec<usize>,

    handles: Arc<Mutex<Vec<IsolateWorkerHandle<RT>>>>,
}

impl<RT: Runtime, W: IsolateWorker<RT>> IsolateScheduler<RT, W> {
    pub fn new(
        rt: RT,
        worker: W,
        max_workers: usize,
        handles: Arc<Mutex<Vec<IsolateWorkerHandle<RT>>>>,
    ) -> Self {
        Self {
            rt,
            worker,
            max_workers,
            available_workers: Vec::new(),
            worker_senders: Vec::new(),
            handles,
        }
    }

    // Creates a worker and returns its index, without adding it to
    // available_workers.
    fn create_worker(&mut self) -> usize {
        let worker_index = self.worker_senders.len();
        let worker = self.worker.clone();
        // Single-producer single-consumer channel sending work from scheduler
        // to worker.
        let heap_stats = SharedIsolateHeapStats::new();
        let (work_sender, work_receiver) = mpsc::channel(1);
        self.worker_senders.push(work_sender);

        let heap_stats_ = heap_stats.clone();
        let handle = self
            .rt
            .spawn_thread(move || worker.service_requests(work_receiver, heap_stats_));
        self.handles
            .lock()
            .push(IsolateWorkerHandle { handle, heap_stats });

        tracing::info!(
            "Created {} isolate worker: {}",
            self.worker.config().name,
            worker_index
        );
        log_pool_allocated_count(self.worker.config().name, self.worker_senders.len());
        worker_index
    }

    // Returns the most recently used worker, creates a new one, or blocks
    // indefinitely.
    async fn get_available_worker(&mut self) -> usize {
        match self.available_workers.pop() {
            Some(value) => value,
            None => {
                // No available worker, create a new one if under the limit
                if self.worker_senders.len() < self.max_workers {
                    return self.create_worker();
                }
                // otherwise block indefinitely.
                future::pending().await
            },
        }
    }

    pub async fn run(mut self, receiver: CoDelQueueReceiver<RT, Request<RT>>) {
        pin_mut!(receiver);
        let mut in_progress_workers: FuturesUnordered<oneshot::Receiver<usize>> =
            FuturesUnordered::new();
        loop {
            let next_worker = loop {
                // First pop all active workers that have completed, then
                // pop an available worker if any.
                select_biased! {
                    completed_worker = in_progress_workers.select_next_some() => {
                        log_pool_running_count(
                            self.worker.config().name,
                            in_progress_workers.len(),
                            "" // This is a single tenant scheduler used in the backend.
                        );
                        let Ok(completed_worker) = completed_worker else {
                            // Worker has shut down, so we should shut down too.
                            tracing::warn!("Worker shut down. Shutting down {} scheduler.", self.worker.config().name);
                            return;
                        };
                        self.available_workers.push(completed_worker);
                    },
                    next_worker = self.get_available_worker().fuse() => {
                        break next_worker;
                    },
                }
            };
            let req = loop {
                match receiver.next().await {
                    Some((req, None)) => break req,
                    Some((req, Some(expired))) => req.expire(expired),
                    // Request queue closed, shutting down.
                    None => return,
                }
            };
            let (done_sender, done_receiver) = oneshot::channel();
            if self.worker_senders[next_worker]
                .try_send((req, done_sender, next_worker))
                .is_err()
            {
                // Available worker should have an empty channel, so if we fail
                // here it must be shut down. We should shut down too.
                tracing::warn!(
                    "Worker sender dropped. Shutting down {} scheduler.",
                    self.worker.config().name
                );
                return;
            }
            in_progress_workers.push(done_receiver);
            // This is a single tenant scheduler used in the backend.
            let client_id = "";
            log_pool_running_count(
                self.worker.config().name,
                in_progress_workers.len(),
                client_id,
            );
        }
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
    available_workers: HashMap<String, VecDeque<IdleWorkerState<RT>>>,
    /// Set of futures awaiting a response from an active worker.
    in_progress_workers: FuturesUnordered<oneshot::Receiver<ActiveWorkerState>>,
    /// Counts the number of active workers per client. Should only contain a
    /// key if the value is greater than 0.
    in_progress_count: HashMap<String, usize>,
    /// The max number of workers this scheduler is permitted to create.
    max_workers: usize,
    handles: Arc<Mutex<Vec<IsolateWorkerHandle<RT>>>>,
    max_percent_per_client: usize,
}

struct IdleWorkerState<RT: Runtime> {
    worker_id: usize,
    last_used_ts: RT::Instant,
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
        handles: Arc<Mutex<Vec<IsolateWorkerHandle<RT>>>>,
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
            let handle = self
                .rt
                .spawn_thread(move || new_worker.service_requests(work_receiver, heap_stats_));
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

pub struct IsolateWorkerHandle<RT: Runtime> {
    pub handle: RT::ThreadHandle,
    heap_stats: SharedIsolateHeapStats,
}

#[derive(Clone)]
pub struct SharedIsolateHeapStats(Arc<Mutex<IsolateHeapStats>>);

impl SharedIsolateHeapStats {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(IsolateHeapStats::default())))
    }

    fn get(&self) -> IsolateHeapStats {
        *self.0.lock()
    }

    pub fn store(&self, stats: IsolateHeapStats) {
        *self.0.lock() = stats;
    }
}

/// State for each "server" thread that handles V8 requests.
#[derive(Clone)]
pub struct BackendIsolateWorker<RT: Runtime> {
    rt: RT,
    config: IsolateConfig,
    // This tokio Mutex is safe only because it's stripped out of production
    // builds. We shouldn't use tokio locks for prod code (see
    // https://github.com/rust-lang/rust/issues/104883 for background and
    // https://github.com/get-convex/convex/pull/19307 for an alternative).
    #[cfg(any(test, feature = "testing"))]
    pause_client: Option<Arc<tokio::sync::Mutex<PauseClient>>>,
}

impl<RT: Runtime> BackendIsolateWorker<RT> {
    pub fn new(rt: RT, config: IsolateConfig) -> Self {
        Self {
            rt,
            config,
            #[cfg(any(test, feature = "testing"))]
            pause_client: None,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_tests(rt: RT, config: IsolateConfig, pause_client: PauseClient) -> Self {
        Self {
            rt,
            config,
            pause_client: Some(Arc::new(tokio::sync::Mutex::new(pause_client))),
        }
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
                req = reqs.next() => {
                    let Some((req, done, done_token)) = req else {
                        return;
                    };
                    let root = initialize_root_from_parent(full_name!(),req.parent_trace.clone());
                    // If we receive a request from a different client (i.e. a different backend),
                    // recreate the isolate. We don't allow an isolate to be reused
                    // across clients for security isolation.
                    if last_client_id.get_or_insert_with(|| {
                        req.client_id.clone()
                    }) != &req.client_id {
                        #[cfg(any(test, feature = "testing"))]
                        if let Some(pause_client) = &mut self.pause_client() {
                            let mut pause_client = pause_client.lock().await;
                            pause_client.wait(PAUSE_RECREATE_CLIENT).await;
                            drop(pause_client);
                        }
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
    #[cfg(any(test, feature = "testing"))]
    fn pause_client(&self) -> Option<Arc<tokio::sync::Mutex<PauseClient>>>;
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

#[async_trait(?Send)]
impl<RT: Runtime> IsolateWorker<RT> for BackendIsolateWorker<RT> {
    #[minitrace::trace]
    async fn handle_request(
        &self,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        Request {
            client_id,
            inner,
            pause_client: _,
            parent_trace: _,
        }: Request<RT>,
        heap_stats: SharedIsolateHeapStats,
    ) -> String {
        match inner {
            RequestType::Udf {
                request,
                environment_data,
                mut response,
                queue_timer,
            } => {
                drop(queue_timer);
                let timer = metrics::service_request_timer(&request.udf_type);
                let udf_path = request.path_and_args.path().udf_path.to_owned();
                let environment = DatabaseUdfEnvironment::new(
                    self.rt.clone(),
                    environment_data,
                    heap_stats.clone(),
                    request,
                );
                let r = environment
                    .run(
                        client_id,
                        isolate,
                        isolate_clean,
                        response.cancellation().boxed(),
                    )
                    .await;
                let status = match &r {
                    Ok((_tx, outcome)) => {
                        if is_developer_ok(outcome) {
                            RequestStatus::Success
                        } else {
                            RequestStatus::DeveloperError
                        }
                    },
                    Err(_) => RequestStatus::SystemError,
                };
                metrics::finish_service_request_timer(timer, status);
                let _ = response.send(r);
                format!("UDF: {udf_path:?}")
            },
            RequestType::HttpAction {
                request,
                environment_data,
                mut response,
                queue_timer,
                action_callbacks,
                fetch_client,
                log_line_sender,
                http_response_streamer,
            } => {
                drop(queue_timer);
                let timer = metrics::service_request_timer(&UdfType::HttpAction);
                let udf_path: CanonicalizedUdfPath = request.router_path.path().udf_path.clone();
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    ComponentPath::root(),
                    environment_data,
                    request.identity,
                    request.transaction,
                    action_callbacks,
                    fetch_client,
                    log_line_sender,
                    Some(http_response_streamer),
                    heap_stats.clone(),
                    request.context,
                );
                let r = environment
                    .run_http_action(
                        client_id,
                        isolate,
                        isolate_clean,
                        request.router_path,
                        request.http_request,
                        response.cancellation().boxed(),
                    )
                    .await;
                let status = match &r {
                    Ok(outcome) => match outcome.result {
                        // Note that the stream could potentially encounter errors later
                        HttpActionResult::Streamed => RequestStatus::Success,
                        HttpActionResult::Error(_) => RequestStatus::DeveloperError,
                    },
                    Err(_) => RequestStatus::SystemError,
                };
                metrics::finish_service_request_timer(timer, status);
                let _ = response.send(r);
                format!("Http: {udf_path:?}")
            },
            RequestType::Action {
                request,
                environment_data,
                mut response,
                queue_timer,
                action_callbacks,
                fetch_client,
                log_line_sender,
            } => {
                drop(queue_timer);
                let timer = metrics::service_request_timer(&UdfType::Action);
                let component = request.params.path_and_args.path().component.clone();
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    component,
                    environment_data,
                    request.identity,
                    request.transaction,
                    action_callbacks,
                    fetch_client,
                    log_line_sender,
                    None,
                    heap_stats.clone(),
                    request.context,
                );
                let r = environment
                    .run_action(
                        client_id,
                        isolate,
                        isolate_clean,
                        request.params.clone(),
                        response.cancellation().boxed(),
                    )
                    .await;
                let status = match &r {
                    Ok(outcome) => {
                        if outcome.result.is_ok() {
                            RequestStatus::Success
                        } else {
                            RequestStatus::DeveloperError
                        }
                    },
                    Err(_) => RequestStatus::SystemError,
                };
                metrics::finish_service_request_timer(timer, status);
                let _ = response.send(r);
                format!("Action: {:?}", request.params.path_and_args.path().udf_path)
            },
            RequestType::Analyze {
                udf_config,
                modules,
                environment_variables,
                response,
            } => {
                let r = AnalyzeEnvironment::analyze::<RT>(
                    client_id,
                    isolate,
                    udf_config,
                    modules,
                    environment_variables,
                )
                .await;

                // Don't bother reusing isolates when used for analyze.
                *isolate_clean = false;

                let _ = response.send(r);
                "Analyze".to_string()
            },
            RequestType::EvaluateSchema {
                schema_bundle,
                source_map,
                rng_seed,
                unix_timestamp,
                response,
            } => {
                let r = SchemaEnvironment::evaluate_schema(
                    client_id,
                    isolate,
                    schema_bundle,
                    source_map,
                    rng_seed,
                    unix_timestamp,
                )
                .await;

                // Don't bother reusing isolates when used for schema evaluation.
                *isolate_clean = false;

                let _ = response.send(r);
                "EvaluateSchema".to_string()
            },
            RequestType::EvaluateAuthConfig {
                auth_config_bundle,
                source_map,
                environment_variables,
                response,
            } => {
                let r = AuthConfigEnvironment::evaluate_auth_config(
                    client_id,
                    isolate,
                    auth_config_bundle,
                    source_map,
                    environment_variables,
                )
                .await;
                // Don't bother reusing isolates when used for auth config evaluation.
                *isolate_clean = false;
                let _ = response.send(r);
                "EvaluateAuthConfig".to_string()
            },
            RequestType::EvaluateAppDefinitions {
                app_definition,
                component_definitions,
                dependency_graph,
                response,
            } => {
                let env = AppDefinitionEvaluator::new(
                    app_definition,
                    component_definitions,
                    dependency_graph,
                );
                let r = env.evaluate(client_id, isolate).await;

                // Don't bother reusing isolates when used for auth config evaluation.
                *isolate_clean = false;
                let _ = response.send(r);
                "EvaluateAppDefinitions".to_string()
            },
        }
    }

    fn config(&self) -> &IsolateConfig {
        &self.config
    }

    fn rt(&self) -> RT {
        self.rt.clone()
    }

    #[cfg(any(test, feature = "testing"))]
    fn pause_client(&self) -> Option<Arc<tokio::sync::Mutex<PauseClient>>> {
        self.pause_client.clone()
    }
}

#[cfg(test)]
mod tests {
    use common::pause::PauseController;
    use pb::common::FunctionResult as FunctionResultProto;
    use proptest::prelude::*;
    use runtime::testing::TestRuntime;
    use sync_types::testing::assert_roundtrips;

    use super::FunctionResult;
    use crate::{
        client::PAUSE_RECREATE_CLIENT,
        test_helpers::{
            test_isolate_not_recreated_with_same_client,
            test_isolate_recreated_with_client_change,
        },
        BackendIsolateWorker,
        IsolateConfig,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_function_result_proto_roundtrips(left in any::<FunctionResult>()) {
            assert_roundtrips::<FunctionResult, FunctionResultProto>(left);
        }
    }

    #[convex_macro::test_runtime]
    async fn test_isolate_recreated_with_client_change_backend_worker(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let isolate_config = IsolateConfig::default();
        let (pause, pause_client) = PauseController::new([PAUSE_RECREATE_CLIENT]);
        let worker = BackendIsolateWorker::new_for_tests(rt.clone(), isolate_config, pause_client);
        test_isolate_recreated_with_client_change(rt, worker, pause).await
    }

    #[convex_macro::test_runtime]
    async fn test_isolate_not_recreated_with_same_client_backend_worker(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let isolate_config = IsolateConfig::default();
        let (pause, pause_client) = PauseController::new([PAUSE_RECREATE_CLIENT]);
        let worker = BackendIsolateWorker::new_for_tests(rt.clone(), isolate_config, pause_client);
        test_isolate_not_recreated_with_same_client(rt, worker, pause).await
    }
}
