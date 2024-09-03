use std::{
    sync::Arc,
    time::Duration,
};

use async_broadcast::broadcast;
use common::runtime::{
    Runtime,
    SpawnHandle,
};
use futures::{
    future,
    future::Either,
    select_biased,
    Future,
    FutureExt,
};
use parking_lot::Mutex;

use crate::{
    metrics,
    termination::{
        ContextHandle,
        TerminationReason,
    },
};

/// A `Timeout` is an asynchronous background job that terminates an
/// `IsolateHandle` after some time has passed. The holder of a `Timeout` can
/// temporarily pause the termination countdown with [`Timeout::pause`] and then
/// resume time tracking by dropping the returned guard.
///
/// If the higher level operation succeeds, call `Timeout::finish` to cancel the
/// background job and prevent it from terminating the isolate.
pub struct Timeout<RT: Runtime> {
    handle: RT::Handle,
    inner: Arc<Mutex<TimeoutInner<RT>>>,
    done_rx: async_broadcast::Receiver<()>,
}

struct TimeoutInner<RT: Runtime> {
    rt: RT,

    start: tokio::time::Instant,
    timeout: Option<Duration>,

    // How long has the timeout been in the paused state?
    pause_elapsed: Duration,
    max_time_paused: Option<Duration>,

    state: TimeoutState,
}

impl<RT: Runtime> TimeoutInner<RT> {
    fn termination_reason_or_wait(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<TerminationReason>, impl Future<Output = ()> + 'static> {
        let initial_deadline = self.start + timeout;
        match self.state {
            TimeoutState::Running => {
                // Extend the deadline by the time spent paused.
                let deadline = initial_deadline + self.pause_elapsed;
                let now = self.rt.monotonic_now();
                if now >= deadline {
                    metrics::log_user_timeout();
                    return Ok(Some(TerminationReason::UserTimeout(timeout)));
                }
                // Wait on our current deadline to pass.
                // TODO: Cancel the timer on `Timeout::finish` so we don't keep an
                // `IsolateHandle` alive for the wait duration.
                Err(Either::Left(self.rt.wait(deadline - now)))
            },
            TimeoutState::Paused {
                ref mut pause_done,
                ref pause_start,
            } => {
                let expired = if let Some(max_time_paused) = self.max_time_paused {
                    let total_pause_elapsed = self.pause_elapsed + pause_start.elapsed();
                    if total_pause_elapsed >= max_time_paused {
                        metrics::log_system_timeout();
                        return Ok(Some(TerminationReason::SystemTimeout(max_time_paused)));
                    }
                    Either::Left(self.rt.wait(max_time_paused - total_pause_elapsed))
                } else {
                    Either::Right(future::pending())
                };
                let mut pause_done = pause_done.clone();
                Err(Either::Right(async move {
                    let _ = future::select(expired, pause_done.recv()).await;
                }))
            },
            TimeoutState::Finished { .. } => Ok(None),
        }
    }
}

#[derive(Debug)]
enum TimeoutState {
    Running,
    Paused {
        pause_start: tokio::time::Instant,
        pause_done: async_broadcast::Receiver<()>,
    },
    Finished,
}

impl<RT: Runtime> Drop for Timeout<RT> {
    fn drop(&mut self) {
        self.finish();
    }
}

// We default to counting everything as user time but we exempt async syscalls
// from the user timeout and count them as system time instead.
impl<RT: Runtime> Timeout<RT> {
    pub fn new(
        rt: RT,
        handle: ContextHandle,
        timeout: Option<Duration>,
        max_time_paused: Option<Duration>,
    ) -> Self {
        let start = rt.monotonic_now();
        let inner = TimeoutInner {
            rt: rt.clone(),
            start,
            timeout,
            pause_elapsed: Duration::ZERO,
            max_time_paused,
            state: TimeoutState::Running,
        };
        let inner = Arc::new(Mutex::new(inner));
        let (done_tx, done_rx) = broadcast(1);
        let handle = rt.spawn("isolate_timeout", Self::go(handle, inner.clone(), done_tx));
        Self {
            handle,
            inner,
            done_rx,
        }
    }

