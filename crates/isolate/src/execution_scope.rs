use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::{
        Deref,
        DerefMut,
    },
};

use anyhow::{
    anyhow,
    bail,
};
use async_recursion::async_recursion;
use common::{
    errors::JsError,
    runtime::Runtime,
    static_span,
    types::UdfType,
};
use deno_core::{
    v8::{
        self,
        HeapStatistics,
    },
    ModuleResolutionError,
    ModuleSpecifier,
};
use errors::ErrorMetadata;
use model::modules::{
    module_versions::FullModuleSource,
    user_error::{
        ModuleNotFoundError,
        SystemModuleNotFoundError,
    },
};
use serde_json::Value as JsonValue;
use value::heap_size::HeapSize;

use crate::{
    bundled_js::system_udf_file,
    environment::IsolateEnvironment,
    helpers::{
        self,
        to_rust_string,
    },
    isolate::{
        CONVEX_SCHEME,
        SYSTEM_PREFIX,
    },
    metrics,
    module_map::{
        ModuleId,
        ModuleMap,
    },
    request_scope::RequestState,
    termination::IsolateHandle,
    IsolateHeapStats,
};

/// V8 will invoke our promise_reject_callback when it determines that a
/// promise rejected without a handler. If there isn't a handler, we'd like to
/// crash the UDF and pass this error on to the user. However, there are
/// common situations where user code can only add a promise handler after the
/// promise rejects: since promises and async functions run synchronously until
/// their first suspend point, any handler registered with `.catch()` may be too
/// late!
///
/// ```js
/// function fetch(url): Promise<Response> {
///   if (!url) return Promise.reject("1 argument required");
///   ...
/// }
/// fetchWrapper().catch(x => console.log('caught', e));
/// ```
///
/// By the time the `fetchWrapper()` function above returns, the promise
/// returned has already rejected. To distinguish between promises rejected
/// with no rejection handling and promises which are handled soon enough we
/// fully drain the microtask queue to give the current task and give
/// other microtasks a chance to add a rejection handler. If at that point no
/// rejection handler has been added to a promise, it's time to crash the UDF.
///
/// This choice matches the behavior in Node.js and the HTML spec where this
/// is called an "unhandled promise rejection."
/// https://nodejs.org/api/process.html#event-unhandledrejection
/// https://html.spec.whatwg.org/multipage/webappapis.html#unhandled-promise-rejections
///
/// Although the promises in question are implicit in the async function syntax,
/// this more complex code will exhibit similar behavior.
///
/// ```js
/// (async () => {
///   try {
///     await (async () => {
///       throw new Error("will invoke promise_reject_callback")
///       await Promise.resolve();
///       throw new Error("will not invoke PromiseRejectWithNoHandler")
///     })();
///   } catch {}
/// })()
/// ```
pub struct PendingUnhandledPromiseRejections {
    pub exceptions: HashMap<v8::Global<v8::Promise>, v8::Global<v8::Value>>,
}

impl PendingUnhandledPromiseRejections {
    pub fn new() -> Self {
        PendingUnhandledPromiseRejections {
            exceptions: HashMap::new(),
        }
    }
}

pub struct PendingDynamicImports {
    pub allow_dynamic_imports: bool,
    pub imports: Vec<(ModuleSpecifier, v8::Global<v8::PromiseResolver>)>,
}

impl PendingDynamicImports {
    pub fn new(allow_dynamic_imports: bool) -> Self {
        PendingDynamicImports {
            allow_dynamic_imports,
            imports: Vec::new(),
        }
    }

    pub fn push(&mut self, specifier: ModuleSpecifier, resolver: v8::Global<v8::PromiseResolver>) {
        self.imports.push((specifier, resolver));
    }

    pub fn take(&mut self) -> Vec<(ModuleSpecifier, v8::Global<v8::PromiseResolver>)> {
        self.imports.split_off(0)
    }
}

