use std::{
    cmp::Ordering,
    collections::BTreeMap,
    sync::{
        Arc,
        OnceLock,
    },
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexSpec,
            IndexedFields,
        },
        IndexConfig,
    },
    document::{
        DocumentUpdate,
        PackedDocument,
        ResolvedDocument,
    },
    index::{
        IndexKey,
        IndexKeyBytes,
    },
    interval::Interval,
    knobs::TRANSACTION_MAX_READ_SIZE_BYTES,
    query::{
        CursorPosition,
        InternalSearch,
        Order,
        SearchVersion,
    },
    runtime,
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        IndexId,
        IndexName,
        TabletIndexName,
        WriteTimestamp,
    },
};
use imbl::OrdMap;
use indexing::{
    backend_in_memory_indexes::{
        DatabaseIndexSnapshot,
        LazyDocument,
        RangeRequest,
    },
    index_registry::{
        Index,
        IndexRegistry,
    },
};
use search::{
    query::RevisionWithKeys,
    CandidateRevision,
    QueryResults,
    Searcher,
    TextIndexManager,
};
use storage::Storage;
use tokio::task;
use value::{
    DeveloperDocumentId,
    FieldPath,
};

use crate::{
    preloaded::PreloadedIndexRange,
    query::IndexRangeResponse,
    reads::TransactionReadSet,
    writes::PendingWrites,
    DEFAULT_PAGE_SIZE,
};

/// [`TransactionIndex`] is an index used by transactions.
/// It gets constructed from [`DatabaseIndexSnapshot`] and [`IndexRegistry`] at
/// a timestamp snapshot. It buffers the transaction pending index updates and
/// merges and overlays them on top of the snapshot to allow the transaction to
/// read its own writes.
#[derive(Clone)]
pub struct TransactionIndex {
    // Metadata about existing indexes with any changes to the index tables applied. Note that
    // those changes are stored separately in `database_index_updates` and `search_index_updates`
    // in their underlying database writes too.
    index_registry: IndexRegistry,
    // Weather the index registry has been updates since the beginning of the transaction.
    index_registry_updated: bool,

    // Database indexes combine a base index snapshot in persistence with pending updates applied
    // in-memory.
    database_index_snapshot: DatabaseIndexSnapshot,
    database_index_updates: OrdMap<IndexId, TransactionIndexMap>,

    // Similar to database indexes, text search indexes are implemented by applying pending updates
    // on top of the transaction base snapshot.
    text_index_snapshot: Arc<dyn TransactionTextSnapshot>,
    text_index_updates: OrdMap<IndexId, Vec<DocumentUpdate>>,
}

impl PendingWrites for TransactionIndex {}

impl TransactionIndex {
    pub fn new(
        index_registry: IndexRegistry,
        database_index_snapshot: DatabaseIndexSnapshot,
        text_index_snapshot: Arc<dyn TransactionTextSnapshot>,
    ) -> Self {
        Self {
            index_registry,
            index_registry_updated: false,
            database_index_snapshot,
            database_index_updates: OrdMap::new(),
            text_index_snapshot,
            text_index_updates: OrdMap::new(),
        }
    }

    pub fn index_registry(&self) -> &IndexRegistry {
        &self.index_registry
    }

