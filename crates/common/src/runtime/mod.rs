//! Runtime trait for abstracting away OS-esque features and allow different
//! implementations for test, dev, prod, etc.

use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    num::TryFromIntError,
    ops::{
        Add,
        Sub,
    },
    pin::Pin,
    sync::LazyLock,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use fastrace::{
    collector::SpanContext,
    func_path,
    future::FutureExt as _,
    Span,
};
use futures::{
    future::{
        BoxFuture,
        FusedFuture,
    },
    select_biased,
    stream,
    FutureExt,
    StreamExt,
    TryStreamExt,
};
pub use governor::nanos::Nanos;
use governor::{
    middleware::NoOpMiddleware,
    state::{
        keyed::DefaultKeyedStateStore,
        InMemoryState,
        NotKeyed,
    },
    Quota,
};
use metrics::CONVEX_METRICS_REGISTRY;
use parking_lot::Mutex;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use rand::RngCore;
use serde::Serialize;
use thiserror::Error;
use tokio::runtime::{
    Handle,
    RuntimeFlavor,
};
use tokio_metrics::Instrumented;
use tokio_metrics_collector::TaskMonitor;
use uuid::Uuid;
use value::heap_size::HeapSize;

use crate::{
    errors::recapture_stacktrace,
    is_canceled::IsCanceled,
    pause::PauseClient,
    types::Timestamp,
};

#[cfg(any(test, feature = "testing"))]
pub mod testing;

#[derive(Error, Debug)]
pub enum JoinError {
    #[error("Future canceled")]
    Canceled,
    #[error("Future panicked: {0:?}")]
    Panicked(anyhow::Error),
}

impl From<tokio::task::JoinError> for JoinError {
    fn from(e: tokio::task::JoinError) -> Self {
        if e.is_canceled() {
            JoinError::Canceled
        } else {
            JoinError::Panicked(anyhow::anyhow!(e
                .into_panic()
                .downcast::<&str>()
                .expect("panic message must be a string")))
        }
    }
}

pub trait SpawnHandle: Send + Sync {
    fn shutdown(&mut self);
    fn join(&mut self) -> BoxFuture<'_, Result<(), JoinError>>;
}

/// Shutdown the associated future, preempting it at its next yield point, and
/// join on its result.
pub async fn shutdown_and_join(mut handle: Box<dyn SpawnHandle>) -> anyhow::Result<()> {
    handle.shutdown();
    if let Err(e) = handle.join().await {
        if !matches!(e, JoinError::Canceled) {
            return Err(e.into());
        }
    }
    Ok(())
}

// Why 20? ¯\_(ツ)_/¯. We use this value a lot elsewhere and it doesn't seem
// unreasonable as a starting point for lightweight things.
const JOIN_BUFFER_SIZE: usize = 20;

pub async fn try_join_buffered<
    RT: Runtime,
    T: Send + 'static,
    C: Default + Send + 'static + Extend<T>,
>(
    name: &'static str,
    tasks: impl Iterator<Item = impl Future<Output = anyhow::Result<T>> + Send + 'static>
        + Send
        + 'static,
) -> anyhow::Result<C> {
    assert_send(
        stream::iter(tasks.map(|task| {
            let span = SpanContext::current_local_parent()
                .map(|ctx| Span::root(format!("{}::{name}", func_path!()), ctx))
                .unwrap_or(Span::noop());
            assert_send(try_join(name, assert_send(task), span))
        }))
        .buffered(JOIN_BUFFER_SIZE)
        .try_collect(),
    )
    .await
}

// Work around "higher-ranked lifetime errors" due to the borrow checker's
// inability (bug) to determine that some futures are in fact send.  See
// https://github.com/rust-lang/rust/issues/102211#issuecomment-1367900125
fn assert_send<'a, T>(
    fut: impl 'a + Send + Future<Output = T>,
) -> impl 'a + Send + Future<Output = T> {
    fut
}

pub async fn try_join_buffer_unordered<
    T: Send + 'static,
    C: Default + Send + 'static + Extend<T>,
