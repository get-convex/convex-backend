use core::hash::Hash;
use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    fmt::Debug,
    sync::Arc,
};

use ::metrics::StatusTimer;
use async_broadcast::Receiver as BroadcastReceiver;
use common::{
    codel_queue::{
        CoDelQueue,
        CoDelQueueReceiver,
        CoDelQueueSender,
    },
    components::{
        ComponentId,
        ComponentPath,
    },
    errors::recapture_stacktrace_noreport,
    runtime::{
        Runtime,
        SpawnHandle,
    },
    types::IndexId,
};
use fastrace::{
    collector::SpanContext,
    future::FutureExt as _,
    Span,
};
use futures::{
    future::BoxFuture,
    FutureExt,
    StreamExt,
};
use lru::LruCache;
use parking_lot::Mutex;
use value::{
    TableMapping,
    TabletId,
};

use crate::metrics::{
    async_lru_compute_timer,
    async_lru_get_timer,
    async_lru_log_eviction,
    log_async_lru_cache_hit,
    log_async_lru_cache_miss,
    log_async_lru_cache_waiting,
    log_async_lru_size,
};

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
pub struct AsyncLru<RT: Runtime, Key, Value: ?Sized> {
    runtime: RT,
    inner: Arc<Mutex<Inner<RT, Key, Value>>>,
    label: &'static str,
    handle: Arc<Box<dyn SpawnHandle>>,
}

pub type SingleValueGenerator<Value> = BoxFuture<'static, anyhow::Result<Value>>;
pub type ValueGenerator<Key, Value> = BoxFuture<'static, HashMap<Key, anyhow::Result<Arc<Value>>>>;

impl<RT: Runtime, Key, Value: ?Sized> Clone for AsyncLru<RT, Key, Value> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            inner: self.inner.clone(),
            label: self.label,
            handle: self.handle.clone(),
        }
    }
}
enum CacheResult<Value: ?Sized> {
    Ready {
        value: Arc<Value>,
        // Memoize the size to guard against implementations of `SizedValue`
        // that (unexpectedly) change while the value is in the cache.
        size: u64,
        added: tokio::time::Instant,
    },
    Waiting {
        receiver: BroadcastReceiver<Result<Arc<Value>, Arc<anyhow::Error>>>,
    },
}

impl<Value: SizedValue + ?Sized> SizedValue for CacheResult<Value> {
    fn size(&self) -> u64 {
        match self {
            CacheResult::Ready { size, .. } => *size,
            CacheResult::Waiting { .. } => 0,
        }
    }
}

struct Inner<RT: Runtime, Key, Value: ?Sized> {
    cache: LruCache<Key, CacheResult<Value>>,
    current_size: u64,
    max_size: u64,
    label: &'static str,
    tx: CoDelQueueSender<RT, BuildValueRequest<Key, Value>>,
}

impl<RT: Runtime, Key, Value: ?Sized> Inner<RT, Key, Value> {
    fn new(
        cache: LruCache<Key, CacheResult<Value>>,
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

/// Wrapper struct when you're inserting values into the LRU that should always
/// be considered to be 1 unit in size.
pub struct UnitSizedValue<T>(pub T);

impl<T> SizedValue for UnitSizedValue<T> {
    fn size(&self) -> u64 {
        1
    }
}

impl<Value: SizedValue> SizedValue for Arc<Value> {
    fn size(&self) -> u64 {
        Value::size(self)
    }
}

// TableMapping and BTreeMap<TabletId, IndexId> don't vary much in size within
// a deployment, so it's easier to think about the caches as having a number of
// items, instead of considering the number of bytes cached.
impl SizedValue for TableMapping {
    fn size(&self) -> u64 {
        1
    }
}
impl SizedValue for BTreeMap<TabletId, IndexId> {
    fn size(&self) -> u64 {
        1
    }
}
impl SizedValue for BTreeMap<ComponentId, ComponentPath> {
    fn size(&self) -> u64 {
        1
    }
}

type BuildValueResult<Value> = Result<Arc<Value>, Arc<anyhow::Error>>;

type BuildValueRequest<Key, Value> = (
    Key,
    ValueGenerator<Key, Value>,
    async_broadcast::Sender<BuildValueResult<Value>>,
);

enum Status<Value: ?Sized> {
    Ready(Arc<Value>),
    Waiting(async_broadcast::Receiver<BuildValueResult<Value>>),
    Kickoff(
        async_broadcast::Receiver<BuildValueResult<Value>>,
        StatusTimer,
    ),
}

impl<Value: ?Sized> std::fmt::Display for Status<Value> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Ready(_) => write!(f, "Ready"),
            Status::Waiting(_) => write!(f, "Waiting"),
            Status::Kickoff(..) => write!(f, "Kickoff"),
        }
    }
}

