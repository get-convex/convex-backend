use std::{
    ffi,
    ptr,
};

use deno_core::v8::{
    self,
};

use super::{
    callback_context::CallbackContext,
    thread::Thread,
};

// Isolate-level struct scoped to a "session," which enables isolate reuse
// across sessions.
pub struct Session<'a> {
    pub handle_scope: v8::HandleScope<'a, ()>,
    heap_ctx_ptr: *mut HeapContext,
}

impl<'a> Session<'a> {
    pub fn new(reactor_thread: &'a mut Thread) -> Self {
        // Set callbacks on the `Isolate` (via the `HandleScope`) that will
        // potentially read state from our session's contexts.
        let mut handle_scope = v8::HandleScope::new(&mut reactor_thread.isolate);

        // Pass ownership of the HeapContext struct to the heap limit callback, which
        // we'll take back in the `Isolate`'s destructor.
        let heap_context = Box::new(HeapContext {
            handle: handle_scope.thread_safe_handle(),
        });
        let heap_ctx_ptr = Box::into_raw(heap_context);
        handle_scope.add_near_heap_limit_callback(
            Self::near_heap_limit_callback,
            heap_ctx_ptr as *mut ffi::c_void,
        );

        handle_scope.set_promise_reject_callback(CallbackContext::promise_reject_callback);

        handle_scope
            .set_host_import_module_dynamically_callback(CallbackContext::dynamic_import_callback);

        Self {
            handle_scope,
            heap_ctx_ptr,
        }
    }

    extern "C" fn near_heap_limit_callback(
        data: *mut ffi::c_void,
        current_heap_limit: usize,
        _initial_heap_limit: usize,
    ) -> usize {
        let heap_ctx = unsafe { &mut *(data as *mut HeapContext) };

        // XXX: heap_ctx.handle.terminate(TerminationReason::OutOfMemory);
        heap_ctx.handle.terminate_execution();

        // Double heap limit to avoid a hard OOM.
        current_heap_limit * 2
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        if !self.heap_ctx_ptr.is_null() {
            // First remove the callback, so V8 can no longer invoke it.
            self.handle_scope
                .remove_near_heap_limit_callback(Self::near_heap_limit_callback, 0);

            // Now that the callback is gone, we can free its context.
            let heap_ctx: Box<HeapContext> = unsafe { Box::from_raw(self.heap_ctx_ptr) };
            drop(heap_ctx);
            self.heap_ctx_ptr = ptr::null_mut();
        }

        // V8's API allows setting null function pointers here, but rusty_v8
        // does not. Use no-op functions instead.
        extern "C" fn null_promise_reject_callback(_message: v8::PromiseRejectMessage) {}
        self.handle_scope
            .set_promise_reject_callback(null_promise_reject_callback);

        fn null_dynamic_import_callback<'s>(
            _scope: &mut v8::HandleScope<'s>,
            _host_defined_options: v8::Local<'s, v8::Data>,
            _resource_name: v8::Local<'s, v8::Value>,
            _specifier: v8::Local<'s, v8::String>,
            _import_assertions: v8::Local<'s, v8::FixedArray>,
        ) -> Option<v8::Local<'s, v8::Promise>> {
            None
        }
        self.handle_scope
            .set_host_import_module_dynamically_callback(null_dynamic_import_callback);
    }
}

struct HeapContext {
    handle: v8::IsolateHandle,
}

pub enum SessionFailure {
    SystemError(anyhow::Error),
    OutOfMemory,
}
