use std::{
    sync::Arc,
    time::Duration,
};

use common::errors::{
    report_error_sync,
    JsError,
    TIMEOUT_ERROR_MESSAGE,
};
use deno_core::v8;
use errors::ErrorMetadata;
use parking_lot::Mutex;
use thiserror::Error;

use crate::{
    isolate::IsolateNotClean,
    metrics::log_isolate_out_of_memory,
    IsolateHeapStats,
};

#[derive(Debug)]
pub enum TerminationReason {
    Isolate(IsolateTerminationReason),
    Context(ContextTerminationReason),
}

/// An error condition that applies to the entire stack of subfunctions.
/// These cannot be caught and will also force the isolate to be recycled.
#[derive(Debug)]
pub enum IsolateTerminationReason {
    SystemError(Option<anyhow::Error>), // None if error already handled.
    UserTimeout(Duration),
    SystemTimeout(Duration),
    OutOfMemory,
}
/// An error condition that applies to just the currently running
/// subfunction, and can be caught by the parent function.
#[derive(Debug)]
pub enum ContextTerminationReason {
    UncatchableDeveloperError(JsError),
    UnhandledPromiseRejection(JsError),
}

impl IsolateTerminationReason {
    fn not_clean(&self) -> IsolateNotClean {
        match self {
            Self::SystemError(_) => IsolateNotClean::SystemError,
            Self::UserTimeout(_) => IsolateNotClean::UserTimeout,
            Self::SystemTimeout(_) => IsolateNotClean::SystemTimeout,
            Self::OutOfMemory => IsolateNotClean::OutOfMemory,
        }
    }
}

impl From<ContextTerminationReason> for TerminationReason {
    fn from(v: ContextTerminationReason) -> Self {
        Self::Context(v)
    }
}

impl From<IsolateTerminationReason> for TerminationReason {
    fn from(v: IsolateTerminationReason) -> Self {
        Self::Isolate(v)
    }
}

pub struct IsolateHandleInner {
    // Reason is set to Some when the isolate is terminated.
    // If the isolate is terminated, it should be dropped and a new isolate
    // should be created. Recovering after terminating an isolate is sometimes
    // possible but has edge cases we don't want to handle.
    reason: Option<TerminationReason>,
    next_context_id: u64,
    context_stack: Vec<u64>,
    request_stream_bytes: Option<usize>,
}

#[derive(Clone)]
pub struct IsolateHandle {
    v8_handle: v8::IsolateHandle,
    inner: Arc<Mutex<IsolateHandleInner>>,
}

impl IsolateHandle {
    pub fn new(v8_handle: v8::IsolateHandle) -> Self {
        Self {
            v8_handle,
            inner: Arc::new(Mutex::new(IsolateHandleInner {
                reason: None,
                next_context_id: 0,
                context_stack: vec![],
                request_stream_bytes: None,
            })),
        }
    }

    pub fn update_request_stream_bytes(&self, request_stream_bytes: usize) {
        let mut inner = self.inner.lock();
        inner.request_stream_bytes = Some(request_stream_bytes)
    }

    /// Marks the isolate as terminating, which causes all JS frames to unwind,
    /// and saves the `reason`.
    ///
    /// If `reason` is a `TerminationReason::Context`, then the error implicitly
    /// targets only the top context in the context stack, and will be cleared
    /// when `pop_context` is called.
    /// On the other hand, a `TerminationReason::Isolate` kills all contexts in
    /// the stack.
    pub fn terminate(&self, reason: TerminationReason) {
        let mut inner = self.inner.lock();
        // N.B.: call terminate_execution under the lock to synchronize with
        // cancel_terminate_execution in `pop_context`
        self.v8_handle.terminate_execution();
        if let Some(existing_reason) = &inner.reason {
            report_error_sync(&mut anyhow::anyhow!(
                "termination after already terminated: {reason:?}"
            ));
            // Replace the termination reason if the new one is more serious.
            if matches!(existing_reason, TerminationReason::Context(_))
                && matches!(reason, TerminationReason::Isolate(_))
            {
                inner.reason = Some(reason);
            }
        } else {
            inner.reason = Some(reason);
        }
    }

    pub fn terminate_and_throw(&self, reason: TerminationReason) -> anyhow::Result<!> {
        self.terminate(reason);
        anyhow::bail!("terminating isolate and throwing to return early");
    }

