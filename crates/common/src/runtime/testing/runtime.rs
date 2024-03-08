//! Test implementation of the Runtime trait.

use std::{
    future::Future,
    marker::Send,
    mem,
    ops::{
        Add,
        Sub,
    },
    pin::Pin,
    sync::{
        Arc,
        Weak,
    },
    time::{
        Duration,
        SystemTime,
    },
};

use async_trait::async_trait;
use cmd_util::env::config_test;
use futures::{
    channel::mpsc,
    future::{
        BoxFuture,
        FusedFuture,
    },
    pin_mut,
    task::{
        waker_ref,
        ArcWake,
        Context,
        Poll,
    },
    StreamExt,
};
use parking_lot::{
    Condvar,
    Mutex,
};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use value::heap_size::HeapSize;

use super::{
    future_set::FutureSet,
    handle::{
        InternalThreadHandle,
        JoinHandle,
        TestFutureHandle,
        TestThreadHandle,
    },
    timer::StepTimer,
};
use crate::runtime::{
    Nanos,
    Runtime,
    RuntimeInstant,
};

const DEFAULT_SEED: u64 = 0;

/// Non-cloneable owner of most of the runtime state that's responsible for
/// driving the test runtime forward.
///
/// The `FutureSet` inside will contain futures that have references to
/// `TestRuntime`, so it's important we don't create a reference cycle:
/// `TestRuntime` must not have a path to `TestDriver`. Simple, acyclic state,
/// like the runtime's RNG, can live in the shared `TestRuntimeState` structure.
pub struct TestDriver {
    futures: FutureSet,
    thread_notify: Arc<ThreadNotify>,

    incoming_rx: mpsc::UnboundedReceiver<BoxFuture<'static, ()>>,
    incoming_tx: mpsc::UnboundedSender<BoxFuture<'static, ()>>,

    // The `TestDriver` owns the `TestRuntimeState`, and it has the unique strong
    // reference. Therefore, we can be guaranteed that it gets dropped when the
    // `TestDriver` drops.
    //
    // NB: It's important that `state` drops after `futures`, since we want all threads to be
    // shutting down from their input channels being dropped.
    state: Arc<Mutex<TestRuntimeState>>,
}

impl Drop for TestDriver {
    fn drop(&mut self) {
        assert_eq!(
            Arc::strong_count(&self.state),
            1,
            "TestDriver leaked a reference to its internal state"
        );
    }
}

struct TestRuntimeState {
    timer: StepTimer,
    rng: ChaCha12Rng,

    // Note that the `JoinHandle` does not keep the thread alive, so this does not introduce a
    // reference cycle involving `TestRuntimeState`.
    threads: Vec<Arc<Mutex<JoinHandle>>>,
}

impl Drop for TestRuntimeState {
    fn drop(&mut self) {
        let mut std_handles = vec![];
        for handle in self.threads.drain(..) {
            let mut handle = handle.lock();
            match mem::replace(&mut *handle, JoinHandle::Joining) {
                JoinHandle::Running(h) => std_handles.push(h),
                JoinHandle::Joining => panic!("Joined handle twice?"),
                JoinHandle::Completed(..) => (),
            }
        }
        for handle in std_handles {
            if let Err(e) = handle.join() {
                tracing::error!("Dangling thread panicked: {e:?}");
            }
        }
    }
}

#[derive(Clone)]
pub struct TestRuntime {
    // It's too much of a pain to have `TestRuntime` maintain a borrow against `TestDriver`, so we
    // just use a weak pointer here instead, and we'll panic if a `TestRuntime` is used after its
    // `TestDriver` is dropped.
    state: Weak<Mutex<TestRuntimeState>>,

    // Spawned tasks get queued here until the runtime drives its futures
    // forward, at which point it'll pull them out of this channel and
    // schedule them.
    // This can't be inside `Inner` because we would deadlock if a task wants
    // to spawn additional work while we're polling the futures.
    incoming_tx: mpsc::UnboundedSender<BoxFuture<'static, ()>>,
}

impl TestRuntime {
    fn with_state<R>(&self, f: impl FnOnce(&mut TestRuntimeState) -> R) -> R {
        let state = self
            .state
            .upgrade()
            .expect("TestRuntime is used after `TestDriver` has been dropped");
        let mut state = state.lock();
        f(&mut state)
    }

    pub fn set_timer_auto_complete(&self, on: bool) {
        self.with_state(|state| state.timer.set_auto_complete(on));
    }

    pub fn advance_time(&self, duration: Duration) {
        self.with_state(|state| state.timer.advance_time(duration));
    }
}

impl TestDriver {
    pub fn new() -> Self {
        Self::new_with_seed(DEFAULT_SEED)
    }

