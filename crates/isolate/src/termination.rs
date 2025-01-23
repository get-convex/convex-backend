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
    SystemError(Option<anyhow::Error>), // None if error already handled.
    UncatchableDeveloperError(JsError),
    UnhandledPromiseRejection(JsError),
    UserTimeout(Duration),
    SystemTimeout(Duration),
    OutOfMemory,
}

impl TerminationReason {
    fn take(&mut self) -> Self {
        match self {
            Self::SystemError(e) => Self::SystemError(e.take()),
            Self::UncatchableDeveloperError(e) => Self::UncatchableDeveloperError(e.clone()),
            Self::UnhandledPromiseRejection(e) => Self::UnhandledPromiseRejection(e.clone()),
            Self::UserTimeout(d) => Self::UserTimeout(*d),
            Self::SystemTimeout(d) => Self::SystemTimeout(*d),
            Self::OutOfMemory => Self::OutOfMemory,
        }
    }

    fn not_clean(&self) -> IsolateNotClean {
        match self {
            Self::SystemError(_) => IsolateNotClean::SystemError,
            Self::UncatchableDeveloperError(_) => IsolateNotClean::UncatchableDeveloperError,
            Self::UnhandledPromiseRejection(_) => IsolateNotClean::UnhandledPromiseRejection,
            Self::UserTimeout(_) => IsolateNotClean::UserTimeout,
            Self::SystemTimeout(_) => IsolateNotClean::SystemTimeout,
            Self::OutOfMemory => IsolateNotClean::OutOfMemory,
        }
    }
}

pub struct IsolateHandleInner {
    // Reason is set to Some when the isolate is terminated.
    // If the isolate is terminated, it should be dropped and a new isolate
    // should be created. Recovering after terminating an isolate is sometimes
    // possible but has edge cases we don't want to handle.
    reason: Option<TerminationReason>,
    // Incrementing counter identifying the current context running in the
    // isolate.
    context_id: usize,
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
                context_id: 0,
                request_stream_bytes: None,
            })),
        }
    }

    pub fn update_request_stream_bytes(&self, request_stream_bytes: usize) {
        let mut inner = self.inner.lock();
        inner.request_stream_bytes = Some(request_stream_bytes)
    }

    pub fn terminate(&self, reason: TerminationReason) {
        self.v8_handle.terminate_execution();
        let mut inner = self.inner.lock();
        if inner.reason.is_none() {
            inner.reason = Some(reason);
        } else {
            report_error_sync(&mut anyhow::anyhow!(
                "termination after already terminated: {reason:?}"
            ));
        }
    }

    pub fn terminate_and_throw(&self, reason: TerminationReason) -> anyhow::Result<!> {
        self.terminate(reason);
        anyhow::bail!("terminating isolate and throwing to return early");
    }

    pub fn is_not_clean(&self) -> Option<IsolateNotClean> {
        self.inner
            .lock()
            .reason
            .as_ref()
            .map(|reason| reason.not_clean())
    }

    pub fn check_terminated(&self) -> anyhow::Result<()> {
        if let Some(e) = self.is_not_clean() {
            anyhow::bail!(
                "Optimistic termination check failed, ending execution early: {:?}",
                e
            );
        }
        Ok(())
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
            Some(reason) => {
                match reason.take() {
                    TerminationReason::SystemError(e) => match e {
                        Some(e) => Err(e),
                        None => anyhow::bail!("isolate terminated and reason already processed"),
                    },
                    TerminationReason::SystemTimeout(max_duration) => Err(anyhow::anyhow!(
                        "Hit maximum total syscall duration (maximum duration: {max_duration:?})"
                    )
                    .context(ErrorMetadata::overloaded(
                        "SystemTimeoutError",
                        TIMEOUT_ERROR_MESSAGE,
                    ))),

                    TerminationReason::UnhandledPromiseRejection(e) => Ok(Err(e)),
                    // OutOfMemory and timeout errors are always the user's fault.
                    TerminationReason::UserTimeout(max_duration) => Ok(Err(JsError::from_message(
                        format!("{}", UserTimeoutError(max_duration)),
                    ))),
                    TerminationReason::OutOfMemory => {
                        log_isolate_out_of_memory();
                        // We report this error here because otherwise it is only surfaced to users
                        // since it is a JsError. Reporting to sentry
                        // enables us to see what instance the request came from.
                        let error = ErrorMetadata::bad_request(
                            "IsolateOutOfMemory",
                            format!(
                                "Isolate ran out of memory during execution with \
                                 request_stream_size: {:?},  heap stats: {heap_stats:?} in \
                                 {source:?}",
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
                    TerminationReason::UncatchableDeveloperError(e) => Ok(Err(e)),
                }
            },
        }
    }

    pub fn new_context_created(&self) -> ContextHandle {
        let mut inner = self.inner.lock();
        inner.context_id += 1;
        ContextHandle {
            isolate_handle: self.clone(),
            context_id: inner.context_id,
        }
    }
}

#[derive(Clone)]
pub struct ContextHandle {
    isolate_handle: IsolateHandle,
    context_id: usize,
}

impl ContextHandle {
    pub fn terminate(&self, reason: TerminationReason) {
        if self.context_id != self.isolate_handle.inner.lock().context_id {
            tracing::error!(
                "termination after context {} completed: {:?}",
                self.context_id,
                reason
            );
            return;
        }
        self.isolate_handle.terminate(reason)
    }
}

#[derive(Debug, Error)]
#[error("JavaScript execution ran out of memory (maximum memory usage: 64 MB)")]
pub struct OutOfMemoryError;

#[derive(Error, Debug)]
#[error("Function execution timed out (maximum duration: {0:?})")]
pub struct UserTimeoutError(Duration);
