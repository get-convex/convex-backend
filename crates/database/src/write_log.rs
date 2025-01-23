use std::{
    borrow::Cow,
    collections::{
        btree_map,
        BTreeMap,
        VecDeque,
    },
    mem,
    sync::Arc,
};

use common::{
    document::{
        DocumentUpdate,
        PackedDocument,
    },
    knobs::{
        WRITE_LOG_MAX_RETENTION_SECS,
        WRITE_LOG_MIN_RETENTION_SECS,
        WRITE_LOG_SOFT_MAX_SIZE_BYTES,
    },
    runtime::block_in_place,
    types::{
        PersistenceVersion,
        Timestamp,
    },
    value::ResolvedDocumentId,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use futures::Future;
use imbl::{
    vector,
    Vector,
};
use parking_lot::Mutex;
use tokio::sync::oneshot;
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

use crate::{
    database::ConflictingReadWithWriteSource,
    metrics,
    reads::ReadSet,
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
        self.old_document.heap_size() + self.new_document.heap_size() + mem::size_of::<bool>()
    }
}

type Writes = WithHeapSize<BTreeMap<ResolvedDocumentId, PackedDocumentUpdate>>;
type OrderedWrites = WithHeapSize<Vec<(ResolvedDocumentId, PackedDocumentUpdate)>>;