>(
    name: &'static str,
    tasks: impl Iterator<Item = impl Future<Output = anyhow::Result<T>> + Send + 'static>
        + Send
        + 'static,
) -> anyhow::Result<C> {
    assert_send(
        stream::iter(tasks.map(|task| {
            let span = SpanContext::current_local_parent()
                .map(|ctx| Span::root(format!("{}::{name}", func_path!()), ctx))
                .unwrap_or(Span::noop());
            try_join(name, task, span)
        }))
        .buffer_unordered(JOIN_BUFFER_SIZE)
        .try_collect(),
    )
    .await
}

pub async fn try_join<T: Send + 'static>(
    name: &'static str,
    fut: impl Future<Output = anyhow::Result<T>> + Send + 'static,
    span: Span,
) -> anyhow::Result<T> {
    let handle = tokio_spawn(name, fut.in_span(span));
    handle.await?.map_err(recapture_stacktrace)
}

/// A Runtime can be considered somewhat like an operating system abstraction
/// for our codebase. Functionality like time, randomness, network access, etc
/// should operate quite differently between test, dev and prod, e.g., we don't
/// want `wait` to actually call `thread::sleep_ms()` in test but instead just
/// to advance local time. This trait should include all functionality that we
/// want to abstract out for different runtime environments so application
/// code can be parameterized by a given runtime implementation.
#[async_trait]
pub trait Runtime: Clone + Sync + Send + 'static {
    /// Sleep for the given duration.
    fn wait(&self, duration: Duration) -> Pin<Box<dyn FusedFuture<Output = ()> + Send + 'static>>;

    /// Spawn a future on the runtime's executor.
    fn spawn(
        &self,
        name: &'static str,
        f: impl Future<Output = ()> + Send + 'static,
    ) -> Box<dyn SpawnHandle>;

    /// Spawn a future on a reserved OS thread. This is only really necessary
    /// for libraries like `V8` that care about being called from a
    /// particular thread.
    #[must_use = "Threads are canceled when their `SpawnHandle` is dropped."]
    fn spawn_thread<Fut: Future<Output = ()>, F: FnOnce() -> Fut + Send + 'static>(
        &self,
        f: F,
    ) -> Box<dyn SpawnHandle>;

    /// Return (a potentially-virtualized) system time. Compare with
    /// `std::time::UNIX_EPOCH` to obtain a Unix timestamp.
    fn system_time(&self) -> SystemTime;

    fn unix_timestamp(&self) -> UnixTimestamp {
        UnixTimestamp(
            self.system_time()
                .duration_since(UNIX_EPOCH)
                .expect("Failed to compute unix timestamp"),
        )
    }

    /// Return (a potentially-virtualized) reading from a monotonic clock.
    fn monotonic_now(&self) -> tokio::time::Instant;

    /// Use the runtime's source of randomness.
    fn rng(&self) -> Box<dyn RngCore>;

    fn new_uuid_v4(&self) -> Uuid {
        let mut rng = self.rng();
        let mut bytes = [0u8; 16];
        rng.fill_bytes(&mut bytes);
        uuid::Builder::from_random_bytes(bytes).into_uuid()
    }

    fn generate_timestamp(&self) -> anyhow::Result<Timestamp> {
        Timestamp::try_from(self.system_time())
    }

    fn pause_client(&self) -> PauseClient;
}

/// Abstraction over a unix timestamp. Internally it stores a Duration since the
/// unix epoch.
///
/// NOTE: Only works for timestamps past the UNIX_EPOCH. Not suitable for user
/// defined input from javascript (where f64 can support timestamps prior to the
/// epoch).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct UnixTimestamp(Duration);

impl UnixTimestamp {
    pub fn from_secs_f64(secs: f64) -> Self {
        UnixTimestamp(Duration::from_secs_f64(secs))
    }

    pub fn from_nanos(nanos: u64) -> Self {
        UnixTimestamp(Duration::from_nanos(nanos))
    }

    pub fn from_millis(ms: u64) -> Self {
        UnixTimestamp(Duration::from_millis(ms))
    }

    pub fn as_nanos(&self) -> u128 {
        self.0.as_nanos()
    }