    /// Range over a index including pending updates.
    /// `max_size` provides an estimate of the number of rows to be
    /// streamed from the database.
    /// The returned vecs may be larger or smaller than `max_size` depending on
    /// pending writes.
    pub(crate) async fn range_no_deps(
        &mut self,
        ranges: &[&RangeRequest],
    ) -> Vec<
        anyhow::Result<(
            Vec<(IndexKeyBytes, LazyDocument, WriteTimestamp)>,
            CursorPosition,
        )>,
    > {
        let snapshot_results = self.database_index_snapshot.range_batch(ranges).await;
        let batch_size = ranges.len();
        let mut results = Vec::with_capacity(batch_size);
        for (&range_request, snapshot_result) in ranges.iter().zip(snapshot_results) {
            let result = try {
                let (snapshot_result_vec, cursor) = snapshot_result?;
                let mut snapshot_it = snapshot_result_vec.into_iter();
                let index_registry = &self.index_registry;
                let database_index_updates = &self.database_index_updates;
                let pending_it = match index_registry.require_enabled(
                    &range_request.index_name,
                    &range_request.printable_index_name,
                ) {
                    Ok(index) => database_index_updates.get(&index.id()),
                    // Range queries on missing tables are allowed for system provided indexes.
                    Err(_) if range_request.index_name.is_by_id_or_creation_time() => None,
                    Err(e) => Err(e)?,
                }
                .map(|pending| pending.range(&range_request.interval))
                .into_iter()
                .flatten();
                let mut pending_it = range_request.order.apply(pending_it);

                let mut snapshot_next = snapshot_it.next();
                let mut pending_next = pending_it.next();
                let mut range_results = vec![];
                loop {
                    task::consume_budget().await;
                    match (snapshot_next, pending_next) {
                        (
                            Some((snapshot_key, snapshot_ts, snapshot_doc)),
                            Some((pending_key, maybe_pending_doc)),
                        ) => {
                            let cmp = match range_request.order {
                                Order::Asc => snapshot_key.cmp(&pending_key),
                                Order::Desc => pending_key.cmp(&snapshot_key),
                            };
                            match cmp {
                                Ordering::Less => {
                                    range_results.push((
                                        snapshot_key,
                                        snapshot_doc,
                                        WriteTimestamp::Committed(snapshot_ts),
                                    ));
                                    snapshot_next = snapshot_it.next();
                                    pending_next = Some((pending_key, maybe_pending_doc));
                                },
                                Ordering::Equal => {
                                    // The pending entry overwrites the snapshot one.
                                    if let Some(pending_doc) = maybe_pending_doc {
                                        range_results.push((
                                            pending_key,
                                            pending_doc.into(),
                                            WriteTimestamp::Pending,
                                        ));
                                    };
                                    snapshot_next = snapshot_it.next();
                                    pending_next = pending_it.next();
                                },
                                Ordering::Greater => {
                                    if let Some(pending_doc) = maybe_pending_doc {
                                        range_results.push((
                                            pending_key,
                                            pending_doc.into(),
                                            WriteTimestamp::Pending,
                                        ));
                                    };
                                    snapshot_next = Some((snapshot_key, snapshot_ts, snapshot_doc));
                                    pending_next = pending_it.next();
                                },
                            }
                        },
                        (Some((snapshot_key, snapshot_ts, snapshot_doc)), None) => {
                            range_results.push((
                                snapshot_key,
                                snapshot_doc,
                                WriteTimestamp::Committed(snapshot_ts),
                            ));
                            snapshot_next = snapshot_it.next();
                            pending_next = None;
                        },
                        (None, Some((pending_key, maybe_pending_doc))) => {
                            if let Some(pending_doc) = maybe_pending_doc {
                                range_results.push((
                                    pending_key,
                                    pending_doc.into(),
                                    WriteTimestamp::Pending,
                                ));
                            };
                            snapshot_next = None;
                            pending_next = pending_it.next();
                        },
                        (None, None) => break,
                    }
                }
                if !range_request.interval.contains_cursor(&cursor) {
                    Err(anyhow::anyhow!(
                        "query for {:?} not making progress",
                        range_request.interval
                    ))?;
                }
                (range_results, cursor)
            };
            results.push(result);
        }
        assert_eq!(results.len(), batch_size);
        results
    }

    #[fastrace::trace]
    pub async fn search(
        &mut self,
        reads: &mut TransactionReadSet,
        query: &InternalSearch,
        index_name: TabletIndexName,
        version: SearchVersion,
    ) -> anyhow::Result<Vec<(CandidateRevision, IndexKeyBytes)>> {
        // We do not allow modifying the index registry and performing a text search
        // in the same transaction. We could implement this by sending the index
        // updates in the search request, but there is no need to bother since we
        // don't yet have a use case of modifying an index metadata and performing
        // a text search in the same transaction.
        anyhow::ensure!(
            !self.index_registry_updated,
            "Text search and index registry update not allowed in the same transaction"
        );
        // HACK: instead of using `self.require_enabled` we access the
        // `IndexRegistry` directly to fetch index info, which skips recording the
        // read of `index.id()` into our `TransactionReadSet`.
        // This avoids invalidating the transaction based on the precise value
        // of the `TextIndexState`, as the transaction does not logically depend
        // on it, and therefore avoids invalidation after flushing or compacting
        // search indexes.
        // TODO(ENG-9324): this has the side effect of failing to invalidate
        // transactions if the search index is removed. In practice, that should
        // only happen as part of a push that would separately invalidate user
        // transactions anyway.
        let index = self
            .index_registry
            .require_enabled(&index_name, &query.printable_index_name()?)?;
        let empty = vec![];
        let pending_updates = self.text_index_updates.get(&index.id).unwrap_or(&empty);
        let results = self
            .text_index_snapshot
            .search(&index, query, version, pending_updates)
            .await?;

        // TODO: figure out if we want to charge database bandwidth for reading search
        // index metadata once search is no longer beta

        // Record the query results in the read set.
        reads.record_search(index_name.clone(), results.reads);

        Ok(results.revisions_with_keys)
    }

    /// Fetch a batch of index ranges. This method does not update the read set,
    /// since we might be fetching more documents than the caller actually needs
    /// due to filtering.
    ///
    /// Callers must call `record_indexed_directly` when consuming the results.
    pub async fn range_batch(
        &mut self,
        ranges: &[&RangeRequest],
    ) -> Vec<anyhow::Result<IndexRangeResponse>> {
        let batch_size = ranges.len();
        let mut results = Vec::with_capacity(batch_size);

        let fetch_results = self.range_no_deps(ranges).await;

        for (
            RangeRequest {
                index_name: _,
                printable_index_name: _,
                interval,
                order: _,
                max_size,
            },
            fetch_result,
        ) in ranges.iter().zip(fetch_results)
        {
            let result: anyhow::Result<_> = try {
                let (documents, fetch_cursor) = fetch_result?;
                let mut total_bytes = 0;
                let mut within_bytes_limit = true;
                let out: Vec<_> = documents
                    .into_iter()
                    .map(|(key, doc, ts)| (key, doc.unpack(), ts))
                    .take(*max_size)
                    .take_while(|(_, document, _)| {
                        within_bytes_limit = total_bytes < *TRANSACTION_MAX_READ_SIZE_BYTES;
                        // Allow the query to exceed the limit by one document so the query
                        // is guaranteed to make progress and probably fail.
                        // Note system document limits are different, so a single document
                        // can be larger than `TRANSACTION_MAX_READ_SIZE_BYTES`.
                        total_bytes += document.size();
                        within_bytes_limit
                    })
                    .collect();

                let cursor = if let Some((last_key, ..)) = out.last()
                    && (out.len() >= *max_size || !within_bytes_limit)
                {
                    // We hit an early termination condition within this page.
                    CursorPosition::After(last_key.clone())
                } else {
                    // Everything fetched will be returned, so the cursor
                    // of the page is the fetch cursor
                    fetch_cursor
                };
                if !interval.contains_cursor(&cursor) {
                    Err(anyhow::anyhow!(
                        "query for {interval:?} not making progress"
                    ))?;
                }
                IndexRangeResponse { page: out, cursor }
            };
            results.push(result);
        }
        assert_eq!(results.len(), batch_size);
        results
    }

