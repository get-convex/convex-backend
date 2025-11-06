use deno_core::v8::{
    self,
    callback_scope,
};

use crate::strings;

pub struct Thread {
    pub isolate: v8::OwnedIsolate,
}

impl Thread {
    pub fn new() -> Self {
        let mut isolate =
            crate::udf_runtime::create_isolate_with_udf_runtime(v8::CreateParams::default());

        // Tells V8 to capture current stack trace when uncaught exception occurs and
        // report it to the message listeners. The option is off by default.
        isolate.set_capture_stack_trace_for_uncaught_exceptions(
            true, // capture
            10,   // frame_limit
        );

        // We never support the `import.meta` object, so set the callback at this layer.
        isolate.set_host_initialize_import_meta_object_callback(Self::import_meta_callback);

        // Disallow synchronous `Atomics.wait`.
        isolate.set_allow_atomics_wait(false);

        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);

        Self { isolate }
    }

    extern "C" fn import_meta_callback(
        context: v8::Local<v8::Context>,
        _module: v8::Local<v8::Module>,
        _meta: v8::Local<v8::Object>,
    ) {
        callback_scope!(unsafe let scope, context);
        let message = strings::import_meta_unsupported
            .create(scope)
            .expect("Failed to create exception string");
        let exception = v8::Exception::type_error(scope, message);
        scope.throw_exception(exception);
    }
}
