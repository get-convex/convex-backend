use common::knobs::{
    ISOLATE_MAX_HEAP_EXTRA_SIZE,
    ISOLATE_MAX_USER_HEAP_SIZE,
};
use deno_core::v8::{
    self,
};

use crate::strings;

/// Set a 64KB initial heap size
const INITIAL_HEAP_SIZE: usize = 1 << 16;

pub struct Thread {
    pub isolate: v8::OwnedIsolate,
}

impl Thread {
    pub fn new() -> Self {
        let create_params = v8::CreateParams::default().heap_limits(
            INITIAL_HEAP_SIZE,
            *ISOLATE_MAX_USER_HEAP_SIZE + *ISOLATE_MAX_HEAP_EXTRA_SIZE,
        );
        let mut isolate = v8::Isolate::new(create_params);

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
        let scope = &mut unsafe { v8::CallbackScope::new(context) };
        let message = strings::import_meta_unsupported
            .create(scope)
            .expect("Failed to create exception string");
        let exception = v8::Exception::type_error(scope, message);
        scope.throw_exception(exception);
    }
}
