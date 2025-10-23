use std::{
    any::{
        type_name,
        Any,
    },
    borrow::Borrow,
    cmp::{
        self,
        Ordering,
    },
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fmt::Debug,
    iter,
    sync::{
        Arc,
        LazyLock,
        OnceLock,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        database_index::DatabaseIndexState,
        IndexConfig,
        TabletIndexMetadata,
    },
    document::{
        PackedDocument,
        ParseDocument,
        ParsedDocument,
        ResolvedDocument,
    },
    index::{
        IndexKey,
        IndexKeyBytes,
    },
    instrument,
    interval::{
        EndRef,
        Interval,
        IntervalSet,
        StartIncluded,
    },
    knobs::TRANSACTION_MAX_READ_SIZE_BYTES,
    persistence::PersistenceSnapshot,
    query::{
        CursorPosition,
        Order,
    },
    runtime::{
        assert_send,
        try_join,
    },
    static_span,
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        IndexId,
        IndexName,
        PersistenceVersion,
        RepeatableTimestamp,
        TabletIndexName,
        Timestamp,
    },
    utils::ReadOnly,
    value::Size,
};
use errors::ErrorMetadata;
use fastrace::local::LocalSpan;
use futures::{
    stream,
    StreamExt as _,
    TryStreamExt,
};
use imbl::{
    OrdMap,
    OrdSet,
};
use itertools::Itertools;
use value::{
    InternalId,
    TableMapping,
    TableName,
    TabletId,
};

use crate::{
    index_registry::{
        IndexRegistry,
        IndexedDocument,
    },
    metrics::log_transaction_cache_query,
};

#[async_trait]
pub trait InMemoryIndexes: Send + Sync {
    /// Returns the index range if it is found in the cache (backend) or loaded
    /// into the cache (function runner). If the index is not supposed to be in
    /// memory, returns None so it is safe to call on any index.
    async fn range(
        &self,
        index_id: IndexId,
        interval: &Interval,
        order: Order,
        tablet_id: TabletId,
        table_name: TableName,
    ) -> anyhow::Result<Option<Vec<(IndexKeyBytes, Timestamp, MemoryDocument)>>>;
}

/// [`BackendInMemoryIndexes`] maintains in-memory database indexes. With the
/// exception of the table scan index, newly created indexes are not initially
/// loaded in memory. A post-commit, asynchronous backfill job is responsible
/// for filling the index.
#[derive(Clone)]
pub struct BackendInMemoryIndexes {
    /// Fully loaded in-memory indexes. If not present, the index is not loaded.
    in_memory_indexes: OrdMap<IndexId, DatabaseIndexMap>,
}

#[async_trait]
impl InMemoryIndexes for BackendInMemoryIndexes {
    async fn range(
        &self,
        index_id: IndexId,
        interval: &Interval,
        order: Order,
        _tablet_id: TabletId,
        _table_name: TableName,
    ) -> anyhow::Result<Option<Vec<(IndexKeyBytes, Timestamp, MemoryDocument)>>> {
        self.range(index_id, interval, order)
    }
}

impl BackendInMemoryIndexes {
    #[fastrace::trace]
    pub fn bootstrap(
        index_registry: &IndexRegistry,
        index_documents: Vec<(Timestamp, PackedDocument)>,
        ts: Timestamp,
    ) -> anyhow::Result<Self> {
        // Load the indexes by_id index
        let meta_index = index_registry
            .get_enabled(&TabletIndexName::by_id(index_registry.index_table()))
            .context("Missing meta index")?;
        let mut meta_index_map = DatabaseIndexMap::new_at(ts);
        for (ts, index_doc) in index_documents {
            let index_key = IndexKey::new(vec![], index_doc.developer_id());
            meta_index_map.insert(index_key.to_bytes(), ts, index_doc);
        }

        let mut in_memory_indexes = OrdMap::new();
        in_memory_indexes.insert(meta_index.id(), meta_index_map);

        Ok(Self { in_memory_indexes })
    }

    /// Fetch tables across all namespaces whose name is in `tables` and load
    /// their enabled indexes into memory.
    #[fastrace::trace]
    pub async fn load_enabled_for_tables(
        &mut self,
        index_registry: &IndexRegistry,
        table_mapping: &TableMapping,
        snapshot: &PersistenceSnapshot,
        tables: &BTreeSet<TableName>,
    ) -> anyhow::Result<()> {
        let enabled_indexes = index_registry.all_enabled_indexes();
        let mut indexes_by_table: BTreeMap<TabletId, Vec<_>> = BTreeMap::new();
        let mut indexes_to_load = 0;
        for index_metadata in enabled_indexes {
            let table_name = table_mapping.tablet_name(*index_metadata.name.table())?;
            if tables.contains(&table_name) {
                match &index_metadata.config {
                    IndexConfig::Database { on_disk_state, .. } => {
                        anyhow::ensure!(
                            *on_disk_state == DatabaseIndexState::Enabled,
                            "Index should have been enabled: {:?}, state: {on_disk_state:?}",
                            index_metadata.name
                        )
                    },
                    IndexConfig::Text { .. } | IndexConfig::Vector { .. } => {
                        // We do not load search or vector indexes into memory.
                        continue;
                    },
                }
                tracing::debug!(
                    "Loading {table_name}.{} ...",
                    index_metadata.name.descriptor()
                );
                indexes_by_table
                    .entry(*index_metadata.name.table())
                    .or_default()
                    .push(index_metadata);
                indexes_to_load += 1;
            }
        }
        tracing::info!(
            "Loading {} tables with {} indexes...",
            indexes_by_table.len(),
            indexes_to_load
        );
        for (tablet_id, index_metadatas) in indexes_by_table {
            let (num_keys, total_bytes) = self
                .load_enabled(tablet_id, index_metadatas, snapshot)
                .await?;
            tracing::debug!("Loaded {num_keys} keys, {total_bytes} bytes.");
        }
        Ok(())
    }

