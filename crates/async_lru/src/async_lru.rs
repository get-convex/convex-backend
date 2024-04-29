use core::hash::Hash;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

use ::metrics::StatusTimer;
use async_broadcast::Receiver as BroadcastReceiver;
#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    codel_queue::{
        new_codel_queue_async,
        CoDelQueueReceiver,
        CoDelQueueSender,
    },
    errors::recapture_stacktrace_noreport,
    runtime::{
        Runtime,
        RuntimeInstant,
    },
};
use futures::{
    future::BoxFuture,
    StreamExt,
};
use lru::LruCache;
use parking_lot::Mutex;

use crate::metrics::{
    async_lru_compute_timer,
    async_lru_get_timer,
    async_lru_log_eviction,
    log_async_lru_cache_hit,
    log_async_lru_cache_miss,
    log_async_lru_cache_waiting,
    log_async_lru_size,
};

#[cfg(any(test, feature = "testing"))]
const PAUSE_DURING_GENERATE_VALUE_LABEL: &str = "generate_value";

/// A write through cache with support for cancelation.
///
/// Use this class over LruCache when you're in an asynchronous context, you
/// may have multiple concurrent requests for the same key and value generation
/// is relatively expensive. Unlike LruCache, this struct will and ensure that
/// any expensive values are calculated exactly once while also notifying all
/// requestors when that single value calculation finishes.
///
/// Cancelation is handled by using internal worker threads (determined by
/// the value passed to `concurrency` in `AsyncLru::new`) to calculate values.
/// Callers wait asynchronously for the calculation to complete, then are
/// notified via channels. This allows any individual caller to be canceled
/// without risking accidentally canceling the value calculation and triggering
/// errors for other requests to the same key that happen to be waiting. The
/// cost is that we have to spawn more value calculating threads and that the
/// desired concurrency of the cache may not match that of the caller.
pub struct AsyncLru<RT: Runtime, Key, Value> {
    inner: Arc<Mutex<Inner<RT, Key, Value>>>,
    label: &'static str,
    handle: Arc<<RT as Runtime>::Handle>,
    // This tokio Mutex is safe only because it's stripped out of production
    // builds. We shouldn't use tokio locks for prod code (see
    // https://github.com/rust-lang/rust/issues/104883 for background and
    // https://github.com/get-convex/convex/pull/19307 for an alternative).
    #[cfg(any(test, feature = "testing"))]
    pause_client: Option<Arc<tokio::sync::Mutex<PauseClient>>>,
}

pub type SingleValueGenerator<Value> = BoxFuture<'static, anyhow::Result<Value>>;
pub type ValueGenerator<Key, Value> = BoxFuture<'static, HashMap<Key, anyhow::Result<Value>>>;

impl<RT: Runtime, Key, Value> Clone for AsyncLru<RT, Key, Value> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            label: self.label,
            #[cfg(any(test, feature = "testing"))]
            pause_client: self.pause_client.clone(),
            handle: self.handle.clone(),
        }
    }
}
enum CacheResult<Value, RT: Runtime> {
    Ready {
        value: Arc<Value>,
        // Memoize the size to guard against implementations of `SizedValue`
        // that (unexpectedly) change while the value is in the cache.
        size: u64,
        added: RT::Instant,
    },
    Waiting {
        receiver: BroadcastReceiver<Result<Arc<Value>, Arc<anyhow::Error>>>,
    },
}

impl<Value: SizedValue, RT: Runtime> SizedValue for CacheResult<Value, RT> {
    fn size(&self) -> u64 {
        match self {
            CacheResult::Ready { size, .. } => *size,
            CacheResult::Waiting { .. } => 0,
        }
    }
}

struct Inner<RT: Runtime, Key, Value> {
    cache: LruCache<Key, CacheResult<Value, RT>>,
    current_size: u64,
    max_size: u64,
    label: &'static str,
    tx: CoDelQueueSender<RT, BuildValueRequest<Key, Value>>,
}

impl<RT: Runtime, Key, Value> Inner<RT, Key, Value> {
    fn new(
        cache: LruCache<Key, CacheResult<Value, RT>>,
        max_size: u64,
        label: &'static str,
        tx: CoDelQueueSender<RT, BuildValueRequest<Key, Value>>,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            cache,
            current_size: 0,
            max_size,
            label,
            tx,
        }))
    }
}

pub trait SizedValue {
    fn size(&self) -> u64;
}

