//! Runtime trait for abstracting away OS-esque features and allow different
//! implementations for test, dev, prod, etc.

use std::{
    future::Future,
    marker::Send,
    num::TryFromIntError,
    ops::{
        Add,
        Sub,
    },
    pin::Pin,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use futures::{
    future::FusedFuture,
    select_biased,
    FutureExt,
};
pub use governor::nanos::Nanos;
use governor::{
    middleware::NoOpMiddleware,
    state::{
        InMemoryState,
        NotKeyed,
    },
    Quota,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use rand::{
    Rng,
    RngCore,
};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;
use value::heap_size::HeapSize;

use crate::{
    is_canceled::IsCanceled,
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
    type Future: Future<Output = Result<(), JoinError>>;
    fn shutdown(&mut self);
    fn into_join_future(self) -> Self::Future;
}

/// Shutdown the associated future, preempting it at its next yield point, and
/// join on its result.
pub async fn shutdown_and_join(mut handle: impl SpawnHandle) -> anyhow::Result<()> {
    handle.shutdown();
    if let Err(e) = handle.into_join_future().await {
        if !matches!(e, JoinError::Canceled) {
            return Err(e.into());
        }
    }
    Ok(())
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
    /// Spawn handle type returned by `spawn`.
    type Handle: SpawnHandle;

    /// Spawn handle type returned by `spawn_thread` (which may be a different
    /// type than `spawn`'s).
    type ThreadHandle: SpawnHandle;

    /// `std::time::Instant`-like type returned by `monotonic_now()`.
    type Instant: RuntimeInstant;

    /// Source of randomness associated with the runtime.
    type Rng: Rng;

    /// Sleep for the given duration.
    fn wait(&self, duration: Duration) -> Pin<Box<dyn FusedFuture<Output = ()> + Send + 'static>>;

    /// Spawn a future on the runtime's executor.
    fn spawn(
        &self,
        name: &'static str,
        f: impl Future<Output = ()> + Send + 'static,
    ) -> Self::Handle;

    /// Spawn a future on a reserved OS thread. This is only really necessary
    /// for libraries like `V8` that care about being called from a
    /// particular thread.
    fn spawn_thread<Fut: Future<Output = ()>, F: FnOnce() -> Fut + Send + 'static>(
        &self,
        f: F,
    ) -> Self::ThreadHandle;

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
    fn monotonic_now(&self) -> Self::Instant;

    /// Use the runtime's source of randomness.
    fn with_rng<R>(&self, f: impl FnOnce(&mut Self::Rng) -> R) -> R;

    fn new_uuid_v4(&self) -> Uuid {
        let bytes = self.with_rng(|rng| {
            let mut bytes = [0u8; 16];
            rng.fill_bytes(&mut bytes);
            bytes
        });
        uuid::Builder::from_random_bytes(bytes).into_uuid()
    }

    fn generate_timestamp(&self) -> anyhow::Result<Timestamp> {
        Timestamp::try_from(self.system_time())
    }
}

/// Abstraction over different `Instant` types associated with a `Runtime`. This
/// is necessary for test runtime instants, which don't use the globally
/// available system clock and need to retain a reference back to their
/// originating runtime.
pub trait RuntimeInstant:
    Add<Duration, Output = Self>
    + Clone
    + Sub<Output = Duration>
    + Sync
    + Send
    + Ord
    + PartialOrd
    + Eq
    + PartialEq
    + HeapSize
{
    fn elapsed(&self) -> Duration;

    /// Convert an instant to nanoseconds relative to some (unspecified) epoch.
    fn as_nanos(&self) -> Nanos;
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

pub fn new_rate_limiter<RT: Runtime>(runtime: RT, quota: Quota) -> RateLimiter<RT> {
    RateLimiter::direct_with_clock(quota, &RuntimeClock { runtime })
}

impl<RT: Runtime> governor::clock::Clock for RuntimeClock<RT> {
    type Instant = Nanos;

    fn now(&self) -> Self::Instant {
        self.runtime.monotonic_now().as_nanos()
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
