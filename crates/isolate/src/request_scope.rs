use std::{
    collections::{
        BTreeMap,
        VecDeque,
    },
    marker::PhantomData,
};

use anyhow::anyhow;
use common::{
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    sync::spsc,
};
use deno_core::{
    serde_v8,
    v8,
    ModuleSpecifier,
};
use encoding_rs::Decoder;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

use crate::{
    concurrency_limiter::ConcurrencyPermit,
    environment::{
        IsolateEnvironment,
        UncatchableDeveloperError,
    },
    execution_scope::{
        ExecutionScope,
        PendingDynamicImports,
        PendingUnhandledPromiseRejections,
    },
    helpers::{
        self,
        pump_message_loop,
    },
    isolate::{
        Isolate,
        SETUP_URL,
    },
    metrics::{
        context_build_timer,
        load_setup_module_timer,
        log_promise_handler_added_after_reject,
        log_promise_rejected_after_resolved,
        log_promise_resolved_after_resolved,
        run_setup_module_timer,
    },
    module_map::ModuleMap,
    ops::{
        run_op,
        start_async_op,
        CryptoOps,
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
};

/// This structure maintains a `v8::Context` (inside a `v8::HandleScope`)
/// that's set up with our `RequestState` and `ModuleMap`. This scope lasts for
/// the entirety of a request, where executing code may enter into potentially
/// nested [`ExecutionScope`]s.
pub struct RequestScope<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> {
    // NB: The default type parameter to `HandleScope` indicates that it has a `Context`, so
    // this scope is attached to our request's context. The `v8::HandleScope<()>`, on
    // the other hand, does not have a currently executing context.
    pub(crate) scope: &'a mut v8::HandleScope<'b>,
    pub(crate) handle: IsolateHandle,
    pub(crate) _pd: PhantomData<(RT, E)>,
}

/// Custom per-request state. All environments have a timeout.
/// Note the IsolateHandle and ModuleMap are stored on separate slots, so
/// they can be fetched without needing the environment type E.
pub struct RequestState<RT: Runtime, E: IsolateEnvironment<RT>> {
    pub rt: RT,
    pub timeout: Timeout<RT>,
    pub permit: Option<ConcurrencyPermit>,
    pub environment: E,

    pub blob_parts: WithHeapSize<BTreeMap<uuid::Uuid, bytes::Bytes>>,
    pub streams: WithHeapSize<BTreeMap<uuid::Uuid, anyhow::Result<ReadableStream>>>,
    pub stream_listeners: WithHeapSize<BTreeMap<uuid::Uuid, StreamListener>>,
    /// Tracks bytes read in HTTP action requests
    pub request_stream_state: Option<RequestStreamState>,
    pub console_timers: WithHeapSize<BTreeMap<String, UnixTimestamp>>,
    // This is not wrapped in `WithHeapSize` so we can return `&mut TextDecoderStream`.
    // Additionally, `TextDecoderResource` should have a fairly small heap size.
    pub text_decoders: BTreeMap<uuid::Uuid, TextDecoderResource>,
}

pub struct RequestStreamState {
    stream_id: uuid::Uuid,
    bytes_read: usize,
}

impl RequestStreamState {
    fn new(stream_id: uuid::Uuid) -> Self {
        Self {
            stream_id,
            bytes_read: 0,
        }
    }

    pub fn stream_id(&self) -> uuid::Uuid {
        self.stream_id
    }

    pub fn track_bytes_read(&mut self, bytes_read: usize) {
        self.bytes_read += bytes_read
    }

    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

pub struct TextDecoderResource {
    pub decoder: Decoder,
    pub fatal: bool,
}

#[derive(Debug, Default)]
pub struct ReadableStream {
    pub parts: WithHeapSize<VecDeque<uuid::Uuid>>,
    pub done: bool,
}

impl HeapSize for ReadableStream {
    fn heap_size(&self) -> usize {
        self.parts.heap_size()
    }
}

pub enum StreamListener {
    JsPromise(v8::Global<v8::PromiseResolver>),
    RustStream(spsc::UnboundedSender<anyhow::Result<bytes::Bytes>>),
}

impl HeapSize for StreamListener {
    fn heap_size(&self) -> usize {
        // TODO: Implement HeapSize for `spsc::UnboundedSender` and fill this out.
        0
    }
}

impl<RT: Runtime, E: IsolateEnvironment<RT>> RequestState<RT, E> {
    pub fn create_blob_part(&mut self, bytes: bytes::Bytes) -> anyhow::Result<uuid::Uuid> {
        let rng = self.environment.rng()?;
        let uuid = CryptoOps::random_uuid(rng)?;
        self.blob_parts.insert(uuid, bytes);
        Ok(uuid)
    }

    pub fn create_stream(&mut self) -> anyhow::Result<uuid::Uuid> {
        let rng = self.environment.rng()?;
        let uuid = CryptoOps::random_uuid(rng)?;
        self.streams.insert(uuid, Ok(ReadableStream::default()));
        Ok(uuid)
    }

    pub fn create_request_stream(&mut self) -> anyhow::Result<uuid::Uuid> {
        let uuid = self.create_stream()?;
        self.request_stream_state = Some(RequestStreamState::new(uuid));
        Ok(uuid)
    }

    pub fn create_text_decoder(
        &mut self,
        decoder: TextDecoderResource,
    ) -> anyhow::Result<uuid::Uuid> {
        let rng = self.environment.rng()?;
        let uuid = CryptoOps::random_uuid(rng)?;
        self.text_decoders.insert(uuid, decoder);
        Ok(uuid)
    }

    pub fn get_text_decoder(
        &mut self,
        decoder_id: &uuid::Uuid,
    ) -> anyhow::Result<&mut TextDecoderResource> {
        let decoder = self
            .text_decoders
            .get_mut(decoder_id)
            .ok_or_else(|| anyhow::anyhow!("Text decoder resource not found"))?;
        Ok(decoder)
    }

    pub fn remove_text_decoder(
        &mut self,
        decoder_id: &uuid::Uuid,
    ) -> anyhow::Result<TextDecoderResource> {
        let decoder = self
            .text_decoders
            .remove(decoder_id)
            .ok_or_else(|| anyhow::anyhow!("Text decoder resource not found"))?;
        Ok(decoder)
    }

    #[allow(unused)]
    pub fn read_part(&self, id: uuid::Uuid) -> anyhow::Result<bytes::Bytes> {
        self.blob_parts
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("unrecognized blob id {id}"))
            .cloned()
    }

    /// As the name implies, the time returned by this function would be a
    /// source of non-determinism, so should not be externalized to
    /// functions. An example of a safe use of this function is `console.time`
    /// where we log a duration, but do not leak this time or duration to the
    /// function.
    pub fn unix_timestamp_non_deterministic(&self) -> UnixTimestamp {
        self.rt.unix_timestamp()
    }
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> RequestScope<'a, 'b, RT, E> {
    #[fastrace::trace]
    pub async fn new(
        scope: &'a mut v8::HandleScope<'b>,
        handle: IsolateHandle,
        state: RequestState<RT, E>,
        allow_dynamic_imports: bool,
    ) -> anyhow::Result<Self> {
        let timer = context_build_timer();

        // These callbacks are global for the entire isolate, so we rely on the
        // isolate only running one request at a time.
        // The callbacks are removed in `Drop` so if they happen between requests
        // they go back to the default behavior.
        scope.set_promise_reject_callback(Self::promise_reject_callback);

        if allow_dynamic_imports {
            scope.set_host_import_module_dynamically_callback(Self::dynamic_import_callback);
        }

        Self::setup_context(scope, state, allow_dynamic_imports)?;
        let mut isolate_context = Self {
            scope,
            handle,
            _pd: PhantomData,
        };
        isolate_context.run_setup_module().await?;
        timer.finish();
        Ok(isolate_context)
    }

    pub(crate) fn setup_context(
        scope: &mut v8::HandleScope,
        state: RequestState<RT, E>,
        allow_dynamic_imports: bool,
    ) -> anyhow::Result<()> {
        let context = scope.get_current_context();
        let global = context.global(scope);

        assert!(context.set_slot(scope, state));
        assert!(context.set_slot(scope, ModuleMap::new()));
        assert!(context.set_slot(scope, PendingUnhandledPromiseRejections::new()));
        assert!(context.set_slot(scope, PendingDynamicImports::new(allow_dynamic_imports)));

        let syscall_template = v8::FunctionTemplate::new(scope, Self::syscall);
        let syscall_value = syscall_template
            .get_function(scope)
            .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;

        let async_syscall_template = v8::FunctionTemplate::new(scope, Self::async_syscall);
        let async_syscall_value = async_syscall_template
            .get_function(scope)
            .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;

        let op_template = v8::FunctionTemplate::new(scope, Self::op);
        let op_value = op_template
            .get_function(scope)
            .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;

        let async_op_template = v8::FunctionTemplate::new(scope, Self::async_op);
        let async_op_value = async_op_template
            .get_function(scope)
            .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;

        let convex_value = v8::Object::new(scope);

        let syscall_key = strings::syscall.create(scope)?;
        convex_value.set(scope, syscall_key.into(), syscall_value.into());

        let op_key = strings::op.create(scope)?;
        convex_value.set(scope, op_key.into(), op_value.into());

        let async_syscall_key = strings::asyncSyscall.create(scope)?;
        convex_value.set(scope, async_syscall_key.into(), async_syscall_value.into());

        let async_op_key = strings::asyncOp.create(scope)?;
        convex_value.set(scope, async_op_key.into(), async_op_value.into());

        let convex_key = strings::Convex.create(scope)?;
        global.set(scope, convex_key.into(), convex_value.into());

        Ok(())
    }

    pub fn handle(&self) -> IsolateHandle {
        self.handle.clone()
    }

    pub(crate) fn op(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        rv: v8::ReturnValue,
    ) {
        let mut scope = ExecutionScope::<RT, E>::new(scope);
        if let Err(e) = run_op(&mut scope, args, rv) {
            Self::handle_syscall_or_op_error(&mut scope, e)
        }
    }

    pub(crate) fn async_op(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        rv: v8::ReturnValue,
    ) {
        let mut scope = ExecutionScope::<RT, E>::new(scope);
        if let Err(e) = start_async_op(&mut scope, args, rv) {
            Self::handle_syscall_or_op_error(&mut scope, e)
        }
    }

    pub(crate) fn syscall(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        rv: v8::ReturnValue,
    ) {
        let mut scope = ExecutionScope::<RT, E>::new(scope);
        if let Err(e) = scope.syscall(args, rv) {
            Self::handle_syscall_or_op_error(&mut scope, e)
        }
    }

    pub(crate) fn async_syscall(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        rv: v8::ReturnValue,
    ) {
        let mut scope = ExecutionScope::<RT, E>::new(scope);
        if let Err(e) = scope.async_syscall(args, rv) {
            Self::handle_syscall_or_op_error(&mut scope, e)
        }
    }

    fn handle_syscall_or_op_error(scope: &mut ExecutionScope<RT, E>, err: anyhow::Error) {
        if let Some(uncatchable_error) = err.downcast_ref::<UncatchableDeveloperError>() {
            scope
                .handle()
                .terminate(TerminationReason::UncatchableDeveloperError(
                    uncatchable_error.js_error.clone(),
                ));
            let message = uncatchable_error.js_error.message.to_string();
            let message_v8 = v8::String::new(scope, &message[..]).unwrap();
            let exception = v8::Exception::error(scope, message_v8);
            scope.throw_exception(exception);
            return;
        }

        if err.is_deterministic_user_error() {
            let message = err.user_facing_message();
            let message_v8 = v8::String::new(scope, &message[..]).unwrap();
            let exception = v8::Exception::error(scope, message_v8);
            scope.throw_exception(exception);
            return;
        }

        // This error is our fault, and we won't externalize it to userspace. Stash it
        // on the UDF context and and forcibly abort the isolate.
        scope
            .handle()
            .terminate(TerminationReason::SystemError(Some(err)));
        // It turns out that `terminate_execution` doesn't, well, terminate execution
        // immediately [1]. So, throw an exception in case the rest of
        // `convex/server`'s syscall handler still runs after this call.
        // Otherwise, it'll observe `syscall` returning successfully and
        // returning `undefined`.
        //
        // [1] https://groups.google.com/g/v8-users/c/PMqxTd7k2wM/m/Io45pgwmgDIJ
        let Ok(message_v8) = strings::internal_error.create(scope) else {
            // We're really in a bad place if we can't allocate a new string. Just return
            // and reenter into JS, since we've already terminated execution
            // above. Even though V8 will continue to execute more user code,
            // our top-level checks will never consider the UDF execution
            // successful.
            return;
        };
        let exception = v8::Exception::error(scope, message_v8);
        scope.throw_exception(exception);
    }

    pub(crate) async fn run_setup_module(&mut self) -> anyhow::Result<()> {
        let timer = load_setup_module_timer();
        let mut scope = ExecutionScope::<RT, E>::new(self.scope);
        let setup_url = ModuleSpecifier::parse(SETUP_URL).unwrap();
        let module = scope.eval_module(&setup_url).await?;
        timer.finish();

        let namespace = module
            .get_module_namespace()
            .to_object(&mut scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;

        let function_str = strings::setup.create(&mut scope)?;
        let function: v8::Local<v8::Function> = namespace
            .get(&mut scope, function_str.into())
            .ok_or_else(|| anyhow!("Couldn't find setup in setup module"))?
            .try_into()?;

        let global = scope.get_current_context().global(&mut scope);
        let timer = run_setup_module_timer();
        scope
            .with_try_catch(|s| function.call(s, global.into(), &[global.into()]))??
            .ok_or_else(|| anyhow!("Successful setup() returned None"))?;
        timer.finish();
        Ok(())
    }

    pub fn scope(&mut self) -> v8::HandleScope {
        v8::HandleScope::new(self.scope)
    }

    /// Begin executing code within a single isolate's scope.
    pub fn enter<'c, 'd>(v8_scope: &'c mut v8::HandleScope<'d>) -> ExecutionScope<'c, 'd, RT, E> {
        ExecutionScope::new(v8_scope)
    }

    pub fn checkpoint(&mut self) {
        self.scope.perform_microtask_checkpoint();
        pump_message_loop(self.scope);
    }

    pub(crate) fn take_state(&mut self) -> Option<RequestState<RT, E>> {
        let context = self.scope.get_current_context();
        context.remove_slot(self.scope)
    }

    pub(crate) fn take_module_map(&mut self) -> Option<ModuleMap> {
        let context = self.scope.get_current_context();
        context.remove_slot(self.scope)
    }

    pub(crate) fn take_pending_unhandled_promise_rejections(
        &mut self,
    ) -> Option<PendingUnhandledPromiseRejections> {
        let context = self.scope.get_current_context();
        context.remove_slot(self.scope)
    }

    pub fn take_pending_dynamic_imports(&mut self) -> Option<PendingDynamicImports> {
        let context = self.scope.get_current_context();
        context.remove_slot(self.scope)
    }

    pub fn take_environment(mut self) -> (E, FunctionExecutionTime) {
        let state = self.take_state().expect("Lost ContextState?");
        (
            state.environment,
            state.timeout.get_function_execution_time(),
        )
    }

    #[allow(unused)]
    pub fn print_heap_statistics(&mut self) {
        let mut stats = v8::HeapStatistics::default();
        self.scope.get_heap_statistics(&mut stats);

        println!("Heap statistics:");
        println!("  total_heap_size: {}", stats.total_heap_size());
        println!(
            "  total_heap_size_executable: {}",
            stats.total_heap_size_executable()
        );
        println!("  total_physical_size: {}", stats.total_physical_size());
        println!("  total_available_size: {}", stats.total_available_size());
        println!(
            "  total_global_handles_size: {}",
            stats.total_global_handles_size()
        );
        println!(
            "  used_global_handles_size: {}",
            stats.used_global_handles_size()
        );
        println!("  used_heap_size: {}", stats.used_heap_size());
        println!("  heap_size_limit: {}", stats.heap_size_limit());
        println!("  malloced_memory: {}", stats.malloced_memory());
        println!("  external_memory: {}", stats.external_memory());
        println!("  peak_malloced_memory: {}", stats.peak_malloced_memory());
        println!(
            "  number_of_native_contexts: {}",
            stats.number_of_native_contexts()
        );
        println!(
            "  number_of_detached_contexts: {}",
            stats.number_of_detached_contexts()
        );
        println!("  does_zap_garbage: {}", stats.does_zap_garbage());
    }

    extern "C" fn promise_reject_callback(message: v8::PromiseRejectMessage) {
        let scope = &mut unsafe { v8::CallbackScope::new(&message) };

        match message.get_event() {
            v8::PromiseRejectEvent::PromiseRejectWithNoHandler => {
                // See comment on PendingUnhandledPromiseRejections.
                // A promise rejection is necessary but not sufficient for an
                // 'unhandledRejection' event, which throws in our runtime.
                // Save the promise and check back in on it once the microtask
                // queue has drained. If it remains unhandled then, throw.
                let Some(e) = message.get_value() else {
                    tracing::warn!("Message missing from call to promise_reject_callback");
                    return;
                };
                let error_global = v8::Global::new(scope, e);
                let promise_global = v8::Global::new(scope, message.get_promise());

                let mut exec_scope = ExecutionScope::<RT, E>::new(scope);
                let rejections = exec_scope.pending_unhandled_promise_rejections_mut();
                rejections.exceptions.insert(promise_global, error_global);
            },
            v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject => {
                tracing::warn!("Promise handler added after reject");
                // If this promise was previously a candidate for an
                // 'unhandledRejection' event, disqualify it by removing it
                // from `pending_unhandled_promise_rejections`.
                let promise_global = v8::Global::new(scope, message.get_promise());
                let mut exec_scope = ExecutionScope::<RT, E>::new(scope);
                let rejections = exec_scope.pending_unhandled_promise_rejections_mut();
                rejections.exceptions.remove(&promise_global);
                log_promise_handler_added_after_reject();
            },
            v8::PromiseRejectEvent::PromiseRejectAfterResolved => {
                log_promise_rejected_after_resolved();
            },
            v8::PromiseRejectEvent::PromiseResolveAfterResolved => {
                log_promise_resolved_after_resolved();
            },
        }
    }

    fn dynamic_import_callback<'s>(
        scope: &mut v8::HandleScope<'s>,
        _host_defined_options: v8::Local<'s, v8::Data>,
        resource_name: v8::Local<'s, v8::Value>,
        specifier: v8::Local<'s, v8::String>,
        _import_assertions: v8::Local<'s, v8::FixedArray>,
    ) -> Option<v8::Local<'s, v8::Promise>> {
        let r: anyhow::Result<_> = try {
            let promise_resolver = v8::PromiseResolver::new(scope)
                .ok_or_else(|| anyhow::anyhow!("Failed to create v8::PromiseResolver"))?;
            let promise = promise_resolver.get_promise(scope);
            let promise_resolver = v8::Global::new(scope, promise_resolver);

            let referrer_name: String = serde_v8::from_v8(scope, resource_name)?;
            let specifier_str = helpers::to_rust_string(scope, &specifier)?;

            let resolved_specifier = deno_core::resolve_import(&specifier_str, &referrer_name)
                .map_err(|e| ErrorMetadata::bad_request("InvalidImport", e.to_string()))?;

            let mut exec_scope = ExecutionScope::<RT, E>::new(scope);
            let dynamic_imports = exec_scope.pending_dynamic_imports_mut();
            if !dynamic_imports.allow_dynamic_imports {
                Err(anyhow::anyhow!(
                    "dynamic_import_callback registered without allow_dynamic_imports?"
                ))?;
            }
            dynamic_imports.push(resolved_specifier, promise_resolver);

            promise
        };
        match r {
            Ok(promise) => Some(promise),
            Err(e) => {
                let mut exec_scope = ExecutionScope::<RT, E>::new(scope);
                Self::handle_syscall_or_op_error(&mut exec_scope, e);
                None
            },
        }
    }
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> Drop for RequestScope<'a, 'b, RT, E> {
    fn drop(&mut self) {
        // Remove state from slot to stop Timeouts.
        self.take_state();
        // Remove module map from slot to avoid memory leak.
        self.take_module_map();
        // Remove rejected promises which shouldn't persist between requests.
        self.take_pending_unhandled_promise_rejections();
        // Remove pending dynamic imports.
        self.take_pending_dynamic_imports();
        // Remove promise reject callback to clean up the isolate between contexts.
        // Ideally we would have a `remove_promise_reject_callback` or set the
        // function pointer to null, but rusty_v8 doesn't seem to support either,
        // so we set the callback to an empty function instead.
        extern "C" fn noop(_: v8::PromiseRejectMessage) {}
        self.scope.set_promise_reject_callback(noop);

        self.scope.set_host_import_module_dynamically_callback(
            Isolate::<RT>::error_dynamic_import_callback,
        )
    }
}
