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
        DatabaseIndexWrite,
        IndexKeyUpdate,
        TextIndexWrite,
    },
    knobs::{
        WRITE_LOG_MAX_RETENTION_SECS,
        WRITE_LOG_MIN_RETENTION_SECS,
        WRITE_LOG_SOFT_MAX_SIZE_BYTES,
    },
    runtime::block_in_place,
    types::{
        RepeatableTimestamp,
        SubscriberId,
        TabletIndexName,
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
use imbl::{
    ordmap::Entry,
    OrdMap,
    Vector,
};
use indexing::{
    backend_in_memory_indexes::TimestampedIndexCache,
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
    TabletId,
};

use crate::{
    database::ConflictingReadWithWriteSource,
    metrics,
    reads::ReadSet,
    Snapshot,
    Token,
};

type OrderedDocumentWrites = Vec<(ResolvedDocumentId, PackedDocumentUpdate)>;

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
/// Indicates whether an index entry in the write log belongs to the
/// `by_database_index` or `by_text_index` map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    Database,
    Text,
}

/// The per-commit index-key writes, split by index kind so each map holds a
/// homogeneous update type.
pub struct OrderedIndexKeyWrites {
    pub database: BTreeMap<TabletIndexName, WithHeapSize<Vector<DatabaseIndexWrite>>>,
    pub text: BTreeMap<TabletIndexName, WithHeapSize<Vector<TextIndexWrite>>>,
}

impl OrderedIndexKeyWrites {
    pub fn empty() -> Self {
        Self {
            database: BTreeMap::new(),
            text: BTreeMap::new(),
        }
    }
}

/// Converts [OrderedDocumentWrites] (the log used in `PendingWrites` that
/// contains full documents) to [OrderedIndexKeyWrites] (the log used
/// in `WriteLog` that contains index keys too).
pub fn index_keys_from_full_documents(
    ordered_writes: OrderedDocumentWrites,
    index_registry: &IndexRegistry,
) -> OrderedIndexKeyWrites {
    let mut database: BTreeMap<TabletIndexName, WithHeapSize<Vector<DatabaseIndexWrite>>> =
        BTreeMap::new();
    let mut text: BTreeMap<TabletIndexName, WithHeapSize<Vector<TextIndexWrite>>> = BTreeMap::new();
    for (_id, update) in ordered_writes.into_iter() {
        for (index_name, index_update) in index_registry
            .document_index_keys(
                update.id,
                update.old_document,
                update.new_document,
                tokenize,
            )
            .0
            .into_iter()
        {
            match index_update.update {
                IndexKeyUpdate::Database(u) => {
                    database
                        .entry(index_name)
                        .or_default()
                        .push_back(DatabaseIndexWrite {
                            document_id: index_update.document_id,
                            update: u,
                            new_document: index_update.new_document,
                        });
                },
                IndexKeyUpdate::Text(u) => {
                    text.entry(index_name)
                        .or_default()
                        .push_back(TextIndexWrite {
                            document_id: index_update.document_id,
                            update: u,
                        });
                },
            }
        }
    }
    OrderedIndexKeyWrites { database, text }
}

#[derive(Clone, PartialEq, Eq)]
pub enum WriteSource {
    /// A user-defined function (mutation) that performed the write.
    Udf(Arc<UdfIdentifier>),
    /// A system UDF (e.g. _system/ mutations) that performed the write.
    /// Separated from `Udf` so callers can choose whether to expose it.
    SystemUdf(Arc<UdfIdentifier>),
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
                let (component, id) = (**identifier).clone().into_component_and_udf_path();
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
            Self::Udf(_) | Self::SystemUdf(_) => std::mem::size_of::<Arc<UdfIdentifier>>(),
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

