use std::{
    collections::{
        BTreeMap,
        VecDeque,
    },
    sync::Arc,
};

use common::{
    document::{
        DocumentUpdate,
        DocumentUpdateRef,
        PackedDocument,
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
        UdfIdentifier,
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
use itertools::Itertools;
use parking_lot::Mutex;
use search::query::tokenize;
use tokio::sync::oneshot;
use value::heap_size::{
    HeapSize,
    WithHeapSize,
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
        Option<PackedDocument>,
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
        Option<PackedDocument>,
    )>,
>;

/// Converts [OrderedDocumentWrites] (the log used in `PendingWrites` that
/// contains full documents) to [OrderedIndexKeyWrites] (the log used
/// in `WriteLog` that contains only index keys).
pub fn index_keys_from_full_documents(
    ordered_writes: OrderedDocumentWrites,
    index_registry: &IndexRegistry,
) -> OrderedIndexKeyWrites {
    WithHeapSize::from(
        ordered_writes
            .into_iter()
            .map(|(id, update)| {
                (
                    id,
                    DocumentIndexKeysUpdate::from_document_update(&update, index_registry),
                    update.new_document,
                )
            })
            .collect_vec(),
    )
}

#[derive(Clone, PartialEq, Eq)]
pub enum WriteSource {
    /// A user-defined function (mutation) that performed the write.
    Udf(UdfIdentifier),
    /// A system UDF (e.g. _system/ mutations) that performed the write.
    /// Separated from `Udf` so callers can choose whether to expose it.
    SystemUdf(UdfIdentifier),
    /// An internal system operation (e.g. "system_table_cleanup").
    System(&'static str),
}

impl std::fmt::Debug for WriteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Udf(id) => write!(f, "Udf({id})"),
            Self::SystemUdf(id) => write!(f, "SystemUdf({id})"),
            Self::System(s) => write!(f, "System({s:?})"),
        }
    }
}

impl WriteSource {
    /// Create a system write source from a static label.
    pub fn system(label: &'static str) -> Self {
        Self::System(label)
    }

    /// Returns a display string for this write source, including the
    /// component path for UDF sources.
    pub fn display_name(&self) -> Option<String> {
        match self {
            Self::Udf(identifier) | Self::SystemUdf(identifier) => {
                let (component, id) = identifier.clone().into_component_and_udf_path();
                Some(match component {
                    Some(component) => format!("{component}/{id}"),
                    None => id,
                })
            },
            Self::System(s) => Some(s.to_string()),
        }
    }

    /// Returns true if this is a user UDF write source.
    pub fn is_udf(&self) -> bool {
        matches!(self, Self::Udf(_))
    }

    /// Returns the UDF identifier if this is a user function write source.
    pub fn udf_identifier(&self) -> Option<&UdfIdentifier> {
        match self {
            Self::Udf(id) => Some(id),
            Self::SystemUdf(_) => None,
            Self::System(_) => None,
        }
    }
}

impl From<&'static str> for WriteSource {
    fn from(value: &'static str) -> Self {
        Self::System(value)
    }
}

impl HeapSize for WriteSource {
    fn heap_size(&self) -> usize {
        match self {
            Self::Udf(_) | Self::SystemUdf(_) => std::mem::size_of::<UdfIdentifier>(),
            Self::System(_) => 0,
        }
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
        for<'a> F: FnMut(Timestamp, IterWrites<'a>, &WriteSource),
    {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| {
            for (ts, writes, write_source) in snapshot.iter(from, to)? {
                f(*ts, writes, write_source);
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
            let result = self.for_each(from, *end_ts, |ts, writes, _write_source| {
                for (_doc_id, index_keys_update, maybe_document) in writes {
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
                        maybe_document.clone(),
                    ) {
                        return;
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