/// Most functionality for executing JS and manipulating objects executes within
/// a [`v8::HandleScope`]. The [`ExecutionScope`] wrapper is a convenience
/// struct that represents executing code within a [`RequestScope`].
pub struct ExecutionScope<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> {
    v8_scope: &'a mut v8::HandleScope<'b>,
    v8_context: v8::Local<'b, v8::Context>,
    _pd: PhantomData<(RT, E)>,
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> Deref for ExecutionScope<'a, 'b, RT, E> {
    type Target = v8::HandleScope<'b>;

    fn deref(&self) -> &v8::HandleScope<'b> {
        self.v8_scope
    }
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> DerefMut
    for ExecutionScope<'a, 'b, RT, E>
{
    fn deref_mut(&mut self) -> &mut v8::HandleScope<'b> {
        self.v8_scope
    }
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    pub fn new(v8_scope: &'a mut v8::HandleScope<'b>) -> Self {
        let v8_context = v8_scope.get_current_context();
        Self {
            v8_scope,
            v8_context,
            _pd: PhantomData,
        }
    }

    pub fn handle(&self) -> &IsolateHandle {
        self.v8_scope
            .get_slot()
            .expect("IsolateHandle disappeared?")
    }

    pub fn state(&mut self) -> anyhow::Result<&RequestState<RT, E>> {
        self.v8_context
            .get_slot(self.v8_scope)
            .ok_or_else(|| anyhow!("ContextState disappeared?"))
    }

    // TODO: Delete this method and use with_state_mut everywhere instead in
    // order to make it impossible to hold state across await points.
    pub fn state_mut(&mut self) -> anyhow::Result<&mut RequestState<RT, E>> {
        self.v8_context
            .get_slot_mut(self.v8_scope)
            .ok_or_else(|| anyhow!("ContextState disappeared?"))
    }

    pub fn with_state_mut<T>(
        &mut self,
        f: impl FnOnce(&mut RequestState<RT, E>) -> T,
    ) -> anyhow::Result<T> {
        let state = self.state_mut()?;
        Ok(f(state))
    }

    pub fn record_heap_stats(&mut self) -> anyhow::Result<()> {
        let mut stats = HeapStatistics::default();
        self.get_heap_statistics(&mut stats);
        self.with_state_mut(|state| {
            let blobs_heap_size = state.blob_parts.heap_size();
            let streams_heap_size = state.streams.heap_size() + state.stream_listeners.heap_size();
            state.environment.record_heap_stats(IsolateHeapStats::new(
                stats,
                blobs_heap_size,
                streams_heap_size,
            ));
        })
    }

    pub fn module_map(&mut self) -> &ModuleMap {
        self.v8_context
            .get_slot(self.v8_scope)
            .expect("ModuleMap disappeared?")
    }

    pub fn module_map_mut(&mut self) -> &mut ModuleMap {
        self.v8_context
            .get_slot_mut(self.v8_scope)
            .expect("ModuleMap disappeared?")
    }

    #[allow(unused)]
    pub fn pending_unhandled_promise_rejections(&mut self) -> &PendingUnhandledPromiseRejections {
        self.v8_context
            .get_slot_mut(self.v8_scope)
            .expect("No PendingUnhandledPromiseRejections found")
    }

    pub fn pending_unhandled_promise_rejections_mut(
        &mut self,
    ) -> &mut PendingUnhandledPromiseRejections {
        self.v8_context
            .get_slot_mut(self.v8_scope)
            .expect("No PendingUnhandledPromiseRejections found")
    }

    pub fn pending_dynamic_imports_mut(&mut self) -> &mut PendingDynamicImports {
        self.v8_context
            .get_slot_mut(self.v8_scope)
            .expect("No PendingDynamicImports found")
    }