    pub fn is_not_clean(&self) -> Option<IsolateNotClean> {
        let inner = self.inner.lock();
        if let Some(TerminationReason::Isolate(reason)) = &inner.reason {
            Some(reason.not_clean())
        } else {
            None
        }
    }

    pub fn check_terminated(&self) -> anyhow::Result<()> {
        let inner = self.inner.lock();
        if let Some(e) = &inner.reason {
            anyhow::bail!(
                "Optimistic termination check failed, ending execution early: {:?}",
                e
            );
        }
        Ok(())
    }

    fn take_context_termination_error(reason: &ContextTerminationReason) -> JsError {
        match reason {
            ContextTerminationReason::UnhandledPromiseRejection(e) => e.clone(),
            ContextTerminationReason::UncatchableDeveloperError(e) => e.clone(),
        }
    }

    pub fn take_termination_error(
        &self,
        heap_stats: Option<IsolateHeapStats>,
        // The isolate environment and function path (if applicable)
        source: &str,
    ) -> anyhow::Result<Result<(), JsError>> {
        let mut inner = self.inner.lock();
        match &mut inner.reason {
            None => Ok(Ok(())),
            Some(TerminationReason::Isolate(reason)) => match reason {
                IsolateTerminationReason::SystemError(e) => match e.take() {
                    Some(e) => Err(e),
                    None => anyhow::bail!("isolate terminated and reason already processed"),
                },
                IsolateTerminationReason::SystemTimeout(max_duration) => Err(anyhow::anyhow!(
                    "Hit maximum total syscall duration (maximum duration: {max_duration:?})"
                )
                .context(ErrorMetadata::overloaded(
                    "SystemTimeoutError",
                    TIMEOUT_ERROR_MESSAGE,
                ))),
                // OutOfMemory and timeout errors are always the user's fault.
                IsolateTerminationReason::UserTimeout(max_duration) => Ok(Err(
                    JsError::from_message(format!("{}", UserTimeoutError(*max_duration))),
                )),
                IsolateTerminationReason::OutOfMemory => {
                    log_isolate_out_of_memory();
                    // We report this error here because otherwise it is only surfaced to users
                    // since it is a JsError. Reporting to sentry
                    // enables us to see what instance the request came from.
                    let error = ErrorMetadata::bad_request(
                        "IsolateOutOfMemory",
                        format!(
                            "Isolate ran out of memory during execution with request_stream_size: \
                             {:?},  heap stats: {heap_stats:?} in {source:?}",
                            inner.request_stream_bytes
                        ),
                    );
                    report_error_sync(&mut error.into());
                    let error_message =
                        if let Some(request_stream_bytes) = inner.request_stream_bytes {
                            format!(
                                "{OutOfMemoryError}: request stream size was \
                                 {request_stream_bytes} bytes"
                            )
                        } else {
                            format!("{OutOfMemoryError}")
                        };
                    Ok(Err(JsError::from_message(error_message)))
                },
            },
            Some(TerminationReason::Context(reason)) => {
                let error = Self::take_context_termination_error(reason);
                inner.reason = None;
                self.v8_handle.cancel_terminate_execution();
                Ok(Err(error))
            },
        }
    }

    pub fn push_context(&self, nested: bool) -> ContextId {
        let mut inner = self.inner.lock();
        let context_id = inner.next_context_id;
        inner.next_context_id += 1;
        if !nested {
            inner.context_stack.clear();
        }
        let handle = ContextId { context_id };
        inner.context_stack.push(context_id);
        handle
    }

    pub fn pop_context(&self, handle: ContextId) -> anyhow::Result<Result<(), JsError>> {
        let mut inner = self.inner.lock();
        anyhow::ensure!(
            inner
                .context_stack
                .last()
                .is_some_and(|id| *id == handle.context_id),
            "pop_context called out of order"
        );
        inner.context_stack.pop();
        if let Some(TerminationReason::Context(reason)) = &inner.reason {
            let error = Self::take_context_termination_error(reason);
            inner.reason = None;
            self.v8_handle.cancel_terminate_execution();
            return Ok(Err(error));
        }
        Ok(Ok(()))
    }
}

#[derive(Clone)]
pub struct ContextId {
    context_id: u64,
}

#[derive(Debug, Error)]
#[error("JavaScript execution ran out of memory (maximum memory usage: 64 MB)")]
pub struct OutOfMemoryError;

#[derive(Error, Debug)]
#[error("Function execution timed out (maximum duration: {0:?})")]
pub struct UserTimeoutError(Duration);
