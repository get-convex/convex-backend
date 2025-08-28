use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::{
        Bound,
        RangeBounds,
    },
    str::FromStr,
    sync::Arc,
};

use async_trait::async_trait;
use enum_iterator::Sequence;
use futures::{
    future,
    stream::BoxStream,
    try_join,
    StreamExt,
    TryStreamExt,
};
use serde_json::Value as JsonValue;
use value::{
    InternalDocumentId,
    TabletId,
};

use crate::{
    document::ResolvedDocument,
    index::{
        IndexEntry,
        IndexKey,
        IndexKeyBytes,
    },
    interval::Interval,
    knobs::DEFAULT_DOCUMENTS_PAGE_SIZE,
    metrics::static_repeatable_ts_timer,
    persistence_helpers::RevisionPair,
    query::Order,
    runtime::Runtime,
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        IndexId,
        PersistenceVersion,
        RepeatableReason,
        RepeatableTimestamp,
        Timestamp,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentLogEntry {
    pub ts: Timestamp,
    pub id: InternalDocumentId,
    pub value: Option<ResolvedDocument>,
    pub prev_ts: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PersistenceIndexEntry {
    pub ts: Timestamp,
    pub index_id: IndexId,
    pub key: IndexKeyBytes,
    pub value: Option<InternalDocumentId>,
}

impl PersistenceIndexEntry {
    pub fn from_index_update(ts: Timestamp, update: DatabaseIndexUpdate) -> Self {
        Self {
            ts,
            index_id: update.index_id,
            key: update.key.to_bytes(),
            value: match update.value {
                DatabaseIndexValue::Deleted => None,
                DatabaseIndexValue::NonClustered(id) => {
                    Some(InternalDocumentId::new(id.tablet_id, id.internal_id()))
                },
            },
        }
    }
}

pub type DocumentStream<'a> = BoxStream<'a, anyhow::Result<DocumentLogEntry>>;

pub type DocumentRevisionStream<'a> = BoxStream<'a, anyhow::Result<RevisionPair>>;

/// No tombstones included
pub type LatestDocumentStream<'a> = BoxStream<'a, anyhow::Result<LatestDocument>>;

pub type IndexStream<'a> = BoxStream<'a, anyhow::Result<(IndexKeyBytes, LatestDocument)>>;

/// A `DocumentLogEntry` that is not a tombstone.
#[derive(Debug, Clone, PartialEq)]
pub struct LatestDocument {
    pub ts: Timestamp,
    pub value: ResolvedDocument,
    pub prev_ts: Option<Timestamp>,
}

/// Indicates how write conflicts should be handled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictStrategy {
    /// If the record being written already exists with the same key, return an
    /// error and abort the write.
    Error,
    /// If the record being written already exists with the same key, overwrite
    /// the record.
    Overwrite,
}

// When adding a new persistence global, make sure it's copied
// or computed in migrate_db_cluster/text_index_worker.
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Sequence)]
pub enum PersistenceGlobalKey {
    /// Minimum snapshot that is retained. Data in earlier snapshots may have
    /// been deleted.
    RetentionMinSnapshotTimestamp,

    /// Timestamp for a snapshot that has been deleted by retention.
    /// This is used as a cursor by retention, bumped after retention deletes
    /// entries at the snapshot.
    RetentionConfirmedDeletedTimestamp,

    /// Minimum timestamp for valid write-ahead log
    DocumentRetentionMinSnapshotTimestamp,

    /// Timestamp for a document that has been deleted by retention.
    /// This is used as a cursor by document retention, bumped after retention
    /// deletes entries at a timestamp.
    DocumentRetentionConfirmedDeletedTimestamp,

    /// Maximum snapshot that is repeatable. All future commits will have
    /// timestamp > this timestamp.
    MaxRepeatableTimestamp,

    /// Latest snapshot of all tables' summaries, cached to speed up startup.
    TableSummary,

