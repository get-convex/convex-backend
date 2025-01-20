mod thread_future;

use std::{
    self,
    pin::Pin,
    sync::{
        Arc,
        LazyLock,
        Weak,
    },
    time::{
        Duration,
        SystemTime,
    },
};

use futures::{
    future::{
        BoxFuture,
        FusedFuture,
    },
    Future,
    FutureExt,
};
use parking_lot::Mutex;
use rand::{
    RngCore,
    SeedableRng,
};
use rand_chacha::ChaCha12Rng;
use thread_future::{
    ThreadFuture,
    ThreadFutureHandle,
};
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
use crate::pause::PauseClient;

pub static CONVEX_EPOCH: LazyLock<SystemTime> =
    LazyLock::new(|| SystemTime::UNIX_EPOCH + Duration::from_secs(1620198000)); // May 5th, 2021 :)

pub struct TestDriver {
    tokio_runtime: Option<tokio::runtime::Runtime>,
    state: Arc<Mutex<TestRuntimeState>>,
    pause_client: PauseClient,
}

impl TestDriver {
    pub fn new() -> Self {
        Self::new_with_seed(0)
    }

    pub fn new_with_seed(seed: u64) -> Self {
        Self::new_with_config(seed, PauseClient::new())
    }

    pub fn new_with_pause_client(pause_client: PauseClient) -> Self {
        Self::new_with_config(0, pause_client)
    }

    pub fn new_with_config(seed: u64, pause_client: PauseClient) -> Self {
        let tokio_seed = RngSeed::from_bytes(&seed.to_le_bytes());
        let tokio_runtime = Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .unhandled_panic(UnhandledPanic::ShutdownRuntime)
            .rng_seed(tokio_seed)
            .build()
            .expect("Failed to create Tokio runtime");
        let rng = ChaCha12Rng::seed_from_u64(seed);
        let creation_time = {
            let _handle = tokio_runtime.enter();
            tokio::time::Instant::now()
        };
        Self {
            tokio_runtime: Some(tokio_runtime),
            state: Arc::new(Mutex::new(TestRuntimeState { rng, creation_time })),
            pause_client,
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
            pause_client: self.pause_client.clone(),
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
}

#[derive(Clone)]
pub struct TestRuntime {
    tokio_handle: tokio::runtime::Handle,
    state: Weak<Mutex<TestRuntimeState>>,
    pause_client: PauseClient,
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

impl Runtime for TestRuntime {
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
    ) -> Box<dyn SpawnHandle> {
        let handle = self.tokio_handle.spawn(f);
        Box::new(TestFutureHandle {
            handle: Some(handle),
        })
    }

    fn spawn_thread<Fut: Future<Output = ()>, F: FnOnce() -> Fut + Send + 'static>(
        &self,
        f: F,
    ) -> Box<dyn SpawnHandle> {
        let handle = self
            .tokio_handle
            .spawn(ThreadFuture::new(self.tokio_handle.clone(), f));
        Box::new(ThreadFutureHandle {
            handle: Some(handle),
        })
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

    fn pause_client(&self) -> PauseClient {
        self.pause_client.clone()
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

pub struct TestFutureHandle {
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl SpawnHandle for TestFutureHandle {
    fn shutdown(&mut self) {
        if let Some(ref mut handle) = self.handle {
            handle.abort();
        }
    }

    fn join(&mut self) -> BoxFuture<'_, Result<(), JoinError>> {
        let handle = self.handle.take();
        let future = async move {
            if let Some(h) = handle {
                h.await?;
            }
            Ok(())
        };
        future.boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::MutexWithTimeout;

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
            let (Ok(()) | Err(JoinError::Canceled)) = r.join().await else {
                panic!("Expected JoinError::Canceled");
            };
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_mutex_with_timeout() -> anyhow::Result<()> {
        let mutex = MutexWithTimeout::new(Duration::from_secs(1), ());
        let _lock = mutex.acquire_lock_with_timeout().await?;
        // Trying to acquire lock while the lock is already held should timeout
        assert!(mutex.acquire_lock_with_timeout().await.is_err());
        Ok(())
    }
}
