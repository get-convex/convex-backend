use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Deref,
    sync::Arc,
};

use anyhow::Context;
use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document::ResolvedDocument,
    index::{
        IndexKey,
        IndexKeyBytes,
    },
    interval::Interval,
    knobs::DOCUMENTS_IN_MEMORY,
    persistence::{
        new_static_repeatable_recent,
        DocumentLogEntry,
        DocumentPrevTsQuery,
        LatestDocument,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    query::{
        CursorPosition,
        Order,
    },
    runtime::Runtime,
    try_chunks::TryChunksExt,
    types::{
        IndexId,
        RepeatableTimestamp,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use futures::{
    pin_mut,
    stream,
    Stream,
    StreamExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use value::{
    InternalDocumentId,
    InternalId,
    TabletId,
};

/// A cursor for use while scanning a table by ID.
///
/// The key is the last element processed thus far.
#[derive(Clone, Debug)]
pub struct TableScanCursor {
    pub index_key: Option<CursorPosition>,
}

impl TableScanCursor {
    pub fn interval(&self) -> Interval {
        match &self.index_key {
            Some(index_key) => {
                let (_, remaining) = Interval::all().split(index_key.clone(), Order::Asc);
                remaining
            },
            None => Interval::all(),
        }
    }

    pub fn advance(&mut self, index_key: CursorPosition) -> anyhow::Result<()> {
        if let Some(existing_key) = &self.index_key {
            anyhow::ensure!(index_key > existing_key);
        }
        self.index_key = Some(index_key);
        Ok(())
    }
}

fn cursor_has_walked(cursor: Option<&CursorPosition>, key: &IndexKeyBytes) -> bool {
    match cursor {
        None => false,
        Some(CursorPosition::End) => true,
        Some(CursorPosition::After(cursor)) => key <= cursor,
    }
}

pub struct TableIterator<RT: Runtime> {
    inner: TableIteratorInner<RT>,
}

impl<RT: Runtime> TableIterator<RT> {
    pub fn new(
        runtime: RT,
        snapshot_ts: RepeatableTimestamp,
        persistence: Arc<dyn PersistenceReader>,
        retention_validator: Arc<dyn RetentionValidator>,
        page_size: usize,
    ) -> Self {
        Self {
            inner: TableIteratorInner {
                runtime,
                persistence,
                retention_validator,
                page_size,
                snapshot_ts,
            },
        }
    }

    /// Create a `MultiTableIterator`, which can iterate multiple tables at the
    /// same snapshot timestamp. This is more efficient than creating a separate
    /// `TableIterator` for each table since each table can share the work of
    /// iterating the document log.
    ///
    /// The iterator will only be able to visit those tables passed to `multi`.
    /// Trying to visit a table that wasn't initially specified will error.
    ///
    /// The iterator keeps some state in memory for each of the `tables`
    /// provided. To reduce memory usage, you can call
    /// [`MultiTableIterator::unregister_table`] if you know that a given table
    /// will not be iterated again.
    ///
    /// Example:
    /// ```no_run
    /// # async fn iterate_example<RT: common::runtime::Runtime>(
    /// #     db: &database::Database<RT>,
    /// #     [table1, table2, table3]: [value::TabletId; 3],
    /// #     by_id: common::types::IndexId,
    /// #     ts: common::types::RepeatableTimestamp,
    /// # ) -> anyhow::Result<()> {
    /// # use futures::stream::TryStreamExt;
    /// # use std::pin::pin;
    /// let tables = vec![table1, table2, table3];
    /// let page_size = 100;
    /// let mut iterator = db.table_iterator(ts, page_size).multi(tables.clone());
    /// for tablet_id in tables {
    ///     iterator.stream_documents_in_table(tablet_id, by_id, None).try_for_each(async |doc| {
    ///         // handle doc
    ///         Ok(())
    ///     }).await?;
    ///     iterator.unregister_table(tablet_id);
    ///     // afterward, `iterator` can no longer visit `tablet_id`
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn multi(self, tables: Vec<TabletId>) -> MultiTableIterator<RT> {
        MultiTableIterator {
            end_ts: self.inner.snapshot_ts,
            buffered_documents: tables
                .into_iter()
                .map(|tablet_id| (tablet_id, BTreeMap::new()))
                .collect(),
            inner: self.inner,
        }
    }

    #[try_stream(ok = LatestDocument, error = anyhow::Error)]
    pub async fn stream_documents_in_table(
        self,
        tablet_id: TabletId,
        by_id: IndexId,
        cursor: Option<ResolvedDocumentId>,
    ) {
        let mut iterator = self.multi(vec![]);
        let stream = iterator.stream_documents_in_table(tablet_id, by_id, cursor);
        pin_mut!(stream);
        while let Some(rev) = stream.try_next().await? {
            yield rev;
        }
    }

    #[try_stream(ok = (IndexKeyBytes, LatestDocument), error = anyhow::Error)]
    pub async fn stream_documents_in_table_by_index(
        self,
        tablet_id: TabletId,
        index_id: IndexId,
        indexed_fields: IndexedFields,
        cursor: Option<CursorPosition>,
    ) {
        let mut iterator = self.multi(vec![]);
        let stream = iterator.stream_documents_in_table_by_index(
            tablet_id,
            index_id,
            indexed_fields,
            cursor,
        );
        pin_mut!(stream);
        while let Some(rev) = stream.try_next().await? {
            yield rev;
        }
    }
}

struct TableIteratorInner<RT: Runtime> {
    runtime: RT,
    persistence: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    page_size: usize,
    snapshot_ts: RepeatableTimestamp,
}
pub struct MultiTableIterator<RT: Runtime> {
    inner: TableIteratorInner<RT>,
    end_ts: RepeatableTimestamp,
    // Buffered document metadata between `snapshot_ts` and `end_ts`.
    // This is useful because there is no way to read the document log for just
    // a single table.
    buffered_documents: BufferedDocumentMetadata,
}

// For each table that we are interested in, stores a map containing every
// observed id in that table and its first prev_ts (or None if the first row had
// no prev_ts)
type BufferedDocumentMetadata = BTreeMap<TabletId, BTreeMap<InternalId, Option<PrevTs>>>;

#[derive(Copy, Clone, Debug)]
struct PrevTs {
    prev_ts: Timestamp,
}

impl<RT: Runtime> MultiTableIterator<RT> {
    /// Signal that the given `tablet_id` will not be iterated in the future.
    /// The `tablet_id` must have been provided during this iterator's
    /// construction.
    ///
    /// Calling this is an optimization to reduce memory usage by dropping the
    /// list of changed documents.
    pub fn unregister_table(&mut self, tablet_id: TabletId) -> anyhow::Result<()> {
        self.buffered_documents
            .remove(&tablet_id)
            .context("unregister_table on an unknown table")?;
        Ok(())
    }

    #[try_stream(ok = LatestDocument, error = anyhow::Error)]
    pub async fn stream_documents_in_table(
        &mut self,
        tablet_id: TabletId,
        by_id: IndexId,
        cursor: Option<ResolvedDocumentId>,
    ) {
        let stream = self.stream_documents_in_table_by_index(
            tablet_id,
            by_id,
            IndexedFields::by_id(),
            cursor.map(|id| CursorPosition::After(IndexKey::new(vec![], id.into()).to_bytes())),
        );
        pin_mut!(stream);
        while let Some((_, rev)) = stream.try_next().await? {
            yield rev;
        }
    }

    /// Algorithm overview:
    /// Walk a table ordered by an index at snapshot_ts which may be outside
    /// retention, so you can't walk the index directly.
    ///
    /// 1. Walk the index at recent, increasing snapshots, and
    /// collect documents that have not changed since snapshot_ts.
    /// 2. Also walk the document log between snapshot_ts and the increasing
    /// snapshots, and for each changed document look at its revision as of
    /// snapshot_ts.
    /// 3. Merge the results.
    ///
    /// Why it works:
    /// Consider a document that exists in the index at snapshot_ts.
    /// Either it has changed since snapshot_ts, in which case (2) will find
    /// it, or it has not, in which case (1) will find it.
    ///
    /// (2) is implemented by recording the *first* prev_ts (if any) for each
    /// document id encountered in the document log. That prev_ts points to the
    /// document that belongs to the snapshot (or, if null, indicates that the
    /// document is new and doesn't belong in the snapshot).
    ///
    /// The document log can't be filtered by table, so `MultiTableIterator`
    /// additionally remembers this metadata for every table that the caller is
    /// interested in. This allows each subsequent table iteration to continue
    /// where the previous walk left off.
    #[try_stream(ok = (IndexKeyBytes, LatestDocument), error = anyhow::Error)]
    pub async fn stream_documents_in_table_by_index(
        &mut self,
        tablet_id: TabletId,
        index_id: IndexId,
        indexed_fields: IndexedFields,
        cursor: Option<CursorPosition>,
    ) {
        let mut cursor = TableScanCursor { index_key: cursor };

        // skipped_keys are keys of documents that were modified after
        // snapshot_ts but before the index key was walked over.
        // Such documents must be
        // (a) modified/deleted after snapshot_ts but before new_end_ts.
        // (b) have key > cursor.
        // We insert skipped documents into future pages of the index walk when we get
        // to them.
        let mut skipped_keys = IterationDocuments::default();
        // observed_ids is the set of document IDs in the table between
        // `(snapshot_ts, end_ts]`, and also corresponds to all the documents
        // that may have been inserted into `skipped_keys` (if present in the
        // snapshot & not already walked)
        let mut observed_ids: BTreeSet<InternalId> = BTreeSet::new();

        if let Some(buffered_documents) = self.buffered_documents.get(&tablet_id) {
            // We've already walked the document log and stored some document
            // timestamps for this table. Load those documents and prepopulate
            // `skipped_keys`.
            observed_ids.extend(buffered_documents.keys().copied());
            let mut revisions_at_snapshot = self
                .inner
                .load_revisions_at_snapshot_ts(stream::iter(buffered_documents.iter().filter_map(
                    |(&id, &timestamps)| {
                        timestamps.map(|ts| Ok((InternalDocumentId::new(tablet_id, id), ts)))
                    },
                )))
                .boxed(); // `boxed()` instead of `pin_mut!` works around https://github.com/rust-lang/rust/issues/96865
            let persistence_version = self.inner.persistence.version();
            while let Some(rev) = revisions_at_snapshot.try_next().await? {
                let index_key = rev
                    .value
                    .index_key(&indexed_fields, persistence_version)
                    .to_bytes();
                skipped_keys.insert(index_key, rev.ts, rev.value, rev.prev_ts);
            }
        } else {
            // As a special case, the very first table visited by a
            // `MultiTableIterator` is allowed to be any table, even if it
            // wasn't specified. This is just to support the single-table
            // iteration methods on `TableIterator`.
            anyhow::ensure!(
                self.inner.snapshot_ts == self.end_ts,
                "this MultiTableIterator has already advanced from {snapshot_ts} to {end_ts}, but \
                 it has no buffered documents for table {tablet_id}",
                snapshot_ts = self.inner.snapshot_ts,
                end_ts = self.end_ts
            );
        }

        loop {
            let pause_client = self.inner.runtime.pause_client();
            pause_client.wait("before_index_page").await;
            let page_start = cursor.index_key.clone();
            let (page, new_end_ts) = self
                .inner
                .fetch_page(index_id, tablet_id, &mut cursor)
                .await?;
            anyhow::ensure!(*new_end_ts >= self.end_ts);
            let page_end = cursor
                .index_key
                .as_ref()
                .context("cursor after page should not be empty")?;
            // Filter out rows from the index scan that were modified after
            // snapshot_ts. Such documents will be found when walking the
            // documents log to generate skipped_keys.
            let page: BTreeMap<_, _> = page
                .into_iter()
                .filter(|(_, rev)| rev.ts <= *self.inner.snapshot_ts)
                .map(|(index_key, LatestDocument { ts, value, prev_ts })| {
                    (index_key, (ts, IterationDocument::Full { value, prev_ts }))
                })
                .collect();

            // 2. Find any keys for documents that were skipped by this
            // page or will be skipped by future pages.
            // These documents are returned with index keys and revisions as
            // they existed at snapshot_ts.
            self.inner
                .fetch_skipped_keys(
                    tablet_id,
                    &indexed_fields,
                    page_start.as_ref(),
                    *self.end_ts,
                    new_end_ts,
                    &mut skipped_keys,
                    &mut observed_ids,
                    &mut self.buffered_documents,
                )
                .await?;
            if let Some((first_skipped_key, _)) = skipped_keys.iter().next() {
                // Check all skipped ids are after the old cursor,
                // which ensures the yielded output is in index key order.
                anyhow::ensure!(!cursor_has_walked(page_start.as_ref(), first_skipped_key));
            }
            self.end_ts = new_end_ts;
            // Extract the documents from skipped_keys that should be returned in
            // the current page.
            let page_skipped_keys = {
                let mut page_skipped_keys = BTreeMap::new();
                while let Some(first_skipped_key) = skipped_keys.keys().next()
                    && cursor_has_walked(Some(page_end), first_skipped_key)
                {
                    let (key, value) = skipped_keys
                        .remove(&first_skipped_key.clone())
                        .context("skipped_keys should be nonempty")?;
                    page_skipped_keys.insert(key, value);
                }
                page_skipped_keys
            };
            // Merge index walk and skipped keys into BTreeMap, which sorts by index key.
            let merged_page =
                IterationDocuments::new(page.into_iter().chain(page_skipped_keys).collect());

            // Sanity check output.
            let all_ids: BTreeSet<_> = merged_page
                .values()
                .map(|(_, doc)| doc.internal_id())
                .collect();
            anyhow::ensure!(
                all_ids.len() == merged_page.len(),
                "duplicate id in table iterator {merged_page:?}"
            );
            anyhow::ensure!(
                merged_page
                    .values()
                    .all(|(ts, _)| *ts <= *self.inner.snapshot_ts),
                "document after snapshot in table iterator {merged_page:?}"
            );
            anyhow::ensure!(
                merged_page.keys().all(|key| {
                    !cursor_has_walked(page_start.as_ref(), key)
                        && cursor_has_walked(Some(page_end), key)
                }),
                "document outside page in table iterator {merged_page:?}"
            );

            let mut merged_page_docs = self.inner.reload_revisions_at_snapshot_ts(merged_page);
            while let Some((key, rev)) = merged_page_docs.try_next().await? {
                // The caller will likely consume the documents in a CPU-intensive loop,
                // and `merged_page_docs.try_next().await` will often be Ready
                // immediately, so it won't yield.
                // Make sure we yield to not starve other tokio tasks.
                tokio::task::consume_budget().await;

                yield (key, rev);
            }
            if matches!(page_end, CursorPosition::End) {
                // If we are done, all skipped_keys would be put in this final page.
                anyhow::ensure!(skipped_keys.is_empty());
                break;
            }
        }
    }
}

impl<RT: Runtime> TableIteratorInner<RT> {
    /// A document may be skipped if:
    /// 1. it is in the correct table
    /// 2. at the snapshot, it had a key higher than what we've walked so far
    /// 3. it was modified after the snapshot but before we walked its key
    /// range.
    #[fastrace::trace]
    async fn fetch_skipped_keys(
        &self,
        tablet_id: TabletId,
        indexed_fields: &IndexedFields,
        lower_bound: Option<&CursorPosition>,
        start_ts: Timestamp,
        end_ts: RepeatableTimestamp,
        output: &mut IterationDocuments,
        observed_ids: &mut BTreeSet<InternalId>,
        buffered_documents: &mut BufferedDocumentMetadata,
    ) -> anyhow::Result<()> {
        let reader = self.persistence.clone();
        let persistence_version = reader.version();
        let skipped_revs = self.walk_document_log(
            tablet_id,
            start_ts,
            end_ts,
            observed_ids,
            buffered_documents,
        );
        pin_mut!(skipped_revs);
        let revisions_at_snapshot = self.load_revisions_at_snapshot_ts(skipped_revs);
        pin_mut!(revisions_at_snapshot);
        while let Some(rev) = revisions_at_snapshot.try_next().await? {
            let index_key = rev
                .value
                .index_key(indexed_fields, persistence_version)
                .to_bytes();
            if !cursor_has_walked(lower_bound, &index_key) {
                output.insert(index_key, rev.ts, rev.value, rev.prev_ts);
            }
        }
        Ok(())
    }

    #[try_stream(ok = (InternalDocumentId, PrevTs), error = anyhow::Error)]
    async fn walk_document_log<'a>(
        &'a self,
        tablet_id: TabletId,
        start_ts: Timestamp,
        end_ts: RepeatableTimestamp,
        observed_ids: &'a mut BTreeSet<InternalId>,
        buffered_documents: &'a mut BufferedDocumentMetadata,
    ) {
        let reader = self.persistence.clone();
        let repeatable_persistence =
            RepeatablePersistence::new(reader, end_ts, self.retention_validator.clone());
        // TODO: don't fetch document contents from the database
        let documents = repeatable_persistence
            .load_documents(TimestampRange::new(start_ts.succ()?..=*end_ts)?, Order::Asc);
        pin_mut!(documents);
        while let Some(entry) = documents.try_next().await? {
            if let Some(buffer) = buffered_documents.get_mut(&entry.id.table()) {
                // Don't overwrite any existing entry at `id`
                buffer
                    .entry(entry.id.internal_id())
                    .or_insert(entry.prev_ts.map(|prev_ts| PrevTs { prev_ts }));
            }

            if entry.id.table() == tablet_id {
                // only yield if this is the first time we have seen this ID
                if observed_ids.insert(entry.id.internal_id()) {
                    // If prev_ts is None, we still add to `observed_ids` to
                    // ignore future log entries with this `id`
                    if let Some(prev_ts) = entry.prev_ts {
                        yield (entry.id, PrevTs { prev_ts });
                    }
                }
            }
        }
    }

    /// We have these constraints:
    ///
    /// 1. we need each walk to be >= snapshot_ts
    /// 2. we need each successive walk to be >= the previous walk
    /// 3. we need each walk to be repeatable
    /// 4. we need each walk to be within retention
    ///
    /// We can satisfy these constraints by always walking at max(snapshot_ts,
    /// new_static_repeatable_recent()).
    ///
    /// 1. max(snapshot_ts, anything) >= snapshot_ts
    /// 2. snapshot_ts never changes and new_static_repeatable_recent is weakly
    ///    monotonically increasing
    /// 3. snapshot_ts and new_static_repeatable_recent are both Repeatable, and
    ///    the max of Repeatable timestamps is repeatable.
    /// 4. new_static_repeatable_recent is within retention, so max(anything,
    ///    new_static_repeatable_recent()) is within retention.
    async fn new_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        Ok(cmp::max(
            self.snapshot_ts,
            new_static_repeatable_recent(self.persistence.as_ref()).await?,
        ))
    }

    #[fastrace::trace]
    async fn fetch_page(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        cursor: &mut TableScanCursor,
    ) -> anyhow::Result<(Vec<(IndexKeyBytes, LatestDocument)>, RepeatableTimestamp)> {
        let ts = self.new_ts().await?;
        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.clone(),
            ts,
            self.retention_validator.clone(),
        );
        let reader = repeatable_persistence.read_snapshot(ts)?;
        let stream = reader.index_scan(
            index_id,
            tablet_id,
            &cursor.interval(),
            Order::Asc,
            self.page_size,
        );
        let documents_in_page: Vec<_> = stream.take(self.page_size).try_collect().await?;
        if documents_in_page.len() < self.page_size {
            cursor.advance(CursorPosition::End)?;
        } else if let Some((index_key, ..)) = documents_in_page.last() {
            cursor.advance(CursorPosition::After(index_key.clone()))?;
        }
        Ok((documents_in_page, ts))
    }

    /// Load the revisions of documents visible at `self.snapshot_ts`.
    /// Documents are yielded in the same order as input, skipping duplicates
    /// and documents that didn't exist at the snapshot.
    #[try_stream(ok = LatestDocument, error = anyhow::Error)]
    async fn load_revisions_at_snapshot_ts<'a>(
        &'a self,
        ids: impl Stream<Item = anyhow::Result<(InternalDocumentId, PrevTs)>> + 'a,
    ) {
        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.clone(),
            self.snapshot_ts,
            self.retention_validator.clone(),
        );

        // Note even though `previous_revisions` can paginate internally, we don't want
        // to hold the entire result set in memory, because documents can be large.
        let id_chunks = ids.try_chunks2(self.page_size);
        pin_mut!(id_chunks);

        let snapshot_ts_succ = self.snapshot_ts.succ()?;
        let make_query =
            |&(id, PrevTs { prev_ts }): &(InternalDocumentId, PrevTs)| DocumentPrevTsQuery {
                id,
                // HAX: we do not remember the `ts` of the original row that this
                // query came from. However, `snapshot_ts_succ` is correct for the
                // purposes of the retention validator since `prev_ts` was latest as of
                // `snapshot_ts`.
                ts: snapshot_ts_succ,
                prev_ts,
            };

        while let Some(chunk) = id_chunks.try_next().await? {
            for q in &chunk {
                anyhow::ensure!(
                    snapshot_ts_succ > q.1.prev_ts,
                    "Querying a prev_ts {prev_ts} that does not lie within the snapshot \
                     {snapshot_ts}",
                    prev_ts = q.1.prev_ts,
                    snapshot_ts = self.snapshot_ts
                );
            }
            let ids_to_load = chunk.iter().map(make_query).collect();
            let mut old_revisions = repeatable_persistence
                .previous_revisions_of_documents(ids_to_load)
                .await?;
            // Yield in the same order as the input
            for q in chunk {
                let DocumentLogEntry {
                    ts, value, prev_ts, ..
                } = old_revisions
                    .remove(&make_query(&q))
                    .with_context(|| format!("Missing revision at snapshot: {:?}", q))?;
                let Some(value) = value else { continue };
                yield LatestDocument { ts, value, prev_ts };
            }
            anyhow::ensure!(
                old_revisions.is_empty(),
                "logic error: unfetched results remain in old_revisions"
            );
        }
    }

    #[try_stream(boxed, ok = (IndexKeyBytes, LatestDocument), error = anyhow::Error)]
    async fn load_index_entries_at_snapshot_ts(
        &self,
        entries: Vec<(InternalDocumentId, Timestamp, IndexKeyBytes)>,
    ) {
        let ids: Vec<_> = entries
            .iter()
            .map(|&(id, ts, _)| (id, PrevTs { prev_ts: ts }))
            .collect();
        let mut key_by_id: BTreeMap<_, _> =
            entries.into_iter().map(|(id, _, key)| (id, key)).collect();
        let revisions = self.load_revisions_at_snapshot_ts(stream::iter(ids.into_iter().map(Ok)));
        pin_mut!(revisions);
        while let Some(rev) = revisions.try_next().await? {
            let key = key_by_id
                .remove(&rev.value.id_with_table_id())
                .context("key_by_id missing")?;
            yield (key, rev);
        }
    }

    /// Like `load_revisions_at_snapshot_ts` but doesn't need to fetch
    /// if the IterationDocument has the Full document.
    #[try_stream(boxed, ok = (IndexKeyBytes, LatestDocument), error = anyhow::Error)]
    async fn reload_revisions_at_snapshot_ts(&self, documents: IterationDocuments) {
        let mut current_batch = Vec::new();
        for (key, (ts, doc)) in documents.into_iter() {
            match doc {
                IterationDocument::Full { value, prev_ts } => {
                    let mut flush = self.load_index_entries_at_snapshot_ts(current_batch);
                    while let Some((key, rev)) = flush.try_next().await? {
                        yield (key, rev);
                    }
                    current_batch = Vec::new();
                    yield (key, LatestDocument { ts, value, prev_ts });
                },
                IterationDocument::Id(id) => {
                    current_batch.push((id, ts, key));
                },
            }
        }
        let mut flush = self.load_index_entries_at_snapshot_ts(current_batch);
        while let Some((key, rev)) = flush.try_next().await? {
            yield (key, rev);
        }
    }
}

