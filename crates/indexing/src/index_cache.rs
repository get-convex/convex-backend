use std::{
    collections::{
        hash_map::Entry,
        BTreeMap,
        HashMap as StdHashMap,
        HashSet,
    },
    sync::{
        atomic::{
            AtomicU32,
            Ordering,
        },
        Arc,
    },
};

use common::{
    document_index_keys::DatabaseIndexWrite,
    interval::Interval,
    query::{
        CursorPosition,
        Order,
    },
    types::{
        IndexId,
        RepeatableTimestamp,
        SubscriberId,
        TabletIndexName,
        Timestamp,
    },
};
use dashmap::DashMap;
use imbl::{
    OrdSet,
    Vector,
};
use interval_map::IntervalMap;
use metrics::StaticMetricLabel;
use moka::{
    notification::RemovalCause,
    ops::compute::Op,
};
use parking_lot::Mutex;
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

use crate::{
    backend_in_memory_indexes::{
        IndexEntry,
        IndexPage,
    },
    index_registry::IndexRegistry,
    metrics::{
        cache_apply_writes_timer,
        index_cache_get_timer,
        index_cache_populate_timer,
        log_index_cache_invalidation,
        log_index_cache_size,
    },
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct DeploymentId(u32);

pub trait WriteLogIndexReader: Send + Sync {
    /// Iterate over writes to an index after the given timestamp.
    fn iter_writes_after(
        &self,
        index_name: TabletIndexName,
        ts: Timestamp,
    ) -> anyhow::Result<
        Option<
            Box<dyn Iterator<Item = (Timestamp, WithHeapSize<Vector<DatabaseIndexWrite>>)> + '_>,
        >,
    >;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    deployment_id: DeploymentId,
    index_id: IndexId,
    interval: Interval,
    order: Order,
    max_size: usize,
}

impl CacheKey {
    fn size(&self) -> usize {
        std::mem::size_of::<Self>() + self.interval.heap_size()
    }
}

struct IndexIntervalsInner {
    map: IntervalMap,
    id_to_params: StdHashMap<SubscriberId, (Interval, Order, usize)>,
    /// Maps each registered (interval, order, max_size) to its (SubscriberId,
    /// refcount). Each call to `insert` increments the refcount; each call to
    /// `remove` decrements it. The entry is only removed from the IntervalMap
    /// when the refcount reaches zero
    /// Refcounting is necessary to prevent the lazy eviction listener
    /// from unregistering an interval that a concurrent populate re-registered.
    params_to_id: StdHashMap<(Interval, Order, usize), (SubscriberId, usize)>,
    next_id: SubscriberId,
}

/// Tracks which (interval, order, max_size) triples are cached for a given
/// index, using an IntervalMap for O((k+1) log n) point queries
#[derive(Clone)]
struct IndexIntervals {
    inner: Arc<Mutex<IndexIntervalsInner>>,
}

impl IndexIntervals {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(IndexIntervalsInner {
                map: IntervalMap::new(),
                id_to_params: StdHashMap::new(),
                params_to_id: StdHashMap::new(),
                next_id: 0,
            })),
        }
    }

    /// Increments the refcount for an (interval, order, max_size) triple,
    /// inserting it into the IntervalMap if this is the first registration.
    fn insert(&self, interval: Interval, order: Order, max_size: usize) {
        let mut inner = self.inner.lock();
        if let Some((_, refcount)) =
            inner
                .params_to_id
                .get_mut(&(interval.clone(), order, max_size))
        {
            *refcount += 1;
            return;
        }
        let id = inner.next_id;
        inner.next_id += 1;
        inner
            .map
            .insert(id, [interval.clone()])
            .expect("stored more than u32::MAX intervals?");
        inner
            .id_to_params
            .insert(id, (interval.clone(), order, max_size));
        inner
            .params_to_id
            .insert((interval, order, max_size), (id, 1));
    }

    /// Decrements the refcount for an (interval, order, max_size) triple,
    /// removing it from the IntervalMap when the refcount reaches zero.
    /// No-op if not present.
    fn remove(&self, interval: &Interval, order: Order, max_size: usize) {
        let mut inner = self.inner.lock();
        if let Entry::Occupied(mut e) =
            inner
                .params_to_id
                .entry((interval.clone(), order, max_size))
        {
            let (id, refcount) = e.get_mut();
            *refcount -= 1;
            if *refcount == 0 {
                let id = *id;
                e.remove();
                inner.id_to_params.remove(&id);
                inner.map.remove(id);
            }
        }
    }

    fn intervals(&self) -> impl Iterator<Item = (Interval, Order, usize)> + '_ {
        let map = self.inner.lock().params_to_id.clone();
        map.into_iter()
            .map(|((interval, order, max_size), _id)| (interval, order, max_size))
    }

    fn is_empty(&self) -> bool {
        self.inner.lock().params_to_id.is_empty()
    }

    fn contains(&self, interval: &Interval, order: Order, max_size: usize) -> bool {
        self.inner
            .lock()
            .params_to_id
            .contains_key(&(interval.clone(), order, max_size))
    }

    /// Returns all (interval, order, max_size) triples whose interval contains
    /// `old` or `new`. Results are deduplicated.
    fn query_keys(
        &self,
        old: Option<&[u8]>,
        new: Option<&[u8]>,
    ) -> HashSet<(Interval, Order, usize)> {
        let inner = self.inner.lock();
        let mut ids = HashSet::new();
        for key in old.into_iter().chain(new) {
            inner.map.query(key, |id| {
                ids.insert(id);
            });
        }
        ids.into_iter()
            .map(|id| inner.id_to_params.get(&id).unwrap().clone())
            .collect()
    }
}