    /// Returns the next page from the index range.
    /// NOTE: the caller must call reads.record_read_document for any
    /// documents yielded from the index scan and
    /// `reads.record_indexed_directly` for the interval actually read.
    /// Returns the remaining interval that was skipped because of max_size or
    /// transaction size limits.
    #[cfg(any(test, feature = "testing"))]
    pub async fn range(
        &mut self,
        range_request: RangeRequest,
    ) -> anyhow::Result<IndexRangeResponse> {
        let [result] = self
            .range_batch(&[&range_request])
            .await
            .try_into()
            .map_err(|_| anyhow::anyhow!("wrong number of results"))?;
        result
    }

    #[fastrace::trace]
    pub async fn preload_index_range(
        &mut self,
        reads: &mut TransactionReadSet,
        tablet_index_name: &TabletIndexName,
        printable_index_name: &IndexName,
        interval: &Interval,
    ) -> anyhow::Result<PreloadedIndexRange> {
        let index = self.require_enabled(reads, tablet_index_name, printable_index_name)?;
        let IndexConfig::Database {
            spec: DatabaseIndexSpec { ref fields, .. },
            ..
        } = index.metadata().config
        else {
            anyhow::bail!("{printable_index_name} isn't a database index");
        };
        let indexed_fields: Vec<FieldPath> = fields.clone().into();
        let indexed_field = indexed_fields[0].clone();
        anyhow::ensure!(indexed_fields.len() == 1);
        let mut remaining_interval = interval.clone();
        let mut preloaded = BTreeMap::new();
        while !remaining_interval.is_empty() {
            let [result] = self
                .range_no_deps(&[&RangeRequest {
                    index_name: tablet_index_name.clone(),
                    printable_index_name: printable_index_name.clone(),
                    interval: remaining_interval,
                    order: Order::Asc,
                    max_size: DEFAULT_PAGE_SIZE,
                }])
                .await
                .try_into()
                .map_err(|_| anyhow::anyhow!("wrong number of results"))?;
            let (documents, cursor) = result?;
            (_, remaining_interval) = interval.split(cursor, Order::Asc);
            for (_, document, _) in documents {
                let document = document.unpack();
                let key = document.value().0.get_path(&indexed_field).cloned();
                anyhow::ensure!(
                    preloaded.insert(key, document).is_none(),
                    "Index {printable_index_name:?} isn't unique",
                );
            }
        }
        // Since PreloadedIndexRange only permits looking up documents by the index
        // key, we don't need to record `interval` as a read dependency. Put another
        // way, even though we're reading all of the rows in `interval`, the layer
        // above is only allowed to do point queries against `index_name`.
        Ok(PreloadedIndexRange::new(
            printable_index_name.table().clone(),
            tablet_index_name.clone(),
            indexed_field,
            preloaded,
        ))
    }

    // TODO: Add precise error types to facilitate detecting which indexing errors
    // are the developer's fault or not.
    pub fn begin_update(
        &mut self,
        old_document: Option<ResolvedDocument>,
        new_document: Option<ResolvedDocument>,
    ) -> anyhow::Result<Update<'_>> {
        let mut registry = self.index_registry.clone();
        registry.update(old_document.as_ref(), new_document.as_ref())?;