#[derive(Debug)]
enum IterationDocument {
    Full {
        value: ResolvedDocument,
        prev_ts: Option<Timestamp>,
    },
    Id(InternalDocumentId),
}

impl IterationDocument {
    fn internal_id(&self) -> InternalId {
        match self {
            Self::Full { value, .. } => value.internal_id(),
            Self::Id(id) => id.internal_id(),
        }
    }
}

/// To avoid storing too many documents in memory, we evict the document values,
/// leaving only the IDs.
#[derive(Default, Debug)]
struct IterationDocuments {
    count_full: usize,
    docs: BTreeMap<IndexKeyBytes, (Timestamp, IterationDocument)>,
}

impl IterationDocuments {
    fn new(docs: BTreeMap<IndexKeyBytes, (Timestamp, IterationDocument)>) -> Self {
        Self {
            count_full: docs
                .values()
                .filter(|(_, doc)| matches!(doc, IterationDocument::Full { .. }))
                .count(),
            docs,
        }
    }

    fn insert(
        &mut self,
        index_key: IndexKeyBytes,
        ts: Timestamp,
        doc: ResolvedDocument,
        prev_ts: Option<Timestamp>,
    ) {
        if self.count_full < *DOCUMENTS_IN_MEMORY {
            self.docs.insert(
                index_key,
                (
                    ts,
                    IterationDocument::Full {
                        value: doc,
                        prev_ts,
                    },
                ),
            );
            self.count_full += 1;
        } else {
            self.docs.insert(
                index_key,
                (ts, IterationDocument::Id(doc.id_with_table_id())),
            );
        }
    }

