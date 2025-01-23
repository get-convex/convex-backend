mod async_syscall;
mod fetch;
mod phase;
mod storage;
mod stream;
mod syscall;
mod task;
mod task_executor;
mod task_order;

use std::{
    cmp::Ordering,
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::anyhow;
use common::{
    components::ComponentId,
    errors::JsError,
    execution_context::ExecutionContext,
    fastrace_helpers::EncodedSpan,
    http::{
        fetch::FetchClient,
        RoutedHttpPath,
    },
    knobs::{
        ACTION_USER_TIMEOUT,
        FUNCTION_MAX_ARGS_SIZE,
        FUNCTION_MAX_RESULT_SIZE,
        V8_ACTION_SYSTEM_TIMEOUT,
    },
    log_lines::{
        LogLevel,
        LogLine,
        SystemLogMetadata,
    },
    runtime::{
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    sync::spsc,
    types::{
        HttpActionRoute,
        UdfType,
    },
    value::ConvexValue,
};
use database::Transaction;
use deno_core::v8;
use futures::{
    future::BoxFuture,
    select_biased,
    stream::BoxStream,
    FutureExt,
    Stream,
    StreamExt,
    TryStreamExt,
};
use http::StatusCode;
use humansize::{
    FormatSize,
    BINARY,
};
use itertools::Itertools;
use keybroker::Identity;
use model::{
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    modules::{
        module_versions::FullModuleSource,
        user_error::FunctionNotFoundError,
    },
};
use parking_lot::Mutex;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use sync_types::{
    CanonicalizedUdfPath,
    ModulePath,
};
use tokio::sync::mpsc;
use udf::{
    helpers::serialize_udf_args,
    validation::ValidatedHttpPath,
    ActionOutcome,
    HttpActionOutcome,
    HttpActionRequest,
    HttpActionRequestHead,
    HttpActionResponseHead,
    HttpActionResponsePart,
    HttpActionResponseStreamer,
    HttpActionResult,
    SyscallTrace,
    HTTP_ACTION_BODY_LIMIT,
};
use value::{
    heap_size::HeapSize,
    ConvexArray,
    JsonPackedValue,
    NamespacedTableMapping,
    Size,
    TableMappingValue,
};

pub use self::{
    async_syscall::parse_name_or_reference,
    task::{
        TaskResponse,
        TaskResponseEnum,
    },
};
use self::{
    phase::ActionPhase,
    task::{
        TaskId,
        TaskRequest,
        TaskRequestEnum,
        TaskType,
    },
    task_executor::TaskExecutor,
};
use super::warnings::{
    approaching_duration_limit_warning,
    approaching_limit_warning,
    SystemWarning,
};
use crate::{
    client::{
        ActionRequestParams,
        EnvironmentData,
        SharedIsolateHeapStats,
    },
    concurrency_limiter::ConcurrencyPermit,
    environment::{
        helpers::{
            module_loader::module_specifier_from_path,
            resolve_promise,
            resolve_promise_allow_all_errors,
            MAX_LOG_LINES,
        },
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    helpers::{
        self,
        deserialize_udf_result,
    },
    http::{
        HttpRequestV8,
        HttpResponseV8,
    },
    isolate::{
        Isolate,
        IsolateHeapStats,
    },
    metrics::{
        self,
        log_isolate_request_cancelled,
        log_unawaited_pending_op,
    },
    ops::OpProvider,
    request_scope::{
        RequestScope,
        StreamListener,
    },
    strings,
    termination::{
        IsolateHandle,
        TerminationReason,
    },
    timeout::{
        FunctionExecutionTime,
        Timeout,
    },
    ActionCallbacks,
};

// `CollectResult` starts off as a future that is forever pending,
// so it never triggers the `select_biased!` until we are actually
// collecting a result. Using None would be nice, but `select_biased!`
// does not like Options.
struct CollectResult<'a, T: Send + 'a> {
    has_started: bool,
    result_stream: BoxStream<'a, anyhow::Result<Result<T, JsError>>>,
}

impl<'a, T: Send> CollectResult<'a, T> {
    fn new() -> Self {
        Self {
            has_started: false,
            result_stream: futures::stream::pending().boxed(),
        }
    }

    fn start(&mut self, stream: BoxStream<'a, anyhow::Result<Result<T, JsError>>>) {
        self.has_started = true;
        self.result_stream = stream;
    }
}

pub struct ActionEnvironment<RT: Runtime> {
    identity: Identity,
    total_log_lines: usize,
    log_line_sender: mpsc::UnboundedSender<LogLine>,
    http_response_streamer: Option<HttpActionResponseStreamer>,

    rt: RT,

    next_task_id: TaskId,
    pending_task_sender: spsc::UnboundedSender<TaskRequest>,

    running_tasks: Option<Box<dyn SpawnHandle>>,

    // We have to store PromiseResolvers separate from TaskRequests because
    // TaskRequests will be executed in parallel, but PromiseResolvers are not Send.
    task_promise_resolvers: BTreeMap<TaskId, (v8::Global<v8::PromiseResolver>, TaskType)>,
    task_responses: mpsc::UnboundedReceiver<TaskResponse>,
    phase: ActionPhase<RT>,
    syscall_trace: Arc<Mutex<SyscallTrace>>,
    heap_stats: SharedIsolateHeapStats,
}

impl<RT: Runtime> ActionEnvironment<RT> {
    pub fn new(
        rt: RT,
        component: ComponentId,
        EnvironmentData {
            key_broker,
            system_env_vars,
            file_storage,
            module_loader,
        }: EnvironmentData<RT>,
        identity: Identity,
        transaction: Transaction<RT>,
        action_callbacks: Arc<dyn ActionCallbacks>,
        fetch_client: Arc<dyn FetchClient>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
        http_response_streamer: Option<HttpActionResponseStreamer>,
        heap_stats: SharedIsolateHeapStats,
        context: ExecutionContext,
    ) -> Self {
        let syscall_trace = Arc::new(Mutex::new(SyscallTrace::new()));
        let (task_retval_sender, task_responses) = mpsc::unbounded_channel();
        let resources = Arc::new(Mutex::new(BTreeMap::new()));
        let function_handles = Arc::new(Mutex::new(BTreeMap::new()));
        let task_executor = TaskExecutor {
            rt: rt.clone(),
            identity: identity.clone(),
            file_storage,
            syscall_trace: syscall_trace.clone(),
            action_callbacks,
            fetch_client,
            _module_loader: module_loader.clone(),
            key_broker,
            task_order: Default::default(),
            task_retval_sender,
            usage_tracker: transaction.usage_tracker.clone(),
            context,
            resources: resources.clone(),
            component_id: component,
            function_handles: function_handles.clone(),
        };
        let (pending_task_sender, pending_task_receiver) = spsc::unbounded_channel();
        let running_tasks = rt.spawn("task_executor", task_executor.go(pending_task_receiver));
        Self {
            identity,
            rt: rt.clone(),
            total_log_lines: 0,
            log_line_sender,
            http_response_streamer,

            next_task_id: TaskId(0),
            pending_task_sender,
            task_responses,
            running_tasks: Some(running_tasks),
            task_promise_resolvers: BTreeMap::new(),
            phase: ActionPhase::new(
                rt.clone(),
                component,
                transaction,
                module_loader,
                system_env_vars,
                resources,
                function_handles,
            ),
            syscall_trace,
            heap_stats,
        }
    }

    #[fastrace::trace]
    pub async fn run_http_action(
        mut self,
        client_id: String,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        http_module_path: ValidatedHttpPath,
        routed_path: RoutedHttpPath,
        request: HttpActionRequest,
        cancellation: BoxFuture<'_, ()>,
    ) -> anyhow::Result<HttpActionOutcome> {
        let client_id = Arc::new(client_id);
        let start_unix_timestamp = self.rt.unix_timestamp();

        // Double check that we correctly initialized `ActionEnvironment` with the right
        // component path and then pass a bare `CanonicalizedUdfPath` to
        // `run_http_action_inner`.
        let component_function_path = http_module_path.path();
        anyhow::ensure!(component_function_path.component == self.phase.component());
        let udf_path = &component_function_path.udf_path;

        let heap_stats = self.heap_stats.clone();
        // See Isolate::with_context for an explanation of this setup code. We can't use
        // that method directly since we want an `await` below, and passing in a
        // generic async closure to `Isolate` is currently difficult.
        let (handle, state) = isolate.start_request(client_id.clone(), self).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);

        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, true).await?;

        let request_head = request.head.clone();

        let mut result = Self::run_http_action_inner(
            client_id,
            &mut isolate_context,
            udf_path,
            routed_path,
            request,
            cancellation,
        )
        .await;
        // Override the returned result if we hit a termination error.
        let termination_error = handle
            .take_termination_error(Some(heap_stats.get()), &format!("http action: {udf_path}"));

        // Perform a microtask checkpoint one last time before taking the environment
        // to ensure the microtask queue is empty. Otherwise, JS from this request may
        // leak to a subsequent one on isolate reuse.
        isolate_context.scope.perform_microtask_checkpoint();
        *isolate_clean = true;

        let execution_time;
        (self, execution_time) = isolate_context.take_environment();
        let http_response_streamer = self
            .http_response_streamer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No HTTP response streamer for HTTP action"))?;
        let total_bytes_sent = http_response_streamer.total_bytes_sent();
        match termination_error {
            Ok(Ok(..)) => (),
            Ok(Err(e)) => {
                if !http_response_streamer.has_started() {
                    result = Ok((request_head.route_for_failure(), HttpActionResult::Error(e)));
                } else {
                    Self::handle_http_streamed_part(&mut self, Err(e))?;
                    result = Ok((request_head.route_for_failure(), HttpActionResult::Streamed))
                }
            },
            Err(e) => {
                result = Err(e);
            },
        }
        self.add_warnings_to_log_lines_http_action(execution_time, total_bytes_sent)?;
        let (route, result) = result?;
        let outcome = HttpActionOutcome::new(
            Some(route),
            request_head,
            self.identity.into(),
            start_unix_timestamp,
            result,
            Some(self.syscall_trace.lock().clone()),
            http_module_path.npm_version().clone(),
        );
        Ok(outcome)
    }

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    async fn run_http_action_inner(
        client_id: Arc<String>,
        isolate: &mut RequestScope<'_, '_, RT, Self>,
        http_module_path: &CanonicalizedUdfPath,
        routed_path: RoutedHttpPath,
        http_request: HttpActionRequest,
        cancellation: BoxFuture<'_, ()>,
    ) -> anyhow::Result<(HttpActionRoute, HttpActionResult)> {
        let handle = isolate.handle();
        let mut v8_scope = isolate.scope();
        let mut scope = RequestScope::<RT, Self>::enter(&mut v8_scope);

        {
            let state = scope.state_mut()?;
            state
                .environment
                .phase
                .initialize(&mut state.timeout, &mut state.permit)
                .await?;
        }

        /*
         * Running an HTTP handler is a two-step process.
         * 1) Call `router.lookup()` to find the route name.
         * 2) Call `router.runRequest()` to execute the request.
         *
         * It is the responsibility of the JavaScript `Router` object
         * to ensure that `router.runRequest()` actually routes the request
         * to the same route reported by `router.lookup()`, i.e. it should
         * use `lookup()` in its implementation.
         *
         * The JavaScript `Router` object is application code and cannot be
         * updated after a developer pushes code to a deployment. New NPM packages
         * can implement new behavior in `Router` (and developers can even
         * implement their own `Routers` although this is not recommended)
         * but this interface must be backward compatible.
         */
        let router: Result<_, JsError> =
            Self::get_router(&mut scope, http_module_path.clone()).await?;

        if let Err(e) = router {
            return Ok((
                http_request.head.route_for_failure(),
                HttpActionResult::Error(e),
            ));
        };
        let router = router?;

        let route_lookup = Self::lookup_route(
            &mut scope,
            &router,
            http_module_path.clone(),
            routed_path.clone(),
            http_request.head.clone(),
        )?;
        let route = match route_lookup {
            None => {
                handle.check_terminated()?;
                let state = scope.state_mut()?;
                let environment = &mut state.environment;
                let response_parts = HttpActionResponsePart::from_text(
                    StatusCode::NOT_FOUND,
                    "No matching routes found".into(),
                );
                for part in response_parts {
                    Self::handle_http_streamed_part(environment, Ok(part))?;
                }
                return Ok((
                    http_request.head.route_for_failure(),
                    HttpActionResult::Streamed,
                ));
            },
            Some(route) => route,
        };

        let run_str = strings::runRequest.create(&mut scope)?.into();
        let v8_function: v8::Local<v8::Function> = router
            .get(&mut scope, run_str)
            .ok_or_else(|| {
                anyhow!(
                    "Couldn't find runRequest method of router in {:?}",
                    http_module_path
                )
            })?
            .try_into()?;

        let stream_id = match http_request.body {
            Some(body) => {
                let stream_id = scope.state_mut()?.create_request_stream()?;
                scope
                    .state_mut()?
                    .environment
                    .send_stream(stream_id, Some(body));
                Some(stream_id)
            },
            None => None,
        };
        let request_str =
            serde_json::to_value(HttpRequestV8::from_request(http_request.head, stream_id)?)?
                .to_string();
        metrics::log_argument_length(&request_str);
        let args_v8_str = v8::String::new(&mut scope, &request_str)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;

        // Pass in `request_route` as a second argument so old clients can ignore it if
        // they're not component aware.
        let request_route_v8_str = v8::String::new(&mut scope, &routed_path)
            .ok_or_else(|| anyhow!("Failed to create request route string"))?;

        let v8_args = [args_v8_str.into(), request_route_v8_str.into()];

        let result = Self::run_inner(
            client_id,
            &mut scope,
            handle,
            UdfType::HttpAction,
            v8_function,
            &v8_args,
            cancellation,
            Self::stream_http_result,
            Self::handle_http_streamed_part,
        )
        .await?;
        match result {
            Ok(()) => Ok((route, HttpActionResult::Streamed)),
            Err(e) => Ok((route, HttpActionResult::Error(e))),
        }
    }

    fn stream_http_result<'a, 'b: 'a>(
        scope: &mut ExecutionScope<'a, 'b, RT, Self>,
        result_str: String,
    ) -> anyhow::Result<
        impl Stream<Item = anyhow::Result<Result<HttpActionResponsePart, JsError>>> + 'static,
    > {
        let json_value: JsonValue = serde_json::from_str(&result_str)?;
        let v8_response: HttpResponseV8 = serde_json::from_value(json_value)?;
        let (raw_response, stream_id) = v8_response.into_response()?;
        let (body_sender, body_receiver) = spsc::unbounded_channel();
        match stream_id {
            Some(stream_id) => {
                scope.new_stream_listener(stream_id, StreamListener::RustStream(body_sender))?
            },
            None => drop(body_sender),
        };
        let head = futures::stream::once(async move {
            Ok(Ok(HttpActionResponsePart::Head(HttpActionResponseHead {
                status: raw_response.status,
                headers: raw_response.headers,
            })))
        });

        Ok(head.chain(
            body_receiver
                .into_stream()
                .map_ok(|b| Ok(HttpActionResponsePart::BodyChunk(b))),
        ))
    }

    fn handle_http_streamed_part(
        environment: &mut ActionEnvironment<RT>,
        part: Result<HttpActionResponsePart, JsError>,
    ) -> anyhow::Result<()> {
        let streamer = environment
            .http_response_streamer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No HTTP response streamer for HTTP action"))?;
        match part {
            Ok(HttpActionResponsePart::Head(h)) => {
                streamer.send_part(HttpActionResponsePart::Head(h))?;
            },
            Ok(HttpActionResponsePart::BodyChunk(b)) => {
                if streamer.total_bytes_sent() > HTTP_ACTION_BODY_LIMIT {
                    // We've already hit the body size limit so should not continue sending more
                    return Ok(());
                }
                if streamer.total_bytes_sent() + b.len() > HTTP_ACTION_BODY_LIMIT {
                    let e = JsError::from_message(format!(
                        "HttpResponseTooLarge: HTTP actions support responses up to {}",
                        HTTP_ACTION_BODY_LIMIT.format_size(BINARY)
                    ));
                    environment.trace_system(SystemWarning {
                        level: LogLevel::Error,
                        messages: vec![e.to_string()],
                        system_log_metadata: SystemLogMetadata {
                            code: "error:httpAction".to_string(),
                        },
                    })?;
                } else {
                    streamer.send_part(HttpActionResponsePart::BodyChunk(b))?;
                }
            },
            Err(e) => environment.trace_system(SystemWarning {
                level: LogLevel::Error,
                messages: vec![e.to_string()],
                system_log_metadata: SystemLogMetadata {
                    code: "error:httpAction".to_string(),
                },
            })?,
        };
        Ok(())
    }

    fn send_stream(
        &mut self,
        stream_id: uuid::Uuid,
        stream: Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
    ) {
        let task_id = self.next_task_id.increment();
        self.pending_task_sender
            .send(TaskRequest {
                task_id,
                variant: TaskRequestEnum::AsyncOp(AsyncOpRequest::SendStream { stream, stream_id }),
                parent_trace: EncodedSpan::from_parent(),
            })
            .expect("TaskExecutor went away?");
    }

    #[fastrace::trace]
    pub async fn run_action(
        mut self,
        client_id: String,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        request_params: ActionRequestParams,
        cancellation: BoxFuture<'_, ()>,
    ) -> anyhow::Result<ActionOutcome> {
        let client_id = Arc::new(client_id);
        let start_unix_timestamp = self.rt.unix_timestamp();
        let heap_stats = self.heap_stats.clone();

        // See Isolate::with_context for an explanation of this setup code. We can't use
        // that method directly since we want an `await` below, and passing in a
        // generic async closure to `Isolate` is currently difficult.

        let (handle, state) = isolate.start_request(client_id.clone(), self).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);

        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, true).await?;

        let mut result = Self::run_action_inner(
            client_id,
            &mut isolate_context,
            request_params.clone(),
            cancellation,
        )
        .await;

        // Perform a microtask checkpoint one last time before taking the environment
        // to ensure the microtask queue is empty. Otherwise, JS from this request may
        // leak to a subsequent one on isolate reuse.
        isolate_context.scope.perform_microtask_checkpoint();
        *isolate_clean = true;

        match handle.take_termination_error(
            Some(heap_stats.get()),
            &format!(
                "{:?}",
                request_params.path_and_args.path().clone().for_logging()
            ),
        ) {
            Ok(Ok(..)) => (),
            Ok(Err(e)) => {
                result = Ok(Err(e));
            },
            Err(e) => {
                result = Err(e);
            },
        }
        let execution_time;
        (self, execution_time) = isolate_context.take_environment();
        let (path, arguments, udf_server_version) = request_params.path_and_args.consume();
        self.add_warnings_to_log_lines_action(
            execution_time,
            &arguments,
            result.as_ref().ok().and_then(|r| r.as_ref().ok()),
        )?;
        let outcome = ActionOutcome {
            path: path.for_logging(),
            arguments,
            unix_timestamp: start_unix_timestamp,
            identity: self.identity.into(),
            result: match result? {
                Ok(v) => Ok(JsonPackedValue::pack(v)),
                Err(e) => Err(e),
            },
            syscall_trace: self.syscall_trace.lock().clone(),
            udf_server_version,
        };
        Ok(outcome)
    }

    #[fastrace::trace]
    async fn run_action_inner(
        client_id: Arc<String>,
        isolate: &mut RequestScope<'_, '_, RT, Self>,
        request_params: ActionRequestParams,
        cancellation: BoxFuture<'_, ()>,
    ) -> anyhow::Result<Result<ConvexValue, JsError>> {
        let handle = isolate.handle();
        let mut v8_scope = isolate.scope();
        let mut scope = RequestScope::<RT, Self>::enter(&mut v8_scope);
        {
            let state = scope.state_mut()?;
            state
                .environment
                .phase
                .initialize(&mut state.timeout, &mut state.permit)
                .await?;
        }
        let (path, arguments, _) = request_params.path_and_args.consume();

        // Don't allow directly running a UDF within the `_deps` directory. We don't
        // really expect users to hit this unless someone is trying to exploit
        // an app written on Convex by calling directly into a compromised
        // dependency. So, consider it a system error so we can just
        // keep a watch on it.
        if path.udf_path.module().is_deps() {
            anyhow::bail!(
                "Refusing to run {:?} within the '_deps' directory",
                path.udf_path
            );
        }

        // First, load the user's module and find the specified function.
        let module_path = path.udf_path.module();
        let Ok(module_specifier) = module_specifier_from_path(module_path) else {
            let message = format!("Invalid module path: {module_path:?}");
            return Ok(Err(JsError::from_message(message)));
        };

        let module = match scope
            .eval_user_module(UdfType::Action, false, &module_specifier)
            .await?
        {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };
        let namespace = module
            .get_module_namespace()
            .to_object(&mut scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let function_name = path.udf_path.function_name();
        let function_str: v8::Local<'_, v8::Value> = v8::String::new(&mut scope, function_name)
            .ok_or_else(|| anyhow!("Failed to create function name string"))?
            .into();

        if namespace.has(&mut scope, function_str) != Some(true) {
            let message = format!(
                "{}",
                FunctionNotFoundError::new(function_name, path.udf_path.module().as_str())
            );
            return Ok(Err(JsError::from_message(message)));
        }
        let function: v8::Local<v8::Object> = namespace
            .get(&mut scope, function_str)
            .ok_or_else(|| anyhow!("Did not find function in module after checking?"))?
            .try_into()?;

        let run_str = strings::invokeAction.create(&mut scope)?.into();
        let v8_function: v8::Local<v8::Function> = function
            .get(&mut scope, run_str)
            .ok_or_else(|| anyhow!("Couldn't find invoke function in {:?}", path.udf_path))?
            .try_into()?;
        let args_str = serialize_udf_args(arguments)?;
        metrics::log_argument_length(&args_str);
        let args_v8_str = v8::String::new(&mut scope, &args_str)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;
        // TODO(rebecca): generate uuid4 here
        let request_id_str = "dummy_request_id";
        let request_id_v8_str = v8::String::new(&mut scope, request_id_str)
            .ok_or_else(|| anyhow!("Failed to create request id string"))?;
        let v8_args = [request_id_v8_str.into(), args_v8_str.into()];

        let mut result = None;

        let run_inner_result = Self::run_inner(
            client_id,
            &mut scope,
            handle,
            UdfType::Action,
            v8_function,
            &v8_args,
            cancellation,
            |_, result_str| {
                let result = deserialize_udf_result(&path, &result_str)?;
                Ok(futures::stream::once(async move { Ok(result) }))
            },
            |_, r| {
                result = Some(r);
                Ok(())
            },
        )
        .await?;

        match run_inner_result {
            Ok(()) => (),
            Err(e) => result = Some(Err(e)),
        }
        result.ok_or_else(|| anyhow::anyhow!("`run_inner` did not populate a result"))
    }

    #[fastrace::trace]
    fn lookup_route(
        scope: &mut ExecutionScope<RT, Self>,
        router: &v8::Local<v8::Object>,
        http_module_path: CanonicalizedUdfPath,
        routed_path: RoutedHttpPath,
        http_request: HttpActionRequestHead,
    ) -> anyhow::Result<Option<HttpActionRoute>> {
        let lookup_str = strings::lookup.create(scope)?.into();
        let routed_path_str = v8::String::new(scope, &routed_path)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;
        let method_str = v8::String::new(scope, http_request.method.as_str())
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;

        let lookup: v8::Local<v8::Function> = router
            .get(scope, lookup_str)
            .ok_or_else(|| {
                anyhow!(
                    "Couldn't find lookup method of router in {:?}",
                    http_module_path
                )
            })?
            .try_into()?;
        let global = scope.get_current_context().global(scope);
        let r = scope
            .with_try_catch(|s| {
                lookup.call(
                    s,
                    global.into(),
                    &[routed_path_str.into(), method_str.into()],
                )
            })??
            .expect("lookup.call() returned None");
        if r.is_null() {
            return Ok(None);
        }

        // function lookup(path: string, method: string): [handler, method, path] | null
        // Drop the handler result at index 0 of the return value on the floor here,
        // it is only part of the return type so that `runRequest` can use it when
        // it calls `lookup` from JavaScript.
        let lookup_result = r.to_object(scope).expect("lookup result");

        let route_method: v8::Local<v8::String> = lookup_result
            .get_index(scope, 1)
            .expect("Failed to get index 1 of lookup result")
            .try_into()?;
        let route_method_s = helpers::to_rust_string(scope, &route_method)?;
        let route_path: v8::Local<v8::String> = lookup_result
            .get_index(scope, 2)
            .expect("Failed to get index 2 of lookup result")
            .try_into()?;
        let route_path_s = helpers::to_rust_string(scope, &route_path)?;

        Ok(Some(format!("{route_method_s} {route_path_s}").parse()?))
    }

    async fn get_router<'a, 'b: 'a>(
        scope: &mut ExecutionScope<'a, 'b, RT, Self>,
        http_module_path: CanonicalizedUdfPath,
    ) -> anyhow::Result<Result<v8::Local<'a, v8::Object>, JsError>> {
        // Except in tests, `http.js` will always be the udf_path.
        // We'll never hit these as long as this HTTP path only runs for
        // `convex/http.js`.
        if http_module_path.module().is_deps() {
            anyhow::bail!("Refusing to run {http_module_path:?} within the '_deps' directory");
        }

        // First, load the user's module and find the specified function.
        let module_path = http_module_path.module().clone();
        let Ok(module_specifier) = module_specifier_from_path(&module_path) else {
            let message = format!("Invalid module path: {module_path:?}");
            return Ok(Err(JsError::from_message(message)));
        };

        let module = match scope
            .eval_user_module(UdfType::HttpAction, false, &module_specifier)
            .await?
        {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };
        let namespace = module
            .get_module_namespace()
            .to_object(scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let export_name = "default";
        let export_str: v8::Local<'_, v8::Value> = v8::String::new(scope, export_name)
            .ok_or_else(|| anyhow!("Failed to create function name string"))?
            .into();

        if namespace.has(scope, export_str) != Some(true) {
            let message = format!(
                r#"Couldn't find default export in module "{:?}"."#,
                http_module_path.module()
            );
            return Ok(Err(JsError::from_message(message)));
        }
        let router: v8::Local<v8::Object> = namespace
            .get(scope, export_str)
            .ok_or_else(|| anyhow!("Did not find router in module"))?
            .try_into()?;

        let is_router_str = strings::isRouter.create(scope)?.into();
        let mut is_router = false;
        if let Some(true) = router.has(scope, is_router_str) {
            is_router = router
                .get(scope, is_router_str)
                .ok_or_else(|| anyhow!("Missing `isRouter` after explicit check"))?
                .is_true();
        }

        if !is_router {
            let message = "The default export of `convex/http.js` is not a Router.".to_string();
            Ok(Err(JsError::from_message(message)))
        } else {
            Ok(Ok(router))
        }
    }

    /// This method is shared between HTTP and non-HTTP actions with
    /// functionality injected via `get_result_stream` and
    /// `handle_result_part`.
    ///
    /// In particular, HTTP actions allow streaming the response while normal
    /// actions do not.
    ///
    /// The outer `Result` in the return type holds any system errors while the
    /// inner `Result` holds any developer errors (JsErrors) that happen
    /// before collecting the result.
    ///
    /// Errors from collecting the result will be surfaced via
    /// `get_result_stream` -> `handle_result_part`
    #[fastrace::trace]
    async fn run_inner<'a, 'b: 'a, T, S>(
        client_id: Arc<String>,
        scope: &mut ExecutionScope<'a, 'b, RT, Self>,
        handle: IsolateHandle,
        udf_type: UdfType,
        v8_function: v8::Local<'_, v8::Function>,
        v8_args: &[v8::Local<'_, v8::Value>],
        cancellation: BoxFuture<'_, ()>,
        get_result_stream: impl FnOnce(
            &mut ExecutionScope<'a, 'b, RT, Self>,
            String,
        ) -> anyhow::Result<S>,
        mut handle_result_part: impl FnMut(
            &mut ActionEnvironment<RT>,
            Result<T, JsError>,
        ) -> anyhow::Result<()>,
    ) -> anyhow::Result<Result<(), JsError>>
    where
        T: Send,
        S: Stream<Item = anyhow::Result<Result<T, JsError>>> + Send + 'static,
    {
        // Switch our phase to executing right before calling into the UDF.
        {
            let state = scope.state_mut()?;
            // This enforces on database access in the router.
            // We might relax this to e.g. implement a JavaScript router with
            // auth middleware which affected the matched route.
            state.environment.phase.begin_execution()?;
        }
        let global = scope.get_current_context().global(scope);
        let promise_r = scope.with_try_catch(|s| v8_function.call(s, global.into(), v8_args));
        // If we hit a system error within a syscall, return `Err`, even if JS thinks it
        // returned successfully. The syscall layer uses
        // `scope.terminate_execution()` when we hit a system error, which
        // unfortunately doesn't actually terminate execution immediately. So, it's
        // possible for JS after the failed syscall to keep running and return a result
        // here before checking the termination flag.
        handle.check_terminated()?;
        let promise: v8::Local<v8::Promise> = match promise_r? {
            Ok(Some(v)) => v.try_into()?,
            Ok(None) => anyhow::bail!("Successful invocation returned None"),
            Err(e) => {
                return Ok(Err(e));
            },
        };
        let mut get_result_stream = Some(get_result_stream);

        let mut collecting_result = CollectResult::new();
        let mut cancellation = cancellation.fuse();
        let result: Result<(), JsError> = loop {
            // Advance the user's promise as far as it can go by draining the microtask
            // queue.
            scope.perform_microtask_checkpoint();
            scope.record_heap_stats()?;
            let request_stream_state = scope.state()?.request_stream_state.as_ref();
            if let Some(request_stream_state) = request_stream_state {
                handle.update_request_stream_bytes(request_stream_state.bytes_read());
            }
            handle.check_terminated()?;

            // Check for rejected promises still unhandled, if so terminate.
            let rejections = scope.pending_unhandled_promise_rejections_mut();
            if let Some(promise) = rejections.exceptions.keys().next().cloned() {
                let error = rejections.exceptions.remove(&promise).unwrap();

                let as_local = v8::Local::new(scope, error);
                let err = match scope.format_traceback(as_local) {
                    Ok(e) => e,
                    Err(e) => {
                        handle.terminate_and_throw(TerminationReason::SystemError(Some(e)))?;
                    },
                };
                handle.terminate_and_throw(TerminationReason::UnhandledPromiseRejection(err))?;
            }

            // Check for dynamic import requests.
            let dynamic_imports = {
                let pending_dynamic_imports = scope.pending_dynamic_imports_mut();
                pending_dynamic_imports.take()
            };
            if !dynamic_imports.is_empty() {
                for (specifier, resolver) in dynamic_imports {
                    match scope.eval_user_module(udf_type, true, &specifier).await? {
                        Ok(module) => {
                            let namespace = module.get_module_namespace();
                            resolve_promise(scope, resolver, Ok(namespace))?;
                        },
                        Err(e) => {
                            resolve_promise(scope, resolver, Err(anyhow::anyhow!(e)))?;
                        },
                    }
                }
                // Go back to the top and perform a microtask checkpoint now that we know we've
                // made progress.
                continue;
            }

            // Check to see if the user's code is blocked.
            match promise.state() {
                v8::PromiseState::Pending => (),
                v8::PromiseState::Fulfilled => {
                    // Call `collect_result` if we haven't already done so, and advance the future
                    // `collecting_result` it returns. If the future is pending,
                    // proceed as if the js future hasn't resolved.
                    // If the future is done, that's our result.
                    if let Some(get_result_stream) = get_result_stream.take() {
                        let promise_result_v8 = promise.result(scope);
                        let result_v8_str: v8::Local<v8::String> = promise_result_v8.try_into()?;
                        let result_str = helpers::to_rust_string(scope, &result_v8_str)?;
                        metrics::log_result_length(&result_str);
                        collecting_result.start(get_result_stream(scope, result_str)?.boxed());
                        // collect_result may have fulfilled promises, so we can go back to
                        // JS now.
                        continue;
                    }
                },
                v8::PromiseState::Rejected => {
                    let e = promise.result(scope);
                    let err = scope.format_traceback(e)?;
                    if collecting_result.has_started {
                        let state = scope.state_mut()?;
                        let environment = &mut state.environment;
                        handle_result_part(environment, Err(err))?;
                        break Ok(());
                    } else {
                        break Err(err);
                    }
                },
            };

            // If the user's promise is blocked, something must be pending:
            // 1. An async syscall, in which case we can execute one syscall before
            //    reentering into JS.
            // 2. Collecting_result, so we try to advance that.
            // 3. In case collecting_result or syscalls are taking too long or deadlocked,
            //    we should timeout.
            let (timeout, permit) = scope.with_state_mut(|state| {
                let timeout = state.timeout.wait_until_completed();
                // Release the permit while we wait on task executor.
                let permit = state
                    .permit
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Running function without permit"))?;
                anyhow::Ok((timeout, permit))
            })??;
            let limiter = permit.limiter().clone();
            drop(permit);

            let environment = &mut scope.state_mut()?.environment;
            select_biased! {
                result = collecting_result.result_stream.next().fuse() => {
                    match result {
                        None => break Ok(()),
                        Some(inner_result) => {
                            handle_result_part(environment, inner_result?)?;
                        }
                    }
                },
                // Normally we'd pause the user-code timeout for the duration of
                // the syscall.
                // However, actions can call queries, mutations, and other actions
                // as syscalls, so these should still count towards the user-code
                // timeout.
                task_response = environment.task_responses.recv().fuse() => {
                    let Some(task_response) = task_response else {
                        anyhow::bail!("Task executor went away?");
                    };
                    match task_response {
                        TaskResponse::StreamExtend { stream_id, chunk } => {
                            match chunk {
                                Ok(chunk) => {
                                    let done = chunk.is_none();
                                    scope.extend_stream(stream_id, chunk, done)?;
                                    // If done, add the total accumulated size to the isolate handle inner.
                                },
                                Err(e) => scope.error_stream(stream_id, e)?,
                            };
                        },
                        TaskResponse::TaskDone { task_id, variant } => {
                            let Some((resolver, _)) = environment
                                .task_promise_resolvers
                                .remove(&task_id) else {
                                    anyhow::bail!("Task with id {} did not have a promise", task_id);
                                };
                            let mut result_scope = v8::HandleScope::new(&mut **scope);
                            let result_v8 = match variant {
                                Ok(v) => Ok(v.into_v8(&mut result_scope)?),
                                Err(e) => Err(e),
                            };
                            resolve_promise_allow_all_errors(
                                &mut result_scope,
                                resolver,
                                result_v8,
                            )?;
                        },
                    };
                },
                // If we the isolate is terminated due to timeout, we start the
                // isolate loop over to run js to handle the timeout.
                _ = timeout.fuse() => {
                    continue;
                },
                _ = cancellation => {
                    log_isolate_request_cancelled();
                    anyhow::bail!("Cancelled");
                },
            }
            let permit_acquire = scope.with_state_mut(|state| {
                state
                    .timeout
                    .with_timeout(limiter.acquire(client_id.clone()))
            })?;
            let permit = permit_acquire.await?;
            scope.with_state_mut(|state| state.permit = Some(permit))?;
            handle.check_terminated()?;
        };
        // Drain all remaining async syscalls that are not sleeps in case the
        // developer forgot to await them.
        let environment = &mut scope.state_mut()?.environment;
        environment.pending_task_sender.close();
        if let Some(mut running_tasks) = environment.running_tasks.take() {
            running_tasks.shutdown();
        }
        Ok(result)
    }

    fn add_warnings_to_log_lines_action(
        &mut self,
        execution_time: FunctionExecutionTime,
        arguments: &ConvexArray,
        result: Option<&ConvexValue>,
    ) -> anyhow::Result<()> {
        if let Some(warning) = approaching_limit_warning(
            arguments.size(),
            *FUNCTION_MAX_ARGS_SIZE,
            "FunctionArgumentsTooLarge",
            || "Large size of the action arguments".to_string(),
            None,
            Some(" bytes"),
            None,
        )? {
            self.trace_system(warning)?;
        }
        self.add_warnings_to_log_lines(execution_time)?;

        if let Some(result) = result {
            if let Some(warning) = approaching_limit_warning(
                result.size(),
                *FUNCTION_MAX_RESULT_SIZE,
                "TooLargeFunctionResult",
                || "Large size of the action return value".to_string(),
                None,
                Some(" bytes"),
                None,
            )? {
                self.trace_system(warning)?;
            }
        };
        Ok(())
    }

    fn add_warnings_to_log_lines_http_action(
        &mut self,
        execution_time: FunctionExecutionTime,
        total_bytes_sent: usize,
    ) -> anyhow::Result<()> {
        if let Some(warning) = approaching_limit_warning(
            total_bytes_sent,
            HTTP_ACTION_BODY_LIMIT,
            "HttpResponseTooLarge",
            || "Large response returned from an HTTP action".to_string(),
            None,
            Some(" bytes"),
            None,
        )? {
            self.trace_system(warning)?;
        }
        self.add_warnings_to_log_lines(execution_time)?;
        Ok(())
    }

    fn add_warnings_to_log_lines(
        &mut self,
        execution_time: FunctionExecutionTime,
    ) -> anyhow::Result<()> {
        let dangling_task_counts = self.dangling_task_counts();
        if !dangling_task_counts.is_empty() {
            let total_dangling_tasks = dangling_task_counts.values().sum();
            let task_names = dangling_task_counts.keys().join(", ");
            log_unawaited_pending_op(total_dangling_tasks, "action");
            let message = format!(
                "{total_dangling_tasks} unawaited operation{}: [{task_names}]. Async operations should be awaited or they might not run. \
                 See https://docs.convex.dev/functions/actions#dangling-promises for more information.",
                if total_dangling_tasks == 1 { "" } else { "s" },
            );
            let warning = SystemWarning {
                level: LogLevel::Warn,
                messages: vec![message],
                system_log_metadata: SystemLogMetadata {
                    code: "UnawaitedOperations".to_string(),
                },
            };
            self.trace_system(warning)?;
        }
        if let Some(warning) = approaching_duration_limit_warning(
            execution_time.elapsed,
            execution_time.limit,
            "UserTimeout",
            "Function execution took a long time",
            None,
        )? {
            self.trace_system(warning)?;
        }
        Ok(())
    }

    fn dangling_task_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for (_, req) in self
            .task_promise_resolvers
            .values()
            .filter(|(_, req)| !matches!(req, TaskType::Sleep))
        {
            let req_name = req.name_when_dangling();
            *counts.entry(req_name).or_default() += 1;
        }
        counts
    }

    fn start_task(
        &mut self,
        request: TaskRequestEnum,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        self.phase.require_executing(&request)?;
        let task_id = self.next_task_id.increment();
        self.task_promise_resolvers
            .insert(task_id, (resolver, request.to_type()));
        self.pending_task_sender
            .send(TaskRequest {
                task_id,
                variant: request,
                parent_trace: EncodedSpan::from_parent(),
            })
            .expect("TaskExecutor went away?");
        Ok(())
    }

    fn trace_system(&mut self, warning: SystemWarning) -> anyhow::Result<()> {
        self.log_line_sender.send(LogLine::new_system_log_line(
            warning.level,
            warning.messages,
            self.rt.unix_timestamp(),
            warning.system_log_metadata,
        ))?;
        Ok(())
    }
}

