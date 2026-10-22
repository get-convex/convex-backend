use std::{
    collections::{
        btree_map::Entry,
        BTreeMap,
        HashSet,
    },
    sync::{
        atomic::{
            AtomicU32,
            AtomicU64,
            Ordering,
        },
        Arc,
    },
};

use common::{
    document_index_keys::DatabaseIndexWrite,
    erased_slot::ErasedSlot,
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
#[cfg(not(feature = "shuttle-testing"))]
use dashmap::DashMap;
use interval_map::IntervalMap;
use metrics::StaticMetricLabel;
use moka::{
    notification::RemovalCause,
    ops::compute::{
        CompResult,
        Op,
    },
};
#[cfg(not(feature = "shuttle-testing"))]
use parking_lot::Mutex;
#[cfg(feature = "shuttle-testing")]
use shuttle_dashmap::DashMap;
#[cfg(feature = "shuttle-testing")]
use shuttle_parking_lot::Mutex;
use value::{
    heap_size::HeapSize,
    TabletId,
};

use crate::{
    atomic_cache::AtomicCache,
    index_reader::{
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
        log_index_cache_size_eviction,
    },
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct DeploymentId(u32);

pub trait WriteLogIndexReader: Send + Sync {
    /// Iterate over writes to an index after the given timestamp.
    fn iter_writes_after<'a>(
        &self,
        index_name: &TabletIndexName,
        ts: Timestamp,
        storage: &'a mut ErasedSlot,
    ) -> anyhow::Result<Box<dyn Iterator<Item = &'a DatabaseIndexWrite> + 'a>>;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    deployment_id: DeploymentId,
    index_id: IndexId,
    interval: Arc<Interval>,
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
    id_to_params: BTreeMap<SubscriberId, (Arc<Interval>, Order, usize)>,
    /// Maps each registered (interval, order, max_size) to its (SubscriberId,
    /// refcount). Each call to `insert` increments the refcount; each call to
    /// `remove` decrements it. The entry is only removed from the IntervalMap
    /// when the refcount reaches zero
    /// Refcounting is necessary to prevent the lazy eviction listener
    /// from unregistering an interval that a concurrent populate re-registered.
    params_to_id: BTreeMap<(Arc<Interval>, Order, usize), (SubscriberId, usize)>,
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
                id_to_params: BTreeMap::new(),
                params_to_id: BTreeMap::new(),
                next_id: 0,
            })),
        }
    }

    /// Increments the refcount for an (interval, order, max_size) triple,
    /// inserting it into the IntervalMap if this is the first registration.
    fn insert(&self, interval: Arc<Interval>, order: Order, max_size: usize) {
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
            .insert(id, [(*interval).clone()])
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
    fn remove(&self, interval: Arc<Interval>, order: Order, max_size: usize) {
        let mut inner = self.inner.lock();
        if let Entry::Occupied(mut e) = inner.params_to_id.entry((interval, order, max_size)) {
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

    fn intervals(&self) -> impl Iterator<Item = (Arc<Interval>, Order, usize)> + '_ {
        let map = self.inner.lock().params_to_id.clone();
        map.into_iter()
            .map(|((interval, order, max_size), _id)| (interval, order, max_size))
    }

    fn is_empty(&self) -> bool {
        self.inner.lock().params_to_id.is_empty()
    }

    fn contains(&self, interval: &Arc<Interval>, order: Order, max_size: usize) -> bool {
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
    ) -> HashSet<(Arc<Interval>, Order, usize)> {
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
    cache: AtomicCache<CacheKey, CachedInterval>,
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
    /// Monotonic id stamped on each `populate`'s Phase-1 entry so that Phase 2
    /// only marks ready the entry THIS call inserted and validated. Without it,
    /// a populate could mark ready an entry that a concurrent populate
    /// re-created for the same key (after this one's entry was
    /// evicted/invalidated), applying a stale write-log validation —
    /// serving stale reads.
    next_populate_id: Arc<AtomicU64>,
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
                if cause == RemovalCause::Size {
                    log_index_cache_size_eviction();
                }
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
                intervals.remove(key.interval.clone(), key.order, key.max_size);
                if let Some(deployment_intervals) = index_to_intervals_clone.get(&key.deployment_id)
                {
                    deployment_intervals.remove_if(&key.index_id, |_, v| v.is_empty());
                }
                index_to_intervals_clone.remove_if(&key.deployment_id, |_, v| v.is_empty());
            })
            .build();
        Self {
            cache: AtomicCache::new(cache),
            index_to_intervals,
            next_deployment_id: Arc::new(AtomicU32::new(0)),
            next_populate_id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn new_handle(&self) -> IndexCacheHandleBuilder {
        let id = self.next_deployment_id.fetch_add(1, Ordering::SeqCst);
        assert_ne!(id, u32::MAX, "DeploymentId overflow");
        IndexCacheHandleBuilder {
            deployment_id: DeploymentId(id),
            shared_cache: self.clone(),
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
                self.evict(deployment_id, index_id, &intervals);
            }
        }
        log_index_cache_size(self.cache.weighted_size());
    }

    /// The caller must release all DashMap locks before evicting cache entries.
    fn evict(&self, deployment_id: DeploymentId, index_id: IndexId, intervals: &IndexIntervals) {
        for (interval, order, max_size) in intervals.intervals() {
            self.cache.remove(CacheKey {
                deployment_id,
                index_id,
                interval,
                order,
                max_size,
            });
        }
    }

    /// Whether `(interval, order, max_size)` is currently registered for this
    /// index, meaning apply_writes can still invalidate its cache entry. An
    /// untracked entry must not be served: the deployment was removed, or the
    /// index was modified (which untracks its intervals).
    fn is_interval_tracked(
        index_to_intervals: &Arc<DashMap<DeploymentId, DashMap<IndexId, IndexIntervals>>>,
        deployment_id: DeploymentId,
        index_id: IndexId,
        interval: &Arc<Interval>,
        order: Order,
        max_size: usize,
    ) -> bool {
        index_to_intervals
            .get(&deployment_id)
            .and_then(|d| d.get(&index_id).map(|iv| iv.value().clone()))
            .map(|iv| iv.contains(interval, order, max_size))
            .unwrap_or(false)
    }
}

