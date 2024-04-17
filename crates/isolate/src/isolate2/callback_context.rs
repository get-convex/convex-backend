use anyhow::anyhow;
use deno_core::v8;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use serde_json::Value as JsonValue;

use super::{
    client::PendingAsyncSyscall,
    context_state::ContextState,
};
use crate::{
    environment::UncatchableDeveloperError,
    helpers::{
        self,
        to_rust_string,
    },
    ops::run_op,
};

pub struct CallbackContext<'callback, 'scope: 'callback> {
    pub scope: &'callback mut v8::HandleScope<'scope>,
    context: v8::Local<'scope, v8::Context>,
}

impl<'callback, 'scope> CallbackContext<'callback, 'scope> {
    fn new(scope: &'callback mut v8::HandleScope<'scope>) -> Self {
        let context = scope.get_current_context();
        Self { scope, context }
    }

    pub fn context_state(&mut self) -> anyhow::Result<&mut ContextState> {
        self.context
            .get_slot_mut::<ContextState>(self.scope)
            .ok_or_else(|| anyhow::anyhow!("ContextState not found in context"))
    }

    pub fn syscall(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let mut ctx = CallbackContext::new(scope);
        match ctx.syscall_impl(args) {
            Ok(v) => rv.set(v),
            Err(e) => ctx.handle_syscall_or_op_error(e),
        }
    }

    fn syscall_impl(
        &mut self,
        args: v8::FunctionCallbackArguments,
    ) -> anyhow::Result<v8::Local<'scope, v8::Value>> {
        if args.length() != 2 {
            // There's not really an expected developer mistake that would lead to them
            // calling Convex.syscall incorrectly -- the bug must be in our
            // convex/server code. Treat this as a system error.
            anyhow::bail!("syscall(name, arg_object) takes two arguments");
        }
        let name: v8::Local<v8::String> = args.get(0).try_into()?;
        let name = to_rust_string(self.scope, &name)?;