    #[fastrace::trace]
    pub async fn load_enabled(
        &mut self,
        tablet_id: TabletId,
        mut indexes: Vec<ParsedDocument<TabletIndexMetadata>>,
        snapshot: &PersistenceSnapshot,
    ) -> anyhow::Result<(usize, usize)> {
        indexes.retain(|index| {
            !self
                .in_memory_indexes
                .contains_key(&index.id().internal_id())
        });
        if indexes.is_empty() {
            // Already loaded in memory.
            return Ok((0, 0));
        }
        for index in &indexes {
            anyhow::ensure!(
                *index.name.table() == tablet_id,
                "Index is for wrong table {:?}",
                index.name.table()
            );
            if let IndexConfig::Database { on_disk_state, .. } = &index.config {
                anyhow::ensure!(
                    *on_disk_state == DatabaseIndexState::Enabled,
                    "Attempting to load index {} that is not backfilled yet {:?}",
                    index.name,
                    index,
                );
            } else {
                anyhow::bail!(
                    "Attempted to load index {} that isn't a database index {:?}",
                    index.name,
                    index,
                )
            }
        }

        // Read the table using an arbitrary index from the list
        let entries: Vec<_> = snapshot
            .index_scan(
                indexes[0].id().internal_id(),
                tablet_id,
                &Interval::all(),
                Order::Asc,
                usize::MAX,
            )
            .try_collect()
            .await?;
        let mut num_keys: usize = 0;
        let mut total_size: usize = 0;
        let mut index_maps = vec![DatabaseIndexMap::new_at(*snapshot.timestamp()); indexes.len()];
        for (_, rev) in entries.into_iter() {
            num_keys += 1;
            total_size += rev.value.value().size();
            let doc = PackedDocument::pack(&rev.value);
            // Calculate all the index keys. For simplicity we throw away the
            // index key that we read from persistence and recalculate it.
            for ((index, index_map), doc) in indexes
                .iter()
                .zip(&mut index_maps)
                .zip(iter::repeat_n(doc, indexes.len()))
            {
                let IndexConfig::Database { spec, .. } = &index.config else {
                    unreachable!()
                };
                let key = doc.index_key_owned(&spec.fields, snapshot.persistence().version());
                index_map.insert(key, rev.ts, doc);
            }
        }

        for (index, index_map) in indexes.iter().zip(index_maps) {
            self.in_memory_indexes
                .insert(index.id().internal_id(), index_map);
        }
        Ok((num_keys, total_size))
    }

    /// Insert enabled indexes for the given `tablet_id` with the provided,
    /// already-fetched documents.
    #[fastrace::trace]
    pub fn load_table(
        &mut self,
        index_registry: &IndexRegistry,
        tablet_id: TabletId,
        documents: Vec<(Timestamp, PackedDocument)>,
        snapshot_timestamp: Timestamp,
        persistence_version: PersistenceVersion,
    ) {
        for index_doc in index_registry.enabled_indexes_for_table(tablet_id) {
            let IndexConfig::Database {
                spec,
                on_disk_state,
                ..
            } = &index_doc.config
            else {
                continue;
            };
            assert_eq!(*on_disk_state, DatabaseIndexState::Enabled); // ensured by IndexRegistry
            let mut index_map = DatabaseIndexMap::new_at(snapshot_timestamp);
            for (ts, doc) in &documents {
                let key = doc.index_key_owned(&spec.fields, persistence_version);
                index_map.insert(key, *ts, doc.clone());
            }
            self.in_memory_indexes
                .insert(index_doc.id().internal_id(), index_map);
        }
    }

    pub fn update(
        &mut self,
        // NB: We assume that `index_registry` has already received this update.
        index_registry: &IndexRegistry,
        ts: Timestamp,
        deletion: Option<ResolvedDocument>,
        insertion: Option<ResolvedDocument>,
    ) -> Vec<DatabaseIndexUpdate> {
        if let (Some(old_document), None) = (&deletion, &insertion) {
            if old_document.id().tablet_id == index_registry.index_table() {
                // Drop the index from memory.
                self.in_memory_indexes
                    .remove(&old_document.id().internal_id());
            }
        }

        // Build up the list of updates to apply to all database indexes.
        let updates = index_registry.index_updates(deletion.as_ref(), insertion.as_ref());

        let mut packed = None;

        // Apply the updates to the subset of database indexes in memory.
        for update in &updates {
            match self.in_memory_indexes.get_mut(&update.index_id) {
                Some(key_set) => match &update.value {
                    DatabaseIndexValue::Deleted => {
                        key_set.remove(&update.key.to_bytes(), ts);
                    },
                    DatabaseIndexValue::NonClustered(doc_id) => {
                        // All in-memory indexes are clustered. Get the document
                        // from the update itself.
                        match insertion {
                            Some(ref doc) => {
                                assert_eq!(*doc_id, doc.id());
                                // reuse the PackedDocument if inserting into more than one index
                                let packed = packed
                                    .get_or_insert_with(|| PackedDocument::pack(doc))
                                    .clone();
                                key_set.insert(update.key.to_bytes(), ts, packed);
                            },
                            None => panic!("Unexpected index update: {:?}", update.value),
                        }
                    },
                },
                None => {},
            };
        }

        updates
    }