impl<
        RT: Runtime,
        Key: Hash + Eq + Debug + Clone + Send + Sync + 'static,
        Value: Send + Sync + 'static + SizedValue + ?Sized,
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
            CoDelQueue::new(rt.clone(), 200),
            rt,
            LruCache::unbounded(),
            max_size,
            concurrency,
            label,
        )
    }

    fn _new(
        queue: CoDelQueue<RT, BuildValueRequest<Key, Value>>,
        rt: RT,
        cache: LruCache<Key, CacheResult<Value>>,
        max_size: u64,
        concurrency: usize,
        label: &'static str,
    ) -> Self {
        let (tx, rx) = queue.into_sender_and_receiver();
        let inner = Inner::new(cache, max_size, label, tx);
        let handle = rt.spawn(
            label,
            Self::value_generating_worker_thread(rt.clone(), rx, inner.clone(), concurrency),
        );
        Self {
            runtime: rt.clone(),
            inner,
            label,
            handle: Arc::new(handle),
        }
    }

    fn drop_waiting(inner: Arc<Mutex<Inner<RT, Key, Value>>>, key: &Key) {
        let mut inner = inner.lock();
        // Only remove if still Waiting
        if matches!(inner.cache.peek(key), Some(CacheResult::Waiting { .. })) {
            inner.cache.pop(key);
        }
    }

    fn update_value(
        rt: RT,
        inner: Arc<Mutex<Inner<RT, Key, Value>>>,
        key: Key,
        value: anyhow::Result<Arc<Value>>,
    ) -> anyhow::Result<Arc<Value>> {
        let mut inner = inner.lock();
        match value {
            Ok(result) => {
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

    pub async fn get<V: 'static>(
        &self,
        key: Key,
        value_generator: SingleValueGenerator<V>,
    ) -> anyhow::Result<Arc<Value>>
    where
        Key: Clone,
        Arc<Value>: From<V>,
    {
        let timer = async_lru_get_timer(self.label);
        let key_ = key.clone();
        let result = self
            ._get(
                &key_,
                Box::pin(async move {
                    let mut hashmap = HashMap::new();
                    hashmap.insert(key, value_generator.await.map(<Arc<Value>>::from));
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
        let pause_client = self.runtime.pause_client();
        let status = self.get_sync(key, value_generator)?;
        tracing::debug!("Getting key {key:?} with status {status}");
        match status {
            Status::Ready(value) => Ok(value),
            Status::Waiting(rx) => Ok(Self::wait_for_value(key, rx).await?),
            Status::Kickoff(rx, timer) => {
                pause_client.wait(PAUSE_DURING_GENERATE_VALUE_LABEL).await;
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

                // Run the value_generator in the span context of the original client that
                // fired off the job. If multiple callers instantiate the same job, only the
                // first one will execute the future and get the sub-spans.
                let span = SpanContext::current_local_parent()
                    .map(|ctx| Span::root("async_lru_compute_value", ctx))
                    .unwrap_or(Span::noop());

                let timer = async_lru_compute_timer(self.label);
                let (tx, rx) = async_broadcast::broadcast(1);
                // If the queue is too full, just bail here. The cache state is unmodified and
                // there can't be any other waiters for this key right now, so
                // it should be safe to abort.
                inner.tx.clone().try_send((
                    key.clone(),
                    value_generator.in_span(span).boxed(),
                    tx,
                ))?;
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

    #[fastrace::trace]
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
