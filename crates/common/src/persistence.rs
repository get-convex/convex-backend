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
    time::Duration,
};

use async_trait::async_trait;
use enum_iterator::{
    self,
    Sequence,
};
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
    TableId,
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
    query::Order,
    runtime::Runtime,
    types::{
        DatabaseIndexUpdate,
        IndexId,
        PersistenceVersion,
        RepeatableReason,
        RepeatableTimestamp,
        Timestamp,
    },
};

pub type DocumentStream<'a> =
    BoxStream<'a, anyhow::Result<(Timestamp, InternalDocumentId, Option<ResolvedDocument>)>>;

/// No tombstones included
pub type LatestDocumentStream<'a> = BoxStream<'a, anyhow::Result<(Timestamp, ResolvedDocument)>>;

pub type IndexStream<'a> =
    BoxStream<'a, anyhow::Result<(IndexKeyBytes, Timestamp, ResolvedDocument)>>;

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
// or computed in migrate_db_cluster/mod.rs.
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
    TablesTableId,
    /// Internal id of _index.by_id index, for bootstrapping.
    IndexByIdIndex,
    /// Internal id of _index table, for bootstrapping.
    IndexTableId,
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
            PersistenceGlobalKey::TablesTableId => "tables_table_id".to_string(),
            PersistenceGlobalKey::IndexByIdIndex => "index_by_id".to_string(),
            PersistenceGlobalKey::IndexTableId => "index_table_id".to_string(),
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
            "tables_table_id" => Ok(Self::TablesTableId),
            "index_by_id" => Ok(Self::IndexByIdIndex),
            "index_table_id" => Ok(Self::IndexTableId),
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

    fn reader(&self) -> Box<dyn PersistenceReader>;

    /// Writes documents and the respective derived indexes.
    async fn write(
        &self,
        documents: Vec<(Timestamp, InternalDocumentId, Option<ResolvedDocument>)>,
        indexes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()>;

    async fn set_read_only(&mut self, read_only: bool) -> anyhow::Result<()>;

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

    async fn index_entries_to_delete(
        &self,
        expired_entries: &Vec<IndexEntry>,
    ) -> anyhow::Result<Vec<IndexEntry>>;
    async fn delete_index_entries(&self, entries: Vec<IndexEntry>) -> anyhow::Result<usize>;

    // Retrieves expired documents
    async fn documents_to_delete(
        &self,
        expired_documents: &Vec<(Timestamp, InternalDocumentId)>,
    ) -> anyhow::Result<Vec<(Timestamp, InternalDocumentId)>>;

    // Deletes documents
    async fn delete(
        &self,
        documents: Vec<(Timestamp, InternalDocumentId)>,
    ) -> anyhow::Result<usize>;

    fn box_clone(&self) -> Box<dyn Persistence>;

    // No-op by default. Persistence implementation can override.
    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Clone for Box<dyn Persistence> {
    fn clone(&self) -> Self {
        self.box_clone()
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
    async fn min_snapshot_ts(&self) -> anyhow::Result<Timestamp>;
    async fn min_document_snapshot_ts(&self) -> anyhow::Result<Timestamp>;

    fn fail_if_falling_behind(&self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait PersistenceReader: Send + Sync + 'static {
    fn box_clone(&self) -> Box<dyn PersistenceReader>;

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
        table_id: TableId,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.load_documents(range, order, page_size, retention_validator)
            .try_filter(move |(_, doc_id, _)| future::ready(*doc_id.table() == table_id))
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
    ) -> anyhow::Result<
        BTreeMap<(InternalDocumentId, Timestamp), (Timestamp, Option<ResolvedDocument>)>,
    >;

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
        table_id: TableId,
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
        table_id: TableId,
        read_timestamp: Timestamp,
        key: IndexKey,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<Option<(Timestamp, ResolvedDocument)>> {
        let mut stream = self.index_scan(
            index_id,
            table_id,
            read_timestamp,
            &Interval::prefix(key.into_bytes().into()),
            Order::Asc,
            2,
            retention_validator,
        );
        match stream.try_next().await? {
            Some((key, ts, doc)) => {
                anyhow::ensure!(
                    stream.try_next().await?.is_none(),
                    "Got multiple values for key {:?}",
                    key
                );
                Ok(Some((ts, doc)))
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
            Arc::new(NoopRetentionValidator),
        );
        let max_repeatable =
            self.get_persistence_global(PersistenceGlobalKey::MaxRepeatableTimestamp);
        let (max_committed, max_repeatable) = try_join!(stream.try_next(), max_repeatable)?;
        let max_committed_ts = max_committed.map(|(ts, ..)| ts);
        let max_repeatable_ts = max_repeatable.map(Timestamp::try_from).transpose()?;
        let max_ts = cmp::max(max_committed_ts, max_repeatable_ts); // note None < Some
        Ok(max_ts)
    }

    fn version(&self) -> PersistenceVersion;

    /// Returns all timestamps and documents in ascending (ts, table_id, id)
    /// order. Only should be used for testing
    #[cfg(any(test, feature = "testing"))]
    fn load_all_documents(&self) -> DocumentStream {
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

impl Clone for Box<dyn PersistenceReader> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
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
    reader: Box<dyn PersistenceReader>,
    upper_bound: RepeatableTimestamp,
    retention_validator: Arc<dyn RetentionValidator>,
}

impl RepeatablePersistence {
    pub fn new(
        reader: Box<dyn PersistenceReader>,
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

    /// Same as [`Persistence::load_all_documents`] but only including documents
    /// in the snapshot range.
    pub fn load_all_documents(
        &self,
        order: Order,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.load_documents(
            TimestampRange::snapshot(*self.upper_bound),
            order,
            retention_validator,
        )
    }

    /// Same as [`Persistence::load_documents`] but only including documents in
    /// the snapshot range.
    pub fn load_documents(
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
        Box::pin(stream.try_filter(|(ts, ..)| future::ready(*ts <= *self.upper_bound)))
    }

    pub async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
    ) -> anyhow::Result<
        BTreeMap<(InternalDocumentId, Timestamp), (Timestamp, Option<ResolvedDocument>)>,
    > {
        for (_, ts) in &ids {
            // Reading documents <ts, so ts-1 needs to be repeatable.
            anyhow::ensure!(*ts <= self.upper_bound.succ()?);
        }
        self.reader.previous_revisions(ids).await
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

const STATIC_REPEATABLE_TIMESTAMP_WAIT: Duration = Duration::from_secs(2);
const STATIC_REPEATABLE_TIMESTAMP_MAX_TOTAL_WAIT: Duration = Duration::from_secs(5 * 60);

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
        None => anyhow::bail!("cannot find static repeatable timestamp"),
        Some(ts) => Ok(RepeatableTimestamp::new_validated(
            ts,
            RepeatableReason::MaxRepeatableTsPersistence,
        )),
    }
}

/// Waits for ts to be repeatable according to max_repeatable_ts.
/// Keep in mind max_repeatable_ts can lag behind the current time by
/// ~seconds to ~minutes. And if the committer is not running, the wait
/// may never complete and this function will error after 5m.
/// If you want a potentially stale timestamp and don't want to wait, use
/// new_recent. If you want a more up-to-date timestamp without waiting
/// as long, see if you can prove repeatability some other way.
pub async fn new_static_repeatable_ts<RT: Runtime>(
    ts: Timestamp,
    reader: &dyn PersistenceReader,
    rt: &RT,
) -> anyhow::Result<RepeatableTimestamp> {
    let _timer = static_repeatable_ts_timer(false);
    wait_for_ts(ts, reader, rt).await?;
    Ok(RepeatableTimestamp::new_validated(
        ts,
        RepeatableReason::MaxRepeatableTsPersistence,
    ))
}

async fn wait_for_ts<RT: Runtime>(
    ts: Timestamp,
    reader: &dyn PersistenceReader,
    rt: &RT,
) -> anyhow::Result<()> {
    let mut total_waited = Duration::from_secs(0);
    while read_max_repeatable_ts(reader).await? < Some(ts) {
        anyhow::ensure!(total_waited < STATIC_REPEATABLE_TIMESTAMP_MAX_TOTAL_WAIT);
        rt.wait(STATIC_REPEATABLE_TIMESTAMP_WAIT).await;
        total_waited += STATIC_REPEATABLE_TIMESTAMP_WAIT;
    }
    Ok(())
}

/// PersistenceSnapshot can perform reads from Persistence at a given
/// snapshot.
#[derive(Clone)]
pub struct PersistenceSnapshot {
    reader: Box<dyn PersistenceReader>,
    at: RepeatableTimestamp,
    retention_validator: Arc<dyn RetentionValidator>,
}

impl PersistenceSnapshot {
    /// Same as [`Persistence::index_scan`] but with fixed timestamp.
    pub fn index_scan(
        &self,
        index_id: IndexId,
        table_id: TableId,
        interval: &Interval,
        order: Order,
        size_hint: usize,
    ) -> IndexStream<'_> {
        self.reader
            .index_scan(
                index_id,
                table_id,
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
        table_id: TableId,
        key: IndexKey,
    ) -> anyhow::Result<Option<(Timestamp, ResolvedDocument)>> {
        let result = self
            .reader
            .index_get(
                index_id,
                table_id,
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

    async fn min_snapshot_ts(&self) -> anyhow::Result<Timestamp> {
        Ok(Timestamp::MIN)
    }

    async fn min_document_snapshot_ts(&self) -> anyhow::Result<Timestamp> {
        Ok(Timestamp::MIN)
    }

    fn fail_if_falling_behind(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn test_persistence_global_roundtrips(key in any::<PersistenceGlobalKey>()) {
            let s: String = key.into();
            let parse_key = s.parse().unwrap();
            assert_eq!(key, parse_key);
        }
    }
}