    pub fn in_memory_indexes_last_modified(&self) -> BTreeMap<IndexId, Timestamp> {
        self.in_memory_indexes
            .iter()
            .map(|(index_id, index_map)| (*index_id, index_map.last_modified))
            .collect()
    }

    pub fn range(
        &self,
        index_id: IndexId,
        interval: &Interval,
        order: Order,
    ) -> anyhow::Result<Option<Vec<(IndexKeyBytes, Timestamp, MemoryDocument)>>> {
        Ok(self
            .in_memory_indexes
            .get(&index_id)
            .map(|index_map| order.apply(index_map.range(interval)).collect()))
    }

    #[cfg(test)]
    pub(crate) fn in_memory_indexes(&self) -> OrdMap<IndexId, DatabaseIndexMap> {
        self.in_memory_indexes.clone()
    }
}

/// Implementor of `InMemoryIndexes` if no indexes are available in-memory.
pub struct NoInMemoryIndexes;
#[async_trait]
impl InMemoryIndexes for NoInMemoryIndexes {
    async fn range(
        &self,
        _index_id: IndexId,
        _interval: &Interval,
        _order: Order,
        _tablet_id: TabletId,
        _table_name: TableName,
    ) -> anyhow::Result<Option<Vec<(IndexKeyBytes, Timestamp, MemoryDocument)>>> {
        Ok(None)
    }
}

#[derive(Debug)]
struct IndexDocument {
    key: IndexKeyBytes,
    ts: Timestamp,
    document: MemoryDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, derive_more::Deref)]
struct ArcIndexDocument(Arc<IndexDocument>);

impl Borrow<[u8]> for ArcIndexDocument {
    fn borrow(&self) -> &[u8] {
        self.0.key.borrow()
    }
}

impl PartialEq for IndexDocument {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}
impl Eq for IndexDocument {}
impl PartialOrd for IndexDocument {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for IndexDocument {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseIndexMap {
    // We use OrdSet to provide efficient copy-on-write.
    // Note that all in-memory indexes are clustered.
    // N.B.: OrdMap/OrdSet are very sensitive to the size of keys and values (as
    // a map stores a minimum of 64 key-value pairs, even if empty) and likes to
    // clone them at will, so we store just a single Arc inside of it
    inner: OrdSet<ArcIndexDocument>,
    /// The timestamp of the last update to the index.
    last_modified: Timestamp,
}

impl DatabaseIndexMap {
    /// Construct an empty set.
    fn new_at(ts: Timestamp) -> Self {
        Self {
            inner: OrdSet::new(),
            last_modified: ts,
        }
    }

    /// The number of keys in the index.
    #[cfg(any(test, feature = "testing"))]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the index that are within `range`, in order.
    fn range(
        &self,
        interval: &Interval,
    ) -> impl DoubleEndedIterator<Item = (IndexKeyBytes, Timestamp, MemoryDocument)> + use<'_> {
        let _s = static_span!();
        self.inner
            .range(interval)
            .map(|e| (e.key.clone(), e.ts, e.document.clone()))
    }

    fn insert(&mut self, key: IndexKeyBytes, ts: Timestamp, document: PackedDocument) {
        self.inner.insert(ArcIndexDocument(Arc::new(IndexDocument {
            key,
            ts,
            document: MemoryDocument {
                packed_document: document,
                cached_system_document: SystemDocument::new(),
            },
        })));
        self.last_modified = cmp::max(self.last_modified, ts);
    }

    fn remove(&mut self, k: &IndexKeyBytes, ts: Timestamp) {
        self.inner.remove::<[u8]>(k);
        self.last_modified = cmp::max(self.last_modified, ts);
    }
}

/// Represents the state of the index at a certain snapshot of persistence.
#[derive(Clone)]
pub struct DatabaseIndexSnapshot {
    index_registry: ReadOnly<IndexRegistry>,
    in_memory_indexes: Arc<dyn InMemoryIndexes>,
    table_mapping: ReadOnly<TableMapping>,

    persistence: PersistenceSnapshot,

    // Cache results reads from the snapshot. The snapshot is immutable and thus
    // we don't have to do any invalidation.
    cache: DatabaseIndexSnapshotCache,
}

enum RangeFetchResult {
    /// The range was served from an in-memory table.
    /// This happens for tables that are statically configured to be kept in
    /// memory (e.g. `APP_TABLES_TO_LOAD_IN_MEMORY`).
    MemoryCached {
        documents: Vec<(IndexKeyBytes, Timestamp, MemoryDocument)>,
        next_cursor: CursorPosition,
    },
    /// The range was against a non-memory table.
    /// Some documents may still have been served from the
    /// `DatabaseIndexSnapshotCache`.
    NonCached {
        index_id: IndexId,
        cache_results: Vec<DatabaseIndexSnapshotCacheResult>,
    },
}

impl DatabaseIndexSnapshot {
    pub fn new(
        index_registry: IndexRegistry,
        in_memory_indexes: Arc<dyn InMemoryIndexes>,
        table_mapping: TableMapping,
        persistence_snapshot: PersistenceSnapshot,
    ) -> Self {
        Self {
            index_registry: ReadOnly::new(index_registry),
            in_memory_indexes,
            table_mapping: ReadOnly::new(table_mapping),
            persistence: persistence_snapshot,
            cache: DatabaseIndexSnapshotCache::new(),
        }
    }