/// Shared cache for index range reads up-to-date as of the latest commits.
#[derive(Clone)]
pub struct IndexCache {
    cache: moka::sync::Cache<CacheKey, CachedInterval>,
    /// Nested map of deployments to indexes to (interval, order, max_size)
    /// triples tracked in the cache. May include intervals that are no
    /// longer tracked because they were evicted by moka, but no interval
    /// can be cached without also being added to this map. False positives
    /// but no false negatives.
    ///
    /// N.B. We only allow the moka lock to be acquired before the
    /// dashmap lock (cache -> index_to_intervals). We always
    /// clone IndexIntervals (releasing the DashMap shard
    /// lock) before acquiring the moka cache lock to avoid deadlocks.
    index_to_intervals: Arc<DashMap<DeploymentId, DashMap<IndexId, IndexIntervals>>>,
    next_deployment_id: Arc<AtomicU32>,
}

impl IndexCache {
    pub fn new(max_weight: u64) -> Self {
        let index_to_intervals: Arc<DashMap<DeploymentId, DashMap<IndexId, IndexIntervals>>> =
            Arc::new(DashMap::new());
        let index_to_intervals_clone = index_to_intervals.clone();
        let cache = moka::sync::Cache::builder()
            .weigher(|key: &CacheKey, value: &CachedInterval| -> u32 {
                // Multiply key size by 2 because we also store it in index_to_intervals. This
                // is an underestimate.
                2 * key.size() as u32 + value.size() as u32
            })
            .max_capacity(max_weight)
            .eviction_listener(move |key: Arc<CacheKey>, _val, cause| {
                // Skip in-place replacements for marking the cache entry as ready.
                // The interval registration is unchanged in that case and the refcount
                // shouldn't be decremented.
                if cause == RemovalCause::Replaced {
                    return;
                }
                // Clone IndexIntervals to release the DashMap shard lock
                // before acquiring IndexIntervals::inner.
                let Some(intervals) = index_to_intervals_clone
                    .get(&key.deployment_id)
                    .and_then(|d| d.get(&key.index_id).map(|r| r.value().clone()))
                else {
                    return;
                };
                intervals.remove(&key.interval, key.order, key.max_size);
                if let Some(deployment_intervals) = index_to_intervals_clone.get(&key.deployment_id)
                {
                    deployment_intervals.remove_if(&key.index_id, |_, v| v.is_empty());
                }
                index_to_intervals_clone.remove_if(&key.deployment_id, |_, v| v.is_empty());
            })
            .build();
        Self {
            cache,
            index_to_intervals,
            next_deployment_id: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn new_handle(&self) -> IndexCacheHandle {
        let id = self.next_deployment_id.fetch_add(1, Ordering::SeqCst);
        assert_ne!(id, u32::MAX, "DeploymentId overflow");
        IndexCacheHandle {
            deployment_id: DeploymentId(id),
            cache: self.clone(),
            write_log_reader: None,
        }
    }

    pub fn remove_deployment(&self, deployment_id: DeploymentId) {
        if let Some((_id, deployment_intervals)) = self.index_to_intervals.remove(&deployment_id) {
            // Clone to release the DashMap lock before removing entries from the cache.
            let entries: Vec<(IndexId, IndexIntervals)> = deployment_intervals
                .iter()
                .map(|i| (*i.key(), i.value().clone()))
                .collect();

            for (index_id, intervals) in entries {
                for (interval, order, max_size) in intervals.intervals() {
                    self.cache.remove(&CacheKey {
                        deployment_id,
                        index_id,
                        interval,
                        order,
                        max_size,
                    });
                }
            }
        }
        log_index_cache_size(self.cache.weighted_size());
    }

    /// Invalidate a cache entry if the write falls within its tracked interval.
    fn apply_write_to_cache(&self, key: &CacheKey, write: &DatabaseIndexWrite) {
        let old_in_interval = write
            .update
            .old
            .as_ref()
            .is_some_and(|k| key.interval.contains(k));
        let new_in_interval = write
            .update
            .new
            .as_ref()
            .is_some_and(|k| key.interval.contains(k));
        if !old_in_interval && !new_in_interval {
            return;
        }
        if self.cache.remove(key).is_some() {
            tracing::debug!(
                deployment_id = ?key.deployment_id,
                "IndexCache::apply_write_to_cache invalidated entry"
            );
            log_index_cache_invalidation();
        }
    }
}

/// A handle to [`IndexCache`] scoped to a single deployment.
///
/// Owns the per-deployment `DeploymentId` and delegates all cache operations
/// to the shared underlying `IndexCache` using that ID.
#[derive(Clone)]
pub struct IndexCacheHandle {
    pub deployment_id: DeploymentId,
    cache: IndexCache,
    /// Write log reader is used to validate cache entries by reading the write
    /// log up to the latest timestamp during populate.
    write_log_reader: Option<Arc<dyn WriteLogIndexReader>>,
}

impl IndexCacheHandle {
    pub fn set_write_log_reader(&mut self, reader: Arc<dyn WriteLogIndexReader>) {
        self.write_log_reader = Some(reader);
    }

    pub fn remove_deployment(&self) {
        self.cache.remove_deployment(self.deployment_id);
    }

    pub fn get(
        &self,
        index_id: IndexId,
        interval: Interval,
        ts: RepeatableTimestamp,
        order: Order,
        max_size: usize,
    ) -> Option<IndexPage> {
        let mut timer = index_cache_get_timer();
        let result = self
            .cache
            .cache
            .get(&CacheKey {
                deployment_id: self.deployment_id,
                index_id,
                interval,
                order,
                max_size,
            })
            .and_then(|cached_interval| cached_interval.index_page_at_ts(ts));
        let hit = result.is_some();
        if hit {
            timer.add_label(StaticMetricLabel::new("status", "hit"));
        } else {
            timer.add_label(StaticMetricLabel::new("status", "miss"));
        }
        tracing::debug!(hit, "IndexCache::get");
        result
    }

    /// Populate an entry in the cache after a cache miss,
    ///
    /// Uses 2PC to ensure the cache entry is only inserted if it is valid as of
    /// latest writes in the write log.
    ///
    /// Phase 1: Insert an entry into the cache with is_ready = false.
    ///
    /// Add the interval to `index_to_intervals`. Any new writes in the interval
    /// will invalidate this entry in `apply_writes`. `apply_writes` is called
    /// after `log.append` is called, so it is not possible to "miss" a write
    /// that would invalidate this interval when we iterate over the write log.
    ///
    /// Iterate over the write
    /// log reader since the ts of this entry to check that there are no
    /// conflicting writes.
    ///
    /// Phase 2: Mark the entry as ready.
    pub fn populate(
        &self,
        index_id: IndexId,
        interval: Interval,
        ts: RepeatableTimestamp,
        order: Order,
        max_size: usize,
        index_page: IndexPage,
        index_registry: &IndexRegistry,
    ) {
        let deployment_id = self.deployment_id;
        let mut timer = index_cache_populate_timer();
        let key = CacheKey {
            deployment_id,
            index_id,
            interval: interval.clone(),
            order,
            max_size,
        };
        // Only insert if there's no existing entry — a prior entry with an earlier
        // begin_ts can serve a wider range of reads.
        if self.cache.cache.contains_key(&key) {
            timer.add_label(StaticMetricLabel::new("result", "already_exists"));
            return;
        }
        let mut entries_size = 0;
        let entries: OrdSet<Arc<IndexEntry>> = index_page
            .entries
            .into_iter()
            .inspect(|entry| {
                entries_size += std::mem::size_of::<IndexEntry>() + entry.heap_size();
            })
            .collect();
        let Some(index) = index_registry.enabled_index_by_index_id(&index_id) else {
            timer.add_label(StaticMetricLabel::new("result", "unknown_index"));
            return;
        };
        let cached_interval = CachedInterval {
            is_ready: false,
            entries,
            order,
            cursor: index_page.cursor,
            entries_size,
            begin_ts: ts,
        };
        self.cache.cache.insert(key.clone(), cached_interval);

        self.cache
            .index_to_intervals
            .entry(deployment_id)
            .or_default()
            .entry(index_id)
            .or_insert_with(IndexIntervals::new)
            .insert(interval.clone(), order, max_size);

        let Ok(writes) = self
            .write_log_reader
            .as_ref()
            .unwrap()
            .iter_writes_after(index.name(), *ts)
        else {
            // Remove the cache entry. The eviction listener will remove from
            // index_to_intervals
            self.cache.cache.remove(&key);
            timer.add_label(StaticMetricLabel::new("result", "out_of_retention"));
            return;
        };
        if let Some(mut writes) = writes {
            let conflicts =
                |(_ts, writes): (Timestamp, WithHeapSize<Vector<DatabaseIndexWrite>>)| {
                    for key in writes.iter().flat_map(|i| i.update.iter()) {
                        if interval.contains(key) {
                            return true;
                        }
                    }
                    false
                };
            if writes.any(conflicts) {
                tracing::debug!(
                    deployment_id = ?deployment_id,
                    "IndexCache::populate rejected by write"
                );
                timer.add_label(StaticMetricLabel::new("result", "invalid"));
                // Remove the cache entry. The eviction listener will remove from
                // index_to_intervals
                self.cache.cache.remove(&key);
                return;
            }
        }

        // Phase 2 of 2PC: mark the cache entry as ready to serve reads if it's still
        // there. If it is missing, it was evicted by a concurrent call to
        // `apply_writes`.
        let index_to_intervals = self.cache.index_to_intervals.clone();
        self.cache.cache.entry(key).and_compute_with(|maybe_entry| {
            if let Some(entry) = maybe_entry
                && entry.value().begin_ts == ts
            {
                let interval_is_recorded = index_to_intervals
                    .get(&deployment_id)
                    .and_then(|d| d.get(&index_id).map(|iv| iv.value().clone()))
                    .map(|iv| iv.contains(&interval, order, max_size))
                    .unwrap_or(false);
                if !interval_is_recorded {
                    tracing::error!(
                        "IndexCache invariant violated: marking entry ready but interval not \
                         registered in index_to_intervals (deployment_id={deployment_id:?}, \
                         index_id={index_id:?})"
                    );
                    timer.add_label(StaticMetricLabel::new("result", "invalid"));
                    return Op::Remove;
                }
                let mut value = entry.into_value();
                value.is_ready = true;
                timer.add_label(StaticMetricLabel::new("result", "populated"));
                tracing::debug!(
                    deployment_id = ?deployment_id,
                    "IndexCache::populate inserted entry"
                );
                Op::Put(value)
            } else {
                timer.add_label(StaticMetricLabel::new("result", "invalid"));
                Op::Nop
            }
        });
        log_index_cache_size(self.cache.cache.weighted_size());
    }

    /// TODO: Remove when IndexCache is stable.
    /// Used when a cached result is found to be incorrect (e.g. it
    /// mismatches persistence) so that subsequent reads do not re-surface
    /// the same error.
    pub fn invalidate(&self, index_id: IndexId, interval: Interval, order: Order, max_size: usize) {
        let key = CacheKey {
            deployment_id: self.deployment_id,
            index_id,
            interval,
            order,
            max_size,
        };
        self.cache.cache.remove(&key);
        log_index_cache_size(self.cache.cache.weighted_size());
    }

    /// Apply index updates and new document value to the cache, invalidating
    /// cache entries with overlapping intervals and tracking writes
    /// in the write buffer.
    pub fn apply_writes(
        &self,
        writes_by_index: &BTreeMap<TabletIndexName, WithHeapSize<Vector<DatabaseIndexWrite>>>,
        index_name_to_id: &dyn Fn(&TabletIndexName) -> Option<IndexId>,
    ) {
        let _timer = cache_apply_writes_timer();
        let deployment_id = self.deployment_id;
        for (index_name, writes) in writes_by_index {
            let Some(index_id) = (index_name_to_id)(index_name) else {
                continue;
            };
            // Clone the Arc-backed IndexIntervals (releasing the DashMap shard
            // lock) before iterating writes so apply_write_to_cache can acquire
            // the same DashMap entry without deadlocking.
            let Some(intervals) = self
                .cache
                .index_to_intervals
                .get(&deployment_id)
                .and_then(|d| d.get(&index_id).map(|r| r.value().clone()))
            else {
                continue;
            };
            for write in writes {
                let matching = intervals.query_keys(
                    write.update.old.as_deref().map(|v| v.as_slice()),
                    write.update.new.as_deref().map(|v| v.as_slice()),
                );
                for (interval, order, max_size) in matching {
                    let key = CacheKey {
                        deployment_id,
                        index_id,
                        interval,
                        order,
                        max_size,
                    };
                    self.cache.apply_write_to_cache(&key, write);
                }
            }
        }
        log_index_cache_size(self.cache.cache.weighted_size());
    }
}

#[derive(Clone)]
pub struct CachedInterval {
    /// Whether this interval is ready to serve reads (it has been validated by
    /// reading the write log up to the latest timestamp)
    is_ready: bool,
    entries: OrdSet<Arc<IndexEntry>>,
    order: Order,
    cursor: CursorPosition,
    entries_size: usize,
    begin_ts: RepeatableTimestamp,
}

impl CachedInterval {
    fn index_page_at_ts(&self, ts: RepeatableTimestamp) -> Option<IndexPage> {
        if !self.is_ready {
            return None;
        }
        if ts < self.begin_ts {
            return None;
        }
        // Since writes to this interval invalidate the cache entry, the entries
        // are always from the original populate snapshot, valid at any ts >=
        // begin_ts. The cursor is also from the original page.
        let entries = self.order.apply(self.entries.iter().cloned()).collect();
        Some(IndexPage {
            entries,
            cursor: self.cursor.clone(),
        })
    }

    fn size(&self) -> usize {
        std::mem::size_of::<Self>() + self.entries_size + self.cursor.heap_size()
    }
}