        Ok(Update {
            index: self,
            deletion: old_document,
            insertion: new_document,
            registry,
        })
    }

    fn finish_update(
        &mut self,
        old_document: Option<ResolvedDocument>,
        new_document: Option<ResolvedDocument>,
    ) -> Vec<DatabaseIndexUpdate> {
        // Update the index registry first.
        let index_registry_updated = self
            .index_registry
            .apply_verified_update(old_document.as_ref(), new_document.as_ref());
        self.index_registry_updated |= index_registry_updated;

        // Then compute the index updates.
        let updates = self
            .index_registry
            .index_updates(old_document.as_ref(), new_document.as_ref());

        // Add the index updates to self.database_index_updates.
        for update in &updates {
            let new_value = match &update.value {
                DatabaseIndexValue::Deleted => None,
                DatabaseIndexValue::NonClustered(doc_id) => {
                    // The pending updates are clustered. Get the document
                    // from the update itself.
                    match new_document {
                        Some(ref doc) => {
                            assert_eq!(doc.id(), *doc_id);
                            Some(doc)
                        },
                        None => panic!("Unexpected index update: {:?}", update.value),
                    }
                },
            };
            self.database_index_updates
                .entry(update.index_id)
                .or_insert_with(TransactionIndexMap::new)
                .insert(update.key.to_bytes(), new_value);
        }

        // If we are updating a document, the old and new ids must be the same.
        let document_id = new_document
            .as_ref()
            .map(|d| d.id())
            .or(old_document.as_ref().map(|d| d.id()));
        if let Some(id) = document_id {
            // Add the update to all affected text search indexes.
            for index in self.index_registry.text_indexes_by_table(id.tablet_id) {
                self.text_index_updates
                    .entry(index.id)
                    .or_default()
                    .push(DocumentUpdate {
                        id,
                        old_document: old_document.clone(),
                        new_document: new_document.clone(),
                    });
            }
        }

        // Note that we do not update the vector index and we always read at the
        // base snapshot.

        updates
    }

    pub fn get_pending(
        &self,
        reads: &mut TransactionReadSet,
        index_name: &TabletIndexName,
    ) -> Option<&Index> {
        self._get(reads, || self.index_registry.get_pending(index_name))
    }

    pub fn get_enabled(
        &self,
        reads: &mut TransactionReadSet,
        index_name: &TabletIndexName,
    ) -> Option<&Index> {
        self._get(reads, || self.index_registry.get_enabled(index_name))
    }

    fn _get<'a>(
        &'a self,
        reads: &mut TransactionReadSet,
        getter: impl FnOnce() -> Option<&'a Index>,
    ) -> Option<&'a Index> {
        let result = getter();
        self.record_interval(reads, result);
        result
    }

    pub fn require_enabled(
        &self,
        reads: &mut TransactionReadSet,
        index_name: &TabletIndexName,
        printable_index_name: &IndexName,
    ) -> anyhow::Result<Index> {
        let result = self
            .index_registry
            .require_enabled(index_name, printable_index_name)?;
        self.record_interval(reads, Some(&result));
        Ok(result)
    }

    fn record_interval(&self, reads: &mut TransactionReadSet, index: Option<&Index>) {
        let index_table = self.index_registry.index_table();
        let index_table_number = self.index_registry.index_table_number();
        let interval = match index {
            // Note there is no _index.by_name index. In order for the
            // name->index mapping to depend only on index id, we rely
            // on index name being immutable.
            Some(index) => {
                let full_index_id = DeveloperDocumentId::new(index_table_number, index.id());
                let index_key = IndexKey::new(vec![], full_index_id);
                Interval::prefix(index_key.to_bytes().into())
            },
            // On a name lookup miss, depend on all indexes.
            None => Interval::all(),
        };
        reads.record_indexed_derived(
            TabletIndexName::by_id(index_table),
            IndexedFields::by_id(),
            interval,
        );
    }

    /// Returns the snapshot the transaction is based on ignoring any pending
    /// updates.
    pub fn base_snapshot(&self) -> &DatabaseIndexSnapshot {
        &self.database_index_snapshot
    }

    pub fn base_snapshot_mut(&mut self) -> &mut DatabaseIndexSnapshot {
        &mut self.database_index_snapshot
    }
}

#[derive(Debug, Clone)]
pub struct TransactionIndexMap {
    /// Unlike IndexMap we can simply use BTreeMap since the TransactionIndexMap
    /// does not get clones. The value needs to be Option<Document> since we
    /// need to distinguish between objects deleted within the transaction
    /// from objects that never existed.
    inner: BTreeMap<Vec<u8>, Option<PackedDocument>>,
}

impl TransactionIndexMap {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn range(
        &self,
        interval: &Interval,
    ) -> impl DoubleEndedIterator<Item = (IndexKeyBytes, Option<ResolvedDocument>)> + use<'_> {
        self.inner
            .range(interval)
            .map(|(k, v)| (IndexKeyBytes(k.clone()), v.as_ref().map(|v| v.unpack())))
    }

    pub fn insert(&mut self, k: IndexKeyBytes, v: Option<&ResolvedDocument>) {
        self.inner.insert(k.0, v.map(PackedDocument::pack));
    }
}

pub struct Update<'a> {
    index: &'a mut TransactionIndex,

    deletion: Option<ResolvedDocument>,
    insertion: Option<ResolvedDocument>,
    registry: IndexRegistry,
}

impl Update<'_> {
    pub fn apply(self) -> Vec<DatabaseIndexUpdate> {
        self.index.finish_update(self.deletion, self.insertion)
    }

    pub fn registry(&self) -> &IndexRegistry {
        &self.registry
    }
}

#[async_trait]
pub trait TransactionTextSnapshot: Send + Sync + 'static {
    // Search at the given snapshot after applying the given writes.
    async fn search(
        &self,
        index: &Index,
        search: &InternalSearch,
        version: SearchVersion,
        // Note that we have to send the writes since we maintain an extremely high
        // bar of determinism - we expect the exact same result regardless if you
        // perform a query from a mutation with some pending writes, or a query after the
        // writes have been committed to the database. The easiest way to achieve
        // this is to send all pending writes back to the backend. This should be fine
        // in practice since mutations with a lot of writes *and* a lot searches
        // should be rare.
        // As a potential future optimization, we could try to make the caller much
        // more coupled with the search algorithm and require it to send bm25 statistics
        // diff, top fuzzy search suggestions and other search specific properties derived
        // from the writes. Alternatively, we could only do subset of that and relax the
        // determinism requirement since we don't really need to have deterministic between
        // search calls in mutations and search calls in queries, and if anyone relies on
        // this they will get random differences due to parallel writes that alter the
        // statistics anyway.
        pending_updates: &Vec<DocumentUpdate>,
    ) -> anyhow::Result<QueryResults>;
}