#[derive(Clone)]
pub struct IndexCacheHandleBuilder {
    pub deployment_id: DeploymentId,
    shared_cache: IndexCache,
}

impl IndexCacheHandleBuilder {
    pub fn build(
        self,
        index_tablet: TabletId,
        write_log_reader: Arc<dyn WriteLogIndexReader>,
    ) -> IndexCacheHandle {
        IndexCacheHandle {
            deployment_id: self.deployment_id,
            index_tablet,
            shared_cache: self.shared_cache,
            write_log_reader,
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
    /// The `_index.by_id` index name for this deployment. Writes to this index
    /// are modifications to index metadata, which evict the modified index
    /// from the cache.
    index_tablet: TabletId,
    shared_cache: IndexCache,
    /// Write log reader is used to validate cache entries by reading the write
    /// log up to the latest timestamp during populate.
    write_log_reader: Arc<dyn WriteLogIndexReader>,
}

impl IndexCacheHandle {
    pub fn remove_deployment(&self) {
        self.shared_cache.remove_deployment(self.deployment_id);
    }

    fn remove_index(&self, index_id: &IndexId) {
        // Release the DashMap locks before acquiring the cache lock in `evict`.
        let removed = self
            .shared_cache
            .index_to_intervals
            .get(&self.deployment_id)
            .and_then(|d| d.remove(index_id));
        if let Some((index_id, intervals)) = removed {
            self.shared_cache
                .evict(self.deployment_id, index_id, &intervals);
        }
    }

    pub(crate) fn get(
        &self,
        index_id: IndexId,
        interval: Arc<Interval>,
        ts: RepeatableTimestamp,
        order: Order,
        max_size: usize,
    ) -> Option<(IndexPage, RepeatableTimestamp)> {
        let mut timer = index_cache_get_timer();
        let key = CacheKey {
            deployment_id: self.deployment_id,
            index_id,
            interval: interval.clone(),
            order,
            max_size,
        };
        let result = self
            .shared_cache
            .cache
            .get(&key)
            .and_then(|cached_interval| cached_interval.index_page_at_ts(ts));
        if let Some((ref _index_page, _cache_ts)) = result {
            // A ready entry must have its interval tracked so apply_writes can invalidate
            // it. If it isn't, this entry would serve stale data — remove it and treat as
            // a miss.
            // DashMap lock is released before any moka call to maintain lock ordering.
            if IndexCache::is_interval_tracked(
                &self.shared_cache.index_to_intervals,
                self.deployment_id,
                index_id,
                &interval,
                order,
                max_size,
            ) {
                timer.add_label(StaticMetricLabel::new("status", "hit"));
            } else {
                self.shared_cache.cache.remove(key);
                timer.add_label(StaticMetricLabel::new("status", "miss"));
                return None;
            }
        } else {
            timer.add_label(StaticMetricLabel::new("status", "miss"));
        }
        tracing::debug!(hit = result.is_some(), "IndexCache::get");
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
    pub(crate) fn populate(
        &self,
        index_id: IndexId,
        interval: Arc<Interval>,
        ts: RepeatableTimestamp,
        order: Order,
        max_size: usize,
        index_page: IndexPage,
        index_registry: &IndexRegistry,
    ) {
        let deployment_id = self.deployment_id;
        let populate_id = self
            .shared_cache
            .next_populate_id
            .fetch_add(1, Ordering::Relaxed);
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
        if self.shared_cache.cache.contains_key(&key) {
            timer.add_label(StaticMetricLabel::new("result", "already_exists"));
            return;
        }
        let IndexPage { entries, cursor } = index_page;
        let entries_size = entries
            .iter()
            .map(|entry| std::mem::size_of::<IndexEntry>() + entry.heap_size())
            .sum();
        let Some(index) = index_registry.enabled_index_by_index_id(&index_id) else {
            timer.add_label(StaticMetricLabel::new("result", "unknown_index"));
            return;
        };
        let cached_interval = CachedInterval {
            is_ready: false,
            entries: entries.into(),
            cursor,
            entries_size,
            begin_ts: ts,
            populate_id,
        };
        let result = self.shared_cache.cache.compute(key.clone(), |maybe_entry| {
            // Only insert if there's no existing entry — a prior entry with an earlier
            // begin_ts can serve a wider range of reads.
            if maybe_entry.is_some() {
                Op::Nop
            } else {
                Op::Put(cached_interval)
            }
        });
        if !matches!(result, CompResult::Inserted(_)) {
            timer.add_label(StaticMetricLabel::new("result", "already_exists"));
            return;
        }

        self.shared_cache
            .index_to_intervals
            .entry(deployment_id)
            .or_default()
            .entry(index_id)
            .or_insert_with(IndexIntervals::new)
            .insert(interval.clone(), order, max_size);

        let (mut s0, mut s1) = (ErasedSlot::new(), ErasedSlot::new());
        let (Ok(mut writes), Ok(index_table_writes)) = (
            self.write_log_reader
                .iter_writes_after(&index.name(), *ts, &mut s0),
            self.write_log_reader.iter_writes_after(
                &TabletIndexName::by_id(self.index_tablet),
                *ts,
                &mut s1,
            ),
        ) else {
            // Remove the cache entry. The eviction listener will remove from
            // index_to_intervals
            self.shared_cache.cache.remove(key);
            timer.add_label(StaticMetricLabel::new("result", "out_of_retention"));
            return;
        };
        if writes.any(|i| i.update.iter().any(|key| interval.contains(key))) {
            tracing::debug!(
                deployment_id = ?deployment_id,
                "IndexCache::populate rejected by write"
            );
            timer.add_label(StaticMetricLabel::new("result", "invalid"));
            // Remove the cache entry. The eviction listener will remove from
            // index_to_intervals
            self.shared_cache.cache.remove(key);
            return;
        }
        for write in index_table_writes {
            if write.document_id.internal_id() == index_id.0 {
                timer.add_label(StaticMetricLabel::new("result", "invalid"));
                // Remove the cache entry. The eviction listener will remove from
                // index_to_intervals
                self.shared_cache.cache.remove(key);
                return;
            }
        }

        // Phase 2 of 2PC: mark the cache entry as ready to serve reads if it's still
        // there. If it is missing, it was evicted by a concurrent call to
        // `apply_writes`.
        self.shared_cache.cache.compute(key, |maybe_entry| {
            if let Some(entry) = maybe_entry
                && entry.value().begin_ts == ts
            {
                // Drop the entry if its interval is no longer registered. Both causes
                // are expected and benign: remove_deployment atomically removes the
                // deployment entry before its cache entries, and a concurrent index
                // modification untracks the index's intervals. Either way this populate's
                // snapshot is no longer safe to serve, so remove it.
                if !IndexCache::is_interval_tracked(
                    &self.shared_cache.index_to_intervals,
                    deployment_id,
                    index_id,
                    &interval,
                    order,
                    max_size,
                ) {
                    timer.add_label(StaticMetricLabel::new("result", "invalid"));
                    return Op::Remove;
                }
                // Make sure we only mark our own entry as ready.
                let is_own = entry.value().populate_id == populate_id;
                if !is_own {
                    timer.add_label(StaticMetricLabel::new("result", "foreign_entry"));
                    return Op::Nop;
                }
                // Tripwire for if we failed to check `is_own`. Unreachable here, used for
                // regression test.
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
        log_index_cache_size(self.shared_cache.cache.weighted_size());
    }

    /// Apply index updates and new document value to the cache, invalidating
    /// cache entries with overlapping intervals and tracking writes
    /// in the write buffer.
    pub fn apply_writes<'a, W>(
        &self,
        writes_by_index: impl IntoIterator<Item = (&'a TabletIndexName, W)>,
        index_name_to_id: &dyn Fn(&TabletIndexName) -> Option<IndexId>,
    ) where
        W: Clone + Iterator<Item = &'a DatabaseIndexWrite>,
    {
        let _timer = cache_apply_writes_timer();
        let deployment_id = self.deployment_id;
        for (index_name, writes) in writes_by_index {
            // When an index is modified, we untrack it and evict its cached entries, so
            // no reader at or after the modification can be served one. This covers
            // staging, which keeps the IndexId while writes to the staged index skip
            // apply_writes, so entries kept through a stage/unstage would be stale.
            if &TabletIndexName::by_id(self.index_tablet) == index_name {
                for write in writes.clone() {
                    self.remove_index(&IndexId(write.document_id.internal_id()))
                }
            }
            let Some(index_id) = (index_name_to_id)(index_name) else {
                continue;
            };
            // Clone the Arc-backed IndexIntervals to avoid holding the DashMap
            // shard lock while iterating over writes.
            let Some(intervals) = self
                .shared_cache
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
                    self.shared_cache.cache.remove(key.clone());
                    tracing::debug!(
                        deployment_id = ?key.deployment_id,
                        "IndexCache::apply_writes invalidated entry"
                    );
                    log_index_cache_invalidation();
                }
            }
        }
        log_index_cache_size(self.shared_cache.cache.weighted_size());
    }
}

#[derive(Clone)]
pub struct CachedInterval {
    /// Whether this interval is ready to serve reads (it has been validated by
    /// reading the write log up to the latest timestamp)
    is_ready: bool,
    /// Stored in the order returned by the index reader.
    entries: Arc<[Arc<IndexEntry>]>,
    cursor: CursorPosition,
    entries_size: usize,
    begin_ts: RepeatableTimestamp,
    /// Identifies the `populate` call that inserted this entry. Phase 2 only
    /// marks ready the entry whose `populate_id` matches the current call, so a
    /// populate never marks an entry as ready that was created by a
    /// different populate
    populate_id: u64,
}

impl CachedInterval {
    fn index_page_at_ts(
        &self,
        ts: RepeatableTimestamp,
    ) -> Option<(IndexPage, RepeatableTimestamp)> {
        if !self.is_ready {
            return None;
        }
        if ts < self.begin_ts {
            return None;
        }
        // Since writes to this interval invalidate the cache entry, the entries
        // are always from the original populate snapshot, valid at any ts >=
        // begin_ts. The cursor is also from the original page.
        let entries = self.entries.iter().cloned().collect();
        Some((
            IndexPage {
                entries,
                cursor: self.cursor.clone(),
            },
            self.begin_ts,
        ))
    }

    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of_val(self.entries.as_ref())
            + self.entries_size
            + self.cursor.heap_size()
    }
}

// Disable these tests when `shuttle-testing` is enabled, as that feature swaps
// in shuttle's synchronization primitives crate-wide, which panic when used
// outside a shuttle runner
#[cfg(all(test, not(feature = "shuttle-testing")))]
mod tests {
    use std::{
        collections::BTreeMap,
        iter,
    };

    use common::{
        bootstrap_model::index::{
            database_index::IndexedFields,
            IndexMetadata,
            INDEX_TABLE,
        },
        document::{
            CreationTime,
            PackedDocument,
            ResolvedDocument,
        },
        document_index_keys::{
            DatabaseIndexWrite,
            Update,
        },
        index::IndexKeyBytes,
        interval::{
            BinaryKey,
            End,
            Interval,
            StartIncluded,
        },
        query::{
            CursorPosition,
            Order,
        },
        testing::TestIdGenerator,
        types::{
            unchecked_repeatable_ts,
            GenericIndexName,
            IndexId,
            TableName,
            Timestamp,
        },
    };
    use value::{
        assert_obj,
        ResolvedDocumentId,
    };

    use super::*;
    use crate::{
        index_reader::IndexPage,
        index_registry::IndexRegistry,
    };

    struct IndexCacheTestFixture {
        handle: IndexCacheHandle,
        id_generator: TestIdGenerator,
        mock_write_log: Arc<MockWriteLogReader>,
        index_id_to_name: BTreeMap<IndexId, TabletIndexName>,
        index_docs: BTreeMap<ResolvedDocumentId, ResolvedDocument>,
        // `_index` tablet id
        index_tablet: TabletId,
        /// Placeholder used in test DatabaseIndexWrites; not read by the cache.
        dummy_doc_id: ResolvedDocumentId,
    }

    impl IndexCacheTestFixture {
        fn new() -> Self {
            let mut id_generator = TestIdGenerator::new();

            // Register INDEX_TABLE and create the required _index.by_id document
            // that IndexRegistry::bootstrap always expects.
            let index_tablet = id_generator.system_table_id(&INDEX_TABLE).tablet_id;
            let index_by_id_doc_id = id_generator.system_generate(&INDEX_TABLE);
            let index_by_id_doc = ResolvedDocument::new(
                index_by_id_doc_id,
                CreationTime::ONE,
                IndexMetadata::new_enabled(
                    GenericIndexName::by_id(index_tablet),
                    IndexedFields::by_id(),
                )
                .try_into()
                .expect("serialize _index.by_id metadata"),
            )
            .expect("create _index.by_id document");
            let mut index_docs = BTreeMap::new();
            index_docs.insert(index_by_id_doc.id(), index_by_id_doc);

            // Pre-generate a placeholder document ID for test DatabaseIndexWrites
            // (document_id is not inspected by apply_writes).
            let dummy_doc_id = id_generator.system_generate(&INDEX_TABLE);

            let mock_write_log = Arc::new(MockWriteLogReader::new());
            let cache = IndexCache::new(100 * 1024 * 1024);
            let handle = cache
                .new_handle()
                .build(index_tablet, mock_write_log.clone());

            Self {
                handle,
                id_generator,
                mock_write_log,
                index_id_to_name: BTreeMap::new(),
                index_docs,
                index_tablet,
                dummy_doc_id,
            }
        }

        fn new_index(&mut self) -> IndexId {
            let n = self.index_id_to_name.len();
            let table_name: TableName =
                format!("test_table_{n}").parse().expect("valid table name");
            let tablet_id = self.id_generator.user_table_id(&table_name).tablet_id;
            let index_name = GenericIndexName::by_id(tablet_id);
            let doc_id = self.id_generator.system_generate(&INDEX_TABLE);
            let doc = ResolvedDocument::new(
                doc_id,
                CreationTime::ONE,
                IndexMetadata::new_enabled(index_name.clone(), IndexedFields::by_id())
                    .try_into()
                    .expect("serialize index metadata"),
            )
            .expect("create index document");
            let index_id = doc_id.internal_id().into();
            self.index_docs.insert(doc.id(), doc);
            self.index_id_to_name.insert(index_id, index_name);
            index_id
        }

        fn build_registry(&self) -> IndexRegistry {
            IndexRegistry::bootstrap(&self.id_generator, self.index_docs.values())
                .expect("build IndexRegistry")
        }

        fn make_doc(&mut self) -> anyhow::Result<(IndexKeyBytes, PackedDocument)> {
            let id = self.id_generator.user_generate(&"users".parse()?);
            let doc = ResolvedDocument::new(id, CreationTime::ONE, assert_obj!())?;
            let key = doc.index_key(&IndexedFields::by_id()).to_bytes();
            let packed = PackedDocument::pack(&doc);
            Ok((key, packed))
        }

        fn make_page(&self, entries: Vec<(IndexKeyBytes, Timestamp, PackedDocument)>) -> IndexPage {
            IndexPage {
                entries: entries
                    .into_iter()
                    .map(|(key, ts, value)| Arc::new(IndexEntry { key, ts, value }))
                    .collect(),
                cursor: CursorPosition::End,
            }
        }

        fn rt(&self, ts: i32) -> RepeatableTimestamp {
            unchecked_repeatable_ts(Timestamp::must(ts))
        }

        fn populate(
            &self,
            index_id: IndexId,
            interval: Interval,
            ts: RepeatableTimestamp,
            order: Order,
            max_size: usize,
            page: IndexPage,
        ) {
            let registry = self.build_registry();
            self.handle.populate(
                index_id,
                Arc::new(interval),
                ts,
                order,
                max_size,
                page,
                &registry,
            );
        }

        fn get_at(&self, index_id: IndexId, ts: i32) -> Option<IndexPage> {
            self.handle
                .get(
                    index_id,
                    Arc::new(Interval::all()),
                    self.rt(ts),
                    Order::Asc,
                    100,
                )
                .map(|(index_page, _ts)| index_page)
        }

        fn cache_entry(
            &self,
            index_id: IndexId,
            interval: Interval,
            order: Order,
            max_size: usize,
        ) -> Option<CachedInterval> {
            self.handle.shared_cache.cache.get(&CacheKey {
                deployment_id: self.handle.deployment_id,
                index_id,
                interval: Arc::new(interval),
                order,
                max_size,
            })
        }

        /// Create a new handle for a separate deployment, sharing the same
        /// underlying cache and mock write log reader.
        fn register_deployment(&self) -> IndexCacheHandle {
            self.handle
                .shared_cache
                .new_handle()
                .build(self.index_tablet, self.mock_write_log.clone())
        }

        /// Simulate modifying `index_id`'s metadata: a write to the `_index`
        /// table whose document is the index's own `_index` row. `apply_writes`
        /// recovers the modified `IndexId` from `document_id.internal_id()` and
        /// evicts it from the cache, so the document id must be the one that
        /// `new_index` minted for this index.
        fn modify_index(&self, index_id: IndexId) {
            let doc_id = *self
                .index_docs
                .keys()
                .find(|id| IndexId::from(id.internal_id()) == index_id)
                .expect("no _index document for index_id");
            let write = DatabaseIndexWrite {
                document_id: doc_id,
                // The remove_index path ignores the key update — it keys off
                // document_id alone.
                update: Update {
                    old: None,
                    new: None,
                },
                new_document: None,
            };
            self.handle.apply_writes(
                [(
                    &TabletIndexName::by_id(self.index_tablet),
                    iter::once(&write),
                )],
                &|name: &TabletIndexName| {
                    self.index_id_to_name
                        .iter()
                        .find_map(|(id, n)| if n == name { Some(*id) } else { None })
                },
            );
        }

        /// Invalidate cache entries by simulating a write to this index for the
        /// default deployment.
        fn apply_writes(
            &self,
            index_id: IndexId,
            old_key: Option<IndexKeyBytes>,
            new_key: Option<IndexKeyBytes>,
        ) {
            self.apply_writes_for_handle(&self.handle, index_id, old_key, new_key);
        }

        /// Invalidate cache entries for a specific deployment handle.
        fn apply_writes_for_handle(
            &self,
            handle: &IndexCacheHandle,
            index_id: IndexId,
            old_key: Option<IndexKeyBytes>,
            new_key: Option<IndexKeyBytes>,
        ) {
            let index_name = self.index_id_to_name[&index_id].clone();
            let write = DatabaseIndexWrite {
                document_id: self.dummy_doc_id,
                update: Update {
                    old: old_key,
                    new: new_key,
                },
                new_document: None,
            };
            handle.apply_writes(
                [(&index_name, iter::once(&write))],
                &|name: &TabletIndexName| {
                    self.index_id_to_name
                        .iter()
                        .find_map(|(id, n)| if n == name { Some(*id) } else { None })
                },
            );
        }

        /// Pre-load a write into the mock write log so that a subsequent
        /// `populate` call for this index at ts < write_ts sees a conflict.
        fn add_mock_write(
            &self,
            index_id: IndexId,
            write_ts: Timestamp,
            old_key: Option<IndexKeyBytes>,
            new_key: Option<IndexKeyBytes>,
        ) {
            let index_name = self.index_id_to_name[&index_id].clone();
            let write = DatabaseIndexWrite {
                document_id: self.dummy_doc_id,
                update: Update {
                    old: old_key,
                    new: new_key,
                },
                new_document: None,
            };
            self.mock_write_log.add_write(index_name, write_ts, write);
        }
    }

}

/// Shuttle-based concurrency tests. See `index_cache/shuttle_tests.rs`.
/// N.B.: `cfg(rust_analyzer)` is enabled in rust-analyzer only, not in rustc.
#[cfg(any(all(test, feature = "shuttle-testing"), rust_analyzer))]
mod shuttle_tests;