    async fn start_range_fetch(
        &self,
        range_request: &RangeRequest,
    ) -> anyhow::Result<RangeFetchResult> {
        let index = match self.index_registry.require_enabled(
            &range_request.index_name,
            &range_request.printable_index_name,
        ) {
            Ok(index) => index,
            Err(e) => {
                // We verify that indexes are enabled at the transaction index layer,
                // so if an index is missing in our `index_registry` (which is from the
                // beginning of the transaction), then it must have been
                // inserted in this transaction. Return an empty result in this
                // condition for all indexes on all tables except the `_index` table, which must
                // always exist.
                if range_request.index_name.table() != &self.index_registry.index_table() {
                    return Ok(RangeFetchResult::MemoryCached {
                        documents: vec![],
                        next_cursor: CursorPosition::End,
                    });
                }
                anyhow::bail!(e);
            },
        };

        // Check that the index is indeed a database index.
        let IndexConfig::Database { on_disk_state, .. } = &index.metadata.config else {
            let err = index_not_a_database_index_error(
                &range_request
                    .index_name
                    .clone()
                    .map_table(&self.table_mapping.tablet_to_name())?,
            );
            anyhow::bail!(err);
        };
        anyhow::ensure!(
            *on_disk_state == DatabaseIndexState::Enabled,
            "Index returned from `require_enabled` but not enabled?"
        );

        // Now that we know it's a database index, serve it from the pinned
        // in-memory index if it's there.
        if let Some(range) = self
            .in_memory_indexes
            .range(
                index.id(),
                &range_request.interval,
                range_request.order,
                *range_request.index_name.table(),
                range_request.printable_index_name.table().clone(),
            )
            .await?
        {
            Self::log_start_range_fetch(
                range_request.printable_index_name.table(),
                1,
                0,
                range_request.max_size,
            );
            return Ok(RangeFetchResult::MemoryCached {
                documents: range,
                next_cursor: CursorPosition::End,
            });
        }

        // Next, try the transaction cache.
        let cache_results =
            self.cache
                .get(index.id(), &range_request.interval, range_request.order);
        let cache_miss_count = cache_results
            .iter()
            .filter(|r| matches!(r, DatabaseIndexSnapshotCacheResult::CacheMiss(_)))
            .count();
        Self::log_start_range_fetch(
            range_request.printable_index_name.table(),
            cache_results.len() - cache_miss_count,
            cache_miss_count,
            range_request.max_size,
        );
        Ok(RangeFetchResult::NonCached {
            index_id: index.id(),
            cache_results,
        })
    }

    fn log_start_range_fetch(
        _table_name: &TableName,
        _num_cached_ranges: usize,
        _num_cache_misses: usize,
        _prefetch_size: usize,
    ) {
        // TODO: This event is reporting to Honeycomb too often
        // Event::add_to_local_parent("start_range_fetch", || {
        //     let table_name = if table_name.is_system() {
        //         table_name.to_string()
        //     } else {
        //         format!("user_table")
        //     };
        //     let cached_ranges = num_cached_ranges.to_string();
        //     let cache_misses = num_cache_misses.to_string();
        //     let prefetch_size = prefetch_size.to_string();
        //     [
        //         (Cow::Borrowed("query.table"), Cow::Owned(table_name)),
        //         (
        //             Cow::Borrowed("query.cached_ranges"),
        //             Cow::Owned(cached_ranges),
        //         ),
        //         (
        //             Cow::Borrowed("query.cache_miss_ranges"),
        //             Cow::Owned(cache_misses),
        //         ),
        //         (
        //             Cow::Borrowed("query.prefetch_size"),
        //             Cow::Owned(prefetch_size),
        //         ),
        //     ]
        // });
    }