        let args_v8: v8::Local<v8::String> = args.get(1).try_into()?;
        let args_s = to_rust_string(self.scope, &args_v8)?;
        let args_v: JsonValue = serde_json::from_str(&args_s).map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "SyscallArgsInvalidJson",
                format!("Received invalid json: {e}"),
            ))
        })?;

        let result = self.context_state()?.environment.syscall(&name, args_v)?;

        let value_s = serde_json::to_string(&result)?;
        let value_v8 = v8::String::new(self.scope, &value_s[..])
            .ok_or_else(|| anyhow!("Failed to create result string"))?;

        Ok(value_v8.into())
    }

    pub fn async_syscall(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let mut ctx = CallbackContext::new(scope);
        match ctx.start_async_syscall_impl(args) {
            Ok(p) => rv.set(p.into()),
            Err(e) => ctx.handle_syscall_or_op_error(e),
        }
    }

    fn start_async_syscall_impl(
        &mut self,
        args: v8::FunctionCallbackArguments,
    ) -> anyhow::Result<v8::Local<'scope, v8::Promise>> {
        if args.length() != 2 {
            // There's not really an expected developer mistake that would lead to them
            // calling Convex.asyncSyscall incorrectly -- the bug must be in our
            // convex/server code. Treat this as a system error.
            anyhow::bail!("asyncSyscall(name, arg_object) takes two arguments");
        }
        let name: v8::Local<v8::String> = args.get(0).try_into()?;
        let name = to_rust_string(self.scope, &name)?;

        let args_v8: v8::Local<v8::String> = args.get(1).try_into()?;
        let args_s = to_rust_string(self.scope, &args_v8)?;
        let args_v: JsonValue = serde_json::from_str(&args_s).map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "SyscallArgsInvalidJson",
                format!("Received invalid json: {e}"),
            ))
        })?;

        let promise_resolver = v8::PromiseResolver::new(self.scope)
            .ok_or_else(|| anyhow::anyhow!("Failed to create v8::PromiseResolver"))?;

        let promise = promise_resolver.get_promise(self.scope);
        let resolver = v8::Global::new(self.scope, promise_resolver);
        {
            let context_state = self.context_state()?;

            let promise_id = context_state.next_promise_id;
            context_state.next_promise_id += 1;

            let pending_async_syscall = PendingAsyncSyscall {
                promise_id,
                name,
                args: args_v,
            };
            context_state
                .pending_async_syscalls
                .push(pending_async_syscall);

            context_state.promise_resolvers.insert(promise_id, resolver);
        };
        Ok(promise)
    }

    pub fn op(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        rv: v8::ReturnValue,
    ) {
        let mut ctx = CallbackContext::new(scope);
        if let Err(e) = run_op(&mut ctx, args, rv) {
            ctx.handle_syscall_or_op_error(e);
        }
    }

    pub extern "C" fn promise_reject_callback(message: v8::PromiseRejectMessage) {
        let mut scope = unsafe { v8::CallbackScope::new(&message) };

        // NB: If we didn't `Context::enter` above in the stack, it's possible
        // that our scope will be attached to the default context at the top of the
        // stack, which then won't have the `RequestState` slot. This will then cause
        // the call into `ctx.push_unhandled_promise_rejection` to fail with a system
        // error, which we'll just trace out here.
        let mut ctx = CallbackContext::new(&mut scope);

        if let Err(e) = ctx.push_unhandled_promise_rejection(message) {
            tracing::error!("Error in promise_reject_callback: {:?}", e);
        }
    }

    fn push_unhandled_promise_rejection(
        &mut self,
        message: v8::PromiseRejectMessage,
    ) -> anyhow::Result<()> {
        match message.get_event() {
            v8::PromiseRejectEvent::PromiseRejectWithNoHandler => {
                // See comment on PendingUnhandledPromiseRejections.
                // A promise rejection is necessary but not sufficient for an
                // 'unhandledRejection' event, which throws in our runtime.
                // Save the promise and check back in on it once the microtask
                // queue has drained. If it remains unhandled then, throw.
                let Some(e) = message.get_value() else {
                    tracing::warn!("Message missing from call to promise_reject_callback");
                    return Ok(());
                };
                let error_global = v8::Global::new(self.scope, e);
                let promise_global = v8::Global::new(self.scope, message.get_promise());
                self.context_state()?
                    .unhandled_promise_rejections
                    .insert(promise_global, error_global);
            },
            v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject => {
                tracing::warn!("Promise handler added after reject");
                // If this promise was previously a candidate for an
                // 'unhandledRejection' event, disqualify it by removing it
                // from `pending_unhandled_promise_rejections`.
                let promise_global = v8::Global::new(self.scope, message.get_promise());
                self.context_state()?
                    .unhandled_promise_rejections
                    .remove(&promise_global);
                // log_promise_handler_added_after_reject();
            },
            v8::PromiseRejectEvent::PromiseRejectAfterResolved => {
                tracing::warn!("Promise rejected after resolved");
            },
            v8::PromiseRejectEvent::PromiseResolveAfterResolved => {
                tracing::warn!("Promise resolved after resolved");
            },
        }
        Ok(())
    }

    pub fn resolve_module(
        context: v8::Local<'callback, v8::Context>,
        specifier: v8::Local<'callback, v8::String>,
        _import_assertions: v8::Local<'callback, v8::FixedArray>,
        referrer: v8::Local<'callback, v8::Module>,
    ) -> Option<v8::Local<'callback, v8::Module>> {
        let mut scope = unsafe { v8::CallbackScope::new(context) };
        let mut ctx = CallbackContext::new(&mut scope);
        ctx.resolve_module_impl(specifier, referrer)
    }

    fn resolve_module_impl(
        &mut self,
        specifier: v8::Local<'scope, v8::String>,
        referrer: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Module>> {
        let r: anyhow::Result<_> = try {
            let referrer_global = v8::Global::new(self.scope, referrer);
            let specifier_str = helpers::to_rust_string(self.scope, &specifier)?;
            let context_state = self.context_state()?;
            let referrer_name = context_state
                .module_map
                .lookup_by_v8_module(&referrer_global)
                .ok_or_else(|| anyhow!("Module not registered"))?
                .to_string();
            let resolved_specifier = deno_core::resolve_import(&specifier_str, &referrer_name)?;
            let module = context_state
                .module_map
                .lookup_module(&resolved_specifier)
                .ok_or_else(|| anyhow!("Couldn't find {resolved_specifier}"))?
                .clone();
            v8::Local::new(self.scope, module)
        };
        match r {
            Ok(m) => Some(m),
            Err(e) => {
                // XXX: This should be a system error!
                helpers::throw_type_error(self.scope, format!("{:?}", e));
                None
            },
        }
    }

    pub fn dynamic_import_callback<'a>(
        scope: &mut v8::HandleScope<'a>,
        _host_defined_options: v8::Local<'a, v8::Data>,
        resource_name: v8::Local<'a, v8::Value>,
        specifier: v8::Local<'a, v8::String>,
        _import_assertions: v8::Local<'a, v8::FixedArray>,
    ) -> Option<v8::Local<'a, v8::Promise>> {
        let mut ctx = CallbackContext::new(scope);
        match ctx.start_dynamic_import(resource_name, specifier) {
            Ok(promise) => Some(promise),
            Err(e) => {
                // XXX: distinguish between system and user errors here.
                helpers::throw_type_error(scope, format!("{:?}", e));
                None
            },
        }
    }

    pub fn start_dynamic_import(
        &mut self,
        resource_name: v8::Local<'scope, v8::Value>,
        specifier: v8::Local<'scope, v8::String>,
    ) -> anyhow::Result<v8::Local<'scope, v8::Promise>> {
        let promise_resolver = v8::PromiseResolver::new(self.scope)
            .ok_or_else(|| anyhow::anyhow!("Failed to create v8::PromiseResolver"))?;

        let promise = promise_resolver.get_promise(self.scope);
        let resolver = v8::Global::new(self.scope, promise_resolver);

        let resource_name: v8::Local<v8::String> = resource_name.try_into()?;
        let referrer_name = helpers::to_rust_string(self.scope, &resource_name)?;
        let specifier_str = helpers::to_rust_string(self.scope, &specifier)?;

        let resolved_specifier = deno_core::resolve_import(&specifier_str, &referrer_name)
            .map_err(|e| ErrorMetadata::bad_request("InvalidImport", e.to_string()))?;

        self.context_state()?
            .pending_dynamic_imports
            .push((resolved_specifier, resolver));

        Ok(promise)
    }

    fn handle_syscall_or_op_error(&mut self, err: anyhow::Error) {
        if let Some(uncatchable_error) = err.downcast_ref::<UncatchableDeveloperError>() {
            // TODO: Terminate the isolate.
            let message = uncatchable_error.js_error.message.to_string();
            let message_v8 = v8::String::new(self.scope, &message[..]).unwrap();
            let exception = v8::Exception::error(self.scope, message_v8);
            self.scope.throw_exception(exception);
            return;
        }

        if err.is_deterministic_user_error() {
            let message = err.user_facing_message();
            let message_v8 = v8::String::new(self.scope, &message[..]).unwrap();
            let exception = v8::Exception::error(self.scope, message_v8);
            self.scope.throw_exception(exception);
            return;
        }

        // TODO: Handle system errors.
        todo!();
    }
}