    pub fn as_secs_f64(&self) -> f64 {
        self.0.as_secs_f64()
    }

    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    pub fn as_system_time(&self) -> SystemTime {
        UNIX_EPOCH + self.0
    }

    pub fn checked_sub(&self, rhs: UnixTimestamp) -> Option<Duration> {
        self.0.checked_sub(rhs.0)
    }

    pub fn as_ms_since_epoch(&self) -> Result<u64, anyhow::Error> {
        self.0
            .as_millis()
            .try_into()
            .map_err(|e: TryFromIntError| anyhow::anyhow!(e))
    }
}

impl HeapSize for UnixTimestamp {
    fn heap_size(&self) -> usize {
        0
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for UnixTimestamp {
    type Parameters = ();

    type Strategy = impl Strategy<Value = UnixTimestamp>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (0..=i64::MAX as u64, 0..i32::MAX as u32)
            .prop_map(|(secs, nanos)| Self(Duration::new(secs, nanos)))
    }
}

impl Sub<UnixTimestamp> for UnixTimestamp {
    type Output = Duration;

    fn sub(self, rhs: UnixTimestamp) -> Duration {
        self.0 - rhs.0
    }
}

impl Add<Duration> for UnixTimestamp {
    type Output = UnixTimestamp;

    fn add(self, rhs: Duration) -> UnixTimestamp {
        UnixTimestamp(self.0 + rhs)
    }
}

impl Sub<Duration> for UnixTimestamp {
    type Output = UnixTimestamp;

    fn sub(self, rhs: Duration) -> UnixTimestamp {
        UnixTimestamp(self.0 - rhs)
    }
}

impl From<UnixTimestamp> for prost_types::Timestamp {
    fn from(ts: UnixTimestamp) -> Self {
        Self {
            seconds: ts.as_secs() as i64,
            nanos: ts.0.subsec_nanos() as i32,
        }
    }
}

impl TryFrom<prost_types::Timestamp> for UnixTimestamp {
    type Error = anyhow::Error;