impl PackedDocumentUpdate {
    pub fn pack(update: DocumentUpdate) -> Self {
        Self {
            id: update.id,
            old_document: update.old_document.map(PackedDocument::pack),
            new_document: update.new_document.map(PackedDocument::pack),
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

pub type IterWrites<'a> = btree_map::Iter<'a, ResolvedDocumentId, PackedDocumentUpdate>;

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
    fn new(initial_timestamp: Timestamp, persistence_version: PersistenceVersion) -> Self {
        let log = WriteLog::new(initial_timestamp, persistence_version);
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

    fn append(&mut self, ts: Timestamp, writes: Writes, write_source: WriteSource) {
        assert!(self.log.max_ts() < ts, "{:?} >= {}", self.log.max_ts(), ts);

        self.log
            .by_ts
            .push_back(Arc::new((ts, writes, write_source)));

        self.notify_waiters();
    }

    /// Returns a future that blocks until the log has advanced past the given
    /// timestamp.
    fn wait_for_higher_ts(&mut self, target_ts: Timestamp) -> impl Future<Output = ()> {
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
    by_ts: WithHeapSize<Vector<Arc<(Timestamp, Writes, WriteSource)>>>,
    purged_ts: Timestamp,
    persistence_version: PersistenceVersion,
}

impl WriteLog {
    fn new(initial_timestamp: Timestamp, persistence_version: PersistenceVersion) -> Self {
        Self {
            by_ts: WithHeapSize::default(),
            purged_ts: initial_timestamp,
            persistence_version,
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
        let focus = focus_after(self.by_ts.focus(), start);
        let iter = focus.into_iter();
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
            Ok(reads.writes_overlap(log_range, self.persistence_version))
        })
    }

    fn refresh_token(&self, mut token: Token, ts: Timestamp) -> anyhow::Result<Option<Token>> {
        metrics::log_read_set_age(ts.secs_since_f64(token.ts()).max(0.0));
        let result = match self.is_stale(token.reads(), token.ts(), ts) {
            Ok(Some(_)) => None,
            Err(e) if e.is_out_of_retention() => {
                metrics::log_reads_refresh_miss();
                None
            },
            Err(e) => return Err(e),
            Ok(None) => {
                if token.ts() < ts {
                    token.advance_ts(ts);
                }
                Some(token)
            },
        };
        Ok(result)
    }
}

// TODO: use .narrow(start..) once https://github.com/jneem/imbl/pull/89 is released
fn focus_after<T>(focus: vector::Focus<'_, T>, start: usize) -> vector::Focus<'_, T> {
    if focus.is_empty() {
        // split_at and narrow don't work if the vector is empty
        focus
    } else if focus.len() == start {
        // narrow doesn't work if start is at the end
        focus.split_at(0).0
    } else {
        focus.narrow(start..)
    }
}

pub fn new_write_log(
    initial_timestamp: Timestamp,
    persistence_version: PersistenceVersion,
) -> (LogOwner, LogReader, LogWriter) {
    let log_manager = Arc::new(Mutex::new(WriteLogManager::new(
        initial_timestamp,
        persistence_version,
    )));
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

    pub fn max_ts(&self) -> Timestamp {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| snapshot.max_ts())
    }

    pub fn refresh_token(&self, token: Token, ts: Timestamp) -> anyhow::Result<Option<Token>> {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| snapshot.refresh_token(token, ts))
    }

    /// Blocks until the log has advanced past the given timestamp.
    pub async fn wait_for_higher_ts(&mut self, target_ts: Timestamp) -> Timestamp {
        let fut = block_in_place(|| self.inner.lock().wait_for_higher_ts(target_ts));
        fut.await;
        let result = block_in_place(|| self.inner.lock().log.max_ts());
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
}

#[derive(Clone)]
pub struct LogReader {
    inner: Arc<Mutex<WriteLogManager>>,
}

impl LogReader {
    #[fastrace::trace]
    pub fn refresh_token(&self, token: Token, ts: Timestamp) -> anyhow::Result<Option<Token>> {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| snapshot.refresh_token(token, ts))
    }

    pub fn refresh_reads_until_max_ts(&self, token: Token) -> anyhow::Result<Option<Token>> {
        let snapshot = { self.inner.lock().log.clone() };
        block_in_place(|| {
            let max_ts = snapshot.max_ts();
            snapshot.refresh_token(token, max_ts)
        })
    }
}

/// LogWriter can append to the log.
pub struct LogWriter {
    inner: Arc<Mutex<WriteLogManager>>,
}

impl LogWriter {
    pub fn append(&mut self, ts: Timestamp, writes: Writes, write_source: WriteSource) {
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
    by_ts: WithHeapSize<BTreeMap<Timestamp, (OrderedWrites, WriteSource)>>,
    persistence_version: PersistenceVersion,
}

impl PendingWrites {
    pub fn new(persistence_version: PersistenceVersion) -> Self {
        Self {
            by_ts: WithHeapSize::default(),
            persistence_version,
        }
    }

    pub fn push_back(
        &mut self,
        ts: Timestamp,
        writes: OrderedWrites,
        write_source: WriteSource,
    ) -> PendingWriteHandle {
        if let Some((last_ts, _)) = self.by_ts.iter().next_back() {
            assert!(*last_ts < ts, "{:?} >= {}", *last_ts, ts);
        }

        self.by_ts.insert(ts, (writes, write_source));
        PendingWriteHandle(Some(ts))
    }

    pub fn iter(
        &self,
        from: Timestamp,
        to: Timestamp,
    ) -> impl Iterator<
        Item = (
            &Timestamp,
            impl Iterator<Item = (&ResolvedDocumentId, &PackedDocumentUpdate)>,
            &WriteSource,
        ),
    > {
        self.by_ts
            .range(from..=to)
            .map(|(ts, (w, source))| (ts, w.iter().map(|(id, update)| (id, update)), source))
    }

    pub fn is_stale(
        &self,
        reads: &ReadSet,
        reads_ts: Timestamp,
        ts: Timestamp,
    ) -> anyhow::Result<Option<ConflictingReadWithWriteSource>> {
        Ok(reads.writes_overlap(self.iter(reads_ts.succ()?, ts), self.persistence_version))
    }

    pub fn pop_first(
        &mut self,
        mut handle: PendingWriteHandle,
    ) -> Option<(Timestamp, OrderedWrites, WriteSource)> {
        let first = self.by_ts.pop_first();
        if let Some((ts, (writes, write_source))) = first {
            if let Some(expected_ts) = handle.0 {
                if ts == expected_ts {
                    handle.0.take();
                }
            }
            Some((ts, writes, write_source))
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
    use common::{
        assert_obj,
        document::{
            CreationTime,
            DocumentUpdate,
            ResolvedDocument,
        },
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
            IndexDescriptor,
            PersistenceVersion,
            TabletIndexName,
            Timestamp,
        },
        value::FieldPath,
    };
    use convex_macro::test_runtime;
    use maplit::btreemap;
    use runtime::testing::TestRuntime;
    use value::val;

    use crate::{
        reads::{
            ReadSet,
            TransactionReadSet,
        },
        write_log::{
            PackedDocumentUpdate,
            WriteLogManager,
            WriteSource,
        },
    };

    #[test]
    fn test_write_log() -> anyhow::Result<()> {
        let mut log_manager =
            WriteLogManager::new(Timestamp::must(1000), PersistenceVersion::default());
        assert_eq!(log_manager.log.purged_ts, Timestamp::must(1000));
        assert_eq!(log_manager.log.max_ts(), Timestamp::must(1000));

        for ts in (1002..=1010).step_by(2) {
            log_manager.append(
                Timestamp::must(ts),
                btreemap!().into(),
                WriteSource::unknown(),
            );
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
        let mut log_manager =
            WriteLogManager::new(Timestamp::must(1000), PersistenceVersion::default());
        let table_id = id_generator.user_table_id(&"t".parse()?).tablet_id;
        let id = id_generator.user_generate(&"t".parse()?);
        let index_key = IndexKey::new(vec![val!(5)], id.into());
        let index_key_binary: BinaryKey = index_key.into_bytes().into();
        let index_name =
            TabletIndexName::new(table_id, IndexDescriptor::new("by_k").unwrap()).unwrap();
        let doc = ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("k" => 5))?;
        log_manager.append(
            Timestamp::must(1003),
            btreemap! {
                id => PackedDocumentUpdate::pack(DocumentUpdate {
                    id,
                    old_document: None,
                    new_document: Some(doc.clone()),
                }),
            }
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

        let mut delete_log_manager =
            WriteLogManager::new(Timestamp::must(1000), PersistenceVersion::default());
        delete_log_manager.append(
            Timestamp::must(1003),
            btreemap! {
                id => PackedDocumentUpdate::pack(DocumentUpdate {
                    id,
                    old_document: Some(doc),
                    new_document: None,
                }),
            }
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
}