        for (index, updates) in writes.database {
            self.log.by_database_index.append(
                index,
                ts,
                updates,
                write_source.clone(),
                IndexKind::Database,
                &mut self.log.size,
                &mut self.log.min_ts_to_index,
            );
        }
        for (index, updates) in writes.text {
            self.log.by_text_index.append(
                index,
                ts,
                updates,
                write_source.clone(),
                IndexKind::Text,
                &mut self.log.size,
                &mut self.log.min_ts_to_index,
            );
        }
        self.log.max_ts = ts;

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
        let hard_limit_ts = current_ts
            .sub(*WRITE_LOG_MIN_RETENTION_SECS)
            .unwrap_or(Timestamp::MIN);
        let soft_limit_ts = current_ts
            .sub(*WRITE_LOG_MAX_RETENTION_SECS)
            .unwrap_or(Timestamp::MIN);
        loop {
            let Some((ts, indexes)) = self
                .log
                .min_ts_to_index
                .get_min()
                .map(|(ts, indexes)| (*ts, indexes.clone()))
            else {
                break;
            };

            if ts >= hard_limit_ts {
                break;
            }

            if ts >= soft_limit_ts && self.log.size < *WRITE_LOG_SOFT_MAX_SIZE_BYTES {
                break;
            }

            self.log.purged_ts = ts;
            self.log.min_ts_to_index.remove(&ts);

            for (index, kind) in indexes {
                match kind {
                    IndexKind::Database => {
                        self.log.by_database_index.remove_at_ts(
                            &index,
                            ts,
                            IndexKind::Database,
                            &mut self.log.size,
                            &mut self.log.min_ts_to_index,
                        );
                    },
                    IndexKind::Text => {
                        self.log.by_text_index.remove_at_ts(
                            &index,
                            ts,
                            IndexKind::Text,
                            &mut self.log.size,
                            &mut self.log.min_ts_to_index,
                        );
                    },
                }
            }
        }
    }
}

/// A typed map from index name to timestamped update vectors.
/// Shared structure for both database and search index maps in the write log.
#[derive(Clone)]
struct WritesByIndex<T: Clone>(
    OrdMap<TabletIndexName, OrdMap<Timestamp, (WithHeapSize<Vector<T>>, WriteSource)>>,
);

impl<T: Clone + HeapSize> WritesByIndex<T> {
    fn new() -> Self {
        Self(OrdMap::new())
    }

    fn append(
        &mut self,
        index: TabletIndexName,
        ts: Timestamp,
        updates: WithHeapSize<Vector<T>>,
        write_source: WriteSource,
        kind: IndexKind,
        by_index_size: &mut usize,
        min_ts_to_index: &mut OrdMap<Timestamp, Vector<(TabletIndexName, IndexKind)>>,
    ) {
        *by_index_size += updates.heap_size();
        match self.0.entry(index.clone()) {
            Entry::Occupied(mut e) => {
                e.get_mut().insert(ts, (updates, write_source));
            },
            Entry::Vacant(e) => {
                let mut inner = OrdMap::new();
                inner.insert(ts, (updates, write_source));
                e.insert(inner);
                min_ts_to_index
                    .entry(ts)
                    .or_default()
                    .push_back((index, kind));
            },
        };
    }

    /// Remove the entry at `ts` for `index`. If the index has remaining
    /// entries, re-register its new minimum timestamp.
    fn remove_at_ts(
        &mut self,
        index: &TabletIndexName,
        ts: Timestamp,
        kind: IndexKind,
        by_index_size: &mut usize,
        min_ts_to_index: &mut OrdMap<Timestamp, Vector<(TabletIndexName, IndexKind)>>,
    ) {
        let Some(inner) = self.0.get_mut(index) else {
            return;
        };
        if let Some((updates, _)) = inner.remove(&ts) {
            *by_index_size = by_index_size.saturating_sub(updates.heap_size());
        }
        if let Some((new_min_ts, _)) = inner.get_min() {
            let new_min_ts = *new_min_ts;
            min_ts_to_index
                .entry(new_min_ts)
                .or_default()
                .push_back((index.clone(), kind));
        } else {
            self.0.remove(index);
        }
    }

    fn iter(
        &self,
    ) -> impl Iterator<
        Item = (
            &TabletIndexName,
            &OrdMap<Timestamp, (WithHeapSize<Vector<T>>, WriteSource)>,
        ),
    > {
        self.0.iter()
    }
}

/// WriteLog holds recent commits that have been written to persistence and
/// snapshot manager. These commits may cause OCC aborts for new commits, and
/// they may trigger subscriptions.
#[derive(Clone)]
struct WriteLog {
    by_database_index: WritesByIndex<DatabaseIndexWrite>,
    by_text_index: WritesByIndex<TextIndexWrite>,
    size: usize,
    /// Keeps track of the minimum timestamps and what indexes have entries in
    /// the maps at those timestamps, used for fast purging. Each entry records
    /// which map (`IndexKind`) the index belongs to so we can remove from the
    /// right map.
    min_ts_to_index: OrdMap<Timestamp, Vector<(TabletIndexName, IndexKind)>>,
    max_ts: Timestamp,
    purged_ts: Timestamp,
}