    fn try_from(ts: prost_types::Timestamp) -> anyhow::Result<Self> {
        let system_time = SystemTime::try_from(ts)?;
        Ok(Self(
            system_time
                .duration_since(UNIX_EPOCH)
                .context("Failed to compute duration from epoch")?,
        ))
    }
}

#[derive(Clone)]
pub struct RuntimeClock<RT: Runtime> {
    runtime: RT,
}

pub type RateLimiter<RT> = governor::RateLimiter<
    NotKeyed,
    InMemoryState,
    RuntimeClock<RT>,
    NoOpMiddleware<<RuntimeClock<RT> as governor::clock::Clock>::Instant>,
>;

pub type KeyedRateLimiter<K, RT> = governor::RateLimiter<
    K,
    DefaultKeyedStateStore<K>,
    RuntimeClock<RT>,
    NoOpMiddleware<<RuntimeClock<RT> as governor::clock::Clock>::Instant>,
>;

pub fn new_rate_limiter<RT: Runtime>(runtime: RT, quota: Quota) -> RateLimiter<RT> {
    RateLimiter::direct_with_clock(quota, &RuntimeClock { runtime })
}

pub fn new_keyed_rate_limiter<RT: Runtime, K: Hash + Eq + Clone>(
    runtime: RT,
    quota: Quota,
) -> KeyedRateLimiter<K, RT> {
    KeyedRateLimiter::dashmap_with_clock(quota, &RuntimeClock { runtime })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct GovernorInstant(tokio::time::Instant);

impl From<tokio::time::Instant> for GovernorInstant {
    fn from(instant: tokio::time::Instant) -> Self {
        Self(instant)
    }
}

impl<RT: Runtime> governor::clock::Clock for RuntimeClock<RT> {
    type Instant = GovernorInstant;

    fn now(&self) -> Self::Instant {
        GovernorInstant(self.runtime.monotonic_now())
    }
}

impl governor::clock::Reference for GovernorInstant {
    fn duration_since(&self, earlier: Self) -> Nanos {
        if earlier.0 < self.0 {
            (self.0 - earlier.0).into()
        } else {
            Nanos::from(Duration::ZERO)
        }
    }

    fn saturating_sub(&self, duration: Nanos) -> Self {
        self.0
            .checked_sub(duration.into())
            .map(GovernorInstant)
            .unwrap_or(*self)
    }
}

impl Add<Nanos> for GovernorInstant {
    type Output = GovernorInstant;

    fn add(self, rhs: Nanos) -> Self::Output {
        GovernorInstant(self.0 + rhs.into())
    }
}

impl<RT: Runtime> governor::clock::ReasonablyRealtime for RuntimeClock<RT> {}

#[async_trait]
pub trait WithTimeout {
    async fn with_timeout<T>(
        &self,
        description: &'static str,
        duration: Duration,
        fut: impl Future<Output = anyhow::Result<T>> + Send,
    ) -> anyhow::Result<T>;
}

#[async_trait]
impl<RT: Runtime> WithTimeout for RT {
    async fn with_timeout<T>(
        &self,
        description: &'static str,
        duration: Duration,
        fut: impl Future<Output = anyhow::Result<T>> + Send,
    ) -> anyhow::Result<T> {
        select_biased! {
            result = fut.fuse() => result,
            _q = self.wait(duration) => {
                anyhow::bail!(TimeoutError{description, duration});
            },
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("'{description}' timeout after {duration:?}")]
pub struct TimeoutError {
    description: &'static str,
    duration: Duration,
}

pub struct MutexWithTimeout<T: Send> {
    timeout: Duration,
    mutex: tokio::sync::Mutex<T>,
}

impl<T: Send> MutexWithTimeout<T> {
    pub fn new(timeout: Duration, value: T) -> Self {
        Self {
            timeout,
            mutex: tokio::sync::Mutex::new(value),
        }
    }

    pub async fn acquire_lock_with_timeout(&self) -> anyhow::Result<tokio::sync::MutexGuard<T>> {
        let acquire_lock = async { Ok(self.mutex.lock().await) };
        select_biased! {
            result = acquire_lock.fuse() => result,
            _ = tokio::time::sleep(self.timeout).fuse() => {
                anyhow::bail!(TimeoutError{description: "acquire_lock", duration: self.timeout});
            },
        }
    }
}

/// Transitional function while we move away from using our own special
/// `spawn`. Just wraps `tokio::spawn` with our tokio metrics
/// integration.
pub fn tokio_spawn<F>(name: &'static str, f: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let monitor = GLOBAL_TASK_MANAGER.lock().get(name);
    tokio::spawn(monitor.instrument(f))
}

pub static GLOBAL_TASK_MANAGER: LazyLock<Mutex<TaskManager>> = LazyLock::new(|| {
    let task_collector = tokio_metrics_collector::default_task_collector();
    CONVEX_METRICS_REGISTRY
        .register(Box::new(task_collector))
        .unwrap();

    let manager = TaskManager {
        monitors: HashMap::new(),
    };
    Mutex::new(manager)
});

pub struct TaskManager {
    monitors: HashMap<&'static str, TaskMonitor>,
}

impl TaskManager {
    pub fn get(&mut self, name: &'static str) -> TaskMonitor {
        if let Some(monitor) = self.monitors.get(name) {
            return monitor.clone();
        }
        let monitor = TaskMonitor::new();
        self.monitors.insert(name, monitor.clone());
        tokio_metrics_collector::default_task_collector()
            .add(name, monitor.clone())
            .expect("Duplicate task label?");
        monitor
    }

    pub fn instrument<F: Future>(name: &'static str, f: F) -> Instrumented<F> {
        let monitor = {
            let mut manager = GLOBAL_TASK_MANAGER.lock();
            manager.get(name)
        };
        monitor.instrument(f)
    }
}

// Helper function to only call into `tokio::task::block_in_place` if we're not
// using the single threaded runtime.
pub fn block_in_place<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let handle = Handle::current();
    if handle.runtime_flavor() == RuntimeFlavor::CurrentThread {
        f()
    } else {
        tokio::task::block_in_place(f)
    }
}