mod op_provider {
    use std::collections::BTreeMap;

    use bytes::Bytes;
    use common::{
        log_lines::LogLevel,
        runtime::UnixTimestamp,
        types::{
            EnvVarName,
            EnvVarValue,
        },
    };
    use deno_core::{
        v8,
        ModuleSpecifier,
    };
    use rand_chacha::ChaCha12Rng;
    use sourcemap::SourceMap;
    use uuid::Uuid;
    use value::{
        heap_size::WithHeapSize,
        TableMapping,
        TableMappingValue,
        VirtualTableMapping,
    };

    use super::CallbackContext;
    use crate::{
        environment::AsyncOpRequest,
        ops::OpProvider,
        request_scope::StreamListener,
    };

    impl<'callback, 'scope: 'callback> OpProvider<'scope> for CallbackContext<'callback, 'scope> {
        fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
            let state = self.context_state()?;
            state.environment.rng()
        }

        fn scope(&mut self) -> &mut v8::HandleScope<'scope> {
            self.scope
        }

        fn lookup_source_map(
            &mut self,
            specifier: &ModuleSpecifier,
        ) -> anyhow::Result<Option<SourceMap>> {
            let context_state = self.context_state()?;
            let Some(source_map) = context_state.module_map.lookup_source_map(specifier) else {
                return Ok(None);
            };
            Ok(Some(SourceMap::from_slice(source_map.as_bytes())?))
        }

        fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
            self.context_state()?.environment.trace(level, messages)
        }

        fn console_timers(
            &mut self,
        ) -> anyhow::Result<&mut WithHeapSize<BTreeMap<String, UnixTimestamp>>> {
            todo!()
        }

        fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
            self.context_state()?.environment.unix_timestamp()
        }

        fn unix_timestamp_non_deterministic(&mut self) -> anyhow::Result<UnixTimestamp> {
            todo!()
        }

        fn start_async_op(
            &mut self,
            _request: AsyncOpRequest,
            _resolver: v8::Global<v8::PromiseResolver>,
        ) -> anyhow::Result<()> {
            todo!();
        }

        fn create_blob_part(&mut self, _bytes: Bytes) -> anyhow::Result<Uuid> {
            todo!()
        }

        fn get_blob_part(&mut self, _uuid: &Uuid) -> anyhow::Result<Option<Bytes>> {
            todo!()
        }

        fn create_stream(&mut self) -> anyhow::Result<Uuid> {
            todo!()
        }

        fn extend_stream(
            &mut self,
            _id: Uuid,
            _bytes: Option<Bytes>,
            _new_done: bool,
        ) -> anyhow::Result<()> {
            todo!()
        }

        fn new_stream_listener(
            &mut self,
            _stream_id: Uuid,
            _listener: StreamListener,
        ) -> anyhow::Result<()> {
            todo!();
        }

        fn get_environment_variable(
            &mut self,
            _name: EnvVarName,
        ) -> anyhow::Result<Option<EnvVarValue>> {
            todo!()
        }

        fn get_all_table_mappings(
            &mut self,
        ) -> anyhow::Result<(TableMapping, VirtualTableMapping)> {
            todo!()
        }

        fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
            todo!()
        }
    }
}