    /// Internal id of _tables.by_id index, for bootstrapping.
    TablesByIdIndex,
    /// Internal id of _tables table, for bootstrapping.
    TablesTabletId,
    /// Internal id of _index.by_id index, for bootstrapping.
    IndexByIdIndex,
    /// Internal id of _index table, for bootstrapping.
    IndexTabletId,
}

impl From<PersistenceGlobalKey> for String {
    fn from(key: PersistenceGlobalKey) -> Self {
        match key {
            PersistenceGlobalKey::RetentionMinSnapshotTimestamp => "min_snapshot_ts".to_string(),
            PersistenceGlobalKey::RetentionConfirmedDeletedTimestamp => {
                "confirmed_deleted_ts".to_string()
            },
            PersistenceGlobalKey::DocumentRetentionMinSnapshotTimestamp => {
                "document_min_snapshot_ts".to_string()
            },
            PersistenceGlobalKey::DocumentRetentionConfirmedDeletedTimestamp => {
                "document_confirmed_deleted_ts".to_string()
            },
            PersistenceGlobalKey::MaxRepeatableTimestamp => "max_repeatable_ts".to_string(),
            PersistenceGlobalKey::TableSummary => "table_summary_v2".to_string(),
            PersistenceGlobalKey::TablesByIdIndex => "tables_by_id".to_string(),
            PersistenceGlobalKey::IndexByIdIndex => "index_by_id".to_string(),
            // NB: For compatibility, these are referred to as "table_id"s, not "tablet_id"s.
            PersistenceGlobalKey::TablesTabletId => "tables_table_id".to_string(),
            PersistenceGlobalKey::IndexTabletId => "index_table_id".to_string(),
        }
    }
}
impl FromStr for PersistenceGlobalKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "min_snapshot_ts" => Ok(Self::RetentionMinSnapshotTimestamp),
            "confirmed_deleted_ts" => Ok(Self::RetentionConfirmedDeletedTimestamp),
            "document_min_snapshot_ts" => Ok(Self::DocumentRetentionMinSnapshotTimestamp),
            "document_confirmed_deleted_ts" => Ok(Self::DocumentRetentionConfirmedDeletedTimestamp),
            "max_repeatable_ts" => Ok(Self::MaxRepeatableTimestamp),
            "table_summary_v2" => Ok(Self::TableSummary),
            "tables_by_id" => Ok(Self::TablesByIdIndex),
            "tables_table_id" => Ok(Self::TablesTabletId),
            "index_by_id" => Ok(Self::IndexByIdIndex),
            "index_table_id" => Ok(Self::IndexTabletId),
            _ => anyhow::bail!("unrecognized persistence global key"),
        }
    }
}

impl PersistenceGlobalKey {
    pub fn all_keys() -> Vec<Self> {
        enum_iterator::all().collect()
    }
}