impl WriteLog {
    fn new(initial_timestamp: Timestamp) -> Self {
        Self {
            by_database_index: WritesByIndex::new(),
            by_text_index: WritesByIndex::new(),
            size: 0,
            min_ts_to_index: OrdMap::new(),
            max_ts: initial_timestamp,
            purged_ts: initial_timestamp,
        }
    }

    fn max_ts(&self) -> Timestamp {
        self.max_ts
    }

    fn is_stale(
        &self,
        reads: &ReadSet,
        reads_ts: Timestamp,
        ts: Timestamp,
    ) -> anyhow::Result<Option<ConflictingReadWithWriteSource>> {
        let from = reads_ts.succ()?;
        anyhow::ensure!(
            from > self.purged_ts,
            anyhow::anyhow!(
                "Timestamp {reads_ts} is outside of write log retention window (minimum timestamp \
                 {})",
                self.purged_ts
            )
            .context(ErrorMetadata::out_of_retention())
        );
        Ok(reads.writes_overlap_by_index(
            &self.by_database_index.0,
            &self.by_text_index.0,
            from,
            ts,
        ))
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
        let max_ts = snapshot.max_ts();
        anyhow::ensure!(
            ts <= max_ts,
            "Can't refresh token to newer timestamp {ts} than max ts {max_ts}"
        );
        snapshot.refresh_token(token, ts)
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

    /// Iterates over all index write log entries in the range [from, to]
    /// (inclusive), calling `f` for each database (index_name, updates) pair
    /// and `g` for each text index (index_name, updates) pair.
    ///
    /// Entries are yielded per-index (not per-document or per-commit). The same
    /// commit may produce entries across multiple index vectors.
    pub fn for_each_index<F, G>(
        &self,
        from: Timestamp,
        to: Timestamp,
        to_notify: &mut BTreeMap<SubscriberId, (Timestamp, Option<WriteSource>, TabletId)>,
        num_index_updates: &mut usize,
        mut f: F,
        mut g: G,
    ) -> anyhow::Result<()>
    where
        F: for<'a> FnMut(
            &'a TabletIndexName,
            Box<
                dyn Iterator<
                        Item = (
                            &'a Timestamp,
                            &'a (WithHeapSize<Vector<DatabaseIndexWrite>>, WriteSource),
                        ),
                    > + 'a,
            >,
            &'a mut BTreeMap<SubscriberId, (Timestamp, Option<WriteSource>, TabletId)>,
            &'a mut usize,
        ),
        G: for<'a> FnMut(
            &'a TabletIndexName,
            Box<
                dyn Iterator<
                        Item = (
                            &'a Timestamp,
                            &'a (WithHeapSize<Vector<TextIndexWrite>>, WriteSource),
                        ),
                    > + 'a,
            >,
            &'a mut BTreeMap<SubscriberId, (Timestamp, Option<WriteSource>, TabletId)>,
            &'a mut usize,
        ),
    {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| {
            anyhow::ensure!(
                from > snapshot.purged_ts,
                anyhow::anyhow!(
                    "Timestamp {from} is outside of write log retention window (minimum timestamp \
                     {})",
                    snapshot.purged_ts
                )
                .context(ErrorMetadata::out_of_retention())
            );
            for (index_name, updates) in snapshot.by_database_index.iter() {
                f(
                    index_name,
                    Box::new(updates.range(from..=to)),
                    to_notify,
                    num_index_updates,
                );
            }
            for (index_name, updates) in snapshot.by_text_index.iter() {
                g(
                    index_name,
                    Box::new(updates.range(from..=to)),
                    to_notify,
                    num_index_updates,
                );
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
            let snapshot = { self.inner.lock().log.clone() };

            if from <= snapshot.purged_ts {
                return Ok(None);
            }

            block_in_place(|| {
                'outer: for (index_name, writes) in snapshot.by_database_index.iter() {
                    let Some(index) = index_registry.get_enabled(index_name) else {
                        continue;
                    };
                    if !cache.is_index_tracked(&index.id()) {
                        continue;
                    }
                    let is_by_id = index.metadata.name.is_by_id();
                    let index_id = index.id();

                    for (ts, (ts_writes, _)) in writes.range(from..=*end_ts) {
                        for write in ts_writes.iter() {
                            if !cache.apply_write(*ts, index_id, is_by_id, write) {
                                break 'outer;
                            }
                        }
                    }
                }
            });
        }

        Ok(Some(TimestampedIndexCache { cache, ts: end_ts }))
    }
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