#[derive(Clone)]
pub struct TextIndexManagerSnapshot {
    index_registry: IndexRegistry,
    text_indexes: TextIndexManager,

    searcher: Arc<dyn Searcher>,
    search_storage: Arc<OnceLock<Arc<dyn Storage>>>,
}

impl TextIndexManagerSnapshot {
    pub fn new(
        index_registry: IndexRegistry,
        text_indexes: TextIndexManager,
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<OnceLock<Arc<dyn Storage>>>,
    ) -> Self {
        Self {
            index_registry,
            text_indexes,
            searcher,
            search_storage,
        }
    }

    // Applies the writes to the base snapshot and returns the new snapshot.
    fn snapshot_with_updates(
        &self,
        pending_updates: &Vec<DocumentUpdate>,
    ) -> anyhow::Result<TextIndexManager> {
        let mut text_indexes = self.text_indexes.clone();
        for DocumentUpdate {
            id: _,
            old_document,
            new_document,
        } in pending_updates
        {
            text_indexes.update(
                &self.index_registry,
                old_document.as_ref(),
                new_document.as_ref(),
                WriteTimestamp::Pending,
            )?;
        }
        Ok(text_indexes)
    }

    fn search_storage(&self) -> Arc<dyn Storage> {
        self.search_storage
            .get()
            .expect("search_storage not initialized")
            .clone()
    }

    #[fastrace::trace]
    pub async fn search_with_compiled_query(
        &self,
        index: &Index,
        printable_index_name: &IndexName,
        query: pb::searchlight::TextQuery,
        pending_updates: &Vec<DocumentUpdate>,
    ) -> anyhow::Result<RevisionWithKeys> {
        let text_indexes_snapshot =
            runtime::block_in_place(|| self.snapshot_with_updates(pending_updates))?;
        text_indexes_snapshot
            .search_with_compiled_query(
                index,
                printable_index_name,
                query,
                self.searcher.clone(),
                self.search_storage(),
            )
            .await
    }
}

#[async_trait]
impl TransactionTextSnapshot for TextIndexManagerSnapshot {
    async fn search(
        &self,
        index: &Index,
        search: &InternalSearch,
        version: SearchVersion,
        pending_updates: &Vec<DocumentUpdate>,
    ) -> anyhow::Result<QueryResults> {
        let text_indexes_snapshot = self.snapshot_with_updates(pending_updates)?;
        text_indexes_snapshot
            .search(
                index,
                search,
                self.searcher.clone(),
                self.search_storage(),
                version,
            )
            .await
    }
}

pub struct SearchNotEnabled;

#[async_trait]
impl TransactionTextSnapshot for SearchNotEnabled {
    async fn search(
        &self,
        _index: &Index,
        _search: &InternalSearch,
        _version: SearchVersion,
        _pending_updates: &Vec<DocumentUpdate>,
    ) -> anyhow::Result<QueryResults> {
        anyhow::bail!("search not implemented in db-info")
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        str::FromStr,
        sync::{
            Arc,
            OnceLock,
        },
    };

    use common::{
        bootstrap_model::index::{
            database_index::IndexedFields,
            IndexMetadata,
            TabletIndexMetadata,
            INDEX_TABLE,
        },
        document::{
            CreationTime,
            PackedDocument,
            ResolvedDocument,
        },
        interval::Interval,
        persistence::{
            now_ts,
            ConflictStrategy,
            DocumentLogEntry,
            Persistence,
            PersistenceIndexEntry,
            RepeatablePersistence,
        },
        query::{
            CursorPosition,
            Order,
        },
        testing::{
            TestIdGenerator,
            TestPersistence,
        },
        types::{
            unchecked_repeatable_ts,
            IndexDescriptor,
            IndexName,
            PersistenceVersion,
            TableName,
            TabletIndexName,
            Timestamp,
            WriteTimestamp,
        },
        value::ResolvedDocumentId,
    };
    use indexing::{
        backend_in_memory_indexes::{
            BackendInMemoryIndexes,
            DatabaseIndexSnapshot,
            RangeRequest,
        },
        index_registry::IndexRegistry,
    };
    use itertools::Itertools;
    use runtime::prod::ProdRuntime;
    use search::{
        searcher::InProcessSearcher,
        TextIndexManager,
        TextIndexManagerState,
    };
    use storage::{
        LocalDirStorage,
        Storage,
    };
    use value::assert_obj;

    use super::TextIndexManagerSnapshot;
    use crate::{
        query::IndexRangeResponse,
        transaction_index::TransactionIndex,
        FollowerRetentionManager,
    };

    fn next_document_id(
        id_generator: &mut TestIdGenerator,
        table_name: &str,
    ) -> anyhow::Result<ResolvedDocumentId> {
        Ok(id_generator.user_generate(&TableName::from_str(table_name)?))
    }

    fn gen_index_document(
        id_generator: &mut TestIdGenerator,
        metadata: TabletIndexMetadata,
    ) -> anyhow::Result<ResolvedDocument> {
        let index_id = id_generator.system_generate(&INDEX_TABLE);
        ResolvedDocument::new(index_id, CreationTime::ONE, metadata.try_into()?)
    }