    pub fn with_try_catch<R>(
        &mut self,
        f: impl FnOnce(&mut v8::HandleScope<'b>) -> R,
    ) -> anyhow::Result<Result<R, JsError>> {
        let mut tc_scope = v8::TryCatch::new(self.v8_scope);
        let r = f(&mut tc_scope);
        if let Some(e) = tc_scope.exception() {
            drop(tc_scope);
            return Ok(Err(self.format_traceback(e)?));
        }
        Ok(Ok(r))
    }

    pub async fn eval_user_module(
        &mut self,
        udf_type: UdfType,
        is_dynamic: bool,
        name: &ModuleSpecifier,
    ) -> anyhow::Result<Result<v8::Local<'a, v8::Module>, JsError>> {
        let timer = metrics::eval_user_module_timer(udf_type, is_dynamic);
        let module = match self.eval_module(name).await {
            Ok(id) => id,
            Err(e) => {
                // TODO: It's a bit awkward that we're calling these "JsError"s, since they
                // don't originate from JavaScript.
                if let Some(e) = e.downcast_ref::<ModuleNotFoundError>() {
                    return Ok(Err(JsError::from_message(format!("{e}"))));
                }
                if let Some(e) = e.downcast_ref::<SystemModuleNotFoundError>() {
                    return Ok(Err(JsError::from_message(format!("{e}"))));
                }
                if let Some(e) = e.downcast_ref::<ModuleResolutionError>() {
                    return Ok(Err(JsError::from_message(format!("{e}"))));
                }
                match e.downcast::<JsError>() {
                    Ok(e) => return Ok(Err(e)),
                    Err(e) => return Err(e),
                }
            },
        };
        timer.finish();
        Ok(Ok(module))
    }

    #[fastrace::trace]
    pub async fn eval_module(
        &mut self,
        name: &ModuleSpecifier,
    ) -> anyhow::Result<v8::Local<'a, v8::Module>> {
        let _s = static_span!();

        // These first two steps of registering and then instantiating the module
        // correspond to `JsRuntime::load_module`. This function is idempotent,
        // so it's safe to rerun.
        let id = self.register_module(name).await?;

        // NB: This part is separate from `self.register_module()` since module
        // registration is recursive, compiling and registering dependencies,
        // where instantiation and evaluation are not.
        self.instantiate_and_eval_module(id)?;

