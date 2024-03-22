mod async_syscall;
mod fetch;
pub mod outcome;
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
    errors::{
        JsError,
        INTERNAL_SERVER_ERROR,
    },
    execution_context::ExecutionContext,
    http::fetch::FetchClient,
    knobs::{
        ACTION_USER_TIMEOUT,
        FUNCTION_MAX_ARGS_SIZE,
        FUNCTION_MAX_RESULT_SIZE,
        ISOLATE_MAX_USER_HEAP_SIZE,
        V8_ACTION_SYSTEM_TIMEOUT,
    },
    log_lines::{
        LogLine,
        TRUNCATED_LINE_SUFFIX,
    },
    runtime::{
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    types::HttpActionRoute,
    value::ConvexValue,
};
use database::Transaction;
use deno_core::v8;
use futures::{
    channel::mpsc,
    select_biased,
    stream::BoxStream,
    Future,
    FutureExt,
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
    modules::module_versions::{
        ModuleSource,
        SourceMap,
    },
};
use parking_lot::Mutex;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use sync_types::{
    CanonicalizedUdfPath,
    ModulePath,
};
use value::{
    heap_size::HeapSize,
    ConvexArray,
    Size,
    TableMapping,
    TableMappingValue,
    VirtualTableMapping,
};

use self::{
    outcome::{
        ActionOutcome,
        HttpActionOutcome,
    },
    phase::ActionPhase,
    task::{
        TaskId,
        TaskRequest,
        TaskRequestEnum,
        TaskResponse,
        TaskType,
    },
    task_executor::TaskExecutor,
};
use super::warnings::{
    warning_if_approaching_duration_limit,
    warning_if_approaching_limit,
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
            validation::ValidatedHttpPath,
            JsonPackedValue,
            SyscallTrace,
            MAX_LOG_LINES,
            MAX_LOG_LINE_LENGTH,
        },
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    helpers::{
        self,
        deserialize_udf_result,
        serialize_udf_args,
    },
    http::{
        HttpRequestV8,
        HttpResponseV8,
    },
    http_action::{
        HttpActionRequest,
        HttpActionResponse,
    },
    isolate::{
        Isolate,
        IsolateHeapStats,
    },
    metrics::{
        self,
        log_unawaited_pending_op,
    },
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
    FunctionNotFoundError,
    HttpActionRequestHead,
    HTTP_ACTION_BODY_LIMIT,
};

pub struct ActionEnvironment<RT: Runtime> {
    identity: Identity,
    total_log_lines: usize,
    log_line_sender: mpsc::UnboundedSender<LogLine>,

    rt: RT,

    next_task_id: TaskId,
    pending_task_sender: mpsc::UnboundedSender<TaskRequest>,