    pub fn new_with_seed(seed: u64) -> Self {
        config_test();
        let (sender, receiver) = mpsc::unbounded();
        let state = TestRuntimeState {
            timer: StepTimer::new(),
            rng: ChaCha12Rng::seed_from_u64(seed),
            threads: vec![],
        };
        Self {
            futures: FutureSet::new(),
            thread_notify: Arc::new(ThreadNotify {
                cv: Condvar::new(),
                mutex: Mutex::new(false),
            }),
            incoming_rx: receiver,
            incoming_tx: sender,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn rt(&self) -> TestRuntime {
        TestRuntime {
            state: Arc::downgrade(&self.state),
            incoming_tx: self.incoming_tx.clone(),
        }
    }

    /// Run the given future on the runtime, potentially scheduling other tasks
    /// submitted to the runtime with `spawn`. There may be incomplete tasks
    /// left in the runtime's executor which can be resumed with further
    /// calls.
    pub fn run_until<F: Future>(&mut self, fut: F) -> F::Output {
        pin_mut!(fut);
        let notify = self.thread_notify.clone();
        let waker = waker_ref(&notify);
        let mut cx = Context::from_waker(&waker);
        loop {
            // If the provided future is done, so are we.
            if let Poll::Ready(output) = fut.as_mut().poll(&mut cx) {
                return output;
            }

            // Advance the runtime.
            self.poll_runtime(&mut cx);

            // If we're ready, where polling the runtime either woke itself or
            // our future up, go back to the start and run another timeslice.
            if ThreadNotify::take_notification(&notify) {
                continue;
            }
            // Otherwise, potentially advance time and wait for a wakeup.
            {
                let mut state = self.state.lock();
                if state.timer.auto_complete() && !state.timer.is_empty() {
                    state.timer.step();
                    continue;
                }
            }
            ThreadNotify::wait(&notify);
        }
    }

    /// Make maximal progress on the pool of spawned tasks, returning only when
    /// no more progress can be made. Does not satisfy any timer requests.
    fn poll_runtime(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(task)) = self.incoming_rx.poll_next_unpin(cx) {
            self.futures.insert(task);
        }
        self.futures.poll_ready(cx);
    }

    /// Advance time and poll ready futures once.
    pub fn run_timeslice(&mut self, cx: &mut Context<'_>) {
        {
            let mut state = self.state.lock();
            if !state.timer.is_empty() {
                state.timer.step();
            }
        }
        self.poll_runtime(cx);
    }
}

#[async_trait]
impl Runtime for TestRuntime {
    type Handle = TestFutureHandle;
    type Instant = TestInstant;
    type Rng = ChaCha12Rng;
    type ThreadHandle = TestThreadHandle;

    fn wait(&self, duration: Duration) -> Pin<Box<dyn FusedFuture<Output = ()> + Send + 'static>> {
        self.with_state(|state| state.timer.wait(duration))
    }

    fn spawn(
        &self,
        _name: &'static str,
        f: impl Future<Output = ()> + Send + 'static,
    ) -> Self::Handle {
        let (fut, handle) = TestFutureHandle::spawn(f);
        self.incoming_tx
            .unbounded_send(Box::pin(fut))
            .expect("spawn failed; TestRuntime is gone");
        handle
    }

    fn spawn_thread<Fut: Future<Output = ()>, F: FnOnce() -> Fut + Send + 'static>(
        &self,
        f: F,
    ) -> Self::ThreadHandle {
        let (internal_handle, thread_handle, join_handle) = InternalThreadHandle::spawn(f);
        self.incoming_tx
            .unbounded_send(Box::pin(internal_handle))
            .expect("spawn failed; TestRuntime is gone");
        self.with_state(|state| state.threads.push(join_handle));
        thread_handle
    }

    fn system_time(&self) -> SystemTime {
        self.with_state(|state| state.timer.current_time())
    }

    fn monotonic_now(&self) -> TestInstant {
        TestInstant {
            rt: self.clone(),
            now: self.with_state(|state| state.timer.current_time()),
        }
    }

    fn with_rng<R>(&self, f: impl FnOnce(&mut Self::Rng) -> R) -> R {
        self.with_state(|state| f(&mut state.rng))
    }
}

#[derive(Clone)]
pub struct TestInstant {
    rt: TestRuntime,
    now: SystemTime,
}

impl RuntimeInstant for TestInstant {
    fn elapsed(&self) -> Duration {
        self.rt
            .system_time()
            .duration_since(self.now)
            .expect("Test runtime's timer went backwards?")
    }

    fn as_nanos(&self) -> Nanos {
        self.rt
            .with_state(|state| Nanos::new(state.timer.timestamp_to_nanos(self.now)))
    }
}

impl HeapSize for TestInstant {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl Sub for TestInstant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Duration {
        self.now
            .duration_since(rhs.now)
            .unwrap_or_else(|_| panic!("{:?} < {:?}", self.now, rhs.now))
    }
}

impl Add<Duration> for TestInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        Self {
            rt: self.rt,
            now: self.now + rhs,
        }
    }
}

impl Ord for TestInstant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.now.cmp(&other.now)
    }
}

impl PartialOrd for TestInstant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for TestInstant {}

impl PartialEq for TestInstant {
    fn eq(&self, other: &Self) -> bool {
        self.now == other.now
    }
}

/// Roughly equivalent to [`futures::executor::local_pool::ThreadNotify`]
struct ThreadNotify {
    cv: Condvar,
    mutex: Mutex<bool>,
}
impl ArcWake for ThreadNotify {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut unparked = arc_self.mutex.lock();
        *unparked = true;
        arc_self.cv.notify_all();
    }
}

impl ThreadNotify {
    fn take_notification(arc_self: &Arc<Self>) -> bool {
        let mut unparked = arc_self.mutex.lock();
        let ret = *unparked;
        *unparked = false;
        ret
    }

    fn wait(arc_self: &Arc<Self>) {
        let mut unparked = arc_self.mutex.lock();
        loop {
            if *unparked {
                return;
            }
            arc_self.cv.wait(&mut unparked);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::TestDriver;
    use crate::runtime::Runtime;

    #[test]
    fn test_block_on_timer() {
        let mut td = TestDriver::new();
        let wait = td.rt().wait(Duration::from_secs(1));
        let fut = async move {
            wait.await;
            Result::<(), anyhow::Error>::Ok(())
        };
        assert!(td.run_until(fut).is_ok());
    }
}