    /// Query the given indexes at the snapshot.
    ///
    /// Returns a separate result for each range request in the batch, in the
    /// same order as the input.
    pub async fn range_batch(
        &mut self,
        range_requests: &[&RangeRequest],
    ) -> Vec<
        anyhow::Result<(
            Vec<(IndexKeyBytes, Timestamp, LazyDocument)>,
            CursorPosition,
        )>,
    > {
        // Preallocate the result slots for each input request. This makes it
        // easier to concurrently populate the result vector.
        let mut results: Vec<_> = std::iter::repeat_with(|| {
            // dummy value
            Ok((vec![], CursorPosition::End))
        })
        .take(range_requests.len())
        .collect();

        // Concurrently run each range request, filling in its corresponding
        // slot in `results`.
        let stream = stream::iter(range_requests.iter().zip(&mut results[..]))
            .map(|(range_request, out)| async {
                let result = self.start_range_fetch(range_request).await;
                let (range_result, populate_cache) = match result {
                    Err(e) => (Err(e), None),
                    Ok(RangeFetchResult::MemoryCached {
                        documents,
                        next_cursor,
                    }) => (
                        Ok((
                            documents
                                .into_iter()
                                .map(|(key, ts, doc)| (key, ts, LazyDocument::Memory(doc)))
                                .collect(),
                            next_cursor,
                        )),
                        None,
                    ),
                    Ok(RangeFetchResult::NonCached {
                        index_id,
                        cache_results,
                    }) => {
                        let any_misses = cache_results.iter().any(|result| {
                            matches!(result, DatabaseIndexSnapshotCacheResult::CacheMiss(_))
                        });
                        let fut = Self::fetch_cache_misses(
                            self.persistence.clone(),
                            index_id,
                            (*range_request).clone(),
                            cache_results,
                        );
                        let fetch_result = if any_misses {
                            // Only spawn onto a new task if any database reads are required
                            try_join("fetch_cache_misses", fut).await
                        } else {
                            fut.await
                        };
                        // If we actually fetched anything, feed those results
                        // into `populate_cache_results` so we can update the
                        // DatabaseIndexSnapshotCache.
                        // We can't do that here because we can't mutate `self`
                        // during the concurrent phase of this future.
                        match fetch_result {
                            Err(e) => (Err(e), None),
                            Ok((fetch_result_vec, cache_miss_results, cursor)) => (
                                Ok((fetch_result_vec, cursor.clone())),
                                Some((*range_request, index_id, cache_miss_results, cursor)),
                            ),
                        }
                    },
                };
                *out = range_result;
                stream::iter(populate_cache)
            })
            .buffer_unordered(20)
            .flatten()
            .collect();
        let populate_cache_results: Vec<(
            &RangeRequest,
            IndexId,
            Vec<(Timestamp, PackedDocument)>,
            CursorPosition,
        )> = assert_send(stream).await; // works around https://github.com/rust-lang/rust/issues/102211

        for (range_request, index_id, cache_miss_results, cursor) in populate_cache_results {
            for (ts, doc) in cache_miss_results {
                // Populate all index point lookups that can result in the given
                // document.
                let index_keys = self
                    .index_registry
                    .index_keys(&doc)
                    .map(|(index, index_key)| (index.id(), index_key));
                self.cache.populate(index_keys, ts, doc.clone());
            }
            let (interval_read, _) = range_request
                .interval
                .split(cursor.clone(), range_request.order);
            // After all documents in an index interval have been
            // added to the cache with `populate_cache`, record the entire interval as
            // being populated.
            self.cache
                .record_interval_populated(index_id, interval_read);
        }

        results
    }

    async fn fetch_cache_misses(
        persistence: PersistenceSnapshot,
        index_id: IndexId,
        range_request: RangeRequest,
        cache_results: Vec<DatabaseIndexSnapshotCacheResult>,
    ) -> anyhow::Result<(
        Vec<(IndexKeyBytes, Timestamp, LazyDocument)>,
        Vec<(Timestamp, PackedDocument)>,
        CursorPosition,
    )> {
        let mut results = vec![];
        let mut cache_miss_results = vec![];
        let mut traced = false;
        for cache_result in cache_results {
            match cache_result {
                DatabaseIndexSnapshotCacheResult::Document(index_key, ts, document) => {
                    // Serve from cache.
                    log_transaction_cache_query(true);
                    results.push((index_key, ts, LazyDocument::Packed(document)));
                },
                DatabaseIndexSnapshotCacheResult::CacheMiss(interval) => {
                    log_transaction_cache_query(false);
                    if !traced {
                        LocalSpan::add_property(|| {
                            ("index", range_request.printable_index_name.to_string())
                        });
                        traced = true;
                    }
                    // Query persistence.
                    let mut stream = persistence.index_scan(
                        index_id,
                        *range_request.index_name.table(),
                        &interval,
                        range_request.order,
                        range_request.max_size,
                    );
                    while let Some((key, rev)) =
                        instrument!(b"Persistence::try_next", stream.try_next()).await?
                    {
                        cache_miss_results.push((rev.ts, PackedDocument::pack(&rev.value)));
                        results.push((key, rev.ts, rev.value.into()));
                        if results.len() >= range_request.max_size {
                            break;
                        }
                    }
                },
            }
            if results.len() >= range_request.max_size {
                let last_key = results
                    .last()
                    .expect("should be at least one result")
                    .0
                    .clone();
                return Ok((results, cache_miss_results, CursorPosition::After(last_key)));
            }
        }
        Ok((results, cache_miss_results, CursorPosition::End))
    }

    pub fn timestamp(&self) -> RepeatableTimestamp {
        self.persistence.timestamp()
    }
}

static MAX_TRANSACTION_CACHE_SIZE: LazyLock<usize> =
    LazyLock::new(|| *TRANSACTION_MAX_READ_SIZE_BYTES);

#[derive(Clone)]
struct DatabaseIndexSnapshotCache {
    /// Cache structure:
    /// Each document is stored, keyed by its index key for each index.
    /// Then for each index we have a set of intervals that are fully populated.
    /// The documents are populated first, then the intervals that contain them.
    ///
    /// For example, suppose a query does
    /// db.query('users').withIndex('by_age', q=>q.gt('age', 18)).collect()
    ///
    /// This will first populate `documents` with
    /// by_age -> <age:30, id:alice> -> (ts:100, { <alice document> })
    /// by_id -> <id:alice> -> (ts:100, { <alice document> })
    /// And it will populate the intervals:
    /// by_age -> <age:30, id:alice>
    /// by_id -> <id:alice>
    /// And it will do this for each document found.
    /// After the query is complete, we insert the final interval, which merges
    /// with the existing intervals:
    /// by_age -> (<age:18>, Unbounded)
    ///
    /// After the cache has been fully populated, `db.get`s which do point
    /// queries against by_id will be cached, and any indexed query against
    /// by_age that is a subset of (<age:18>, Unbounded) will be cached.
    documents: OrdMap<(IndexId, IndexKeyBytes), (Timestamp, PackedDocument)>,
    intervals: OrdMap<IndexId, IntervalSet>,
    cache_size: usize,
}

#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
enum DatabaseIndexSnapshotCacheResult {
    Document(IndexKeyBytes, Timestamp, PackedDocument),
    CacheMiss(Interval),
}

/// How many persistence index scans to do for a single interval.
/// If some results are cached, we can do multiple index scans to avoid
/// re-fetching the cached results. But we don't want to perform too many index
/// scans because there is fixed overhead for each one.
const MAX_CACHED_RANGES_PER_INTERVAL: usize = 3;

impl DatabaseIndexSnapshotCache {
    fn new() -> Self {
        Self {
            documents: OrdMap::new(),
            intervals: OrdMap::new(),
            cache_size: 0,
        }
    }