impl<Value: SizedValue> SizedValue for Arc<Value> {
    fn size(&self) -> u64 {
        Value::size(self)
    }
}

type BuildValueResult<Value> = Result<Arc<Value>, Arc<anyhow::Error>>;

type BuildValueRequest<Key, Value> = (
    Key,
    ValueGenerator<Key, Value>,
    async_broadcast::Sender<BuildValueResult<Value>>,
);

enum Status<Value> {
    Ready(Arc<Value>),
    Waiting(async_broadcast::Receiver<BuildValueResult<Value>>),
    Kickoff(
        async_broadcast::Receiver<BuildValueResult<Value>>,
        StatusTimer,
    ),
}

impl<
        RT: Runtime,
        Key: Hash + Eq + Debug + Clone + Send + Sync + 'static,
        Value: Send + Sync + 'static + SizedValue,
    > AsyncLru<RT, Key, Value>
{
    /// Create a new fixed size LRU where the maximum size is determined by
    /// `max_size` and the size of each entry is determined by the
    /// implementation of `SizedValue` for the corresponding value.
    ///
    /// label - a string for logging to differentiate between LRUs.
    /// max_size - the maximum number of Values that will be kept in memory by
    /// the LRU. Must be > 0, or we will panic.
    /// generate_value - a function that generates a new value for a given Key
    /// if no value is present.
    /// concurrency - The number of values that can be concurrently generated.
    /// This should be set based on system values.
    pub fn new(rt: RT, max_size: u64, concurrency: usize, label: &'static str) -> Self {
        Self::_new(
            rt,
            LruCache::unbounded(),
            max_size,
            concurrency,
            label,
            #[cfg(any(test, feature = "testing"))]
            None,
        )
    }

    #[cfg(any(test, feature = "testing"))]
    #[allow(unused)]
    fn new_for_tests(
        rt: RT,
        max_size: u64,
        label: &'static str,
        pause_client: Option<PauseClient>,
    ) -> Self {
        let lru = LruCache::unbounded();
        Self::_new(rt, lru, max_size, 1, label, pause_client)
    }

    fn _new(
        rt: RT,
        cache: LruCache<Key, CacheResult<Value, RT>>,
        max_size: u64,
        concurrency: usize,
        label: &'static str,
        #[cfg(any(test, feature = "testing"))] pause_client: Option<PauseClient>,
    ) -> Self {
        let (tx, rx) = new_codel_queue_async(rt.clone(), 200);
        let inner = Inner::new(cache, max_size, label, tx);
        let handle = rt.spawn(
            label,
            Self::value_generating_worker_thread(rt.clone(), rx, inner.clone(), concurrency),
        );
        Self {
            inner,
            label,
            handle: Arc::new(handle),
            #[cfg(any(test, feature = "testing"))]
            pause_client: pause_client
                .map(|pause_client| Arc::new(tokio::sync::Mutex::new(pause_client))),
        }
    }

    fn drop_waiting(inner: Arc<Mutex<Inner<RT, Key, Value>>>, key: &Key) {
        let mut inner = inner.lock();
        if let Some(value) = inner.cache.pop(key)
            && matches!(value, CacheResult::Ready { .. })
        {
            panic!("Dropped a ready result without changing the cache's size!");
        }
    }

    fn update_value(
        rt: RT,
        inner: Arc<Mutex<Inner<RT, Key, Value>>>,
        key: Key,
        value: anyhow::Result<Value>,
    ) -> anyhow::Result<Arc<Value>> {
        let mut inner = inner.lock();
        match value {
            Ok(value) => {
                let result = Arc::new(value);
                let new_value = CacheResult::Ready {
                    size: result.size(),
                    value: result.clone(),
                    added: rt.monotonic_now(),
                };
                inner.current_size += new_value.size();
                // Ideally we'd not change the LRU order by putting here...
                if let Some(old_value) = inner.cache.put(key, new_value) {
                    // Allow overwriting entries (Waiting or Ready) which may have been populated
                    // by racing requests with prefetches.
                    inner.current_size -= old_value.size();
                }
                Self::trim_to_size(&mut inner);

                Ok(result)
            },
            Err(e) => {
                inner.cache.pop(&key);
                Err(e)
            },
        }
    }

    // This may evict 'waiting' entries under high load. That will
    // cause a channel error for callers who could choose to retry.
    // If this becomes an issue, we can iterate over the entries,
    // collect a set of keys to evict and manually pop each key
    // from the LRU.
    fn trim_to_size(inner: &mut Inner<RT, Key, Value>) {
        while inner.current_size > inner.max_size {
            let (_, evicted) = inner
                .cache
                .pop_lru()
                .expect("Over max size, but no more entries");
            // This isn't catastrophic necessarily, but it may lead to
            // under / over counting of the cache's size.
            if let CacheResult::Ready {
                ref value,
                size,
                ref added,
            } = evicted
            {
                if size != value.size() {
                    tracing::warn!(
                        "Value changed size from {} to {} while in the {} cache!",
                        size,
                        value.size(),
                        inner.label
                    )
                }
                async_lru_log_eviction(inner.label, added.elapsed());
            }
            inner.current_size -= evicted.size();
        }
    }

    pub fn size(&self) -> u64 {
        let inner = self.inner.lock();
        inner.current_size
    }

    pub async fn get_and_prepopulate(
        &self,
        key: Key,
        value_generator: ValueGenerator<Key, Value>,
    ) -> anyhow::Result<Arc<Value>> {
        let timer = async_lru_get_timer(self.label);
        let result = self._get(&key, value_generator).await;
        timer.finish(result.is_ok());
        result
    }

    pub async fn get(
        &self,
        key: Key,
        value_generator: SingleValueGenerator<Value>,
    ) -> anyhow::Result<Arc<Value>>
    where
        Key: Clone,
    {
        let timer = async_lru_get_timer(self.label);
        let key_ = key.clone();
        let result = self
            ._get(
                &key_,
                Box::pin(async move {
                    let mut hashmap = HashMap::new();
                    hashmap.insert(key, value_generator.await);
                    hashmap
                }),
            )
            .await;
        timer.finish(result.is_ok());
        result
    }

    async fn _get(
        &self,
        key: &Key,
        value_generator: ValueGenerator<Key, Value>,
    ) -> anyhow::Result<Arc<Value>> {
        match self.get_sync(key, value_generator)? {
            Status::Ready(value) => Ok(value),
            Status::Waiting(rx) => Ok(Self::wait_for_value(key, rx).await?),
            Status::Kickoff(rx, timer) => {
                #[cfg(any(test, feature = "testing"))]
                if let Some(pause_client) = &mut self.pause_client.clone() {
                    let mut pause_client = pause_client.lock().await;
                    pause_client.wait(PAUSE_DURING_GENERATE_VALUE_LABEL).await;
                    drop(pause_client);
                }
                let result = Self::wait_for_value(key, rx).await?;
                timer.finish();
                Ok(result)
            },
        }
    }

    fn get_sync(
        &self,
        key: &Key,
        value_generator: ValueGenerator<Key, Value>,
    ) -> anyhow::Result<Status<Value>> {
        let mut inner = self.inner.lock();
        log_async_lru_size(inner.cache.len(), inner.current_size, self.label);
        match inner.cache.get(key) {
            Some(CacheResult::Ready { value, .. }) => {
                log_async_lru_cache_hit(self.label);
                Ok(Status::Ready(value.clone()))
            },
            Some(CacheResult::Waiting { receiver }) => {
                log_async_lru_cache_waiting(self.label);
                let receiver = receiver.clone();
                Ok(Status::Waiting(receiver))
            },
            None => {
                log_async_lru_cache_miss(self.label);
                let timer = async_lru_compute_timer(self.label);
                let (tx, rx) = async_broadcast::broadcast(1);
                // If the queue is too full, just bail here. The cache state is unmodified and
                // there can't be any other waiters for this key right now, so
                // it should be safe to abort.
                inner
                    .tx
                    .clone()
                    .try_send((key.clone(), value_generator, tx))?;
                inner.cache.put(
                    key.clone(),
                    CacheResult::Waiting {
                        receiver: rx.clone(),
                    },
                );
                Ok(Status::Kickoff(rx, timer))
            },
        }
    }

    async fn wait_for_value(
        key: &Key,
        mut receiver: async_broadcast::Receiver<BuildValueResult<Value>>,
    ) -> anyhow::Result<Arc<Value>> {
        // No work should be canceled while anyone is waiting on it, so it's a
        // developer error if recv ever returns a failure due to the channel
        // being closed.
        let recv_result = receiver.recv().await?;
        match recv_result {
            Ok(value) => {
                tracing::debug!("Finished waiting on another task to fetch key {key:?}");
                Ok(value)
            },
            // We recapture the error in the string so that we don't lose the stacktrace since the
            // original stacktrace is stuck inside an Arc<anyhow::Error>
            Err(e) => Err(recapture_stacktrace_noreport(&e)),
        }
    }

    async fn value_generating_worker_thread(
        rt: RT,
        rx: CoDelQueueReceiver<RT, BuildValueRequest<Key, Value>>,
        inner: Arc<Mutex<Inner<RT, Key, Value>>>,
        concurrency: usize,
    ) {
        rx.for_each_concurrent(concurrency, |((key, generator, tx), expired)| {
            let inner = inner.clone();
            let rt = rt.clone();
            async move {
                if let Some(expired) = expired {
                    Self::drop_waiting(inner, &key);
                    let _ = tx.broadcast(Err(Arc::new(anyhow::anyhow!(expired)))).await;
                    return;
                }

                let values = generator.await;

                for (k, value) in values {
                    let is_requested_key = k == key;
                    let to_broadcast =
                        Self::update_value(rt.clone(), inner.clone(), k, value).map_err(Arc::new);
                    if is_requested_key {
                        let _ = tx.broadcast(to_broadcast).await;
                    }
                }
            }
        })
        .await;
        tracing::warn!("Worker shut down, shutting down scheduler");
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::Arc,
    };

    use common::{
        pause::PauseController,
        runtime,
    };
    use futures::{
        future::join_all,
        select,
        FutureExt,
    };
    use parking_lot::Mutex;
    use runtime::testing::TestRuntime;

    use super::SizedValue;
    use crate::async_lru::{
        AsyncLru,
        PAUSE_DURING_GENERATE_VALUE_LABEL,
    };

    struct SneakyMutableValue {
        size: Mutex<u64>,
    }

    impl SizedValue for SneakyMutableValue {
        fn size(&self) -> u64 {
            *self.size.lock()
        }
    }

    #[derive(Clone)]
    struct GenerateSneakyMutableValue;
    impl GenerateSneakyMutableValue {
        async fn generate_value(_key: &'static str) -> anyhow::Result<SneakyMutableValue> {
            Ok(SneakyMutableValue {
                size: Mutex::new(1),
            })
        }
    }

    #[derive(Clone)]
    struct GenerateRandomValue;
    impl GenerateRandomValue {
        async fn generate_value(_key: &'static str) -> anyhow::Result<u32> {
            Ok(rand::random::<u32>())
        }
    }

    struct SizeTwoValue;

    impl SizedValue for SizeTwoValue {
        fn size(&self) -> u64 {
            2
        }
    }

    #[derive(Clone)]
    struct GenerateSizeTwoValue;

    impl GenerateSizeTwoValue {
        async fn generate_value(_key: &'static str) -> anyhow::Result<SizeTwoValue> {
            Ok(SizeTwoValue)
        }
    }

    #[derive(Clone)]
    struct FailAlways;

    impl FailAlways {
        async fn generate_value(_key: &'static str) -> anyhow::Result<u32> {
            let mut err = anyhow::anyhow!("original error");
            err = err.context("NO!");
            Err(err)
        }
    }

    impl SizedValue for u32 {
        fn size(&self) -> u64 {
            1
        }
    }

    #[convex_macro::test_runtime]
    async fn get_only_generates_once_per_key(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new(rt, 1, 1, "label");
        let first = cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        let second = cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        assert_eq!(first, second);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn can_hold_multiple_values(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new(rt, 3, 1, "label");
        let keys = ["key1", "key2", "key3"];

        let get_all_values = || async {
            join_all(
                keys.into_iter()
                    .map(|key| cache.get(key, GenerateRandomValue::generate_value(key).boxed())),
            )
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()
        };

        let initial_values = get_all_values().await?;
        let second_values = get_all_values().await?;
        assert_eq!(initial_values, second_values);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_get_and_prepopulate(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new(rt, 10, 1, "label");
        let first = cache
            .get_and_prepopulate(
                "k1",
                async move {
                    let mut hashmap = HashMap::new();
                    hashmap.insert("k1", Ok(1));
                    hashmap.insert("k2", Ok(2));
                    hashmap.insert("k3", Err(anyhow::anyhow!("k3 failed")));
                    hashmap
                }
                .boxed(),
            )
            .await?;
        assert_eq!(*first, 1);
        let k1_again = cache
            .get("k1", GenerateRandomValue::generate_value("k1").boxed())
            .await?;
        assert_eq!(*k1_again, 1);
        let k2_prepopulated = cache
            .get("k2", GenerateRandomValue::generate_value("k2").boxed())
            .await?;
        assert_eq!(*k2_prepopulated, 2);
        let k3_prepopulated = cache.get("k3", async move { Ok(3) }.boxed()).await?;
        assert_eq!(*k3_prepopulated, 3);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn get_generates_new_value_after_eviction(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new(rt, 1, 1, "label");
        let first = cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        cache
            .get(
                "other_key",
                GenerateRandomValue::generate_value("other_key").boxed(),
            )
            .await?;
        let second = cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        assert_ne!(first, second);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn get_with_failure_propagates_error(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new(rt, 1, 1, "label");
        let result = cache
            .get("key", FailAlways::generate_value("key").boxed())
            .await;
        assert_is_our_error(result);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn get_when_canceled_during_calculate_returns_value(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let (mut pause, pause_client) = PauseController::new([PAUSE_DURING_GENERATE_VALUE_LABEL]);
        let cache = AsyncLru::new_for_tests(rt, 1, "label", Some(pause_client));
        let mut first = cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .boxed();
        let mut wait_for_blocked = pause
            .wait_for_blocked(PAUSE_DURING_GENERATE_VALUE_LABEL)
            .boxed();
        loop {
            select! {
                _ = first.as_mut().fuse() => {
                    // This first get should get to the calculating stage,
                    // then pause, then be canceled, so it should never finish.
                    anyhow::bail!("get finished first?!");
                }
                pause_guard = wait_for_blocked.as_mut().fuse() => {
                    if let Some(mut pause_guard) = pause_guard {
                        // Cancel the first request in the middle of calculating.
                        drop(first);
                        // Unblock (not technically required).
                        pause_guard.unpause();
                        // Let pause be used again later.
                        drop(wait_for_blocked);
                        break
                    }
                }
            }
        }

        // Make sure that a subsequent get() can succeed.
        cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn size_is_zero_initially(rt: TestRuntime) {
        let cache: AsyncLru<TestRuntime, &str, u32> = AsyncLru::new_for_tests(rt, 1, "label", None);
        assert_eq!(0, cache.size());
    }

    #[convex_macro::test_runtime]
    async fn size_increases_on_put(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new_for_tests(rt, 2, "label", None);
        cache
            .get("key1", GenerateRandomValue::generate_value("key1").boxed())
            .await?;
        assert_eq!(1, cache.size());
        cache
            .get("key2", GenerateRandomValue::generate_value("key2").boxed())
            .await?;
        assert_eq!(2, cache.size());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn size_with_custom_size_increases_on_put(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new_for_tests(rt, 4, "label", None);
        cache
            .get("key1", GenerateSizeTwoValue::generate_value("key1").boxed())
            .await?;
        assert_eq!(2, cache.size());
        cache
            .get("key2", GenerateSizeTwoValue::generate_value("key2").boxed())
            .await?;
        assert_eq!(4, cache.size());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn size_does_not_increase_on_get(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new_for_tests(rt, 1, "label", None);
        cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        cache
            .get("key", GenerateRandomValue::generate_value("key").boxed())
            .await?;
        assert_eq!(1, cache.size());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn size_with_custom_size_does_not_increase_on_get(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new_for_tests(rt, 2, "label", None);
        cache
            .get("key", GenerateSizeTwoValue::generate_value("key").boxed())
            .await?;
        cache
            .get("key", GenerateSizeTwoValue::generate_value("key").boxed())
            .await?;
        assert_eq!(2, cache.size());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn size_when_value_size_changes_is_consistent(rt: TestRuntime) -> anyhow::Result<()> {
        let cache = AsyncLru::new_for_tests(rt, 1, "label", None);
        let value = cache
            .get(
                "key",
                GenerateSneakyMutableValue::generate_value("key").boxed(),
            )
            .await?;
        *value.size.lock() += 10;
        cache
            .get(
                "otherKey",
                GenerateSneakyMutableValue::generate_value("otherKey").boxed(),
            )
            .await?;
        assert_eq!(1, cache.size());
        Ok(())
    }

    fn assert_is_our_error(result: anyhow::Result<Arc<u32>>) {
        let err = result.unwrap_err();
        assert!(
            format!("{:?}", err).contains("NO!"),
            "Expected our test error, but instead got: {:?}",
            err,
        );
        assert!(
            format!("{:?}", err).contains("original error"),
            "Expected our test error, but instead got: {:?}",
            err,
        );
        assert!(
            format!("{:?}", err).contains("Orig Error"),
            "Expected our test error, but instead got: {:?}",
            err,
        );
    }
}