    // Returns a future that resolves when the background timeout thread has
    // completed. This can either happen if the isolate has been terminated
    // due to timeout or the Timeout has been dropped due to an error.
    pub fn wait_until_completed(&self) -> impl Future<Output = ()> + 'static {
        let mut done_rx = self.done_rx.clone();
        async move {
            let _ = done_rx.recv().await;
        }
    }

    /// Runs the future until completely or until the timeout has expired.
    /// Returns an error in the latter case.
    pub fn with_timeout<'a, T>(
        &self,
        f: impl Future<Output = T> + 'a,
    ) -> impl Future<Output = anyhow::Result<T>> + 'a {
        let completed = self.wait_until_completed();
        async move {
            select_biased! {
                result = f.fuse() => Ok(result),
                // NOTE: When the background thread returns, it either means we have
                // terminated due to timeout or have been dropped. Either way, it
                // is ok to stop executing. The exact error we throw here doesn't
                // matter since we know the isolate layer overrides the error if there
                // is a termination reason.
                _ = completed.fuse() => anyhow::bail!("Timed out"),
            }
        }
    }

    pub fn pause(&mut self) -> PauseGuard<'_, RT> {
        let (tx, rx) = broadcast(1);
        let pause_start = {
            let mut inner = self.inner.lock();
            let TimeoutState::Running {} = inner.state else {
                panic!("Overlapping calls to timeout.pause()");
            };
            let pause_start = inner.rt.monotonic_now();
            inner.state = TimeoutState::Paused {
                pause_done: rx,
                pause_start,
            };
            pause_start
        };
        PauseGuard {
            timeout: self,
            pause_start,
            pause_done: Some(tx),
        }
    }

    pub fn finish(&mut self) {
        {
            let mut inner = self.inner.lock();
            if matches!(inner.state, TimeoutState::Finished { .. }) {
                return;
            }
            inner.state = TimeoutState::Finished;
        }
        self.handle.shutdown();
    }

    async fn go(
        handle: ContextHandle,
        inner: Arc<Mutex<TimeoutInner<RT>>>,
        done_tx: async_broadcast::Sender<()>,
    ) {
        let timeout = match inner.lock().timeout {
            None => return,
            Some(timeout) => timeout,
        };
        let termination_reason = loop {
            let future = match inner.lock().termination_reason_or_wait(timeout) {
                Ok(termination_reason) => break termination_reason,
                Err(future) => future,
            };
            future.await;
        };
        if let Some(reason) = termination_reason {
            handle.terminate(reason);
        }
        let _ = done_tx.try_broadcast(());
    }

    pub fn get_function_execution_time(&self) -> FunctionExecutionTime {
        let inner = self.inner.lock();
        let elapsed = inner.rt.monotonic_now() - inner.start - inner.pause_elapsed;
        let limit = inner.timeout.unwrap_or(Duration::ZERO);
        FunctionExecutionTime { elapsed, limit }
    }
}

pub struct FunctionExecutionTime {
    pub elapsed: Duration,
    pub limit: Duration,
}

pub struct PauseGuard<'a, RT: Runtime> {
    timeout: &'a mut Timeout<RT>,
    pause_start: tokio::time::Instant,
    pause_done: Option<async_broadcast::Sender<()>>,
}

impl<'a, RT: Runtime> PauseGuard<'a, RT> {
    pub fn resume(self) {
        drop(self);
    }
}

impl<'a, RT: Runtime> Drop for PauseGuard<'a, RT> {
    fn drop(&mut self) {
        let Some(tx) = self.pause_done.take() else {
            return;
        };
        {
            let mut inner = self.timeout.inner.lock();
            assert!(matches!(inner.state, TimeoutState::Paused { .. }));

            inner.pause_elapsed += self.pause_start.elapsed();
            inner.state = TimeoutState::Running;
        }
        let _ = tx.try_broadcast(());
    }
}