    fn populate(
        &mut self,
        index_keys: impl IntoIterator<
            Item = (InternalId, <PackedDocument as IndexedDocument>::IndexKey),
        >,
        ts: Timestamp,
        doc: PackedDocument,
    ) {
        if self.cache_size <= *MAX_TRANSACTION_CACHE_SIZE {
            for (index_id, index_key_bytes) in index_keys {
                let _s = static_span!();
                // Allow cache to exceed max size by one document, so we can detect that
                // the cache has maxed out.
                let interval = Interval::prefix(index_key_bytes.clone().into());
                self.documents
                    .insert((index_id, index_key_bytes), (ts, doc.clone()));
                self.intervals.entry(index_id).or_default().add(interval);
            }
            let result_size: usize = doc.value().size();
            self.cache_size += result_size;
        }
    }

    fn record_interval_populated(&mut self, index_id: IndexId, interval: Interval) {
        if self.cache_size <= *MAX_TRANSACTION_CACHE_SIZE {
            self.intervals.entry(index_id).or_default().add(interval);
        }
    }

    fn get(
        &self,
        index_id: IndexId,
        interval: &Interval,
        order: Order,
    ) -> Vec<DatabaseIndexSnapshotCacheResult> {
        let components = match self.intervals.get(&index_id) {
            None => {
                return vec![DatabaseIndexSnapshotCacheResult::CacheMiss(
                    interval.clone(),
                )]
            },
            Some(interval_set) => interval_set.split_interval_components(interval.as_ref()),
        };
        let mut results = vec![];
        let mut cache_hit_count = 0;
        for (in_set, component_interval) in components {
            // There are better ways to pick which cached intervals to use
            // (use the biggest ones, allow an extra if it's at the end),
            // but those are more complicated to implement so we can improve when a
            // use-case requires it. For now we pick the first cached ranges
            // until we hit `MAX_CACHED_RANGES_PER_INTERVAL`.
            if cache_hit_count >= MAX_CACHED_RANGES_PER_INTERVAL {
                results.push(DatabaseIndexSnapshotCacheResult::CacheMiss(Interval {
                    start: StartIncluded(component_interval.start.to_vec().into()),
                    end: interval.end.clone(),
                }));
                break;
            }
            if in_set {
                cache_hit_count += 1;
                let range = self
                    .documents
                    .range(
                        // TODO: `to_vec()` is not necessary
                        (index_id, IndexKeyBytes(component_interval.start.to_vec()))..,
                    )
                    .take_while(|&((index, key), _)| {
                        *index == index_id
                            && match &component_interval.end {
                                EndRef::Excluded(end) => key[..] < end[..],
                                EndRef::Unbounded => true,
                            }
                    });
                results.extend(range.map(|((_, index_key), (ts, doc))| {
                    DatabaseIndexSnapshotCacheResult::Document(index_key.clone(), *ts, doc.clone())
                }));
            } else {
                results.push(DatabaseIndexSnapshotCacheResult::CacheMiss(
                    component_interval.to_owned(),
                ));
            }
        }
        order.apply(results.into_iter()).collect_vec()
    }
}

pub fn index_not_a_database_index_error(name: &IndexName) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexNotADatabaseIndex",
        format!("Index {name} is not a database index"),
    )
}

#[cfg(test)]
mod cache_tests {
    use common::{
        bootstrap_model::index::database_index::IndexedFields,
        document::{
            CreationTime,
            PackedDocument,
            ResolvedDocument,
        },
        interval::{
            BinaryKey,
            End,
            Interval,
            StartIncluded,
        },
        query::Order,
        testing::TestIdGenerator,
        types::{
            PersistenceVersion,
            Timestamp,
        },
    };
    use value::{
        assert_obj,
        val,
        values_to_bytes,
    };

    use super::DatabaseIndexSnapshotCache;
    use crate::backend_in_memory_indexes::DatabaseIndexSnapshotCacheResult;

    #[test]
    fn cache_point_lookup() -> anyhow::Result<()> {
        let mut cache = DatabaseIndexSnapshotCache::new();
        let mut id_generator = TestIdGenerator::new();
        let index_id = id_generator.generate_internal();
        let id = id_generator.user_generate(&"users".parse()?);
        let doc = ResolvedDocument::new(id, CreationTime::ONE, assert_obj!())?;
        let index_key_bytes = doc
            .index_key(&IndexedFields::by_id(), PersistenceVersion::default())
            .to_bytes();
        let ts = Timestamp::must(100);
        let doc = PackedDocument::pack(&doc);
        cache.populate([(index_id, index_key_bytes.clone())], ts, doc.clone());

        let cached_result = cache.get(
            index_id,
            &Interval::prefix(values_to_bytes(&[Some(id.into())]).into()),
            Order::Asc,
        );
        assert_eq!(
            cached_result,
            vec![DatabaseIndexSnapshotCacheResult::Document(
                index_key_bytes,
                ts,
                doc
            )]
        );
        Ok(())
    }