        let module_map = self.module_map();
        let module = module_map
            .handle_by_id(id)
            .ok_or_else(|| anyhow!("Non-existent module ID {id}"))?;
        let module = v8::Local::new(self, module);
        Ok(module)
    }

    #[async_recursion(?Send)]
    async fn register_module(&mut self, name: &ModuleSpecifier) -> anyhow::Result<ModuleId> {
        let _s = static_span!();
        {
            let module_map = self.module_map();
            if let Some(id) = module_map.get_by_name(name) {
                return Ok(id);
            }
        }
        let (id, import_specifiers) = {
            let FullModuleSource { source, source_map } = self.lookup_source(name).await?;

            // Step 1: Compile the module and discover its imports.
            let timer = metrics::compile_module_timer();

            let name_str = v8::String::new(self, name.as_str())
                .ok_or_else(|| anyhow!("Failed to create name string"))?;
            let source_str = v8::String::new(self, &source)
                .ok_or_else(|| anyhow!("Failed to create source string"))?;

            let origin = helpers::module_origin(self, name_str);
            let v8_source = v8::script_compiler::Source::new(source_str, Some(&origin));

            let module = self
                .with_try_catch(|s| v8::script_compiler::compile_module(s, v8_source))??
                .ok_or_else(|| anyhow!("Unexpected module compilation error"))?;

            assert_eq!(module.get_status(), v8::ModuleStatus::Uninstantiated);
            let mut import_specifiers = vec![];
            let module_requests = module.get_module_requests();
            for i in 0..module_requests.length() {
                let module_request: v8::Local<v8::ModuleRequest> = module_requests
                    .get(self, i)
                    .ok_or_else(|| anyhow!("Module request {} out of bounds", i))?
                    .try_into()?;
                let import_specifier =
                    helpers::to_rust_string(self, &module_request.get_specifier())?;
                let module_specifier = deno_core::resolve_import(&import_specifier, name.as_str())?;
                let offset = module_request.get_source_offset();
                let location = module.source_offset_to_location(offset);
                import_specifiers.push((module_specifier, location));
            }
            timer.finish();

            // Step 2: Register the module with the module map.
            let id = {
                let module_v8 = v8::Global::<v8::Module>::new(self, module);
                let module_map = self.module_map_mut();
                module_map.register(name, module_v8, source_map)
            };
            (id, import_specifiers)
        };

        // Step 3: Recursively load the dependencies. Since we've already registered
        // ourselves, this won't create an infinite loop on import cycles.
        for (import_specifier, location) in import_specifiers {
            self.register_module(&import_specifier).await.map_err(|e| {
                let Err(e) = self.nicely_show_line_number_on_error(name, location, e);
                e
            })?;
        }

        Ok(id)
    }

    async fn lookup_source(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> anyhow::Result<FullModuleSource> {
        let _s = static_span!();
        if module_specifier.scheme() != CONVEX_SCHEME {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnsupportedScheme",
                format!(
                    "Unsupported scheme ({}) in {}",
                    module_specifier.scheme(),
                    module_specifier
                ),
            ));
        }
        if module_specifier.has_authority() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnsupportedAuthority",
                format!(
                    "Module URL {} must not have an authority. Has {}",
                    module_specifier,
                    module_specifier.authority()
                ),
            ));
        }
        if module_specifier.cannot_be_a_base() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "CannotBeABase",
                format!(
                    "Module URL {} is a cannot-be-a-base URL which is disallowed.",
                    module_specifier
                ),
            ));
        }
        let module_path = module_specifier
            .path()
            .strip_prefix('/')
            .ok_or_else(|| anyhow!("Path for {:?} did not start with a slash", module_specifier))?;

        let timer = metrics::lookup_source_timer(module_path.starts_with(SYSTEM_PREFIX));

        // Overlay our "_system/" files on top of the user's UDFs.
        if let Some(system_path) = module_path.strip_prefix(SYSTEM_PREFIX) {
            let (source, source_map) = system_udf_file(system_path)
                .ok_or_else(|| SystemModuleNotFoundError::new(system_path))?;
            let result = FullModuleSource {
                source: source.to_string(),
                source_map: source_map.as_ref().map(|s| s.to_string()),
            };
            timer.finish();
            return Ok(result);
        }

        let state = self.state_mut()?;
        let result = state
            .environment
            .lookup_source(module_path, &mut state.timeout, &mut state.permit)
            .await?
            .ok_or_else(|| ModuleNotFoundError::new(module_path))?;

        timer.finish();

        Ok(result)
    }

    fn instantiate_and_eval_module(&mut self, id: ModuleId) -> anyhow::Result<()> {
        let _s = static_span!();
        let module = {
            let module_map = self.module_map();
            let handle = module_map
                .handle_by_id(id)
                .ok_or_else(|| anyhow!("ModuleInfo not found for {}", id))?;
            v8::Local::new(self, handle)
        };

        match module.get_status() {
            v8::ModuleStatus::Errored => bail!("Module {} is in errored state", id),
            v8::ModuleStatus::Evaluated => return Ok(()),
            _ => (),
        }
        // Instantiate the module, loading its dependencies.
        {
            let timer = metrics::instantiate_module_timer();
            let result = self.with_try_catch(|s| {
                module.instantiate_module(s, Self::module_resolve_callback)
            })??;
            if matches!(result, Some(false) | None) {
                anyhow::bail!("Unexpected successful instantiate result: {result:?}");
            }
            anyhow::ensure!(module.get_status() == v8::ModuleStatus::Instantiated);
            timer.finish();
        };

        let value = {
            let timer = metrics::evaluate_module_timer();
            let result = self
                .with_try_catch(|s| module.evaluate(s))??
                .ok_or_else(|| anyhow!("Missing result from successful module evaluation"))?;
            // TODO: Check if we have a terminating error here.
            timer.finish();
            result
        };

        let status = module.get_status();
        anyhow::ensure!(
            status == v8::ModuleStatus::Evaluated || status == v8::ModuleStatus::Errored
        );
        let promise = v8::Local::<v8::Promise>::try_from(value)
            .map_err(|e| anyhow!("Module evaluation did not return a promise: {:?}", e))?;
        match promise.state() {
            v8::PromiseState::Pending => {
                bail!(JsError::from_message(
                    "Top-level awaits in source files are unsupported".to_string()
                ))
            },
            v8::PromiseState::Fulfilled => {
                anyhow::ensure!(status == v8::ModuleStatus::Evaluated);
            },
            v8::PromiseState::Rejected => {
                let e = promise.result(self.v8_scope);
                return Err(self.format_traceback(e)?.into());
            },
        }
        Ok(())
    }

    fn module_resolve_callback<'c>(
        context: v8::Local<'c, v8::Context>,
        specifier: v8::Local<'c, v8::String>,
        _import_assertions: v8::Local<'c, v8::FixedArray>,
        referrer: v8::Local<'c, v8::Module>,
    ) -> Option<v8::Local<'c, v8::Module>> {
        let scope = &mut unsafe { v8::CallbackScope::new(context) };
        match Self::_module_resolve_callback(scope, referrer, specifier) {
            Ok(m) => Some(m),
            Err(e) => {
                helpers::throw_type_error(scope, format!("{:?}", e));
                None
            },
        }
    }

    fn _module_resolve_callback<'c>(
        scope: &mut v8::CallbackScope<'c>,
        referrer: v8::Local<'c, v8::Module>,
        specifier: v8::Local<'c, v8::String>,
    ) -> anyhow::Result<v8::Local<'c, v8::Module>> {
        let mut scope = ExecutionScope::<RT, E>::new(scope);
        let referrer_global = v8::Global::new(&mut scope, referrer);
        let specifier_str = helpers::to_rust_string(&mut scope, &specifier)?;

        let module_map = scope.module_map();
        let referrer_name = module_map
            .name_by_handle(&referrer_global)
            .ok_or_else(|| anyhow::anyhow!("Couldn't find referring module"))?
            .to_string();
        let resolved_specifier = deno_core::resolve_import(&specifier_str, &referrer_name)?;
        let id = module_map
            .get_by_name(&resolved_specifier)
            .ok_or_else(|| anyhow!("Couldn't find {resolved_specifier}"))?;
        let handle = module_map
            .handle_by_id(id)
            .ok_or_else(|| anyhow!("Couldn't find {specifier_str} in {referrer_name}"))?;

        Ok(v8::Local::new(&mut scope, handle))
    }

    pub fn syscall(
        &mut self,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) -> anyhow::Result<()> {
        let _s = static_span!();
        if args.length() != 2 {
            // There's not really an expected developer mistake that would lead to them
            // calling Convex.syscall incorrectly -- the bug must be in our
            // convex/server code. Treat this as a system error.
            anyhow::bail!("syscall(op, arg_object) takes two arguments");
        }
        let op_name: v8::Local<v8::String> = args.get(0).try_into()?;
        let op_name = to_rust_string(self, &op_name)?;

        let timer = metrics::syscall_timer(&op_name);

        let args_v8: v8::Local<v8::String> = args.get(1).try_into()?;
        let args_s = to_rust_string(self, &args_v8)?;
        let args_v: JsonValue = serde_json::from_str(&args_s).map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "SyscallArgsInvalidJson",
                format!("Received invalid json: {e}"),
            ))
        })?;

        let state = self.state_mut()?;
        let result = state.environment.syscall(&op_name[..], args_v)?;

        let value_s = serde_json::to_string(&result)?;
        let value_v8 = v8::String::new(self, &value_s[..])
            .ok_or_else(|| anyhow!("Failed to create result string"))?;
        rv.set(value_v8.into());

        timer.finish();
        Ok(())
    }

    pub fn async_syscall(
        &mut self,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) -> anyhow::Result<()> {
        if args.length() != 2 {
            anyhow::bail!("asyncSyscall(op, arg_object) takes two arguments");
        }
        let op_name: v8::Local<v8::String> = args.get(0).try_into()?;
        let op_name = to_rust_string(self, &op_name)?;

        let args_v8: v8::Local<v8::String> = args.get(1).try_into()?;
        let args_s = to_rust_string(self, &args_v8)?;
        let args_v: JsonValue = serde_json::from_str(&args_s).map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "SyscallArgsInvalidJson",
                format!("Received invalid json: {e}"),
            ))
        })?;

        let resolver = v8::PromiseResolver::new(self)
            .ok_or_else(|| anyhow!("Failed to create PromiseResolver"))?;
        let promise = resolver.get_promise(self);
        let resolver = v8::Global::new(self, resolver);
        {
            let state = self.state_mut()?;
            state
                .environment
                .start_async_syscall(op_name, args_v, resolver)?;
        }
        rv.set(promise.into());
        Ok(())
    }
}
