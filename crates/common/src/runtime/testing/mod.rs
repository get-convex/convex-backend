use std::{
    mem,
    pin::Pin,
    sync::{
        Arc,
        LazyLock,
        Weak,
    },
    task::{
        Context,
        Poll,
        Waker,
    },
    time::{
        Duration,
        SystemTime,
    },
};

use futures::{
    channel::oneshot,
    future::{
        self,
        FusedFuture,
    },
    pin_mut,
    Future,
    FutureExt,
    TryFutureExt,
};
use parking_lot::Mutex;
use rand::{
    RngCore,
    SeedableRng,
};
use rand_chacha::ChaCha12Rng;
use tokio::runtime::{
    Builder,
    RngSeed,
    UnhandledPanic,
};

use super::{
    JoinError,
    Runtime,
    SpawnHandle,
};
use crate::knobs::RUNTIME_STACK_SIZE;

pub static CONVEX_EPOCH: LazyLock<SystemTime> =
    LazyLock::new(|| SystemTime::UNIX_EPOCH + Duration::from_secs(1620198000)); // May 5th, 2021 :)

pub struct TestDriver {
    tokio_runtime: Option<tokio::runtime::Runtime>,
    state: Arc<Mutex<TestRuntimeState>>,
}

impl TestDriver {
    pub fn new() -> Self {
        Self::new_with_seed(0)
    }

    pub fn new_with_seed(seed: u64) -> Self {
        let tokio_seed = RngSeed::from_bytes(&seed.to_le_bytes());
        let tokio_runtime = Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .unhandled_panic(UnhandledPanic::ShutdownRuntime)
            .rng_seed(tokio_seed)
            .on_thread_start(|| {
                panic!("TestDriver should not start any threads");
            })
            .build()
            .expect("Failed to create Tokio runtime");
        let rng = ChaCha12Rng::seed_from_u64(seed);
        let creation_time = {
            let _handle = tokio_runtime.enter();
            tokio::time::Instant::now()
        };
        Self {
            tokio_runtime: Some(tokio_runtime),
            state: Arc::new(Mutex::new(TestRuntimeState {
                rng,
                creation_time,
                handles: vec![],
            })),
        }
    }

    pub fn rt(&self) -> TestRuntime {
        TestRuntime {
            tokio_handle: self
                .tokio_runtime
                .as_ref()
                .expect("tokio_runtime disappeared?")
                .handle()
                .clone(),
            state: Arc::downgrade(&self.state),
        }
    }

    pub fn run_until<F: Future>(&self, f: F) -> F::Output {
        self.tokio_runtime
            .as_ref()
            .expect("tokio_runtime disappeared?")
            .block_on(f)
    }
}

impl Drop for TestDriver {
    fn drop(&mut self) {
        assert_eq!(Arc::strong_count(&self.state), 1);
        self.tokio_runtime
            .take()
            .expect("tokio_runtime disappeared?")
            .shutdown_background();
    }
}

struct TestRuntimeState {
    creation_time: tokio::time::Instant,
    rng: ChaCha12Rng,
    handles: Vec<Arc<Mutex<JoinHandle>>>,
}

impl Drop for TestRuntimeState {
    fn drop(&mut self) {
        let mut std_handles = vec![];
        for handle in self.handles.drain(..) {
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
    tokio_handle: tokio::runtime::Handle,
    state: Weak<Mutex<TestRuntimeState>>,
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

    pub async fn advance_time(&self, duration: Duration) {
        tokio::time::advance(duration).await
    }
}

enum JoinHandle {
    Running(std::thread::JoinHandle<()>),
    Joining,
    Completed(Result<(), String>),
}

pub struct TestThreadHandle {
    #[allow(unused)]
    local_handle: tokio::task::JoinHandle<()>,
    command_tx: crossbeam_channel::Sender<ThreadCommand>,
    completion_rx: oneshot::Receiver<bool>,
    handle: Arc<Mutex<JoinHandle>>,
}

enum ThreadCommand {
    Poll(Waker),
    Shutdown,
}

impl Runtime for TestRuntime {
    type Handle = TestFutureHandle;
    type ThreadHandle = TestThreadHandle;

    fn wait(&self, duration: Duration) -> Pin<Box<dyn FusedFuture<Output = ()> + Send + 'static>> {
        // NB: `TestRuntime` uses Tokio's current thread runtime with the timer paused,
        // so can still achieve determinism. This sleep will suspend until either time
        // is manually advanced forward, or the Tokio runtime runs out of work to do and
        // auto advances to the next pending timer.
        Box::pin(tokio::time::sleep(duration).fuse())
    }

    fn spawn(
        &self,
        _name: &'static str,
        f: impl Future<Output = ()> + Send + 'static,
    ) -> Self::Handle {
        let handle = self.tokio_handle.spawn(f);
        TestFutureHandle { handle }
    }

    fn spawn_thread<Fut: Future<Output = ()>, F: FnOnce() -> Fut + Send + 'static>(
        &self,
        f: F,
    ) -> Self::ThreadHandle {
        let (command_tx, command_rx) = crossbeam_channel::bounded::<ThreadCommand>(1);
        let (response_tx, response_rx) = crossbeam_channel::bounded(1);
        let (completion_tx, completion_rx) = oneshot::channel();

        let tokio_handle = self.tokio_handle.clone();
        let std_handle = std::thread::Builder::new()
            .stack_size(*RUNTIME_STACK_SIZE)
            .spawn(move || {
                let _guard = tokio_handle.enter();

                let fut = f().fuse();
                pin_mut!(fut);

                loop {
                    let Ok(command) = command_rx.recv() else {
                        // The future was dropped from the TestRuntime; just stop execution and
                        // join.
                        return;
                    };
                    match command {
                        ThreadCommand::Shutdown => {
                            let _ = response_tx.send(Poll::Ready(true));
                            return;
                        },
                        ThreadCommand::Poll(waker) => {
                            let mut cx = Context::from_waker(&waker);
                            let response = fut.poll_unpin(&mut cx).map(|_| false);
                            response_tx.send(response).expect(
                                "TestRuntime went away without waiting for a poll response",
                            );
                            if response.is_ready() {
                                return;
                            }
                        },
                    }
                }
            })
            .expect("Failed to start new thread");

        let handle = Arc::new(Mutex::new(JoinHandle::Running(std_handle)));
        self.with_state(|state| state.handles.push(handle.clone()));

        let command_tx_ = command_tx.clone();
        let mut completion_tx = Some(completion_tx);
        let local_future = future::poll_fn(move |cx| {
            // Forward the poll request to the thread.
            // It's okay if this fails as it indicates the thread panicked or was
            // shut down by the `TestThreadHandle`, and we'll see a response
            // indicating the latter below.
            let _ = command_tx_.send(ThreadCommand::Poll(cx.waker().clone()));

            match response_rx.recv() {
                Ok(Poll::Ready(was_canceled)) => {
                    let _ = completion_tx
                        .take()
                        .expect("Future completed twice?")
                        .send(was_canceled);
                    Poll::Ready(())
                },
                Ok(Poll::Pending) => Poll::Pending,
                Err(..) => {
                    // The thread panicked or shut down without notifying us. Treat this future as
                    // completed.
                    completion_tx.take();
                    Poll::Ready(())
                },
            }
        });
        let local_handle = self.tokio_handle.spawn(local_future);
        TestThreadHandle {
            local_handle,
            command_tx,
            completion_rx,
            handle,
        }
    }