    #[test]
    fn cache_full_interval() -> anyhow::Result<()> {
        let mut cache = DatabaseIndexSnapshotCache::new();
        let mut id_generator = TestIdGenerator::new();
        let index_id = id_generator.generate_internal();
        let id1 = id_generator.user_generate(&"users".parse()?);
        let doc1 = ResolvedDocument::new(id1, CreationTime::ONE, assert_obj!("age" => 30.0))?;
        let fields = vec!["age".parse()?];
        let index_key_bytes1 = doc1
            .index_key(&fields, PersistenceVersion::default())
            .to_bytes();
        let ts1 = Timestamp::must(100);
        let doc1 = PackedDocument::pack(&doc1);
        cache.populate([(index_id, index_key_bytes1.clone())], ts1, doc1.clone());

        let id2 = id_generator.user_generate(&"users".parse()?);
        let doc2 = ResolvedDocument::new(id2, CreationTime::ONE, assert_obj!("age" => 40.0))?;
        let index_key_bytes2 = doc2
            .index_key(&fields, PersistenceVersion::default())
            .to_bytes();
        let ts2 = Timestamp::must(150);
        let doc2 = PackedDocument::pack(&doc2);
        cache.populate([(index_id, index_key_bytes2.clone())], ts2, doc2.clone());

        let interval_gt_18 = Interval {
            start: StartIncluded(values_to_bytes(&[Some(val!(18.0))]).into()),
            end: End::Unbounded,
        };

        let d = DatabaseIndexSnapshotCacheResult::Document;
        let cache_miss = DatabaseIndexSnapshotCacheResult::CacheMiss;
        // All documents populated but we don't know what the queried interval is.
        assert_eq!(
            cache.get(index_id, &interval_gt_18, Order::Asc),
            vec![
                cache_miss(Interval {
                    start: interval_gt_18.start.clone(),
                    end: End::Excluded(index_key_bytes1.clone().into()),
                }),
                d(index_key_bytes1.clone(), ts1, doc1.clone()),
                cache_miss(Interval {
                    start: StartIncluded(
                        BinaryKey::from(index_key_bytes1.clone())
                            .increment()
                            .unwrap()
                    ),
                    end: End::Excluded(index_key_bytes2.clone().into()),
                }),
                d(index_key_bytes2.clone(), ts2, doc2.clone()),
                cache_miss(Interval {
                    start: StartIncluded(
                        BinaryKey::from(index_key_bytes2.clone())
                            .increment()
                            .unwrap()
                    ),
                    end: End::Unbounded,
                }),
            ]
        );
        // Impossible interval (e.g. age > 18 && age < 16) is always cached.
        let interval_impossible = Interval {
            start: StartIncluded(BinaryKey::min()),
            end: End::Excluded(BinaryKey::min()),
        };
        assert_eq!(
            cache.get(index_id, &interval_impossible, Order::Asc),
            vec![]
        );

        cache.record_interval_populated(index_id, interval_gt_18.clone());

        assert_eq!(
            cache.get(index_id, &interval_gt_18, Order::Asc),
            vec![
                d(index_key_bytes1.clone(), ts1, doc1.clone()),
                d(index_key_bytes2.clone(), ts2, doc2.clone()),
            ]
        );
        // Reverse order also cached.
        assert_eq!(
            cache.get(index_id, &interval_gt_18, Order::Desc),
            vec![
                d(index_key_bytes2.clone(), ts2, doc2.clone()),
                d(index_key_bytes1.clone(), ts1, doc1.clone()),
            ]
        );
        // Sub-interval also cached.
        let interval_gt_35 = Interval {
            start: StartIncluded(values_to_bytes(&[Some(val!(35.0))]).into()),
            end: End::Unbounded,
        };
        assert_eq!(
            cache.get(index_id, &interval_gt_35, Order::Asc),
            vec![d(index_key_bytes2.clone(), ts2, doc2.clone())]
        );
        // Empty sub-interval also cached.
        let interval_eq_35 = Interval::prefix(values_to_bytes(&[Some(val!(35.0))]).into());
        assert_eq!(cache.get(index_id, &interval_eq_35, Order::Asc), vec![]);
        // Super-interval partially cached.
        let interval_gt_16 = Interval {
            start: StartIncluded(values_to_bytes(&[Some(val!(16.0))]).into()),
            end: End::Unbounded,
        };
        assert_eq!(
            cache.get(index_id, &interval_gt_16, Order::Asc),
            vec![
                cache_miss(Interval {
                    start: interval_gt_16.start.clone(),
                    end: End::Excluded(values_to_bytes(&[Some(val!(18.0))]).into())
                }),
                d(index_key_bytes1.clone(), ts1, doc1.clone()),
                d(index_key_bytes2.clone(), ts2, doc2.clone()),
            ]
        );
        // Super-interval in reverse partially cached.
        assert_eq!(
            cache.get(index_id, &interval_gt_16, Order::Desc),
            vec![
                d(index_key_bytes2, ts2, doc2),
                d(index_key_bytes1, ts1, doc1),
                cache_miss(Interval {
                    start: interval_gt_16.start.clone(),
                    end: End::Excluded(values_to_bytes(&[Some(val!(18.0))]).into())
                }),
            ]
        );
        Ok(())
    }

