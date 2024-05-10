use anyhow::anyhow;
use common::{
    errors::JsError,
    types::UdfType,
};
use deno_core::{
    serde_v8,
    v8,
    ModuleSpecifier,
};
use model::modules::user_error::{
    FunctionNotFoundError,
    ModuleNotFoundError,
};
use serde_json::Value as JsonValue;
use sourcemap::SourceMap;
use sync_types::CanonicalizedUdfPath;
use value::ConvexArray;

use super::{
    client::{
        Completions,
        EvaluateResult,
    },
    context::PendingFunction,
    context_state::ContextState,
    environment::EnvironmentOutcome,
};
use crate::{
    bundled_js::system_udf_file,
    deserialize_udf_result,
    environment::helpers::{
        module_loader::module_specifier_from_path,
        resolve_promise,
    },
    error::extract_source_mapped_error,
    helpers::{
        self,
        to_rust_string,
    },
    isolate::SETUP_URL,
    isolate2::callback_context::CallbackContext,
    metrics,
    strings::{
        self,
        StaticString,
    },
};

pub struct EnteredContext<'enter, 'scope: 'enter> {
    scope: &'enter mut v8::HandleScope<'scope>,
    context: v8::Local<'scope, v8::Context>,
}

impl<'enter, 'scope: 'enter> EnteredContext<'enter, 'scope> {
    pub fn new(
        scope: &'enter mut v8::HandleScope<'scope>,
        context: v8::Local<'scope, v8::Context>,
    ) -> Self {
        Self { scope, context }
    }

    pub fn context_state_mut(&mut self) -> anyhow::Result<&mut ContextState> {
        self.context
            .get_slot_mut::<ContextState>(self.scope)
            .ok_or_else(|| anyhow::anyhow!("ContextState not found in context"))
    }

    pub fn context_state(&mut self) -> anyhow::Result<&ContextState> {
        self.context
            .get_slot::<ContextState>(self.scope)
            .ok_or_else(|| anyhow::anyhow!("ContextState not found in context"))
    }

    pub fn run_setup_module(&mut self) -> anyhow::Result<()> {
        let setup_url = ModuleSpecifier::parse(SETUP_URL)?;
        let (source, source_map) =
            system_udf_file("setup.js").ok_or_else(|| anyhow!("Setup module not found"))?;
        let unresolved_imports =
            self.register_module(&setup_url, source, source_map.map(|s| s.to_string()))?;
        anyhow::ensure!(
            unresolved_imports.is_empty(),
            "Unexpected import specifiers for setup module"
        );
        let module = self.evaluate_module(&setup_url)?;
        let namespace = module
            .get_module_namespace()
            .to_object(self.scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let function_str = strings::setup.create(self.scope)?;
        let function: v8::Local<v8::Function> = namespace
            .get(self.scope, function_str.into())
            .ok_or_else(|| anyhow!("Couldn't find setup in setup module"))?
            .try_into()?;

        let global = self.scope.get_current_context().global(self.scope);

        self.execute_user_code(|scope| function.call(scope, global.into(), &[global.into()]))?
            .ok_or_else(|| anyhow!("Successful setup() returned None"))?;

        Ok(())
    }

    // NB: This can be called from the top-level (i.e. entering from the context
    // into user code) but also from within a callback (e.g. following an object
    // property in an op handler).
    pub fn execute_user_code<R>(
        &mut self,
        f: impl FnOnce(&mut v8::HandleScope<'scope>) -> R,
    ) -> anyhow::Result<R> {
        let mut tc_scope = v8::TryCatch::new(self.scope);
        let r = f(&mut tc_scope);
        if let Some(e) = tc_scope.exception() {
            drop(tc_scope);
            return Err(self.format_traceback(e)?.into());
        }
        drop(tc_scope);
        // XXX: check terminating error here. (call to unsupported syscall)
        if self.scope.is_execution_terminating() {
            anyhow::bail!("Execution terminated");
        }
        // Executing just about any user code can lead to an unhandled promise
        // rejection (e.g. calling `Promise.reject`). However, it's important
        // to only fail the session when we receive control... XXX explain more.
        let promise_rejection = {
            let context_state = self.context_state_mut()?;

            // Only use the first unhandled promise rejection.
            let rejection = context_state.unhandled_promise_rejections.drain().next();
            context_state.unhandled_promise_rejections.clear();
            rejection
        };
        if let Some((_promise, error_global)) = promise_rejection {
            let error = v8::Local::new(self.scope, error_global);
            let err = self.format_traceback(error)?;
            return Err(err.into());
        }
        Ok(r)
    }

    pub fn register_module(
        &mut self,
        url: &ModuleSpecifier,
        source: &str,
        source_map: Option<String>,
    ) -> anyhow::Result<Vec<ModuleSpecifier>> {
        {
            let context_state = self.context_state_mut()?;
            anyhow::ensure!(
                !context_state.module_map.contains_module(url),
                "Module already registered"
            );
        }
        let name_str = v8::String::new(self.scope, url.as_str())
            .ok_or_else(|| anyhow!("Failed to create name string"))?;
        let source_str = v8::String::new(self.scope, source)
            .ok_or_else(|| anyhow!("Failed to create source string"))?;

        let origin = helpers::module_origin(self.scope, name_str);
        let v8_source = v8::script_compiler::Source::new(source_str, Some(&origin));

        let module = self
            .execute_user_code(|s| v8::script_compiler::compile_module(s, v8_source))?
            .ok_or_else(|| anyhow!("Unexpected module compilation error"))?;

        anyhow::ensure!(module.get_status() == v8::ModuleStatus::Uninstantiated);
        let mut import_specifiers: Vec<ModuleSpecifier> = vec![];
        let module_requests = module.get_module_requests();
        for i in 0..module_requests.length() {
            let module_request: v8::Local<v8::ModuleRequest> = module_requests
                .get(self.scope, i)
                .ok_or_else(|| anyhow!("Module request {} out of bounds", i))?
                .try_into()?;
            let import_specifier =
                helpers::to_rust_string(self.scope, &module_request.get_specifier())?;
            let module_specifier = deno_core::resolve_import(&import_specifier, url.as_str())?;
            import_specifiers.push(module_specifier);
        }
        let module = v8::Global::new(self.scope, module);
        let unresolved_imports = {
            let context_state = self.context_state_mut()?;
            import_specifiers.retain(|s| !context_state.module_map.contains_module(s));
            context_state
                .module_map
                .register(url.clone(), module, source_map)?;
            import_specifiers
        };
        Ok(unresolved_imports)
    }

    pub fn evaluate_module(
        &mut self,
        url: &ModuleSpecifier,
    ) -> anyhow::Result<v8::Local<'enter, v8::Module>> {
        let module_global = {
            let context_state = self.context_state()?;
            context_state
                .module_map
                .lookup_module(url)
                .ok_or_else(|| anyhow!("Module not registered"))?
                .clone()
        };
        let module = v8::Local::new(self.scope, module_global);
        match module.get_status() {
            v8::ModuleStatus::Uninstantiated => (),
            s => anyhow::bail!("Module {url} is in invalid state: {s:?}"),
        }

        let instantiation_result = self
            .execute_user_code(|s| module.instantiate_module(s, CallbackContext::resolve_module))?;

        if matches!(instantiation_result, Some(false) | None) {
            anyhow::bail!("Unexpected successful instantiate result: {instantiation_result:?}");
        }
        anyhow::ensure!(module.get_status() == v8::ModuleStatus::Instantiated);

        let evaluation_result = self
            .execute_user_code(|s| module.evaluate(s))?
            .ok_or_else(|| anyhow!("Missing result from successful module evaluation"))?;

        let status = module.get_status();
        anyhow::ensure!(
            status == v8::ModuleStatus::Evaluated || status == v8::ModuleStatus::Errored
        );
        let promise = v8::Local::<v8::Promise>::try_from(evaluation_result)
            .map_err(|e| anyhow!("Module evaluation did not return a promise: {:?}", e))?;
        match promise.state() {
            v8::PromiseState::Pending => {
                anyhow::bail!(JsError::from_message(
                    "Top-level awaits in source files are unsupported".to_string()
                ))
            },
            v8::PromiseState::Fulfilled => {
                anyhow::ensure!(status == v8::ModuleStatus::Evaluated);
            },
            v8::PromiseState::Rejected => {
                let e = promise.result(self.scope);
                return Err(self.format_traceback(e)?.into());
            },
        }
        Ok(module)
    }

    pub fn start_evaluate_function(
        &mut self,
        udf_type: UdfType,
        udf_path: &CanonicalizedUdfPath,
        arguments: ConvexArray,
    ) -> anyhow::Result<(v8::Global<v8::Promise>, EvaluateResult)> {
        let module_url = module_specifier_from_path(udf_path.module())?;
        let module_global = {
            let context_state = self.context_state_mut()?;
            context_state.environment.start_execution()?;
            context_state
                .module_map
                .lookup_module(&module_url)
                .ok_or_else(|| {
                    let err = ModuleNotFoundError::new(udf_path.module().as_str());
                    JsError::from_message(err.to_string())
                })?
                .clone()
        };
        let module = v8::Local::new(self.scope, module_global);
        match module.get_status() {
            v8::ModuleStatus::Evaluated => (),
            s => anyhow::bail!("Module is in invalid state: {s:?}"),
        }

        let exports = module.get_module_namespace();
        let exports = v8::Local::new(self.scope, exports);
        let exports = exports
            .to_object(self.scope)
            .ok_or_else(|| anyhow!("Module exports not an object"))?;

        let name_str = v8::String::new(self.scope, udf_path.function_name())
            .ok_or_else(|| anyhow::anyhow!("Failed to create name string"))?
            .into();
        if exports.has(self.scope, name_str) != Some(true) {
            let err =
                FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str());
            anyhow::bail!(JsError::from_message(err.to_string()));
        }
        let function: v8::Local<v8::Function> = exports
            .get(self.scope, name_str)
            .ok_or_else(|| {
                let err = FunctionNotFoundError::new(
                    udf_path.function_name(),
                    udf_path.module().as_str(),
                );
                JsError::from_message(err.to_string())
            })?
            .try_into()?;
        let invoke_str = self.classify_function(udf_type, udf_path, &function)?;

        let args_str = serde_json::to_string(&JsonValue::from(arguments))?;
        let args_v8_str = v8::String::new(self.scope, &args_str)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;

        let invoke: v8::Local<v8::Function> = function
            .get(self.scope, invoke_str.into())
            .ok_or_else(|| {
                let msg = format!("Couldn't find invoke function in {udf_path:?}");
                JsError::from_message(msg)
            })?
            .try_into()?;

        let global = self.scope.get_current_context().global(self.scope);

        let promise: v8::Local<v8::Promise> = self
            .execute_user_code(|scope| invoke.call(scope, global.into(), &[args_v8_str.into()]))?
            .ok_or_else(|| anyhow!("Failed to call invoke function"))?
            .try_into()?;

        // Calling into our function can put entries into the microtask queue, so
        // ensure that the microtask queue is clean before returning to the Tokio
        // thread. This ensure that we've driven the promise as far as possible
        // before collecting what it's blocked on.
        self.execute_user_code(|s| s.perform_microtask_checkpoint())?;

        let evaluate_result = self.check_promise_result(udf_path, &promise)?;
        Ok((v8::Global::new(self.scope, promise), evaluate_result))
    }

    fn classify_function(
        &mut self,
        udf_type: UdfType,
        udf_path: &CanonicalizedUdfPath,
        function: &v8::Local<v8::Function>,
    ) -> anyhow::Result<v8::Local<'scope, v8::String>> {
        let is_query = self.classify_function_object(&strings::isQuery, function)?;
        let is_mutation = self.classify_function_object(&strings::isMutation, function)?;
        let is_action = self.classify_function_object(&strings::isAction, function)?;

        let invoke_str = match (udf_type, is_query, is_mutation, is_action) {
            (UdfType::Query, true, false, false) => strings::invokeQuery.create(self.scope)?,
            (UdfType::Mutation, false, true, false) => {
                strings::invokeMutation.create(self.scope)?
            },
            (UdfType::Query, false, true, _) => {
                let message = format!(
                    "Function {udf_path:?} is registered as a mutation but is being run as a \
                     query."
                );
                anyhow::bail!(JsError::from_message(message));
            },
            (UdfType::Mutation, true, false, _) => {
                let message = format!(
                    "Function {udf_path:?} is registered as a query but is being run as a \
                     mutation."
                );
                anyhow::bail!(JsError::from_message(message));
            },
            (UdfType::Query | UdfType::Mutation, false, false, _) => {
                let message = format!(
                    "Function {udf_path:?} is neither a query or mutation. Did you forget to wrap \
                     it with `query` or `mutation`?"
                );
                anyhow::bail!(JsError::from_message(message));
            },
            // TODO: Action support.
            _ => {
                anyhow::bail!(
                    "Unexpected function classification: {udf_type} vs. (is_query: {is_query}, \
                     is_mutation: {is_mutation}, is_actino: {is_action})"
                );
            },
        };
        Ok(invoke_str)
    }

    fn classify_function_object(
        &mut self,
        function_type: &'static StaticString,
        function: &v8::Local<v8::Function>,
    ) -> anyhow::Result<bool> {
        let function_type_str = function_type.create(self.scope)?.into();
        let has_function_type = function.has(self.scope, function_type_str) == Some(true);
        let is_function_type = has_function_type
            && function
                .get(self.scope, function_type_str)
                .ok_or_else(|| anyhow!("Failed to get {} property", function_type.rust_str()))?
                .is_true();
        Ok(is_function_type)
    }

    pub fn poll_function(
        &mut self,
        pending_function: &PendingFunction,
        completions: Completions,
    ) -> anyhow::Result<EvaluateResult> {
        let (async_syscalls, async_ops) = {
            let context_state = self.context_state_mut()?;
            let mut async_syscalls = vec![];
            for completion in completions.async_syscalls {
                let resolver = context_state.take_promise(completion.promise_id)?;
                async_syscalls.push((resolver, completion.result));
            }
            let mut async_ops = vec![];
            for completion in completions.async_ops {
                let resolver = context_state.take_promise(completion.promise_id)?;
                async_ops.push((resolver, completion.result));
            }
            (async_syscalls, async_ops)
        };
        for (resolver, result) in async_syscalls {
            let result_v8 = match result {
                Ok(v) => Ok(serde_v8::to_v8(self.scope, v)?),
                Err(e) => Err(e),
            };
            resolve_promise(self.scope, resolver, result_v8)?;
        }
        for (resolver, result) in async_ops {
            let result_v8 = match result {
                Ok(v) => Ok(v.into_v8(self.scope)?),
                Err(e) => Err(e),
            };
            resolve_promise(self.scope, resolver, result_v8)?;
        }

        self.execute_user_code(|s| s.perform_microtask_checkpoint())?;

        let promise = v8::Local::new(self.scope, &pending_function.promise);
        self.check_promise_result(&pending_function.udf_path, &promise)
    }

    fn check_promise_result(
        &mut self,
        udf_path: &CanonicalizedUdfPath,
        promise: &v8::Local<v8::Promise>,
    ) -> anyhow::Result<EvaluateResult> {
        let context = self.context_state_mut()?;
        let pending = context.take_pending();
        match promise.state() {
            v8::PromiseState::Pending if pending.is_empty() => {
                anyhow::bail!(JsError::from_message(
                    "Returned promise will never resolve".to_string()
                ))
            },
            v8::PromiseState::Rejected => {
                let e = promise.result(self.scope);
                anyhow::bail!(self.format_traceback(e)?);
            },
            v8::PromiseState::Fulfilled if pending.is_empty() => {
                let v8_result: v8::Local<v8::String> = promise.result(self.scope).try_into()?;
                let result_str = helpers::to_rust_string(self.scope, &v8_result)?;
                let result = deserialize_udf_result(udf_path, &result_str)??;
                Ok(EvaluateResult::Ready(result))
            },
            v8::PromiseState::Pending | v8::PromiseState::Fulfilled => {
                Ok(EvaluateResult::Pending(pending))
            },
        }
    }

    pub fn shutdown(&mut self) -> anyhow::Result<EnvironmentOutcome> {
        let context_state = self.context_state_mut()?;
        let outcome = context_state.environment.finish_execution()?;
        Ok(outcome)
    }

    pub fn format_traceback(&mut self, exception: v8::Local<v8::Value>) -> anyhow::Result<JsError> {
        // Check if we hit a system error or timeout and can't run any JavaScript now.
        // Abort with a system error here, and we'll (in the best case) pull out
        // the original system error that initiated the termination.
        if self.scope.is_execution_terminating() {
            anyhow::bail!("Execution terminated");
        }
        let err: anyhow::Result<_> = try {
            let (message, frame_data, custom_data) =
                extract_source_mapped_error(self.scope, exception)?;
            JsError::from_frames(message, frame_data, custom_data, |s| {
                let context_state = self.context_state()?;
                let Some(source_map) = context_state.module_map.lookup_source_map(s) else {
                    return Ok(None);
                };
                Ok(Some(SourceMap::from_slice(source_map.as_bytes())?))
            })
        };
        let err = match err {
            Ok(e) => e,
            Err(e) => {
                let message = v8::Exception::create_message(self.scope, exception);
                let message = message.get(self.scope);
                let message = to_rust_string(self.scope, &message)?;
                metrics::log_source_map_failure(&message, &e);
                JsError::from_message(message)
            },
        };
        Ok(err)
    }
}
