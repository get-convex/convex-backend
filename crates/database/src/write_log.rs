use std::{
    borrow::Cow,
    collections::{
        BTreeMap,
        BTreeSet,
        VecDeque,
    },
    sync::Arc,
};

use common::{
    bootstrap_model::tables::TableMetadata,
    document::{
        DocumentUpdate,
        DocumentUpdateRef,
        PackedDocument,
        ParseDocument,
        ParsedDocument,
    },
    document_index_keys::{
        DocumentIndexKeyValue,
        DocumentIndexKeys,
    },
    index::IndexKeyBytes,
    knobs::{
        WRITE_LOG_MAX_RETENTION_SECS,
        WRITE_LOG_MIN_RETENTION_SECS,
        WRITE_LOG_SOFT_MAX_SIZE_BYTES,
    },
    runtime::block_in_place,
    types::{
        IndexId,
        RepeatableTimestamp,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use futures::Future;
use imbl::Vector;
use indexing::{
    backend_in_memory_indexes::{
        DatabaseIndexSnapshotCache,
        TimestampedIndexCache,
    },
    index_registry::IndexRegistry,
};
use parking_lot::Mutex;
use search::query::tokenize;
use tokio::sync::oneshot;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    TableName,
    TabletId,
};

use crate::{
    database::ConflictingReadWithWriteSource,
    metrics,
    reads::ReadSet,
    Snapshot,
    Token,
};

#[derive(Clone)]
pub struct PackedDocumentUpdate {
    pub id: ResolvedDocumentId,
    pub old_document: Option<PackedDocument>,
    pub new_document: Option<PackedDocument>,
}

impl HeapSize for PackedDocumentUpdate {
    fn heap_size(&self) -> usize {
        self.old_document.heap_size() + self.new_document.heap_size()
    }
}

type OrderedDocumentWrites = WithHeapSize<Vec<(ResolvedDocumentId, PackedDocumentUpdate)>>;

impl PackedDocumentUpdate {
    pub fn pack(update: &impl DocumentUpdateRef) -> Self {
        Self {
            id: update.id(),
            old_document: update.old_document().map(PackedDocument::pack),
            new_document: update.new_document().map(PackedDocument::pack),
        }
    }

    pub fn unpack(&self) -> DocumentUpdate {
        DocumentUpdate {
            id: self.id,
            old_document: self.old_document.as_ref().map(|doc| doc.unpack()),
            new_document: self.new_document.as_ref().map(|doc| doc.unpack()),
        }
    }
}

pub type IterWrites<'a> = std::slice::Iter<
    'a,
    (
        ResolvedDocumentId,
        DocumentIndexKeysUpdate,
        Option<RefreshableTabletUpdate>,
    ),
>;

#[derive(Clone)]
pub struct DocumentIndexKeysUpdate {
    pub id: ResolvedDocumentId,
    pub old_document_keys: Option<DocumentIndexKeys>,
    pub new_document_keys: Option<DocumentIndexKeys>,
}

impl DocumentIndexKeysUpdate {
    pub fn from_document_update(
        full: &PackedDocumentUpdate,
        index_registry: &IndexRegistry,
    ) -> Self {
        Self {
            id: full.id,
            old_document_keys: full
                .old_document
                .as_ref()
                .map(|old_doc| index_registry.document_index_keys(old_doc, tokenize)),
            new_document_keys: full
                .new_document
                .as_ref()
                .map(|new_doc| index_registry.document_index_keys(new_doc, tokenize)),
        }
    }
}

impl HeapSize for DocumentIndexKeysUpdate {
    fn heap_size(&self) -> usize {
        self.old_document_keys.heap_size() + self.new_document_keys.heap_size()
    }
}

/// Optionally contains [`RefreshableTabletUpdate`] if the document is in system
/// table whose query caches should be refreshable.
type OrderedIndexKeyWrites = WithHeapSize<
    Vec<(
        ResolvedDocumentId,
        DocumentIndexKeysUpdate,
        Option<RefreshableTabletUpdate>,
    )>,
>;

/// None if the update was a delete.
pub struct RefreshableTabletUpdate(Option<PackedDocument>);

impl RefreshableTabletUpdate {
    pub fn document(&self) -> Option<&PackedDocument> {
        self.0.as_ref()
    }
}

impl HeapSize for RefreshableTabletUpdate {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

/// Converts [OrderedDocumentWrites] (the log used in `PendingWrites` that
/// contains full documents) to [OrderedIndexKeyWrites] (the log used
/// in `WriteLog` that contains only index keys).
///
/// Also updates `refreshable_tablets` if we see changes to table documents that
/// match `refreshable_tables`.
pub fn index_keys_from_full_documents(
    ordered_writes: OrderedDocumentWrites,
    index_registry: &IndexRegistry,
    tables_tablet_id: TabletId,
    refreshable_tables: &BTreeSet<TableName>,
    refreshable_tablets: &mut BTreeSet<TabletId>,
) -> OrderedIndexKeyWrites {
    let mut writes = vec![];
    for (id, update) in ordered_writes {
        if id.tablet_id == tables_tablet_id
            && let Some(new_doc) = update.new_document.as_ref()
        {
            let maybe_table_metadata: anyhow::Result<ParsedDocument<TableMetadata>> =
                new_doc.parse();
            if let Ok(table_doc) = maybe_table_metadata {
                if refreshable_tables.contains(&table_doc.name) {
                    refreshable_tablets.insert(TabletId(id.internal_id()));
                }
            } else {
                tracing::error!(
                    "Failed to parse table metadata from doc {id} with tables_tablet \
                     {tables_tablet_id}, may not be able to refresh system table index caches"
                );
            }
        }
        writes.push((
            id,
            DocumentIndexKeysUpdate::from_document_update(&update, index_registry),
            refreshable_tablets
                .contains(&id.tablet_id)
                .then_some(RefreshableTabletUpdate(update.new_document)),
        ));
    }
    WithHeapSize::from(writes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteSource(pub(crate) Option<Cow<'static, str>>);
impl WriteSource {
    pub fn unknown() -> Self {
        Self(None)
    }

    pub fn new(source: impl Into<Cow<'static, str>>) -> Self {
        Self(Some(source.into()))
    }
}

impl From<Option<String>> for WriteSource {
    fn from(value: Option<String>) -> Self {
        Self(value.map(|value| value.into()))
    }
}

impl From<String> for WriteSource {
    fn from(value: String) -> Self {
        Self(Some(value.into()))
    }
}

impl From<&'static str> for WriteSource {
    fn from(value: &'static str) -> Self {
        Self(Some(value.into()))
    }
}

impl HeapSize for WriteSource {
    fn heap_size(&self) -> usize {
        self.0
            .as_ref()
            .filter(|value| value.is_owned())
            .map(|value| value.len())
            .unwrap_or_default()
    }
}

struct WriteLogManager {
    log: WriteLog,
    waiters: VecDeque<(Timestamp, oneshot::Sender<()>)>,
}

impl WriteLogManager {
    fn new(initial_timestamp: Timestamp) -> Self {
        let log = WriteLog::new(initial_timestamp);
        let waiters = VecDeque::new();
        Self { log, waiters }
    }

    fn notify_waiters(&mut self) {
        let ts = self.log.max_ts();
        // Notify waiters
        let mut i = 0;
        while i < self.waiters.len() {
            if ts > self.waiters[i].0 || self.waiters[i].1.is_closed() {
                // Remove from the waiters.
                let w = self.waiters.swap_remove_back(i).expect("checked above");
                // Notify. Ignore if receiver has dropped.
                let _ = w.1.send(());
                // Continue without increasing i, since we just swapped the
                // element and that position and need to check it too.
                continue;
            }
            i += 1;
        }
    }

    fn append(&mut self, ts: Timestamp, writes: OrderedIndexKeyWrites, write_source: WriteSource) {
        assert!(self.log.max_ts() < ts, "{:?} >= {}", self.log.max_ts(), ts);

        self.log
            .by_ts
            .push_back(Arc::new((ts, writes, write_source)));

        self.notify_waiters();
    }

    /// Returns a future that blocks until the log has advanced past the given
    /// timestamp.
    fn wait_for_higher_ts(&mut self, target_ts: Timestamp) -> impl Future<Output = ()> + use<> {
        // Clean up waiters that are canceled.
        self.notify_waiters();

        let receiver = if self.log.max_ts() <= target_ts {
            let (sender, receiver) = oneshot::channel();
            self.waiters.push_back((target_ts, sender));
            Some(receiver)
        } else {
            None
        };

        async move {
            if let Some(receiver) = receiver {
                _ = receiver.await;
            }
        }
    }

    fn enforce_retention_policy(&mut self, current_ts: Timestamp) {
        let max_ts = current_ts
            .sub(*WRITE_LOG_MIN_RETENTION_SECS)
            .unwrap_or(Timestamp::MIN);
        let target_ts = current_ts
            .sub(*WRITE_LOG_MAX_RETENTION_SECS)
            .unwrap_or(Timestamp::MIN);
        while let Some((ts, ..)) = self.log.by_ts.front().map(|entry| &**entry) {
            let ts = *ts;

            // We never trim past max_ts, even if the size of the write log
            // is larger.
            if ts >= max_ts {
                break;
            }

            // Trim the log based on both target_ts and size.
            if ts >= target_ts && self.log.by_ts.heap_size() < *WRITE_LOG_SOFT_MAX_SIZE_BYTES {
                break;
            }

            self.log.purged_ts = ts;
            self.log.by_ts.pop_front();
        }
    }
}

/// WriteLog holds recent commits that have been written to persistence and
/// snapshot manager. These commits may cause OCC aborts for new commits, and
/// they may trigger subscriptions.
#[derive(Clone)]
struct WriteLog {
    by_ts: WithHeapSize<Vector<Arc<(Timestamp, OrderedIndexKeyWrites, WriteSource)>>>,
    purged_ts: Timestamp,
}

impl WriteLog {
    fn new(initial_timestamp: Timestamp) -> Self {
        Self {
            by_ts: WithHeapSize::default(),
            purged_ts: initial_timestamp,
        }
    }

    fn max_ts(&self) -> Timestamp {
        match self.by_ts.back() {
            Some(entry) => entry.0,
            None => self.purged_ts,
        }
    }

    // Runtime: O((log n) + k) where n is total length of the write log and k is
    // the number of elements in the returned iterator.
    fn iter(
        &self,
        from: Timestamp,
        to: Timestamp,
    ) -> anyhow::Result<impl Iterator<Item = (&Timestamp, IterWrites<'_>, &WriteSource)> + '_> {
        anyhow::ensure!(
            from > self.purged_ts,
            anyhow::anyhow!(
                "Timestamp {from} is outside of write log retention window (minimum timestamp {})",
                self.purged_ts
            )
            .context(ErrorMetadata::out_of_retention())
        );
        let start = match self.by_ts.binary_search_by_key(&from, |entry| entry.0) {
            Ok(i) => i,
            Err(i) => i,
        };
        let iter = self.by_ts.focus().narrow(start..).into_iter();
        Ok(iter
            .map(|entry| &**entry)
            .take_while(move |(t, ..)| *t <= to)
            .map(|(ts, writes, write_source)| (ts, writes.iter(), write_source)))
    }

    #[fastrace::trace]
    fn is_stale(
        &self,
        reads: &ReadSet,
        reads_ts: Timestamp,
        ts: Timestamp,
    ) -> anyhow::Result<Option<ConflictingReadWithWriteSource>> {
        block_in_place(|| {
            let log_range = self.iter(reads_ts.succ()?, ts)?;
            Ok(reads.writes_overlap_index_keys(log_range))
        })
    }

    /// Returns Err(write_ts) if the token could not be refreshed, where
    /// write_ts is the timestamp of a conflicting write (if known)
    fn refresh_token(
        &self,
        mut token: Token,
        ts: Timestamp,
    ) -> anyhow::Result<Result<Token, Option<Timestamp>>> {
        metrics::log_read_set_age(ts.secs_since_f64(token.ts()).max(0.0));
        let result = match self.is_stale(token.reads(), token.ts(), ts) {
            Ok(Some(conflict)) => Err(Some(conflict.write_ts)),
            Err(e) if e.is_out_of_retention() => {
                metrics::log_reads_refresh_miss();
                Err(None)
            },
            Err(e) => return Err(e),
            Ok(None) => {
                if token.ts() < ts {
                    token.advance_ts(ts);
                }
                Ok(token)
            },
        };
        Ok(result)
    }
}

pub fn new_write_log(initial_timestamp: Timestamp) -> (LogOwner, LogReader, LogWriter) {
    let log_manager = Arc::new(Mutex::new(WriteLogManager::new(initial_timestamp)));
    (
        LogOwner {
            inner: log_manager.clone(),
        },
        LogReader {
            inner: log_manager.clone(),
        },
        LogWriter { inner: log_manager },
    )
}

/// LogOwner consumes the log and is responsible for trimming it.
pub struct LogOwner {
    inner: Arc<Mutex<WriteLogManager>>,
}

impl LogOwner {
    pub fn enforce_retention_policy(&mut self, current_ts: Timestamp) {
        self.inner.lock().enforce_retention_policy(current_ts)
    }

    pub fn reader(&self) -> LogReader {
        LogReader {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Clone)]
pub struct LogReader {
    inner: Arc<Mutex<WriteLogManager>>,
}

impl LogReader {
    #[fastrace::trace]
    pub fn refresh_token(
        &self,
        token: Token,
        ts: Timestamp,
    ) -> anyhow::Result<Result<Token, Option<Timestamp>>> {
        if token.ts() == ts {
            // Nothing to do. We can return Ok even if `token.ts()` has fallen
            // out of the write log retention window.
            return Ok(Ok(token));
        }
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| {
            let max_ts = snapshot.max_ts();
            anyhow::ensure!(
                ts <= max_ts,
                "Can't refresh token to newer timestamp {ts} than max ts {max_ts}"
            );
            snapshot.refresh_token(token, ts)
        })
    }

    pub fn refresh_reads_until_max_ts(
        &self,
        token: Token,
    ) -> anyhow::Result<Result<Token, Option<Timestamp>>> {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| {
            let max_ts = snapshot.max_ts();
            snapshot.refresh_token(token, max_ts)
        })
    }

    pub fn max_ts(&self) -> Timestamp {
        let snapshot = { self.inner.lock().log.clone() };
        snapshot.max_ts()
    }

    /// Blocks until the log has advanced past the given timestamp.
    pub async fn wait_for_higher_ts(&self, target_ts: Timestamp) -> Timestamp {
        let fut = self.inner.lock().wait_for_higher_ts(target_ts);
        fut.await;
        let result = self.inner.lock().log.max_ts();
        assert!(result > target_ts);
        result
    }

    pub fn for_each<F>(&self, from: Timestamp, to: Timestamp, mut f: F) -> anyhow::Result<()>
    where
        for<'a> F: FnMut(Timestamp, IterWrites<'a>),
    {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| {
            for (ts, writes, _) in snapshot.iter(from, to)? {
                f(*ts, writes);
            }
            Ok(())
        })
    }

    /// Walks the write log and updates the index cache with documents in
    /// RefreshableTablets with index updates the cache is already tracking.
    ///
    /// Returns None if the begin_ts is out of the retention window.
    pub async fn fast_forward_index_cache(
        &self,
        cache: TimestampedIndexCache,
        index_registry: &IndexRegistry, // Must be from the snapshot at end_ts
        end_ts: RepeatableTimestamp,
    ) -> anyhow::Result<Option<TimestampedIndexCache>> {
        let TimestampedIndexCache {
            mut cache,
            ts: begin_ts,
        } = cache;
        anyhow::ensure!(*begin_ts <= *end_ts);
        // Drop any cached indexes that are no longer in the registry (e.g.
        // deleted or no longer enabled).
        let unknown_indexes: Vec<_> = cache
            .tracked_index_ids()
            .filter(|id| index_registry.enabled_index_by_index_id(id).is_none())
            .collect();
        for index_id in unknown_indexes {
            cache.remove_index(index_id);
        }
        if *begin_ts != *end_ts {
            let from = (*begin_ts).succ()?;
            let result = self.for_each(from, *end_ts, |ts, writes| {
                for (_doc_id, index_keys_update, refreshable_update) in writes {
                    if let Some(refreshable_update) = refreshable_update {
                        let old_keys = resolve_db_index_keys(
                            index_registry,
                            index_keys_update.old_document_keys.as_ref(),
                            &cache,
                        );
                        let new_keys = resolve_db_index_keys(
                            index_registry,
                            index_keys_update.new_document_keys.as_ref(),
                            &cache,
                        );
                        if !cache.apply_write(
                            ts,
                            old_keys.unwrap_or_default(),
                            new_keys.unwrap_or_default(),
                            refreshable_update.document().cloned(),
                        ) {
                            return;
                        }
                    } else {
                        remove_non_refreshable_indexes(
                            index_keys_update,
                            index_registry,
                            &mut cache,
                        );
                    }
                }
            });
            match result {
                Ok(()) => {},
                Err(e) if e.is_out_of_retention() => return Ok(None),
                Err(e) => return Err(e),
            }
        }

        Ok(Some(TimestampedIndexCache { cache, ts: end_ts }))
    }
}

fn remove_non_refreshable_indexes(
    index_keys_update: &DocumentIndexKeysUpdate,
    index_registry: &IndexRegistry,
    index_cache: &mut DatabaseIndexSnapshotCache,
) {
    for doc_keys in [
        &index_keys_update.old_document_keys,
        &index_keys_update.new_document_keys,
    ]
    .into_iter()
    .flatten()
    {
        for (index_name, key_value) in doc_keys.iter() {
            if matches!(key_value, DocumentIndexKeyValue::Standard(_))
                && let Some(index) = index_registry.get_enabled(index_name)
            {
                index_cache.remove_index(index.id());
            }
        }
    }
}

/// Only resolve keys for db indexes already tracked in the cache and in enabled
/// indexes
fn resolve_db_index_keys(
    index_registry: &IndexRegistry,
    doc_keys: Option<&DocumentIndexKeys>,
    index_cache: &DatabaseIndexSnapshotCache,
) -> Option<Vec<(IndexId, bool, IndexKeyBytes)>> {
    doc_keys.map(|keys| {
        keys.iter()
            .filter_map(|(index_name, key_value)| {
                if let DocumentIndexKeyValue::Standard(index_key) = key_value {
                    index_registry
                        .get_enabled(index_name)
                        .filter(|index| index_cache.is_index_tracked(&index.id()))
                        .map(|index| {
                            (
                                index.id(),
                                index.metadata.name.is_by_id(),
                                index_key.clone(),
                            )
                        })
                } else {
                    None
                }
            })
            .collect()
    })
}

/// LogWriter can append to the log.
pub struct LogWriter {
    inner: Arc<Mutex<WriteLogManager>>,
}

impl LogWriter {
    // N.B.: `writes` is `OrderedWrites` because that's what the committer
    // already has, but the write log doesn't actually care about the ordering.
    pub fn append(
        &mut self,
        ts: Timestamp,
        writes: OrderedIndexKeyWrites,
        write_source: WriteSource,
    ) {
        block_in_place(|| self.inner.lock().append(ts, writes, write_source));
    }

    pub fn is_stale(
        &self,
        reads: &ReadSet,
        reads_ts: Timestamp,
        ts: Timestamp,
    ) -> anyhow::Result<Option<ConflictingReadWithWriteSource>> {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| snapshot.is_stale(reads, reads_ts, ts))
    }
}

/// Pending writes are used by the committer to detect conflicts between a new
/// commit and a commit that has started but has not finished writing to
/// persistence and snapshot_manager.
/// These pending writes do not conflict with each other so any subset of them
/// may be written to persistence, in any order.
pub struct PendingWrites {
    by_ts: BTreeMap<Timestamp, (OrderedDocumentWrites, WriteSource, Snapshot)>,
}

impl PendingWrites {
    pub fn new() -> Self {
        Self {
            by_ts: BTreeMap::new(),
        }
    }

    pub fn push_back(
        &mut self,
        ts: Timestamp,
        writes: OrderedDocumentWrites,
        write_source: WriteSource,
        snapshot: Snapshot,
    ) -> PendingWriteHandle {
        if let Some((last_ts, _)) = self.by_ts.iter().next_back() {
            assert!(*last_ts < ts, "{:?} >= {}", *last_ts, ts);
        }

        self.by_ts.insert(ts, (writes, write_source, snapshot));
        PendingWriteHandle(Some(ts))
    }

    pub fn latest_snapshot(&self) -> Option<Snapshot> {
        self.by_ts
            .iter()
            .next_back()
            .map(|(_, (_, _, snapshot))| snapshot.clone())
    }

    /// Recomputes the snapshot associated with each pending write, rebasing the
    /// pending writes on the new base snapshot provided.
    pub fn recompute_pending_snapshots(&mut self, mut base_snapshot: Snapshot) {
        for (ts, (ordered_writes, _, snapshot)) in self.by_ts.iter_mut() {
            for (_id, document_update) in ordered_writes.iter() {
                base_snapshot
                    .update(&document_update.unpack(), *ts)
                    .expect("Failed to update snapshot");
            }
            *snapshot = base_snapshot.clone();
        }
    }

    pub fn iter(
        &self,
        from: Timestamp,
        to: Timestamp,
    ) -> impl Iterator<
        Item = (
            &Timestamp,
            impl Iterator<Item = &(ResolvedDocumentId, PackedDocumentUpdate)>,
            &WriteSource,
        ),
    > {
        self.by_ts
            .range(from..=to)
            .map(|(ts, (w, source, _snapshot))| (ts, w.iter(), source))
    }

    pub fn is_stale(
        &self,
        reads: &ReadSet,
        reads_ts: Timestamp,
        ts: Timestamp,
    ) -> anyhow::Result<Option<ConflictingReadWithWriteSource>> {
        Ok(reads.writes_overlap_docs(self.iter(reads_ts.succ()?, ts)))
    }

    pub fn pop_first(
        &mut self,
        mut handle: PendingWriteHandle,
    ) -> Option<(Timestamp, OrderedDocumentWrites, WriteSource, Snapshot)> {
        let first = self.by_ts.pop_first();
        if let Some((ts, (writes, write_source, snapshot))) = first {
            if let Some(expected_ts) = handle.0
                && ts == expected_ts
            {
                handle.0.take();
            }
            Some((ts, writes, write_source, snapshot))
        } else {
            None
        }
    }

    pub fn min_ts(&self) -> Option<Timestamp> {
        self.by_ts.first_key_value().map(|(ts, _)| *ts)
    }
}

pub struct PendingWriteHandle(Option<Timestamp>);

impl PendingWriteHandle {
    pub fn must_commit_ts(&self) -> Timestamp {
        self.0.expect("pending write already committed")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common::{
        self,
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
        document_index_keys::DocumentIndexKeys,
        index::IndexKey,
        interval::{
            BinaryKey,
            End,
            Interval,
            StartIncluded,
        },
        knobs::WRITE_LOG_MAX_RETENTION_SECS,
        testing::TestIdGenerator,
        types::{
            unchecked_repeatable_ts,
            GenericIndexName,
            IndexDescriptor,
            IndexId,
            PersistenceVersion,
            TabletIndexName,
            Timestamp,
        },
        value::{
            FieldPath,
            ResolvedDocumentId,
        },
    };
    use convex_macro::test_runtime;
    use indexing::{
        backend_in_memory_indexes::{
            DatabaseIndexSnapshotCache,
            TimestampedIndexCache,
        },
        index_registry::IndexRegistry,
    };
    use runtime::testing::TestRuntime;
    use value::{
        assert_obj,
        val,
    };

    use crate::{
        reads::{
            ReadSet,
            TransactionReadSet,
        },
        write_log::{
            new_write_log,
            remove_non_refreshable_indexes,
            resolve_db_index_keys,
            DocumentIndexKeysUpdate,
            RefreshableTabletUpdate,
            WriteLogManager,
            WriteSource,
        },
    };

    #[test]
    fn test_write_log() -> anyhow::Result<()> {
        let mut log_manager = WriteLogManager::new(Timestamp::must(1000));
        assert_eq!(log_manager.log.purged_ts, Timestamp::must(1000));
        assert_eq!(log_manager.log.max_ts(), Timestamp::must(1000));

        for ts in (1002..=1010).step_by(2) {
            log_manager.append(Timestamp::must(ts), vec![].into(), WriteSource::unknown());
            assert_eq!(log_manager.log.purged_ts, Timestamp::must(1000));
            assert_eq!(log_manager.log.max_ts(), Timestamp::must(ts));
        }

        assert!(log_manager
            .log
            .iter(Timestamp::must(1000), Timestamp::must(1010))
            .is_err());
        assert_eq!(
            log_manager
                .log
                .iter(Timestamp::must(1001), Timestamp::must(1010))?
                .map(|(ts, ..)| *ts)
                .collect::<Vec<_>>(),
            (1002..=1010)
                .step_by(2)
                .map(Timestamp::must)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            log_manager
                .log
                .iter(Timestamp::must(1004), Timestamp::must(1008))?
                .map(|(ts, ..)| *ts)
                .collect::<Vec<_>>(),
            (1004..=1008)
                .step_by(2)
                .map(Timestamp::must)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            log_manager
                .log
                .iter(Timestamp::must(1004), Timestamp::must(1020))?
                .map(|(ts, ..)| *ts)
                .collect::<Vec<_>>(),
            (1004..=1010)
                .step_by(2)
                .map(Timestamp::must)
                .collect::<Vec<_>>()
        );

        log_manager.enforce_retention_policy(
            Timestamp::must(1005)
                .add(*WRITE_LOG_MAX_RETENTION_SECS)
                .unwrap(),
        );
        assert_eq!(log_manager.log.purged_ts, Timestamp::must(1004));
        assert_eq!(log_manager.log.max_ts(), Timestamp::must(1010));

        assert!(log_manager
            .log
            .iter(Timestamp::must(1004), Timestamp::must(1010))
            .is_err());
        assert_eq!(
            log_manager
                .log
                .iter(Timestamp::must(1005), Timestamp::must(1010))?
                .map(|(ts, ..)| *ts)
                .collect::<Vec<_>>(),
            (1006..=1010)
                .step_by(2)
                .map(Timestamp::must)
                .collect::<Vec<_>>()
        );

        Ok(())
    }

    #[test_runtime]
    async fn test_is_stale(_rt: TestRuntime) -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let mut log_manager = WriteLogManager::new(Timestamp::must(1000));
        let table_id = id_generator.user_table_id(&"t".parse()?).tablet_id;
        let id = id_generator.user_generate(&"t".parse()?);
        let index_key = IndexKey::new(vec![val!(5)], id.into());
        let index_key_binary: BinaryKey = index_key.to_bytes().into();
        let index_name =
            TabletIndexName::new(table_id, IndexDescriptor::new("by_k").unwrap()).unwrap();
        log_manager.append(
            Timestamp::must(1003),
            vec![(
                id,
                DocumentIndexKeysUpdate {
                    id,
                    old_document_keys: None,
                    new_document_keys: Some(DocumentIndexKeys::with_standard_index_for_test(
                        index_name.clone(),
                        index_key.clone(),
                    )),
                },
                None,
            )]
            .into(),
            WriteSource::unknown(),
        );
        let read_set = |interval: Interval| -> ReadSet {
            let field_path: FieldPath = "k".parse().unwrap();
            let mut reads = TransactionReadSet::new();
            reads
                .record_indexed_directly(
                    index_name.clone(),
                    vec![field_path].try_into().unwrap(),
                    interval,
                )
                .unwrap();
            reads.into_read_set()
        };
        // Write conflicts with read.
        let read_set_conflict = read_set(Interval::all());
        assert_eq!(
            log_manager
                .log
                .is_stale(
                    &read_set_conflict,
                    Timestamp::must(1001),
                    Timestamp::must(1004)
                )?
                .unwrap()
                .read
                .index,
            index_name.clone()
        );
        // Write happened after read finished.
        assert_eq!(
            log_manager.log.is_stale(
                &read_set_conflict,
                Timestamp::must(1001),
                Timestamp::must(1002)
            )?,
            None
        );
        // Write happened before read started.
        assert_eq!(
            log_manager.log.is_stale(
                &read_set_conflict,
                Timestamp::must(1003),
                Timestamp::must(1004)
            )?,
            None
        );
        // Different intervals, some of which intersect the write.
        let empty_read_set = read_set(Interval::empty());
        assert_eq!(
            log_manager.log.is_stale(
                &empty_read_set,
                Timestamp::must(1001),
                Timestamp::must(1004)
            )?,
            None
        );
        let prefix_read_set = read_set(Interval::prefix(index_key_binary.clone()));
        assert_eq!(
            log_manager
                .log
                .is_stale(
                    &prefix_read_set,
                    Timestamp::must(1001),
                    Timestamp::must(1004)
                )?
                .unwrap()
                .read
                .index,
            index_name.clone()
        );
        let end_excluded_read_set = read_set(Interval {
            start: StartIncluded(BinaryKey::min()),
            end: End::Excluded(index_key_binary.clone()),
        });
        assert_eq!(
            log_manager.log.is_stale(
                &end_excluded_read_set,
                Timestamp::must(1001),
                Timestamp::must(1004)
            )?,
            None
        );
        let start_included_read_set = read_set(Interval {
            start: StartIncluded(index_key_binary),
            end: End::Unbounded,
        });
        assert_eq!(
            log_manager
                .log
                .is_stale(
                    &start_included_read_set,
                    Timestamp::must(1001),
                    Timestamp::must(1004)
                )?
                .unwrap()
                .read
                .index,
            index_name.clone()
        );

        let mut delete_log_manager = WriteLogManager::new(Timestamp::must(1000));
        delete_log_manager.append(
            Timestamp::must(1003),
            vec![(
                id,
                DocumentIndexKeysUpdate {
                    id,
                    old_document_keys: Some(DocumentIndexKeys::with_standard_index_for_test(
                        index_name.clone(),
                        index_key,
                    )),
                    new_document_keys: None,
                },
                None,
            )]
            .into(),
            WriteSource::unknown(),
        );
        assert_eq!(
            delete_log_manager
                .log
                .is_stale(
                    &read_set_conflict,
                    Timestamp::must(1001),
                    Timestamp::must(1004)
                )?
                .unwrap()
                .read
                .index,
            index_name
        );
        assert_eq!(
            delete_log_manager.log.is_stale(
                &empty_read_set,
                Timestamp::must(1001),
                Timestamp::must(1004)
            )?,
            None
        );
        Ok(())
    }

    struct FastForwardIndexCacheFixture {
        id_generator: TestIdGenerator,
        index_registry: IndexRegistry,
        table_index_name: TabletIndexName,
        index_id: IndexId,
    }

    impl FastForwardIndexCacheFixture {
        fn new() -> anyhow::Result<Self> {
            let mut id_generator = TestIdGenerator::new();
            let table_name = "users".parse()?;
            let table_id = id_generator.user_table_id(&table_name);

            let by_id = GenericIndexName::by_id(table_id.tablet_id);
            let by_name =
                GenericIndexName::new(table_id.tablet_id, IndexDescriptor::new("by_name")?)?;

            let indexes = vec![
                IndexMetadata::new_enabled(by_id, IndexedFields::by_id()),
                IndexMetadata::new_enabled(by_name.clone(), vec!["name".parse()?].try_into()?),
            ];

            let index_documents = gen_index_documents(&mut id_generator, indexes)?;
            let index_registry = IndexRegistry::bootstrap(
                &id_generator,
                index_documents.values(),
                PersistenceVersion::default(),
            )?;

            let index_id = index_registry.get_enabled(&by_name).unwrap().id();

            Ok(Self {
                id_generator,
                index_registry,
                table_index_name: by_name,
                index_id,
            })
        }

        fn cache_tracking_index(&self) -> DatabaseIndexSnapshotCache {
            let mut cache = DatabaseIndexSnapshotCache::new();
            cache.track_index_for_testing(self.index_id);
            cache
        }

        fn make_doc(
            &mut self,
            obj: common::value::ConvexObject,
        ) -> anyhow::Result<(ResolvedDocumentId, IndexKey, PackedDocument)> {
            let id = self.id_generator.user_generate(&"users".parse()?);
            let doc = ResolvedDocument::new(id, CreationTime::ONE, obj)?;
            let fields: Vec<FieldPath> = vec!["name".parse()?];
            let key = doc.index_key(&fields);
            let packed = PackedDocument::pack(&doc);
            Ok((id, key, packed))
        }
    }

    fn gen_index_documents(
        id_generator: &mut TestIdGenerator,
        mut indexes: Vec<common::bootstrap_model::index::TabletIndexMetadata>,
    ) -> anyhow::Result<BTreeMap<ResolvedDocumentId, ResolvedDocument>> {
        let mut index_documents = BTreeMap::new();
        let index_table = id_generator.system_table_id(&INDEX_TABLE);
        indexes.push(IndexMetadata::new_enabled(
            GenericIndexName::by_id(index_table.tablet_id),
            IndexedFields::by_id(),
        ));
        for metadata in indexes {
            let id = id_generator.system_generate(&INDEX_TABLE);
            let doc = ResolvedDocument::new(id, CreationTime::ONE, metadata.try_into()?)?;
            index_documents.insert(doc.id(), doc);
        }
        Ok(index_documents)
    }

    #[test]
    fn resolve_db_index_keys_insert() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (_id, key, _doc) = f.make_doc(assert_obj!("name" => "alice"))?;

        let doc_keys = DocumentIndexKeys::with_standard_index_for_test(
            f.table_index_name.clone(),
            key.clone(),
        );

        let cache = f.cache_tracking_index();
        let old_keys = resolve_db_index_keys(&f.index_registry, None, &cache);
        let new_keys = resolve_db_index_keys(&f.index_registry, Some(&doc_keys), &cache);
        assert!(old_keys.is_none());
        assert_eq!(new_keys.unwrap(), vec![(f.index_id, false, key.to_bytes())]);
        Ok(())
    }

    #[test]
    fn resolve_db_index_keys_update() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (_id, old_key, _old_doc) = f.make_doc(assert_obj!("name" => "alice"))?;
        let (_id2, new_key, _new_doc) = f.make_doc(assert_obj!("name" => "bob"))?;

        let old_doc_keys = DocumentIndexKeys::with_standard_index_for_test(
            f.table_index_name.clone(),
            old_key.clone(),
        );
        let new_doc_keys = DocumentIndexKeys::with_standard_index_for_test(
            f.table_index_name.clone(),
            new_key.clone(),
        );

        let cache = f.cache_tracking_index();
        let old_keys = resolve_db_index_keys(&f.index_registry, Some(&old_doc_keys), &cache);
        let new_keys = resolve_db_index_keys(&f.index_registry, Some(&new_doc_keys), &cache);
        assert_eq!(
            old_keys.unwrap(),
            vec![(f.index_id, false, old_key.to_bytes())]
        );
        assert_eq!(
            new_keys.unwrap(),
            vec![(f.index_id, false, new_key.to_bytes())]
        );
        Ok(())
    }

    #[test]
    fn resolve_db_index_keys_delete() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (_id, key, _doc) = f.make_doc(assert_obj!("name" => "alice"))?;

        let doc_keys = DocumentIndexKeys::with_standard_index_for_test(
            f.table_index_name.clone(),
            key.clone(),
        );

        let cache = f.cache_tracking_index();
        let old_keys = resolve_db_index_keys(&f.index_registry, Some(&doc_keys), &cache);
        let new_keys = resolve_db_index_keys(&f.index_registry, None, &cache);
        assert_eq!(old_keys.unwrap(), vec![(f.index_id, false, key.to_bytes())]);
        assert!(new_keys.is_none());
        Ok(())
    }

    #[test]
    fn resolve_db_index_keys_skips_untracked_indexes() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (_id, key, _doc) = f.make_doc(assert_obj!("name" => "alice"))?;

        let doc_keys = DocumentIndexKeys::with_standard_index_for_test(
            f.table_index_name.clone(),
            key.clone(),
        );

        // Use an empty cache — no indexes are tracked.
        let empty_cache = DatabaseIndexSnapshotCache::new();
        let keys = resolve_db_index_keys(&f.index_registry, Some(&doc_keys), &empty_cache);
        assert_eq!(keys.unwrap(), vec![]);
        Ok(())
    }

    #[test]
    fn non_refreshable_insert() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (_id, key, _doc) = f.make_doc(assert_obj!("name" => "alice"))?;

        let update = DocumentIndexKeysUpdate {
            id: _id,
            old_document_keys: None,
            new_document_keys: Some(DocumentIndexKeys::with_standard_index_for_test(
                f.table_index_name.clone(),
                key,
            )),
        };

        let mut cache = f.cache_tracking_index();
        assert!(cache.is_index_tracked(&f.index_id));
        remove_non_refreshable_indexes(&update, &f.index_registry, &mut cache);
        assert!(!cache.is_index_tracked(&f.index_id));
        Ok(())
    }

    #[test]
    fn non_refreshable_delete() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (id, key, _doc) = f.make_doc(assert_obj!("name" => "alice"))?;

        let update = DocumentIndexKeysUpdate {
            id,
            old_document_keys: Some(DocumentIndexKeys::with_standard_index_for_test(
                f.table_index_name.clone(),
                key,
            )),
            new_document_keys: None,
        };

        let mut cache = f.cache_tracking_index();
        assert!(cache.is_index_tracked(&f.index_id));
        remove_non_refreshable_indexes(&update, &f.index_registry, &mut cache);
        assert!(!cache.is_index_tracked(&f.index_id));
        Ok(())
    }

    #[test]
    fn non_refreshable_search_index_filtered() -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;

        // Use a search index key value (not Standard) — should be filtered out.
        let search_keys = DocumentIndexKeys::with_search_index_for_test(
            f.table_index_name.clone(),
            "name".parse()?,
            common::document_index_keys::SearchValueTokens::from_iter_for_test(vec![
                "hello".to_string()
            ]),
        );

        let update = DocumentIndexKeysUpdate {
            id: f.id_generator.user_generate(&"users".parse()?),
            old_document_keys: None,
            new_document_keys: Some(search_keys),
        };

        let mut cache = f.cache_tracking_index();
        remove_non_refreshable_indexes(&update, &f.index_registry, &mut cache);
        // Index should still be tracked since search indexes are filtered out.
        assert!(cache.is_index_tracked(&f.index_id));
        Ok(())
    }

    #[test_runtime]
    async fn test_fast_forward_index_cache_same_ts(_rt: TestRuntime) -> anyhow::Result<()> {
        let f = FastForwardIndexCacheFixture::new()?;
        let (_owner, reader, _writer) = new_write_log(Timestamp::must(1000));

        let ts = unchecked_repeatable_ts(Timestamp::must(1000));
        let cache = TimestampedIndexCache {
            cache: f.cache_tracking_index(),
            ts,
        };
        let result = reader
            .fast_forward_index_cache(cache, &f.index_registry, ts)
            .await?;
        assert!(result.is_some());
        assert_eq!(*result.unwrap().ts, *ts);
        Ok(())
    }

    #[test_runtime]
    async fn test_fast_forward_index_cache_with_writes(_rt: TestRuntime) -> anyhow::Result<()> {
        let mut f = FastForwardIndexCacheFixture::new()?;
        let (_owner, reader, mut writer) = new_write_log(Timestamp::must(1000));

        let (id, key, doc) = f.make_doc(assert_obj!("name" => "alice"))?;
        let expected_key_bytes = key.to_bytes();
        writer.append(
            Timestamp::must(1002),
            vec![(
                id,
                DocumentIndexKeysUpdate {
                    id,
                    old_document_keys: None,
                    new_document_keys: Some(DocumentIndexKeys::with_standard_index_for_test(
                        f.table_index_name.clone(),
                        key,
                    )),
                },
                Some(RefreshableTabletUpdate(Some(doc))),
            )]
            .into(),
            WriteSource::unknown(),
        );

        let index_cache = f.cache_tracking_index();
        assert!(index_cache.documents_for_index(&f.index_id).is_empty());

        let begin_ts = unchecked_repeatable_ts(Timestamp::must(1000));
        let end_ts = unchecked_repeatable_ts(Timestamp::must(1002));
        let cache = TimestampedIndexCache {
            cache: index_cache,
            ts: begin_ts,
        };
        let result = reader
            .fast_forward_index_cache(cache, &f.index_registry, end_ts)
            .await?;
        let ff_cache = result.unwrap();
        assert_eq!(*ff_cache.ts, *end_ts);
        let cached_docs = ff_cache.cache.documents_for_index(&f.index_id);
        assert_eq!(cached_docs.len(), 1);
        let (cached_key, cached_ts, cached_doc) = &cached_docs[0];
        assert_eq!(*cached_key, expected_key_bytes);
        assert_eq!(*cached_ts, Timestamp::must(1002));
        assert_eq!(cached_doc.id(), id);
        Ok(())
    }

    #[test_runtime]
    async fn test_fast_forward_index_cache_out_of_retention(
        _rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let f = FastForwardIndexCacheFixture::new()?;
        let (mut owner, reader, mut writer) = new_write_log(Timestamp::must(1000));

        writer.append(Timestamp::must(1001), vec![].into(), WriteSource::unknown());

        // Enforce retention far in the future so ts=1001 gets purged.
        owner.enforce_retention_policy(
            Timestamp::must(1001)
                .add(*WRITE_LOG_MAX_RETENTION_SECS)
                .unwrap()
                .succ()?,
        );

        let begin_ts = unchecked_repeatable_ts(Timestamp::must(1000));
        let end_ts = unchecked_repeatable_ts(Timestamp::must(1001));
        let cache = TimestampedIndexCache {
            cache: f.cache_tracking_index(),
            ts: begin_ts,
        };
        let result = reader
            .fast_forward_index_cache(cache, &f.index_registry, end_ts)
            .await?;
        assert!(result.is_none());
        Ok(())
    }
}