    async fn bootstrap_index(
        id_generator: &mut TestIdGenerator,
        mut indexes: Vec<TabletIndexMetadata>,
        persistence: RepeatablePersistence,
    ) -> anyhow::Result<(
        IndexRegistry,
        BackendInMemoryIndexes,
        TextIndexManager,
        BTreeMap<TabletIndexName, ResolvedDocumentId>,
    )> {
        let mut index_id_by_name = BTreeMap::new();
        let mut index_documents = Vec::new();

        let index_table = id_generator.system_table_id(&INDEX_TABLE).tablet_id;
        // Add the _index.by_id index.
        indexes.push(IndexMetadata::new_enabled(
            TabletIndexName::by_id(index_table),
            IndexedFields::by_id(),
        ));
        let ts = Timestamp::MIN;
        for metadata in indexes {
            let doc = gen_index_document(id_generator, metadata.clone())?;
            index_id_by_name.insert(metadata.name.clone(), doc.id());
            index_documents.push((ts, PackedDocument::pack(&doc)));
        }

        let index_registry = IndexRegistry::bootstrap(
            id_generator,
            index_documents.iter().map(|(_, d)| d.clone()),
            PersistenceVersion::default(),
        )?;
        let index = BackendInMemoryIndexes::bootstrap(&index_registry, index_documents, ts)?;

        let search =
            TextIndexManager::new(TextIndexManagerState::Bootstrapping, persistence.version());

        Ok((index_registry, index, search, index_id_by_name))
    }

    #[convex_macro::prod_rt_test]
    async fn test_transaction_index_missing_index(rt: ProdRuntime) -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();

        let persistence = Arc::new(TestPersistence::new());
        let retention_manager =
            Arc::new(FollowerRetentionManager::new(rt.clone(), persistence.clone()).await?);

        // Create a transactions with `by_name` index missing before the transaction
        // started.
        let rp = RepeatablePersistence::new(
            Arc::new(TestPersistence::new()),
            unchecked_repeatable_ts(Timestamp::must(1000)),
            retention_manager,
        );
        let ps = rp.read_snapshot(unchecked_repeatable_ts(Timestamp::must(1000)))?;

        let table_id = id_generator.user_table_id(&"messages".parse()?).tablet_id;
        let messages_by_name = TabletIndexName::new(table_id, IndexDescriptor::new("by_name")?)?;
        let printable_messages_by_name =
            IndexName::new("messages".parse()?, IndexDescriptor::new("by_name")?)?;
        let (index_registry, inner, search, _) = bootstrap_index(
            &mut id_generator,
            vec![IndexMetadata::new_enabled(
                TabletIndexName::by_id(table_id),
                IndexedFields::by_id(),
            )],
            rp,
        )
        .await?;

        let searcher = Arc::new(InProcessSearcher::new(rt.clone())?);
        let search_storage = Arc::new(LocalDirStorage::new(rt)?);
        let mut index = TransactionIndex::new(
            index_registry.clone(),
            DatabaseIndexSnapshot::new(
                index_registry.clone(),
                Arc::new(inner),
                id_generator.clone(),
                ps,
            ),
            Arc::new(TextIndexManagerSnapshot::new(
                index_registry.clone(),
                search,
                searcher.clone(),
                Arc::new(OnceLock::from(search_storage as Arc<dyn Storage>)),
            )),
        );

        // Query the missing index. It should return an error because index is missing.
        {
            let result = index
                .range(RangeRequest {
                    index_name: messages_by_name.clone(),
                    printable_index_name: printable_messages_by_name.clone(),
                    interval: Interval::all(),
                    order: Order::Asc,
                    max_size: 100,
                })
                .await;
            assert!(result.is_err());
            match result {
                Ok(_) => panic!("Should have failed!"),
                Err(ref err) => {
                    assert!(
                        format!("{err:?}").contains("Index messages.by_name not found."),
                        "Actual: {err:?}"
                    )
                },
            };
        }

        // Add the index. It should start returning errors since the index was not
        // backfilled at the snapshot.
        let by_name_metadata = IndexMetadata::new_backfilling(
            Timestamp::must(1000),
            messages_by_name.clone(),
            vec!["name".parse()?].try_into()?,
        );
        let by_name = gen_index_document(&mut id_generator, by_name_metadata)?;
        index.begin_update(None, Some(by_name))?.apply();

        let result = index
            .range(RangeRequest {
                index_name: messages_by_name,
                printable_index_name: printable_messages_by_name,
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 100,
            })
            .await;
        assert!(result.is_err());
        match result {
            Ok(_) => panic!("Should have failed!"),
            Err(ref err) => {
                assert!(
                    format!("{err:?}").contains("Index messages.by_name is currently backfilling"),
                    "Actual: {err:?}"
                )
            },
        };

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_transaction_index_missing_table(rt: ProdRuntime) -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_id = id_generator.user_table_id(&"messages".parse()?).tablet_id;
        let by_id = TabletIndexName::by_id(table_id);
        let printable_by_id = IndexName::by_id("messages".parse()?);
        let by_name = TabletIndexName::new(table_id, IndexDescriptor::new("by_name")?)?;
        let printable_by_name =
            IndexName::new("messages".parse()?, IndexDescriptor::new("by_name")?)?;