    /// If the cache has a lot of points, we don't want to have a ton of small
    /// cache misses that require persistence queries. We restrict the number of
    /// persistence queries.
    #[test]
    fn sparse_cache() -> anyhow::Result<()> {
        let mut cache = DatabaseIndexSnapshotCache::new();
        let mut id_generator = TestIdGenerator::new();
        let index_id = id_generator.generate_internal();
        let ts = Timestamp::must(100);
        let mut make_doc = |age: f64| {
            let id = id_generator.user_generate(&"users".parse().unwrap());
            let doc =
                ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("age" => age)).unwrap();
            let fields = vec!["age".parse().unwrap()];
            let index_key_bytes = doc
                .index_key(&fields, PersistenceVersion::default())
                .to_bytes();
            let doc = PackedDocument::pack(&doc);
            cache.populate([(index_id, index_key_bytes.clone())], ts, doc.clone());
            (index_key_bytes, doc)
        };
        let (index_key1, doc1) = make_doc(30.0);
        let (index_key2, doc2) = make_doc(35.0);
        let (index_key3, doc3) = make_doc(40.0);
        let _ = make_doc(45.0);
        let _ = make_doc(50.0);
        let interval_gt_18 = Interval {
            start: StartIncluded(values_to_bytes(&[Some(val!(18.0))]).into()),
            end: End::Unbounded,
        };
        let d = DatabaseIndexSnapshotCacheResult::Document;
        let cache_miss = DatabaseIndexSnapshotCacheResult::CacheMiss;
        assert_eq!(
            cache.get(index_id, &interval_gt_18, Order::Asc),
            vec![
                cache_miss(Interval {
                    start: interval_gt_18.start.clone(),
                    end: End::Excluded(index_key1.clone().into()),
                }),
                d(index_key1.clone(), ts, doc1),
                cache_miss(Interval {
                    start: StartIncluded(BinaryKey::from(index_key1).increment().unwrap()),
                    end: End::Excluded(index_key2.clone().into()),
                }),
                d(index_key2.clone(), ts, doc2),
                cache_miss(Interval {
                    start: StartIncluded(BinaryKey::from(index_key2).increment().unwrap()),
                    end: End::Excluded(index_key3.clone().into()),
                }),
                d(index_key3.clone(), ts, doc3),
                cache_miss(Interval {
                    start: StartIncluded(BinaryKey::from(index_key3).increment().unwrap()),
                    end: End::Unbounded,
                }),
            ]
        );
        Ok(())
    }
}

pub type BatchKey = usize;

#[derive(Debug, Clone)]
pub struct RangeRequest {
    pub index_name: TabletIndexName,
    pub printable_index_name: IndexName,
    pub interval: Interval,
    pub order: Order,
    pub max_size: usize,
}

pub enum LazyDocument {
    Resolved(ResolvedDocument),
    Packed(PackedDocument),
    Memory(MemoryDocument),
}

/// A system document fetched from an in-memory index. This is internally
/// reference-counted and cheaply cloneable.
#[derive(Clone, Debug)]
pub struct MemoryDocument {
    pub packed_document: PackedDocument,
    pub cached_system_document: SystemDocument,
}
impl MemoryDocument {
    /// Parse and return the document. The same document must not be parsed
    /// twice with different types `T`.
    pub fn force<T: Send + Sync + 'static>(&self) -> anyhow::Result<Arc<ParsedDocument<T>>>
    where
        for<'a> &'a PackedDocument: ParseDocument<T>,
    {
        self.cached_system_document.force(&self.packed_document)
    }
}

/// Stores a lazily-populated, cached `ParsedDocument` of the right type for
/// this system document.
#[derive(Clone, Default, Debug)]
pub struct SystemDocument(Arc<OnceLock<Arc<dyn Any + Send + Sync>>>);

impl SystemDocument {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn force<T: Send + Sync + 'static>(
        &self,
        doc: &PackedDocument,
    ) -> anyhow::Result<Arc<ParsedDocument<T>>>
    where
        for<'a> &'a PackedDocument: ParseDocument<T>,
    {
        if let Ok(val) = self
            .0
            .get_or_try_init(|| doc.parse().map(|doc| Arc::new(doc) as Arc<_>))?
            .clone()
            .downcast()
        {
            return Ok(val);
        }
        // This is unexpected; it could happen if there is more than one
        // SystemTable type pointing at a table.
        let msg = format!(
            "doc {} already has a cached system document not of type {}",
            doc.id(),
            type_name::<T>()
        );
        if cfg!(debug_assertions) {
            panic!("{msg}");
        }
        tracing::warn!("{msg}");
        doc.parse().map(Arc::new)
    }
}

impl From<ResolvedDocument> for LazyDocument {
    fn from(value: ResolvedDocument) -> Self {
        Self::Resolved(value)
    }
}

impl LazyDocument {
    pub fn unpack(self) -> ResolvedDocument {
        match self {
            LazyDocument::Resolved(doc) => doc,
            LazyDocument::Packed(doc) => doc.unpack(),
            LazyDocument::Memory(doc) => doc.packed_document.unpack(),
        }
    }

    pub fn approximate_size(&self) -> usize {
        match self {
            LazyDocument::Resolved(doc) => doc.size(),
            // This is the size of the PackedValue representation, not the
            // proper size of the ConvexValue
            LazyDocument::Packed(doc) => doc.value().size(),
            LazyDocument::Memory(doc) => doc.packed_document.value().size(),
        }
    }
}