    fn remove(
        &mut self,
        index_key: &IndexKeyBytes,
    ) -> Option<(IndexKeyBytes, (Timestamp, IterationDocument))> {
        let removed = self.docs.remove_entry(index_key);
        if let Some((_, (_, IterationDocument::Full { .. }))) = &removed {
            self.count_full -= 1;
        }
        removed
    }
}

impl IntoIterator for IterationDocuments {
    type IntoIter =
        <BTreeMap<IndexKeyBytes, (Timestamp, IterationDocument)> as IntoIterator>::IntoIter;
    type Item = (IndexKeyBytes, (Timestamp, IterationDocument));

    fn into_iter(self) -> Self::IntoIter {
        self.docs.into_iter()
    }
}

impl Deref for IterationDocuments {
    type Target = BTreeMap<IndexKeyBytes, (Timestamp, IterationDocument)>;

    fn deref(&self) -> &Self::Target {
        &self.docs
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{
        BTreeMap,
        BTreeSet,
    };

    use cmd_util::env::env_config;
    use common::{
        bootstrap_model::index::{
            database_index::IndexedFields,
            IndexMetadata,
        },
        pause::PauseController,
        runtime::Runtime,
        types::{
            unchecked_repeatable_ts,
            GenericIndexName,
            IndexDescriptor,
            IndexName,
        },
        value::{
            ConvexObject,
            FieldName,
            FieldPath,
            TableName,
        },
    };
    use futures::TryStreamExt;
    use keybroker::Identity;
    use prop::collection::vec as prop_vec;
    use proptest::prelude::*;
    use runtime::testing::{
        TestDriver,
        TestRuntime,
    };
    use tokio::sync::oneshot;
    use value::{
        assert_obj,
        assert_val,
        proptest::{
            RestrictNaNs,
            ValueBranching,
        },
        resolved_object_strategy,
        resolved_value_strategy,
        ExcludeSetsAndMaps,
        InternalId,
        TableNamespace,
    };

    use crate::{
        test_helpers::{
            new_test_database,
            DbFixtures,
        },
        IndexModel,
        IndexWorker,
        TestFacingModel,
        Transaction,
        UserFacingModel,
    };

    fn small_user_object() -> impl Strategy<Value = ConvexObject> {
        let values = resolved_value_strategy(
            FieldName::user_strategy,
            ValueBranching::small(),
            ExcludeSetsAndMaps(false),
            RestrictNaNs(false),
        );
        resolved_object_strategy(FieldName::user_strategy(), values, 0..4)
    }

    #[derive(Debug, proptest_derive::Arbitrary)]
    enum Update {
        Insert {
            #[proptest(strategy = "small_user_object()")]
            object: ConvexObject,
        },
        Replace {
            index: usize,
            #[proptest(strategy = "small_user_object()")]
            object: ConvexObject,
        },
        Delete {
            index: usize,
        },
    }

    fn racing_updates() -> impl Strategy<Value = Vec<Vec<Update>>> {
        prop_vec(prop_vec(any::<Update>(), 0..4), 0..4)
    }

    fn small_user_objects() -> impl Strategy<Value = Vec<ConvexObject>> {
        prop_vec(small_user_object(), 1..8)
    }

    fn by_id_index<RT: Runtime>(
        tx: &mut Transaction<RT>,
        table_name: &TableName,
    ) -> anyhow::Result<InternalId> {
        let by_id = IndexName::by_id(table_name.clone());
        let by_id_metadata = IndexModel::new(tx)
            .enabled_index_metadata(TableNamespace::test_user(), &by_id)?
            .unwrap();
        Ok(by_id_metadata.id().internal_id())
    }

    fn iterator_includes_all_documents_test(table_name: TableName, objects: Vec<ConvexObject>) {
        let td = TestDriver::new();
        let runtime = td.rt();
        let test = async {
            let database = new_test_database(runtime.clone()).await;
            let mut expected = BTreeSet::new();
            let mut tx = database.begin(Identity::system()).await?;
            for object in objects {
                let id = TestFacingModel::new(&mut tx)
                    .insert(&table_name, object)
                    .await?;
                expected.insert(id.internal_id());
            }
            let table_mapping = tx.table_mapping().namespace(TableNamespace::test_user());
            let by_id = by_id_index(&mut tx, &table_name)?;
            database.commit(tx).await?;
            let iterator = database.table_iterator(database.now_ts_for_reads(), 2);
            let tablet_id = table_mapping.id(&table_name)?.tablet_id;
            let revision_stream = iterator.stream_documents_in_table(tablet_id, by_id, None);
            futures::pin_mut!(revision_stream);
            let mut actual = BTreeSet::new();
            while let Some(revision) = revision_stream.try_next().await? {
                actual.insert(revision.value.id().internal_id());
            }
            assert_eq!(expected, actual);
            Ok::<_, anyhow::Error>(())
        };

        td.run_until(test).unwrap();
    }

    async fn racing_commits_test(
        runtime: TestRuntime,
        table_name: TableName,
        initial: Vec<ConvexObject>,
        update_batches: Vec<Vec<Update>>,
        pause: PauseController,
    ) -> anyhow::Result<()> {
        let database = new_test_database(runtime.clone()).await;
        let mut objects = BTreeMap::new();
        let mut tx = database.begin(Identity::system()).await?;
        for object in initial {
            let inserted_id = TestFacingModel::new(&mut tx)
                .insert(&table_name, object)
                .await?;
            let inserted = tx.get(inserted_id).await?.unwrap();
            objects.insert(inserted_id, inserted.to_developer());
        }
        // We expect the iterator to produce the initial objects.
        let expected = objects.clone();

        let table_mapping = tx.table_mapping().namespace(TableNamespace::test_user());
        let by_id = by_id_index(&mut tx, &table_name)?;
        database.commit(tx).await?;

        let hold_guard = pause.hold("before_index_page");
        let snapshot_ts = database.now_ts_for_reads();
        let iterator = database.table_iterator(snapshot_ts, 2);
        let tablet_id = table_mapping.id(&table_name)?.tablet_id;
        let revision_stream = iterator.stream_documents_in_table(tablet_id, by_id, None);
        let table_name_ = table_name.clone();
        let database_ = database.clone();

        let (stream_done_tx, mut stream_done_rx) = oneshot::channel();
        let test_driver = async move {
            let mut hold_guard = hold_guard;
            for update_batch in update_batches {
                // Run the backfill process until it hits our breakpoint.
                let pause_guard = tokio::select! {
                    _ = &mut stream_done_rx => break,
                    pause_guard = hold_guard.wait_for_blocked() => match pause_guard {
                        Some(pause_guard) => pause_guard,
                        // If the worker has finished processing index pages, stop agitating.
                        None => break,
                    },
                };

                // Agitate by doing a concurrent update while the worker is blocked.
                for update in update_batch {
                    let mut tx = database_.begin(Identity::system()).await?;
                    match update {
                        Update::Insert { object } => {
                            let inserted_id = TestFacingModel::new(&mut tx)
                                .insert(&table_name_, object)
                                .await?;
                            let inserted = tx.get(inserted_id).await?.unwrap();

                            objects.insert(inserted_id, inserted.to_developer());
                        },
                        Update::Replace { index, object } => {
                            if objects.is_empty() {
                                continue;
                            }
                            let id = *(objects.keys().nth(index % objects.len()).unwrap());
                            let replaced = UserFacingModel::new_root_for_test(&mut tx)
                                .replace(id.into(), object)
                                .await?;
                            objects.insert(id, replaced);
                        },
                        Update::Delete { index } => {
                            if objects.is_empty() {
                                continue;
                            }
                            let id = *(objects.keys().nth(index % objects.len()).unwrap());
                            UserFacingModel::new_root_for_test(&mut tx)
                                .delete(id.into())
                                .await?;
                            objects.remove(&id).unwrap();
                        },
                    }
                    database_.commit(tx).await?;
                    // TableIterator walks by_id at pages with timestamp
                    // max(snapshot_ts, max_repeatable_ts), so we bump max_repeatable_ts
                    // to make the commit visible to TableIterator.
                    database_.bump_max_repeatable_ts().await?;
                }
                hold_guard = pause.hold("before_index_page");
                // Continue the worker.
                pause_guard.unpause();
            }
            anyhow::Ok(())
        };

        let documents_from_iterator = async move {
            futures::pin_mut!(revision_stream);
            let mut actual = BTreeMap::new();
            let mut prev_doc_id = None;
            while let Some(rev) = revision_stream.try_next().await? {
                assert!(rev.ts <= *snapshot_ts);
                assert!(prev_doc_id < Some(rev.value.id()));
                prev_doc_id = Some(rev.value.id());
                actual.insert(rev.value.id(), rev.value.to_developer());
            }
            let _ = stream_done_tx.send(());
            Ok(actual)
        };

        let result_future = async move { futures::try_join!(documents_from_iterator, test_driver) };
        let (computed, _) = result_future.await?;

        assert_eq!(expected, computed);

        anyhow::Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_deleted(rt: TestRuntime, pause: PauseController) -> anyhow::Result<()> {
        racing_commits_test(
            rt,
            "A".parse()?,
            vec![assert_obj!()],
            vec![vec![Update::Delete { index: 0 }]],
            pause,
        )
        .await
    }

    #[convex_macro::test_runtime]
    async fn test_index_key_change(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures {
            db: database,
            tp: persistence,
            ..
        } = DbFixtures::new(&rt).await?;
        let table_name: TableName = "a".parse()?;

        // Create a.by_k and backfill.
        let index_name = GenericIndexName::new(table_name.clone(), IndexDescriptor::new("by_k")?)?;
        let field: FieldPath = "k".parse()?;
        let index_fields = IndexedFields::try_from(vec![field.clone()])?;
        let mut tx = database.begin(Identity::system()).await?;
        IndexModel::new(&mut tx)
            .add_application_index(
                TableNamespace::test_user(),
                IndexMetadata::new_enabled(index_name.clone(), index_fields.clone()),
            )
            .await?;
        database.commit(tx).await?;
        IndexWorker::new_terminating(
            rt.clone(),
            persistence.clone(),
            database.retention_validator(),
            database.clone(),
        )
        .await?;

        // Two documents, one which changes from "z" to "a".
        // In the first page, we read "a" and skip it.
        // Then we walk the documents log, and put it in skipped_keys as "z".
        // In the second page, we read "m" and output it. Then we output "z"
        // from the skipped keys.

        let mut tx = database.begin(Identity::system()).await?;
        TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("k" => "m"))
            .await?;
        let id = TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("k" => "z"))
            .await?;
        let table_mapping = tx.table_mapping().namespace(TableNamespace::test_user());
        let by_k_metadata = IndexModel::new(&mut tx)
            .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
            .unwrap();
        let by_k_id = by_k_metadata.id().internal_id();
        let snapshot_ts = unchecked_repeatable_ts(database.commit(tx).await?);

        let mut tx = database.begin(Identity::system()).await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .replace(id.into(), assert_obj!("k" => "a"))
            .await?;
        database.commit(tx).await?;
        database.bump_max_repeatable_ts().await?;

        let iterator = database.table_iterator(snapshot_ts, 1);
        let tablet_id = table_mapping.id(&table_name)?.tablet_id;
        let revisions: Vec<_> = iterator
            .stream_documents_in_table_by_index(tablet_id, by_k_id, index_fields, None)
            .try_collect()
            .await?;
        assert_eq!(revisions.len(), 2);
        let k_values: Vec<_> = revisions
            .iter()
            .map(|(_, rev)| rev.value.value().get("k").unwrap().clone())
            .collect();
        assert_eq!(k_values, vec![assert_val!("m"), assert_val!("z")]);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_multi_iterator(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db: database, .. } = DbFixtures::new(&rt).await?;
        let mut tx = database.begin(Identity::system()).await?;
        let num_tables = 8;
        let table_names: Vec<TableName> = (0..num_tables)
            .map(|i| format!("table{i}").parse())
            .collect::<Result<_, _>>()?;
        let mut docs = vec![];
        for (i, table_name) in table_names.iter().enumerate() {
            let mut docs_in_table = vec![];
            for j in 0..=i {
                docs_in_table.push(
                    TestFacingModel::new(&mut tx)
                        .insert_and_get(
                            table_name.clone(),
                            assert_obj!("a" => format!("value{i}_{j}")),
                        )
                        .await?,
                );
            }
            docs_in_table.sort_by_key(|d| d.id());
            docs.push(docs_in_table);
        }
        let table_mapping = tx.table_mapping().namespace(TableNamespace::test_user());
        let tablet_ids: Vec<_> = table_names
            .iter()
            .map(|name| Ok(table_mapping.id(name)?.tablet_id))
            .collect::<anyhow::Result<_>>()?;
        let by_ids: Vec<_> = table_names
            .iter()
            .map(|name| by_id_index(&mut tx, name))
            .collect::<anyhow::Result<_>>()?;
        database.commit(tx).await?;
        let snapshot_ts = unchecked_repeatable_ts(database.bump_max_repeatable_ts().await?);

        let mut iterator = database
            .table_iterator(snapshot_ts, 3)
            .multi(tablet_ids.clone());

        for (i, &tablet_id) in tablet_ids.iter().enumerate() {
            // Should observe the original version of the document in the table
            let documents: Vec<_> = iterator
                .stream_documents_in_table(tablet_id, by_ids[i], None)
                .try_collect()
                .await?;
            assert_eq!(
                documents
                    .iter()
                    .map(|d| d.value.clone())
                    .collect::<Vec<_>>(),
                docs[i]
            );

            // Do some more changes to interfere with the next iteration
            let mut tx = database.begin(Identity::system()).await?;
            for table in &table_names {
                TestFacingModel::new(&mut tx)
                    .insert(table, assert_obj!("a" => "blah"))
                    .await?;
            }
            for docs_in_table in &docs {
                for doc in docs_in_table {
                    tx.replace_inner(doc.id(), assert_obj!("a" => "changed"))
                        .await?;
                }
            }
            database.commit(tx).await?;
            database.bump_max_repeatable_ts().await?;

            // Also, it should be ok to query the same table more than once
            let documents_again: Vec<_> = iterator
                .stream_documents_in_table(tablet_id, by_ids[i], None)
                .try_collect()
                .await?;
            assert_eq!(documents, documents_again);

            iterator.unregister_table(tablet_id)?;
        }
        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]


        #[test]
        fn test_iterator_includes_all_documents(
            table_name in TableName::user_strategy(),
            objects in small_user_objects(),
        ) {
            iterator_includes_all_documents_test(table_name, objects);
        }

        #[test]
        fn test_racing_commits(
            table_name in TableName::user_strategy(),
            initial in small_user_objects(),
            update_batches in racing_updates(),
        ) {
            let (pause, pause_client) = PauseController::new();
            let td = TestDriver::new_with_pause_client(pause_client);
            td.run_until(
                racing_commits_test(td.rt(), table_name, initial, update_batches, pause),
            ).unwrap();
        }

    }
}