        // Create a transactions with table missing before the transaction started.
        let persistence = Arc::new(TestPersistence::new());
        let persistence_version = persistence.reader().version();
        let retention_manager =
            Arc::new(FollowerRetentionManager::new(rt.clone(), persistence.clone()).await?);
        let rp = RepeatablePersistence::new(
            persistence,
            unchecked_repeatable_ts(Timestamp::must(1000)),
            retention_manager,
        );
        let ps = rp.read_snapshot(unchecked_repeatable_ts(Timestamp::must(1000)))?;

        let (index_registry, inner, search, _) =
            bootstrap_index(&mut id_generator, vec![], rp).await?;

        let searcher = Arc::new(InProcessSearcher::new(rt.clone())?);
        let search_storage = Arc::new(LocalDirStorage::new(rt)?);
        let mut index = TransactionIndex::new(
            index_registry.clone(),
            DatabaseIndexSnapshot::new(
                index_registry.clone(),
                Arc::new(inner),
                id_generator.clone(),
                ps,
            ),
            Arc::new(TextIndexManagerSnapshot::new(
                index_registry.clone(),
                search,
                searcher.clone(),
                Arc::new(OnceLock::from(search_storage as Arc<dyn Storage>)),
            )),
        );

        // Query the missing table using table scan index. It should return no results.
        let IndexRangeResponse {
            page: results,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_id.clone(),
                printable_index_name: printable_by_id.clone(),
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 100,
            })
            .await?;
        assert!(matches!(cursor, CursorPosition::End));
        assert!(results.is_empty());

        // Query by any other index should return an error.
        {
            let result = index
                .range(RangeRequest {
                    index_name: by_name,
                    printable_index_name: printable_by_name,
                    interval: Interval::all(),
                    order: Order::Asc,
                    max_size: 100,
                })
                .await;
            assert!(result.is_err());
            match result {
                Ok(_) => panic!("Should have failed!"),
                Err(ref err) => {
                    assert!(format!("{err:?}").contains("Index messages.by_name not found."),)
                },
            };
        }

        // Add the table scan index. It should still give no results.
        let metadata = IndexMetadata::new_enabled(by_id.clone(), IndexedFields::by_id());
        let by_id_index = gen_index_document(&mut id_generator, metadata.clone())?;
        index.begin_update(None, Some(by_id_index))?.apply();

        let IndexRangeResponse {
            page: results,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_id.clone(),
                printable_index_name: printable_by_id.clone(),
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 100,
            })
            .await?;
        assert!(matches!(cursor, CursorPosition::End));
        assert!(results.is_empty());

        // Add a document and make sure we see it.
        let doc = ResolvedDocument::new(
            next_document_id(&mut id_generator, "messages")?,
            CreationTime::ONE,
            assert_obj!(
                "content" => "hello there!",
            ),
        )?;
        index.begin_update(None, Some(doc.clone()))?.apply();
        let IndexRangeResponse {
            page: result,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_id,
                printable_index_name: printable_by_id,
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 100,
            })
            .await?;
        assert_eq!(
            result,
            vec![(
                doc.index_key(&IndexedFields::by_id()[..], persistence_version)
                    .to_bytes(),
                doc,
                WriteTimestamp::Pending
            )],
        );
        assert!(matches!(cursor, CursorPosition::End));

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_transaction_index_merge(rt: ProdRuntime) -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let by_id_fields = vec![];
        let by_name_fields = vec!["name".parse()?];
        let now0 = now_ts(Timestamp::MIN, &rt)?;
        let ps = Arc::new(TestPersistence::new());
        let persistence_version = ps.reader().version();
        let retention_manager =
            Arc::new(FollowerRetentionManager::new(rt.clone(), ps.clone()).await?);
        let rp = RepeatablePersistence::new(
            ps.reader(),
            unchecked_repeatable_ts(now0),
            retention_manager.clone(),
        );
        let table: TableName = "users".parse()?;
        let table_id = id_generator.user_table_id(&table).tablet_id;
        let by_id = TabletIndexName::by_id(table_id);
        let printable_by_id = IndexName::by_id(table.clone());
        let by_name = TabletIndexName::new(table_id, IndexDescriptor::new("by_name")?)?;
        let printable_by_name = IndexName::new(table.clone(), IndexDescriptor::new("by_name")?)?;
        let (mut index_registry, mut index, search, _index_ids) = bootstrap_index(
            &mut id_generator,
            vec![
                IndexMetadata::new_enabled(by_id.clone(), by_id_fields.clone().try_into()?),
                IndexMetadata::new_enabled(by_name.clone(), by_name_fields.clone().try_into()?),
            ],
            rp,
        )
        .await?;

        async fn add(
            index_registry: &mut IndexRegistry,
            index: &mut BackendInMemoryIndexes,
            ps: &TestPersistence,
            ts: Timestamp,
            doc: ResolvedDocument,
        ) -> anyhow::Result<()> {
            index_registry.update(None, Some(&doc))?;
            let index_updates = index.update(index_registry, ts, None, Some(doc.clone()));
            ps.write(
                &[(DocumentLogEntry {
                    ts,
                    id: doc.id_with_table_id(),
                    value: Some(doc.clone()),
                    prev_ts: None,
                })],
                &index_updates
                    .into_iter()
                    .map(|u| PersistenceIndexEntry::from_index_update(ts, &u))
                    .collect_vec(),
                ConflictStrategy::Error,
            )
            .await?;
            Ok(())
        }

        // Add "Alice", "Bob" and "Zack"
        let alice = ResolvedDocument::new(
            next_document_id(&mut id_generator, "users")?,
            CreationTime::ONE,
            assert_obj!(
                "name" => "alice",
            ),
        )?;
        let now1 = now0.succ()?;
        add(&mut index_registry, &mut index, &ps, now1, alice.clone()).await?;
        let bob = ResolvedDocument::new(
            next_document_id(&mut id_generator, "users")?,
            CreationTime::ONE,
            assert_obj!(
                "name" => "bob",
            ),
        )?;
        let now2 = now1.succ()?;
        add(&mut index_registry, &mut index, &ps, now2, bob.clone()).await?;
        let zack = ResolvedDocument::new(
            next_document_id(&mut id_generator, "users")?,
            CreationTime::ONE,
            assert_obj!(
                "name" => "zack",
            ),
        )?;
        let now3 = now2.succ()?;
        add(&mut index_registry, &mut index, &ps, now3, zack.clone()).await?;

        id_generator.write_tables(ps.clone()).await?;

        let now4 = now3.succ()?;
        // Start a transaction, add "David" and delete "Bob"
        let ps = RepeatablePersistence::new(ps, unchecked_repeatable_ts(now4), retention_manager)
            .read_snapshot(unchecked_repeatable_ts(now4))?;

        let searcher = Arc::new(InProcessSearcher::new(rt.clone())?);
        let search_storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let mut index = TransactionIndex::new(
            index_registry.clone(),
            DatabaseIndexSnapshot::new(
                index_registry.clone(),
                Arc::new(index),
                id_generator.clone(),
                ps,
            ),
            Arc::new(TextIndexManagerSnapshot::new(
                index_registry.clone(),
                search,
                searcher.clone(),
                Arc::new(OnceLock::from(search_storage as Arc<dyn Storage>)),
            )),
        );
        let david = ResolvedDocument::new(
            next_document_id(&mut id_generator, "users")?,
            CreationTime::ONE,
            assert_obj!("name" => "david"),
        )?;
        index.begin_update(None, Some(david.clone()))?.apply();
        index.begin_update(Some(bob), None)?.apply();

        // Query by id
        let IndexRangeResponse {
            page: results,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_id.clone(),
                printable_index_name: printable_by_id,
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 100,
            })
            .await?;
        assert!(matches!(cursor, CursorPosition::End));
        assert_eq!(
            results,
            vec![
                (
                    alice
                        .index_key(&by_id_fields[..], persistence_version)
                        .to_bytes(),
                    alice.clone(),
                    WriteTimestamp::Committed(now1)
                ),
                (
                    zack.index_key(&by_id_fields[..], persistence_version)
                        .to_bytes(),
                    zack.clone(),
                    WriteTimestamp::Committed(now3)
                ),
                (
                    david
                        .index_key(&by_id_fields[..], persistence_version)
                        .to_bytes(),
                    david.clone(),
                    WriteTimestamp::Pending
                ),
            ]
        );
        // Query by name in ascending order
        let IndexRangeResponse {
            page: results,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_name.clone(),
                printable_index_name: printable_by_name.clone(),
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 100,
            })
            .await?;
        assert!(matches!(cursor, CursorPosition::End));
        assert_eq!(
            results,
            vec![
                (
                    alice
                        .index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    alice.clone(),
                    WriteTimestamp::Committed(now1)
                ),
                (
                    david
                        .index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    david.clone(),
                    WriteTimestamp::Pending
                ),
                (
                    zack.index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    zack.clone(),
                    WriteTimestamp::Committed(now3)
                ),
            ]
        );
        // Query by name in ascending order with limit=2.
        // Returned cursor should be After("david").
        let IndexRangeResponse {
            page: results,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_name.clone(),
                printable_index_name: printable_by_name.clone(),
                interval: Interval::all(),
                order: Order::Asc,
                max_size: 2,
            })
            .await?;
        assert_eq!(
            cursor,
            CursorPosition::After(
                david
                    .index_key(&by_name_fields[..], persistence_version)
                    .to_bytes()
            )
        );
        assert_eq!(
            results,
            vec![
                (
                    alice
                        .index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    alice.clone(),
                    WriteTimestamp::Committed(now1)
                ),
                (
                    david
                        .index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    david.clone(),
                    WriteTimestamp::Pending
                ),
            ]
        );

        // Query by name in descending order
        let IndexRangeResponse {
            page: result,
            cursor,
        } = index
            .range(RangeRequest {
                index_name: by_name,
                printable_index_name: printable_by_name,
                interval: Interval::all(),
                order: Order::Desc,
                max_size: 100,
            })
            .await?;
        assert!(matches!(cursor, CursorPosition::End));
        assert_eq!(
            result,
            vec![
                (
                    zack.index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    zack,
                    WriteTimestamp::Committed(now3)
                ),
                (
                    david
                        .index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    david,
                    WriteTimestamp::Pending
                ),
                (
                    alice
                        .index_key(&by_name_fields[..], persistence_version)
                        .to_bytes(),
                    alice,
                    WriteTimestamp::Committed(now1)
                ),
            ]
        );

        Ok(())
    }
}