    running_tasks: Option<RT::Handle>,

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
        heap_stats: SharedIsolateHeapStats,
        context: ExecutionContext,
    ) -> Self {
        let syscall_trace = Arc::new(Mutex::new(SyscallTrace::new()));
        let (task_retval_sender, task_responses) = mpsc::unbounded();
        let task_executor = TaskExecutor {
            rt: rt.clone(),
            identity: identity.clone(),
            file_storage,
            syscall_trace: syscall_trace.clone(),
            action_callbacks,
            fetch_client,
            module_loader: module_loader.clone(),
            key_broker,
            task_order: Default::default(),
            task_retval_sender,
            usage_tracker: transaction.usage_tracker.clone(),
            context,
        };
        let (pending_task_sender, pending_task_receiver) = mpsc::unbounded();
        let running_tasks = rt.spawn("task_executor", task_executor.go(pending_task_receiver));
        Self {
            identity,
            rt: rt.clone(),
            total_log_lines: 0,
            log_line_sender,

            next_task_id: TaskId(0),
            pending_task_sender,
            task_responses,
            running_tasks: Some(running_tasks),
            task_promise_resolvers: BTreeMap::new(),
            phase: ActionPhase::new(rt.clone(), transaction, module_loader, system_env_vars),
            syscall_trace,
            heap_stats,
        }
    }

    pub async fn run_http_action(
        mut self,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        validated_path: ValidatedHttpPath,
        request: HttpActionRequest,
    ) -> anyhow::Result<HttpActionOutcome> {
        let start_unix_timestamp = self.rt.unix_timestamp();

        // See Isolate::with_context for an explanation of this setup code. We can't use
        // that method directly since we want an `await` below, and passing in a
        // generic async closure to `Isolate` is currently difficult.
        let (handle, state) = isolate.start_request(self).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);

        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, true).await?;

        let request_head = request.head.clone();
        let mut result = Self::run_http_action_inner(
            &mut isolate_context,
            validated_path.canonicalized_udf_path(),
            request,
        )
        .await;
        // Override the returned result if we hit a termination error.
        match handle.take_termination_error() {
            Ok(Ok(..)) => (),
            Ok(Err(e)) => {
                result = Ok((request_head.route_for_failure()?, Err(e)));
            },
            Err(e) => {
                result = Err(e);
            },
        }

        // Perform a microtask checkpoint one last time before taking the environment
        // to ensure the microtask queue is empty. Otherwise, JS from this request may
        // leak to a subsequent one on isolate reuse.
        isolate_context.scope.perform_microtask_checkpoint();
        *isolate_clean = true;

        let execution_time;
        (self, execution_time) = isolate_context.take_environment();
        self.add_warnings_to_log_lines_http_action(
            execution_time,
            result
                .as_ref()
                .ok()
                .and_then(|(_, response)| response.as_ref().ok()),
        )?;
        let (route, result) = result?;
        let outcome = HttpActionOutcome {
            route,
            http_request: request_head,
            unix_timestamp: start_unix_timestamp,
            identity: self.identity.into(),
            result,
            syscall_trace: self.syscall_trace.lock().clone(),
            udf_server_version: validated_path.npm_version().clone(),
            memory_in_mb: (*ISOLATE_MAX_USER_HEAP_SIZE / (1 << 20))
                .try_into()
                .unwrap(),
        };
        Ok(outcome)
    }

    #[convex_macro::instrument_future]
    async fn run_http_action_inner(
        isolate: &mut RequestScope<'_, '_, RT, Self>,
        router_path: &CanonicalizedUdfPath,
        http_request: HttpActionRequest,
    ) -> anyhow::Result<(HttpActionRoute, Result<HttpActionResponse, JsError>)> {
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
        let router: Result<_, JsError> = Self::get_router(&mut scope, router_path.clone()).await?;

        if let Err(e) = router {
            return Ok((http_request.head.route_for_failure()?, Err(e)));
        };
        let router = router?;

        let route_lookup = Self::lookup_route(
            &mut scope,
            &router,
            router_path.clone(),
            http_request.head.clone(),
        )?;
        let route = match route_lookup {
            None => {
                handle.check_terminated()?;
                return Ok((
                    http_request.head.route_for_failure()?,
                    Ok(HttpActionResponse::from_text(
                        StatusCode::NOT_FOUND,
                        "No matching routes found".into(),
                    )),
                ));
            },
            Some(route) => route,
        };

        let run_str = strings::runRequest.create(&mut scope)?.into();
        let v8_function: v8::Local<v8::Function> = router
            .get(&mut scope, run_str)
            .ok_or_else(|| anyhow!("Couldn't find runRequest method of router in {router_path:?}"))?
            .try_into()?;

        let stream_id = match http_request.body {
            Some(body) => {
                let stream_id = scope.state_mut()?.create_stream()?;
                scope
                    .state_mut()?
                    .environment
                    .send_stream(stream_id, Some(body));
                Some(stream_id)
            },
            None => None,
        };
        let args_str =
            serde_json::to_value(HttpRequestV8::from_request(http_request.head, stream_id)?)?
                .to_string();
        metrics::log_argument_length(&args_str);
        let args_v8_str = v8::String::new(&mut scope, &args_str)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;
        let v8_args = [args_v8_str.into()];

        let result = Self::run_inner(
            &mut scope,
            handle,
            v8_function,
            &v8_args,
            Self::collect_http_result,
        )
        .await?;
        Ok((route, result))
    }

    fn collect_http_result<'a, 'b: 'a>(
        scope: &mut ExecutionScope<'a, 'b, RT, Self>,
        result_str: String,
    ) -> anyhow::Result<
        impl Future<Output = anyhow::Result<Result<HttpActionResponse, JsError>>> + 'static,
    > {
        let json_value: JsonValue = serde_json::from_str(&result_str)?;
        let v8_response: HttpResponseV8 = serde_json::from_value(json_value)?;
        let (mut raw_response, stream_id) = v8_response.into_response()?;
        let (body_sender, mut body_receiver) = mpsc::unbounded();
        match stream_id {
            Some(stream_id) => {
                scope.new_stream_listener(stream_id, StreamListener::RustStream(body_sender))?
            },
            None => body_sender.close_channel(),
        };
        Ok(async move {
            let mut body = Vec::new();
            while let Some(chunk) = TryStreamExt::try_next(&mut body_receiver).await? {
                body.extend(chunk.into_iter());
            }
            raw_response.body = Some(body);
            match &raw_response.body {
                Some(body) if body.len() > HTTP_ACTION_BODY_LIMIT => {
                    Ok(Err(JsError::from_message(format!(
                        "{INTERNAL_SERVER_ERROR}: HTTP actions support responses up to {} \
                         (returned response was {} bytes)",
                        HTTP_ACTION_BODY_LIMIT.format_size(BINARY),
                        body.len().format_size(BINARY),
                    ))))
                },
                _ => Ok(Ok(HttpActionResponse::from_http_response(raw_response))),
            }
        })
    }

    fn send_stream(
        &mut self,
        stream_id: uuid::Uuid,
        stream: Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
    ) {
        let task_id = self.next_task_id.increment();
        self.pending_task_sender
            .unbounded_send(TaskRequest {
                task_id,
                variant: TaskRequestEnum::AsyncOp(AsyncOpRequest::SendStream { stream, stream_id }),
            })
            .expect("TaskExecutor went away?");
    }

    pub async fn run_action(
        mut self,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        request_params: ActionRequestParams,
    ) -> anyhow::Result<ActionOutcome> {
        let start_unix_timestamp = self.rt.unix_timestamp();

        // See Isolate::with_context for an explanation of this setup code. We can't use
        // that method directly since we want an `await` below, and passing in a
        // generic async closure to `Isolate` is currently difficult.
        let (handle, state) = isolate.start_request(self).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);

        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, true).await?;

        let mut result = Self::run_action_inner(&mut isolate_context, request_params.clone()).await;

        // Perform a microtask checkpoint one last time before taking the environment
        // to ensure the microtask queue is empty. Otherwise, JS from this request may
        // leak to a subsequent one on isolate reuse.
        isolate_context.scope.perform_microtask_checkpoint();
        *isolate_clean = true;

        match handle.take_termination_error() {
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
        let (udf_path, arguments, udf_server_version) = request_params.path_and_args.consume();
        self.add_warnings_to_log_lines_action(
            execution_time,
            &arguments,
            result.as_ref().ok().and_then(|r| r.as_ref().ok()),
        )?;
        let outcome = ActionOutcome {
            udf_path,
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

    async fn run_action_inner(
        isolate: &mut RequestScope<'_, '_, RT, Self>,
        request_params: ActionRequestParams,
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

        let (udf_path, arguments, _) = request_params.path_and_args.consume();

        // Don't allow directly running a UDF within the `_deps` directory. We don't
        // really expect users to hit this unless someone is trying to exploit
        // an app written on Convex by calling directly into a compromised
        // dependency. So, consider it a system error so we can just
        // keep a watch on it.
        if udf_path.module().is_deps() {
            anyhow::bail!("Refusing to run {udf_path:?} within the '_deps' directory");
        }

        // First, load the user's module and find the specified function.
        let module_path = udf_path.module().clone();
        let Ok(module_specifier) = module_specifier_from_path(&module_path) else {
            let message = format!("Invalid module path: {module_path:?}");
            return Ok(Err(JsError::from_message(message)));
        };

        let module = match scope.eval_user_module(&module_specifier).await? {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };
        let namespace = module
            .get_module_namespace()
            .to_object(&mut scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let function_name = udf_path.function_name();
        let function_str: v8::Local<'_, v8::Value> = v8::String::new(&mut scope, function_name)
            .ok_or_else(|| anyhow!("Failed to create function name string"))?
            .into();

        if namespace.has(&mut scope, function_str) != Some(true) {
            let message = format!(
                "{}",
                FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
            );
            return Ok(Err(JsError::from_message(message)));
        }
        let function: v8::Local<v8::Function> = namespace
            .get(&mut scope, function_str)
            .ok_or_else(|| anyhow!("Did not find function in module after checking?"))?
            .try_into()?;

        let run_str = strings::invokeAction.create(&mut scope)?.into();
        let v8_function: v8::Local<v8::Function> = function
            .get(&mut scope, run_str)
            .ok_or_else(|| anyhow!("Couldn't find invoke function in {udf_path:?}"))?
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

        Self::run_inner(
            &mut scope,
            handle,
            v8_function,
            &v8_args,
            |_, result_str| {
                let result = deserialize_udf_result(&udf_path, &result_str)?;
                Ok(async move { Ok(result) })
            },
        )
        .await
    }

    fn lookup_route(
        scope: &mut ExecutionScope<RT, Self>,
        router: &v8::Local<v8::Object>,
        router_path: CanonicalizedUdfPath,
        http_request: HttpActionRequestHead,
    ) -> anyhow::Result<Option<HttpActionRoute>> {
        let lookup_str = strings::lookup.create(scope)?.into();
        let path_str = v8::String::new(scope, http_request.url.path())
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;
        let method_str = v8::String::new(scope, http_request.method.as_str())
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;

        let lookup: v8::Local<v8::Function> = router
            .get(scope, lookup_str)
            .ok_or_else(|| anyhow!("Couldn't find lookup method of router in {router_path:?}"))?
            .try_into()?;
        let global = scope.get_current_context().global(scope);
        let r = scope
            .with_try_catch(|s| {
                lookup.call(s, global.into(), &[path_str.into(), method_str.into()])
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
        Ok(Some(HttpActionRoute {
            method: route_method_s.parse()?,
            path: route_path_s,
        }))
    }

    async fn get_router<'a, 'b: 'a>(
        scope: &mut ExecutionScope<'a, 'b, RT, Self>,
        router_path: CanonicalizedUdfPath,
    ) -> anyhow::Result<Result<v8::Local<'a, v8::Object>, JsError>> {
        // Except in tests, `http.js` will always be the udf_path.
        // We'll never hit these as long as this HTTP path only runs for
        // `convex/http.js`.
        if router_path.module().is_deps() {
            anyhow::bail!("Refusing to run {router_path:?} within the '_deps' directory");
        }

        // First, load the user's module and find the specified function.
        let module_path = router_path.module().clone();
        let Ok(module_specifier) = module_specifier_from_path(&module_path) else {
            let message = format!("Invalid module path: {module_path:?}");
            return Ok(Err(JsError::from_message(message)));
        };

        let module = match scope.eval_user_module(&module_specifier).await? {
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
                router_path.module()
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

    async fn run_inner<'a, 'b: 'a, T, Fut>(
        scope: &mut ExecutionScope<'a, 'b, RT, Self>,
        handle: IsolateHandle,
        v8_function: v8::Local<'_, v8::Function>,
        v8_args: &[v8::Local<'_, v8::Value>],
        collect_result: impl FnOnce(
            &mut ExecutionScope<'a, 'b, RT, Self>,
            String,
        ) -> anyhow::Result<Fut>,
    ) -> anyhow::Result<Result<T, JsError>>
    where
        Fut: Future<Output = anyhow::Result<Result<T, JsError>>> + Send + 'static,
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
            Err(e) => return Ok(Err(e)),
        };
        let mut collect_result = Some(collect_result);
        // `collecting_result` starts off as a future that is forever pending,
        // so it never triggers the `select_biased!` below until we are actually
        // collecting a result. Using None would be nice, but `select_biased!`
        // does not like Options.
        let mut collecting_result = (async { std::future::pending().await }).boxed().fuse();
        let result = loop {
            // Advance the user's promise as far as it can go by draining the microtask
            // queue.
            scope.perform_microtask_checkpoint();
            scope.record_heap_stats()?;
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
                    match scope.eval_user_module(&specifier).await? {
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
                    if let Some(collect_result) = collect_result.take() {
                        let promise_result_v8 = promise.result(scope);
                        let result_v8_str: v8::Local<v8::String> = promise_result_v8.try_into()?;
                        let result_str = helpers::to_rust_string(scope, &result_v8_str)?;
                        metrics::log_result_length(&result_str);
                        collecting_result = collect_result(scope, result_str)?.boxed().fuse();
                        // collect_result may have fulfilled promises, so we can go back to
                        // JS now.
                        continue;
                    }
                },
                v8::PromiseState::Rejected => {
                    let e = promise.result(scope);
                    break Err(scope.format_traceback(e)?);
                },
            }

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
                result = collecting_result => {
                    break result?;
                },
                // Normally we'd pause the user-code timeout for the duration of
                // the syscall.
                // However, actions can call queries, mutations, and other actions
                // as syscalls, so these should still count towards the user-code
                // timeout.
                task_response = environment.task_responses.next() => {
                    let Some(task_response) = task_response else {
                        anyhow::bail!("Task executor went away?");
                    };
                    match task_response {
                        TaskResponse::StreamExtend { stream_id, chunk } => {
                            match chunk {
                                Ok(chunk) => {
                                    let done = chunk.is_none();
                                    scope.extend_stream(stream_id, chunk, done)?;
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
                            let result_v8 = match variant {
                                Ok(v) => Ok(v.into_v8(scope)?),
                                Err(e) => Err(e),
                            };
                            resolve_promise_allow_all_errors(scope, resolver, result_v8)?;
                        },
                    };
                },
                // If we the isolate is terminated due to timeout, we start the
                // isolate loop over to run js to handle the timeout.
                _ = timeout.fuse() => {
                    continue;
                },
            }
            let permit_acquire =
                scope.with_state_mut(|state| state.timeout.with_timeout(limiter.acquire()))?;
            let permit = permit_acquire.await?;
            scope.with_state_mut(|state| state.permit = Some(permit))?;
            handle.check_terminated()?;
        };
        // Drain all remaining async syscalls that are not sleeps in case the
        // developer forgot to await them.
        let environment = &mut scope.state_mut()?.environment;
        environment.pending_task_sender.close_channel();
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
        let argument_size_warning = warning_if_approaching_limit(
            arguments.size(),
            *FUNCTION_MAX_ARGS_SIZE,
            "FunctionArgumentsTooLarge",
            || "Large size of the action arguments".to_string(),
            None,
            Some(" bytes"),
            None,
        );
        if let Some(warning) = argument_size_warning {
            self.trace_system(warning)?;
        }

        self.add_warnings_to_log_lines(execution_time)?;

        let result_size_warning = result.and_then(|result| {
            warning_if_approaching_limit(
                result.size(),
                *FUNCTION_MAX_RESULT_SIZE,
                "TooLargeFunctionResult",
                || "Large size of the action return value".to_string(),
                None,
                Some(" bytes"),
                None,
            )
        });
        if let Some(warning) = result_size_warning {
            self.trace_system(warning)?;
        }
        Ok(())
    }

    fn add_warnings_to_log_lines_http_action(
        &mut self,
        execution_time: FunctionExecutionTime,
        http_result: Option<&HttpActionResponse>,
    ) -> anyhow::Result<()> {
        self.add_warnings_to_log_lines(execution_time)?;

        let response_size_warning = http_result
            .and_then(|response| response.body.as_ref())
            .and_then(|body| {
                warning_if_approaching_limit(
                    body.len(),
                    HTTP_ACTION_BODY_LIMIT,
                    "HttpResponseTooLarge",
                    || "Large response returned from an HTTP action".to_string(),
                    None,
                    Some(" bytes"),
                    None,
                )
            });
        if let Some(warning) = response_size_warning {
            self.trace_system(warning)?;
        }
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
            self.trace_system(LogLine::Unstructured(format!(
                    "[WARN] {total_dangling_tasks} unawaited operation{}: [{task_names}]. Async operations should be awaited or they might not run. \
                     See https://docs.convex.dev/functions/actions#dangling-promises for more information.",
                    if total_dangling_tasks == 1 { "" } else { "s" },
                )))?;
        }

        let timeout_warning = warning_if_approaching_duration_limit(
            execution_time.elapsed,
            execution_time.limit,
            "UserTimeout",
            "Function execution took a long time",
            None,
        );
        if let Some(warning) = timeout_warning {
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
            .unbounded_send(TaskRequest {
                task_id,
                variant: request,
            })
            .expect("TaskExecutor went away?");
        Ok(())
    }
}

impl<RT: Runtime> IsolateEnvironment<RT> for ActionEnvironment<RT> {
    type Rng = ChaCha12Rng;

    fn trace(&mut self, message: String) -> anyhow::Result<()> {
        // - 1 to reserve for the [ERROR] log line

        match self.total_log_lines.cmp(&(MAX_LOG_LINES - 1)) {
            Ordering::Less => {
                if message.len() > MAX_LOG_LINE_LENGTH {
                    self.log_line_sender
                        .unbounded_send(LogLine::Unstructured(format!(
                            "{}{TRUNCATED_LINE_SUFFIX}",
                            &message[..message.floor_char_boundary(
                                MAX_LOG_LINE_LENGTH - TRUNCATED_LINE_SUFFIX.len()
                            )]
                        )))?;
                    self.total_log_lines += 1;
                } else {
                    self.log_line_sender
                        .unbounded_send(LogLine::Unstructured(message))?;
                    self.total_log_lines += 1;
                }
            },
            Ordering::Equal => {
                // Add a message about omitting log lines once
                self.log_line_sender
                    .unbounded_send(LogLine::Unstructured(format!(
                        "[ERROR] Log overflow (maximum {MAX_LOG_LINES}). Remaining log lines \
                         omitted."
                    )))?;
                self.total_log_lines += 1;
            },
            Ordering::Greater => (),
        };
        Ok(())
    }

    fn trace_system(&mut self, message: LogLine) -> anyhow::Result<()> {
        // Don't check length limits or count this towards total log lines since
        // this is a system log line
        self.log_line_sender.unbounded_send(message)?;
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut Self::Rng> {
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

    fn get_all_table_mappings(&mut self) -> anyhow::Result<(TableMapping, VirtualTableMapping)> {
        anyhow::bail!("get_all_table_mappings unsupported in actions")
    }

    // We lookup all modules' sources upfront when initializing the action
    // environment, so this function always returns immediately.
    async fn lookup_source(
        &mut self,
        path: &str,
        timeout: &mut Timeout<RT>,
        permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<(ModuleSource, Option<SourceMap>)>> {
        let user_module_path: ModulePath = path.parse()?;
        let result = self
            .phase
            .get_module(&user_module_path, timeout, permit)?
            .map(|module_version| (module_version.source, module_version.source_map));
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