#[async_trait]
pub trait Persistence: Sync + Send + 'static {
    /// Whether the persistence layer is freshely created or not.
    fn is_fresh(&self) -> bool;

    fn reader(&self) -> Arc<dyn PersistenceReader>;

    /// Writes documents and the respective derived indexes.
    async fn write(
        &self,
        documents: Vec<DocumentLogEntry>,
        indexes: BTreeSet<PersistenceIndexEntry>,
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()>;

    async fn set_read_only(&self, read_only: bool) -> anyhow::Result<()>;

    /// Writes global key-value data for the whole persistence.
    /// This is expected to be small data that does not make sense in a
    /// versioned or transaction context. See `PersistenceGlobalKey`.
    async fn write_persistence_global(
        &self,
        key: PersistenceGlobalKey,
        value: JsonValue,
    ) -> anyhow::Result<()>;

    async fn load_index_chunk(
        &self,
        cursor: Option<IndexEntry>,
        chunk_size: usize,
    ) -> anyhow::Result<Vec<IndexEntry>>;

    async fn delete_index_entries(&self, entries: Vec<IndexEntry>) -> anyhow::Result<usize>;

    // Deletes documents
    async fn delete(
        &self,
        documents: Vec<(Timestamp, InternalDocumentId)>,
    ) -> anyhow::Result<usize>;

    // No-op by default. Persistence implementation can override.
    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn import_documents_batch(
        &self,
        mut documents: BoxStream<'_, Vec<DocumentLogEntry>>,
    ) -> anyhow::Result<()> {
        while let Some(chunk) = documents.next().await {
            self.write(chunk, BTreeSet::new(), ConflictStrategy::Error)
                .await?;
        }
        Ok(())
    }

    async fn import_indexes_batch(
        &self,
        mut indexes: BoxStream<'_, BTreeSet<PersistenceIndexEntry>>,
    ) -> anyhow::Result<()> {
        while let Some(chunk) = indexes.next().await {
            self.write(vec![], chunk, ConflictStrategy::Error).await?;
        }
        Ok(())
    }

    async fn finish_loading(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimestampRange {
    start_bound: Bound<Timestamp>,
    end_bound: Bound<Timestamp>,
}

impl TimestampRange {
    pub fn new<T: RangeBounds<Timestamp>>(range: T) -> anyhow::Result<Self> {
        // Bounds check.
        Self::min_inclusive(&range.start_bound().cloned())?;
        Self::max_exclusive(&range.end_bound().cloned())?;
        Ok(Self {
            start_bound: range.start_bound().cloned(),
            end_bound: range.end_bound().cloned(),
        })
    }

    pub fn snapshot(ts: Timestamp) -> Self {
        Self {
            start_bound: Bound::Unbounded,
            end_bound: Bound::Included(ts),
        }
    }

    pub fn all() -> Self {
        Self {
            start_bound: Bound::Unbounded,
            end_bound: Bound::Unbounded,
        }
    }

    pub fn at(ts: Timestamp) -> Self {
        Self {
            start_bound: Bound::Included(ts),
            end_bound: Bound::Included(ts),
        }
    }

    pub fn greater_than(t: Timestamp) -> Self {
        Self {
            start_bound: Bound::Excluded(t),
            end_bound: Bound::Unbounded,
        }
    }

    fn min_inclusive(start_bound: &Bound<Timestamp>) -> anyhow::Result<Timestamp> {
        Ok(match start_bound {
            Bound::Included(t) => *t,
            Bound::Excluded(t) => t.succ()?,
            Bound::Unbounded => Timestamp::MIN,
        })
    }

    pub fn min_timestamp_inclusive(&self) -> Timestamp {
        Self::min_inclusive(&self.start_bound).unwrap()
    }

    fn max_exclusive(end_bound: &Bound<Timestamp>) -> anyhow::Result<Timestamp> {
        Ok(match end_bound {
            Bound::Included(t) => t.succ()?,
            Bound::Excluded(t) => *t,
            Bound::Unbounded => Timestamp::MAX,
        })
    }

    pub fn max_timestamp_exclusive(&self) -> Timestamp {
        Self::max_exclusive(&self.end_bound).unwrap()
    }

    pub fn contains(&self, ts: Timestamp) -> bool {
        self.min_timestamp_inclusive() <= ts && ts < self.max_timestamp_exclusive()
    }
}

#[async_trait]
pub trait RetentionValidator: Sync + Send {
    /// Call optimistic_validate_snapshot *before* reading at the snapshot,
    /// to confirm all data in the snapshot may be within retention, so it's
    /// worth continuing.
    fn optimistic_validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()>;
    /// Call validate_snapshot *after* reading at the snapshot, to confirm all
    /// data in the snapshot is within retention.
    async fn validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()>;
    /// Call validate_document_snapshot *after* reading at the snapshot, to
    /// confirm the documents log is valid at this snapshot.
    async fn validate_document_snapshot(&self, ts: Timestamp) -> anyhow::Result<()>;
    async fn min_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp>;
    async fn min_document_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp>;

    fn fail_if_falling_behind(&self) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentPrevTsQuery {
    pub id: InternalDocumentId,
    pub ts: Timestamp,
    pub prev_ts: Timestamp,
}

#[async_trait]
pub trait PersistenceReader: Send + Sync + 'static {
    /// The persistence is required to load documents within the given timestamp
    /// range.
    /// page_size is how many documents to fetch with a single query. It doesn't
    /// affect load_documents results, just efficiency of the internal queries.
    fn load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_>;

    /// Loads documents within the given table and the given timestamp range.
    ///
    /// page_size is how many documents to fetch with a single query. It doesn't
    /// affect load_documents results, just efficiency of the internal queries.
    ///
    /// NOTE: The filter is implemented entirely in memory. We can potentially
    /// add indexes to the documents table to allow for an efficient database
    /// version of this query, but have not yet done so.
    fn load_documents_from_table(
        &self,
        tablet_id: TabletId,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.load_documents(range, order, page_size, retention_validator)
            .try_filter(move |doc| future::ready(doc.id.table() == tablet_id))
            .boxed()
    }

    /// Look up the previous revision of `(id, ts)`, returning a map where for
    /// each `(id, ts)` we have...
    ///
    /// 1. no value: there are no revisions of `id` before ts.
    /// 2. (prev_ts, None): the previous revision is a delete @ prev_ts.
    /// 3. (prev_ts, Some(document)): the previous revision @ prev_ts.
    async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>>;

    /// Look up documents at exactly the specified prev_ts timestamps, returning
    /// a map where for each `DocumentPrevTsQuery` we have an entry only if
    /// a document exists at `(id, prev_ts)`.
    async fn previous_revisions_of_documents(
        &self,
        ids: BTreeSet<DocumentPrevTsQuery>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<DocumentPrevTsQuery, DocumentLogEntry>>;

    /// Loads documentIds with respective timestamps that match the
    /// index query criteria.
    /// `size_hint` is a best-effort estimate of the number of
    /// rows to be consumed from returned stream. This argument should only be
    /// used to tune batching in order to balance round trips and redundant
    /// queries. The returned stream should always yield the same results if
    /// fully consumed regardless of the estimate.
    fn index_scan(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        range: &Interval,
        order: Order,
        size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> IndexStream<'_>;

    async fn get_persistence_global(
        &self,
        key: PersistenceGlobalKey,
    ) -> anyhow::Result<Option<JsonValue>>;

    /// Performs a single point get using an index.
    async fn index_get(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        key: IndexKey,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<Option<LatestDocument>> {
        let mut stream = self.index_scan(
            index_id,
            tablet_id,
            read_timestamp,
            &Interval::prefix(key.to_bytes().into()),
            Order::Asc,
            2,
            retention_validator,
        );
        match stream.try_next().await? {
            Some((key, rev)) => {
                anyhow::ensure!(
                    stream.try_next().await?.is_none(),
                    "Got multiple values for key {:?}",
                    key
                );
                Ok(Some(rev))
            },
            None => Ok(None),
        }
    }

    /// max_ts is the largest timestamp written to persistence.
    /// It's not necessarily safe to read snapshots at this timestamp.
    /// Use a RepeatableTimestamp constructor to find a safe timestamp for
    /// reads. It is safe to read at max_ts iff there are no ongoing
    /// commits, e.g. when a database is loading and has acquired the lease
    /// but not begun commits.
    async fn max_ts(&self) -> anyhow::Result<Option<Timestamp>> {
        // Fetch the document with the maximum timestamp and also MaxRepeatableTimestamp
        // in parallel.
        let mut stream = self.load_documents(
            TimestampRange::all(),
            Order::Desc,
            1,
            // We don't know the ID of the most recent document, so we
            // need to scan the entire timestamp range to find it
            // (this may include looking at the `documents` log outside of the retention window)
            Arc::new(NoopRetentionValidator),
        );
        let max_repeatable =
            self.get_persistence_global(PersistenceGlobalKey::MaxRepeatableTimestamp);
        let (max_committed, max_repeatable) = try_join!(stream.try_next(), max_repeatable)?;
        let max_committed_ts = max_committed.map(|entry| entry.ts);
        let max_repeatable_ts = max_repeatable.map(Timestamp::try_from).transpose()?;
        let max_ts = cmp::max(max_committed_ts, max_repeatable_ts); // note None < Some
        Ok(max_ts)
    }

    fn version(&self) -> PersistenceVersion;

    async fn table_size_stats(&self) -> anyhow::Result<Vec<PersistenceTableSize>> {
        Ok(vec![])
    }

    /// Returns all timestamps and documents in ascending (ts, tablet_id, id)
    /// order. Only should be used for testing
    #[cfg(any(test, feature = "testing"))]
    fn load_all_documents(&self) -> DocumentStream<'_> {
        self.load_documents(
            TimestampRange::all(),
            Order::Asc,
            *DEFAULT_DOCUMENTS_PAGE_SIZE,
            Arc::new(NoopRetentionValidator),
        )
    }
}

pub fn now_ts<RT: Runtime>(max_ts: Timestamp, rt: &RT) -> anyhow::Result<Timestamp> {
    let ts = cmp::max(rt.generate_timestamp()?, max_ts);
    Ok(ts)
}

/// Timestamp that is repeatable because the caller is holding the lease and
/// no one is writing to persistence. In particular the Committer is not
/// running. So all future commits will be after the returned
/// IdleRepeatableTimestamp (even when commits write in parallel). e.g. this can
/// be used on database load.
pub async fn new_idle_repeatable_ts<RT: Runtime>(
    persistence: &dyn Persistence,
    rt: &RT,
) -> anyhow::Result<RepeatableTimestamp> {
    let reader = persistence.reader();
    let max_ts = reader.max_ts().await?.unwrap_or(Timestamp::MIN);
    let now = now_ts(max_ts, rt)?;
    // Enforce that all subsequent commits are > now by writing to MaxRepeatableTs.
    persistence
        .write_persistence_global(PersistenceGlobalKey::MaxRepeatableTimestamp, now.into())
        .await?;
    Ok(RepeatableTimestamp::new_validated(
        now,
        RepeatableReason::IdleMaxTs,
    ))
}

/// RepeatablePersistence can read from Persistence at a range of snapshots
/// <= the given snapshot. Therefore reads from RepeatablePersistence
/// will not see new writes, i.e. all reads will see the same data.
#[derive(Clone)]
pub struct RepeatablePersistence {
    reader: Arc<dyn PersistenceReader>,
    upper_bound: RepeatableTimestamp,
    retention_validator: Arc<dyn RetentionValidator>,
}

impl RepeatablePersistence {
    pub fn new(
        reader: Arc<dyn PersistenceReader>,
        upper_bound: RepeatableTimestamp,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> Self {
        Self {
            reader,
            upper_bound,
            retention_validator,
        }
    }

    pub fn upper_bound(&self) -> RepeatableTimestamp {
        self.upper_bound
    }

    /// Same as [`Persistence::load_documents`] but only including documents in
    /// the snapshot range.
    pub fn load_documents(&self, range: TimestampRange, order: Order) -> DocumentStream<'_> {
        let stream = self.reader.load_documents(
            range,
            order,
            *DEFAULT_DOCUMENTS_PAGE_SIZE,
            self.retention_validator.clone(),
        );
        Box::pin(stream.try_filter(|entry| future::ready(entry.ts <= *self.upper_bound)))
    }

    /// Same as [`Persistence::load_documents_from_table`] but only including
    /// documents in the snapshot range.
    pub fn load_documents_from_table(
        &self,
        tablet_id: TabletId,
        range: TimestampRange,
        order: Order,
    ) -> DocumentStream<'_> {
        let stream = self.reader.load_documents_from_table(
            tablet_id,
            range,
            order,
            *DEFAULT_DOCUMENTS_PAGE_SIZE,
            self.retention_validator.clone(),
        );
        Box::pin(stream.try_filter(|entry| future::ready(entry.ts <= *self.upper_bound)))
    }

    /// Same as `load_documents` but doesn't use the `RetentionValidator` from
    /// this `RepeatablePersistence`. Instead, the caller can choose its
    /// own validator.
    pub fn load_documents_with_retention_validator(
        &self,
        range: TimestampRange,
        order: Order,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        let stream = self.reader.load_documents(
            range,
            order,
            *DEFAULT_DOCUMENTS_PAGE_SIZE,
            retention_validator,
        );
        Box::pin(stream.try_filter(|entry| future::ready(entry.ts <= *self.upper_bound)))
    }

    pub async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>> {
        for (_, ts) in &ids {
            // Reading documents <ts, so ts-1 needs to be repeatable.
            anyhow::ensure!(*ts <= self.upper_bound.succ()?);
        }
        self.reader
            .previous_revisions(ids, self.retention_validator.clone())
            .await
    }

    /// Allows a retention validator to be explicitly passed in
    pub async fn previous_revisions_with_validator(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>> {
        for (_, ts) in &ids {
            // Reading documents <ts, so ts-1 needs to be repeatable.
            anyhow::ensure!(*ts <= self.upper_bound.succ()?);
        }
        self.reader
            .previous_revisions(ids, retention_validator.clone())
            .await
    }

    pub async fn previous_revisions_of_documents(
        &self,
        ids: BTreeSet<DocumentPrevTsQuery>,
    ) -> anyhow::Result<BTreeMap<DocumentPrevTsQuery, DocumentLogEntry>> {
        for DocumentPrevTsQuery { prev_ts, .. } in &ids {
            // Reading documents with timestamp prev_ts, so it needs to be repeatable.
            anyhow::ensure!(*prev_ts <= self.upper_bound);
        }
        self.reader
            .previous_revisions_of_documents(ids, self.retention_validator.clone())
            .await
    }

    pub fn read_snapshot(&self, at: RepeatableTimestamp) -> anyhow::Result<PersistenceSnapshot> {
        anyhow::ensure!(at <= self.upper_bound);
        self.retention_validator.optimistic_validate_snapshot(*at)?;
        Ok(PersistenceSnapshot {
            reader: self.reader.clone(),
            at,
            retention_validator: self.retention_validator.clone(),
        })
    }

    pub fn version(&self) -> PersistenceVersion {
        self.reader.version()
    }
}

async fn read_max_repeatable_ts(
    reader: &dyn PersistenceReader,
) -> anyhow::Result<Option<Timestamp>> {
    let value = reader
        .get_persistence_global(PersistenceGlobalKey::MaxRepeatableTimestamp)
        .await?;
    value.map(Timestamp::try_from).transpose()
}

/// This timestamp is determined to be repeatable by reading max_repeatable_ts
/// from persistence. It may be lagging a few minutes behind live writes.
/// It is expected only to be called from background tasks that don't need to
/// read recent writes.
pub async fn new_static_repeatable_recent(
    reader: &dyn PersistenceReader,
) -> anyhow::Result<RepeatableTimestamp> {
    let _timer = static_repeatable_ts_timer(true);
    match read_max_repeatable_ts(reader).await? {
        None => Ok(RepeatableTimestamp::MIN),
        Some(ts) => Ok(RepeatableTimestamp::new_validated(
            ts,
            RepeatableReason::MaxRepeatableTsPersistence,
        )),
    }
}

/// PersistenceSnapshot can perform reads from Persistence at a given
/// snapshot.
#[derive(Clone)]
pub struct PersistenceSnapshot {
    reader: Arc<dyn PersistenceReader>,
    at: RepeatableTimestamp,
    retention_validator: Arc<dyn RetentionValidator>,
}

impl PersistenceSnapshot {
    /// Same as [`Persistence::index_scan`] but with fixed timestamp.
    pub fn index_scan(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        interval: &Interval,
        order: Order,
        size_hint: usize,
    ) -> IndexStream<'_> {
        self.reader
            .index_scan(
                index_id,
                tablet_id,
                *self.at,
                interval,
                order,
                size_hint,
                self.retention_validator.clone(),
            )
            .boxed()
    }

    /// Same as [`Persistence::index_get`] but with fixed timestamp.
    pub async fn index_get(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        key: IndexKey,
    ) -> anyhow::Result<Option<LatestDocument>> {
        let result = self
            .reader
            .index_get(
                index_id,
                tablet_id,
                *self.at,
                key,
                self.retention_validator.clone(),
            )
            .await?;
        Ok(result)
    }

    pub fn timestamp(&self) -> RepeatableTimestamp {
        self.at
    }

    pub fn persistence(&self) -> &dyn PersistenceReader {
        self.reader.as_ref()
    }
}

/// Test-only snapshot validator that doesn't validate anything.
/// Prod and most tests should use (Follower|Leader)RetentionManager,
#[derive(Clone, Copy)]
pub struct NoopRetentionValidator;

#[async_trait]
impl RetentionValidator for NoopRetentionValidator {
    fn optimistic_validate_snapshot(&self, _ts: Timestamp) -> anyhow::Result<()> {
        Ok(())
    }

    async fn validate_snapshot(&self, _ts: Timestamp) -> anyhow::Result<()> {
        Ok(())
    }

    async fn validate_document_snapshot(&self, _ts: Timestamp) -> anyhow::Result<()> {
        Ok(())
    }

    async fn min_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        Ok(RepeatableTimestamp::MIN)
    }

    async fn min_document_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
        Ok(RepeatableTimestamp::MIN)
    }

    fn fail_if_falling_behind(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod fake_retention_validator {
    use async_trait::async_trait;
    use sync_types::Timestamp;

    use super::RetentionValidator;
    use crate::types::{
        unchecked_repeatable_ts,
        RepeatableTimestamp,
    };

    #[derive(Clone, Copy)]
    pub struct FakeRetentionValidator {
        pub min_index_ts: RepeatableTimestamp,
        pub min_document_ts: RepeatableTimestamp,
    }

    impl FakeRetentionValidator {
        pub fn new(min_index_ts: Timestamp, min_document_ts: Timestamp) -> Self {
            Self {
                min_index_ts: unchecked_repeatable_ts(min_index_ts),
                min_document_ts: unchecked_repeatable_ts(min_document_ts),
            }
        }
    }

    #[async_trait]
    impl RetentionValidator for FakeRetentionValidator {
        fn optimistic_validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
            anyhow::ensure!(ts >= self.min_index_ts);
            Ok(())
        }

        async fn validate_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
            anyhow::ensure!(ts >= self.min_index_ts);
            Ok(())
        }

        async fn validate_document_snapshot(&self, ts: Timestamp) -> anyhow::Result<()> {
            anyhow::ensure!(ts >= self.min_document_ts);
            Ok(())
        }

        async fn min_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
            Ok(self.min_index_ts)
        }

        async fn min_document_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
            Ok(self.min_document_ts)
        }

        fn fail_if_falling_behind(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct PersistenceTableSize {
    /// The name of the underlying persistence table
    pub table_name: String,
    pub data_bytes: u64,
    pub index_bytes: u64,
    pub row_count: Option<u64>,
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_persistence_global_roundtrips(key in any::<PersistenceGlobalKey>()) {
            let s: String = key.into();
            let parse_key = s.parse().unwrap();
            assert_eq!(key, parse_key);
        }
    }
}
