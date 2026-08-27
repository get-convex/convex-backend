//! V6 index-table definitions.

use std::{
    fmt::Write,
    iter,
    ops::Bound,
};

use common::{
    index::{
        SplitKey,
        MAX_INDEX_KEY_PREFIX_LEN,
    },
    interval::{
        End,
        Interval,
        StartIncluded,
    },
    query::Order,
    sha256::Sha256,
    types::{
        PersistenceIndexId,
        Timestamp,
    },
    value::InternalDocumentId,
};
use itertools::Itertools;
use mysql_async::Value;

use super::DeploymentId;

pub(crate) const LATEST_TABLE: &str = "indexes_latest";
pub(crate) const LOG_BUCKET_SIZE_NANOS: i64 = 10 * 60 * 1_000_000_000;
pub(crate) const LIST_LOG_TABLES: &str = r#"
SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES
WHERE TABLE_SCHEMA = ? AND TABLE_NAME LIKE 'indexes\_log\_%'
"#;

/// The hash component used to disambiguate V6 key prefixes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeySuffixHash(Vec<u8>);

impl KeySuffixHash {
    /// An empty suffix has no hash bytes so it is stored as `X''`.
    pub(crate) fn from_suffix(suffix: Option<&[u8]>) -> Self {
        match suffix {
            None | Some([]) => Self(Vec::new()),
            Some(suffix) => Self(Sha256::hash(suffix).as_ref()[..16].to_vec()),
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A V6 index key split into a prefix, optional suffix, and suffix hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexKey {
    pub(crate) key_prefix: Vec<u8>,
    pub(crate) key_suffix: Option<Vec<u8>>,
    pub(crate) key_suffix_hash: KeySuffixHash,
}

impl IndexKey {
    pub(crate) fn from_key(key: Vec<u8>) -> Self {
        let SplitKey { prefix, suffix } = SplitKey::new(key);
        let key_suffix_hash = KeySuffixHash::from_suffix(suffix.as_deref());
        Self {
            key_prefix: prefix,
            key_suffix: suffix,
            key_suffix_hash,
        }
    }
}

/// A ten-minute successor-timestamp bucket used in a V6 log table name.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct LogBucket(i64);

impl LogBucket {
    pub(crate) fn from_successor_ts(successor_ts: Timestamp) -> Self {
        Self(i64::from(successor_ts).div_euclid(LOG_BUCKET_SIZE_NANOS))
    }

    pub(crate) const fn value(self) -> i64 {
        self.0
    }

    pub(crate) fn table_name(self) -> String {
        format!("indexes_log_{}", self.0)
    }

    pub(crate) fn from_table_name(table_name: &str) -> anyhow::Result<Self> {
        let bucket = table_name
            .strip_prefix("indexes_log_")
            .ok_or_else(|| anyhow::anyhow!("not a V6 index log table: {table_name}"))?
            .parse()?;
        Ok(Self(bucket))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexRow {
    pub(crate) deployment_id: DeploymentId,
    pub(crate) index_id: PersistenceIndexId,
    pub(crate) key: IndexKey,
    pub(crate) ts: Timestamp,
    pub(crate) document_id: InternalDocumentId,
}

impl IndexRow {
    /// Parameters for one row of either index table, in column order.
    /// `successor_ts` is present exactly for log rows, where the column sits
    /// between `ts` and `table_id`.
    fn params_with_successor(&self, successor_ts: Option<Timestamp>) -> Vec<Value> {
        let mut params = vec![
            self.deployment_id.into(),
            self.index_id.value().into(),
            Value::Bytes(self.key.key_prefix.clone()),
            self.key.key_suffix.clone().into(),
            self.key.key_suffix_hash.as_bytes().into(),
            Value::Int(i64::from(self.ts)),
        ];
        params.extend(successor_ts.map(|ts| Value::Int(i64::from(ts))));
        params.extend([
            Value::Bytes(self.document_id.table().0.into()),
            Value::Bytes(self.document_id.internal_id().into()),
        ]);
        params
    }

    /// An `IndexRow` is always a live revision, so `deleted` binds false. It is
    /// bound rather than left to the column default so that a live write
    /// landing on a tombstone clears it; see `upsert_latest_chunk`.
    pub(crate) fn params(&self) -> Vec<Value> {
        let mut params = self.params_with_successor(None);
        params.push(false.into());
        params
    }

    pub(crate) fn delete_params(&self) -> Vec<Value> {
        vec![
            self.deployment_id.into(),
            self.index_id.value().into(),
            Value::Bytes(self.key.key_prefix.clone()),
            self.key.key_suffix_hash.as_bytes().into(),
            self.key.key_suffix.clone().into(),
            Value::Int(i64::from(self.ts)),
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LogRow {
    pub(crate) row: IndexRow,
    pub(crate) successor_ts: Timestamp,
}

impl LogRow {
    pub(crate) fn params(&self) -> Vec<Value> {
        self.row.params_with_successor(Some(self.successor_ts))
    }

    pub(crate) fn bucket(&self) -> LogBucket {
        LogBucket::from_successor_ts(self.successor_ts)
    }
}

/// The key fields used to seek the V6 index primary keys.
#[derive(Clone)]
pub(crate) struct SqlKey {
    pub(crate) prefix: Vec<u8>,
    pub(crate) suffix_hash: Vec<u8>,
}

impl SqlKey {
    fn min_with_same_prefix(key: Vec<u8>) -> Self {
        let SplitKey { prefix, .. } = SplitKey::new(key);
        Self {
            prefix,
            suffix_hash: vec![],
        }
    }

    fn max_with_same_prefix(key: Vec<u8>) -> Self {
        let SplitKey { prefix, .. } = SplitKey::new(key);
        Self {
            prefix,
            suffix_hash: vec![u8::MAX; 16],
        }
    }
}

/// Converts an index interval into bounds on the V6 primary key. A long-key
/// prefix can include additional keys, which the caller filters after reading.
pub(crate) fn to_sql_bounds(interval: Interval) -> (Bound<SqlKey>, Bound<SqlKey>) {
    let lower = match interval.start {
        StartIncluded(key) => Bound::Included(SqlKey::min_with_same_prefix(key.into())),
    };
    let upper = match interval.end {
        End::Excluded(key) if key.len() < MAX_INDEX_KEY_PREFIX_LEN => {
            Bound::Excluded(SqlKey::min_with_same_prefix(key.into()))
        },
        End::Excluded(key) => Bound::Included(SqlKey::max_with_same_prefix(key.into())),
        End::Unbounded => Bound::Unbounded,
    };
    (lower, upper)
}

/// Which end of a scanned range a bound closes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundSide {
    Lower,
    Upper,
}

impl BoundSide {
    /// Admits keys strictly past the bound.
    const fn exclusive(self) -> &'static str {
        match self {
            Self::Lower => ">",
            Self::Upper => "<",
        }
    }

    /// Also admits keys equal to the bound.
    const fn inclusive(self) -> &'static str {
        match self {
            Self::Lower => ">=",
            Self::Upper => "<=",
        }
    }
}

fn append_bound_clause(
    clause: &mut String,
    params: &mut Vec<Value>,
    bound: &Bound<SqlKey>,
    side: BoundSide,
) {
    let prefix_comparator = side.exclusive();
    // Only an inclusive bound admits rows whose suffix hash equals the bound's.
    let (key, hash_comparator) = match bound {
        Bound::Unbounded => return,
        Bound::Included(key) => (key, side.inclusive()),
        Bound::Excluded(key) => (key, prefix_comparator),
    };
    write!(
        clause,
        " AND (key_prefix {prefix_comparator} ? OR (key_prefix = ? AND key_suffix_hash \
         {hash_comparator} ?))"
    )
    .expect("writing to a String cannot fail");
    params.extend([
        Value::Bytes(key.prefix.clone()),
        Value::Bytes(key.prefix.clone()),
        Value::Bytes(key.suffix_hash.clone()),
    ]);
}

fn index_query_params(
    deployment_id: DeploymentId,
    index_id: PersistenceIndexId,
    read_timestamp: Timestamp,
    bounds: &[Value],
    successor_filter: bool,
) -> Vec<Value> {
    let mut params = vec![
        deployment_id.into(),
        index_id.value().into(),
        Value::Int(i64::from(read_timestamp)),
    ];
    params.extend(bounds.iter().cloned());
    if successor_filter {
        params.push(Value::Int(i64::from(read_timestamp)));
    }
    params
}

/// Resolves each index row to its document revision.
const DOCUMENT_JOIN: &str = r#"LEFT JOIN @db_name.documents D FORCE INDEX FOR JOIN (PRIMARY)
ON D.deployment_id = U.deployment_id
    AND D.ts = U.ts
    AND D.table_id = U.table_id
    AND D.id = U.document_id"#;

/// Unions one arm per snapshot source: `indexes_latest` plus each log bucket.
/// `arm` renders an arm from its table expression and the predicate that arm
/// needs beyond the shared key bounds -- hiding tombstones for the latest
/// table, restricting to revisions still live at the snapshot for a log bucket.
/// `arm_params` supplies that arm's parameters, taking whether the successor
/// predicate is present.
fn union_arms(
    log_buckets: &[LogBucket],
    mut arm: impl FnMut(&str, &str) -> String,
    mut arm_params: impl FnMut(bool) -> Vec<Value>,
) -> (String, Vec<Value>) {
    let mut arms = vec![arm(
        &format!("{LATEST_TABLE} FORCE INDEX (PRIMARY)"),
        "AND deleted = false",
    )];
    let mut params = arm_params(false);
    for bucket in log_buckets {
        arms.push(arm(&bucket.table_name(), "AND successor_ts > ?"));
        params.extend(arm_params(true));
    }
    (arms.join("\nUNION ALL\n"), params)
}

/// Builds the V6 range query over `indexes_latest` and the supplied log
/// buckets. The union is duplicate-free because each key has one revision
/// visible at a snapshot across the latest and log tables.
///
/// Rows come back ordered by `key_suffix_hash`, which for keys past the prefix
/// limit is not the order of the keys themselves. Pagination is consistent --
/// the cursor is a `(prefix, suffix_hash)` pair in that same order -- but a
/// caller that needs key order has to buffer each equal-prefix run and sort it
/// by the reconstructed key, which is what the V6 index scan does.
pub(crate) fn index_query(
    deployment_id: DeploymentId,
    index_id: PersistenceIndexId,
    read_timestamp: Timestamp,
    lower: Bound<SqlKey>,
    upper: Bound<SqlKey>,
    order: Order,
    batch_size: usize,
    log_buckets: &[LogBucket],
) -> (String, Vec<Value>) {
    let order = match order {
        Order::Asc => "ASC",
        Order::Desc => "DESC",
    };
    let mut where_clause = "deployment_id = ? AND index_id = ? AND ts <= ?".to_owned();
    let mut bounds = vec![];
    append_bound_clause(&mut where_clause, &mut bounds, &lower, BoundSide::Lower);
    append_bound_clause(&mut where_clause, &mut bounds, &upper, BoundSide::Upper);

    let (arms, params) = union_arms(
        log_buckets,
        |table, successor_filter| {
            format!(
                r#"(SELECT deployment_id, index_id, key_prefix, key_suffix_hash, key_suffix, ts, table_id, document_id
FROM @db_name.{table}
WHERE {where_clause} {successor_filter}
ORDER BY index_id {order}, key_prefix {order}, key_suffix_hash {order}
LIMIT {batch_size})"#
            )
        },
        |successor_filter| {
            index_query_params(
                deployment_id,
                index_id,
                read_timestamp,
                &bounds,
                successor_filter,
            )
        },
    );
    (
        format!(
            r#"
SELECT U.index_id, U.key_prefix, U.key_suffix_hash, U.key_suffix, U.ts, U.table_id, U.document_id, D.json_value, D.prev_ts
FROM (
{arms}
) U
{DOCUMENT_JOIN}
ORDER BY U.key_prefix {order}, U.key_suffix_hash {order}
LIMIT {batch_size}
"#
        ),
        params,
    )
}

/// `deleted` trails the columns the log tables share, which have no such
/// column. Updating it alongside the rest is what lets a newer live revision
/// take over a key that is currently a tombstone.
pub(crate) fn upsert_latest_chunk(chunk_size: usize) -> String {
    let values = iter::repeat_n("(?, ?, ?, ?, ?, ?, ?, ?, ?)", chunk_size).join(", ");
    format!(
        r#"INSERT INTO @db_name.indexes_latest
    (deployment_id, index_id, key_prefix, key_suffix, key_suffix_hash, ts, table_id, document_id, deleted)
    VALUES {values}
    ON DUPLICATE KEY UPDATE
    key_suffix = IF(VALUES(ts) > ts, VALUES(key_suffix), key_suffix),
    table_id = IF(VALUES(ts) > ts, VALUES(table_id), table_id),
    document_id = IF(VALUES(ts) > ts, VALUES(document_id), document_id),
    deleted = IF(VALUES(ts) > ts, VALUES(deleted), deleted),
    ts = IF(VALUES(ts) > ts, VALUES(ts), ts)"#
    )
}

pub(crate) fn insert_log_chunk(bucket: LogBucket, chunk_size: usize) -> String {
    let values = iter::repeat_n("(?, ?, ?, ?, ?, ?, ?, ?, ?)", chunk_size).join(", ");
    format!(
        r#"INSERT INTO @db_name.{}
    (deployment_id, index_id, key_prefix, key_suffix, key_suffix_hash, ts, successor_ts, table_id, document_id)
    VALUES {values}
    ON DUPLICATE KEY UPDATE successor_ts = VALUES(successor_ts)"#,
        bucket.table_name()
    )
}

pub(crate) fn delete_latest_chunk(chunk_size: usize) -> String {
    let predicates = iter::repeat_n(
        "(deployment_id = ? AND index_id = ? AND key_prefix = ? AND key_suffix_hash = ? AND \
         key_suffix <=> ? AND ts <= ?)",
        chunk_size,
    )
    .join(" OR ");
    format!("DELETE FROM @db_name.indexes_latest WHERE {predicates}")
}

pub(crate) fn drop_log_ddl(bucket: LogBucket) -> String {
    format!("DROP TABLE IF EXISTS @db_name.{}", bucket.table_name())
}

pub(crate) fn log_ddl(bucket: LogBucket) -> String {
    format!(
        r#"
CREATE TABLE IF NOT EXISTS @db_name.{} (
    deployment_id INT UNSIGNED NOT NULL,
    index_id INT UNSIGNED NOT NULL,
    key_prefix VARBINARY(2500) NOT NULL,
    key_suffix LONGBLOB NULL,
    key_suffix_hash VARBINARY(16) NOT NULL,
    ts BIGINT NOT NULL,
    successor_ts BIGINT NOT NULL,
    table_id BINARY(16) NULL,
    document_id BINARY(16) NULL,
    PRIMARY KEY (deployment_id, index_id, key_prefix, key_suffix_hash, ts),
    INDEX indexes_log_by_successor_ts (deployment_id, index_id, successor_ts)
) ROW_FORMAT=DYNAMIC PARTITION BY KEY(index_id) PARTITIONS 16;
"#,
        bucket.table_name()
    )
}