    fn system_time(&self) -> SystemTime {
        let elapsed = tokio::time::Instant::now() - self.with_state(|state| state.creation_time);
        *CONVEX_EPOCH + elapsed
    }

    fn monotonic_now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }

    fn rng(&self) -> Box<dyn RngCore> {
        Box::new(TestRng { rt: self.clone() })
    }
}

struct TestRng {
    rt: TestRuntime,
}

impl RngCore for TestRng {
    fn next_u32(&mut self) -> u32 {
        self.rt.with_state(|state| state.rng.next_u32())
    }

    fn next_u64(&mut self) -> u64 {
        self.rt.with_state(|state| state.rng.next_u64())
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rt.with_state(|state| state.rng.fill_bytes(dest))
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.rt.with_state(|state| state.rng.try_fill_bytes(dest))
    }
}

impl SpawnHandle for TestThreadHandle {
    type Future = Pin<Box<dyn Future<Output = Result<(), JoinError>> + Send>>;

    fn shutdown(&mut self) {
        let _ = self.command_tx.try_send(ThreadCommand::Shutdown);
    }

    fn into_join_future(self) -> Self::Future {
        async move {
            // Handle clean exit (either by shutdown or task completion)
            if let Ok(was_canceled) = self.completion_rx.await {
                return if !was_canceled {
                    Ok(())
                } else {
                    Err(JoinError::Canceled)
                };
            }

            let std_handle = {
                // Holding the lock only while swapping the state doesn't protect us from
                // deadlocks where we join on our own thread, but it's better to be defensive
                // and not hold a lock during the potentially long join.
                let mut handle = self.handle.lock();
                match std::mem::replace(&mut *handle, JoinHandle::Joining) {
                    JoinHandle::Running(h) => h,
                    JoinHandle::Joining => panic!("Handle joined twice?"),
                    JoinHandle::Completed(r) => {
                        *handle = JoinHandle::Completed(r.clone());
                        let message = r.expect_err("Unclean exit didn't produce a panic?");
                        return Err(JoinError::Panicked(anyhow::anyhow!(message)));
                    },
                }
            };
            let message = std_handle
                .join()
                .expect_err("Unclean exit didn't produce a panic?")
                .downcast::<&str>()
                .expect("Panic message must be a string")
                .to_string();
            {
                let mut handle = self.handle.lock();
                *handle = JoinHandle::Completed(Err(message.clone()));
            }
            Err(JoinError::Panicked(anyhow::anyhow!(message)))
        }
        .boxed()
    }
}

pub struct TestFutureHandle {
    handle: tokio::task::JoinHandle<()>,
}

impl SpawnHandle for TestFutureHandle {
    type Future = Pin<Box<dyn Future<Output = Result<(), JoinError>> + Send>>;

    fn shutdown(&mut self) {
        self.handle.abort();
    }

    fn into_join_future(self) -> Self::Future {
        self.handle.map_err(|e| e.into()).boxed()
    }
}

#[test]
fn test_runtime2() -> anyhow::Result<()> {
    let td = TestDriver::new_with_seed(0);
    let rt = td.rt();
    td.run_until(async {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut r = rt.spawn_thread(|| async move {
            println!("hi!");
            let _ = tx.send(());
        });
        println!("there!");
        let _ = rx.await;
        r.shutdown();
        let (Ok(()) | Err(JoinError::Canceled)) = r.into_join_future().await else {
            panic!("Expected JoinError::Canceled");
        };
    });
    Ok(())
}