impl<RT: Runtime> IsolateEnvironment<RT> for ActionEnvironment<RT> {
    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        // - 1 to reserve for the [ERROR] log line

        match self.total_log_lines.cmp(&(MAX_LOG_LINES - 1)) {
            Ordering::Less => {
                self.log_line_sender.send(LogLine::new_developer_log_line(
                    level,
                    messages,
                    self.rt.unix_timestamp(),
                ))?;
                self.total_log_lines += 1;
            },
            Ordering::Equal => {
                // Add a message about omitting log lines once
                self.log_line_sender.send(LogLine::new_developer_log_line(
                    LogLevel::Error,
                    vec![format!(
                        "Log overflow (maximum {MAX_LOG_LINES}). Remaining log lines omitted."
                    )],
                    self.rt.unix_timestamp(),
                ))?;
                self.total_log_lines += 1;
            },
            Ordering::Greater => (),
        };
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        self.phase.rng()
    }

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        self.phase.unix_timestamp()
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        self.phase.get_environment_variable(name)
    }

    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
        anyhow::bail!("get_table_mapping_without_system_tables unsupported in actions")
    }

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping> {
        anyhow::bail!("get_all_table_mappings unsupported in actions")
    }

    // We lookup all modules' sources upfront when initializing the action
    // environment, so this function always returns immediately.
    async fn lookup_source(
        &mut self,
        path: &str,
        timeout: &mut Timeout<RT>,
        permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>> {
        let user_module_path: ModulePath = path.parse()?;
        let result = self.phase.get_module(&user_module_path, timeout, permit)?;
        Ok(result)
    }

    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue> {
        self.syscall_impl(name, args)
    }

    fn start_async_syscall(
        &mut self,
        name: String,
        args: JsonValue,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        self.start_task(TaskRequestEnum::AsyncSyscall { name, args }, resolver)
    }

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        self.start_task(TaskRequestEnum::AsyncOp(request), resolver)
    }

    fn record_heap_stats(&self, mut isolate_stats: IsolateHeapStats) {
        // Add the memory allocated by the environment itself.
        isolate_stats.environment_heap_size = self.syscall_trace.lock().heap_size();
        self.heap_stats.store(isolate_stats);
    }

    fn user_timeout(&self) -> std::time::Duration {
        *ACTION_USER_TIMEOUT
    }

    fn system_timeout(&self) -> std::time::Duration {
        *V8_ACTION_SYSTEM_TIMEOUT
    }
}
