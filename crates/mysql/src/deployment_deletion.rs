use std::{
    cmp,
    collections::VecDeque,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use anyhow::Context;
use common::{
    knobs::DATABASE_USE_PREPARED_STATEMENTS,
    persistence::PersistenceGlobalKey,
    runtime::Runtime,
    types::{
        DeploymentId,
        PersistenceVersion,
        Timestamp,
    },
    value::ConvexValue,
};
use mysql_async::{
    consts::ColumnType,
    Row,
    Value,
};
use url::Url;

use crate::{
    connection::MySqlConnection,
    v6::{
        sql::{
            LogBucket,
            LIST_LOG_TABLES,
        },
        PersistenceDeploymentId,
    },
    ConvexMySqlPool,
    MySqlInstanceName,
};

const BATCH_SIZE: usize = 2500;
const DOCUMENT_KEY: &[&str] = &["ts", "table_id", "id"];
const V5_INDEX_KEY: &[&str] = &["index_id", "key_prefix", "key_sha256", "ts"];
const V6_INDEX_KEY: &[&str] = &["index_id", "key_prefix", "key_suffix_hash"];
const V6_LOG_KEY: &[&str] = &["index_id", "key_prefix", "key_suffix_hash", "ts"];

const ER_NO_SUCH_TABLE: u16 = 1146;

#[derive(Clone, Debug)]
pub enum DeploymentDeletionTarget {
    V5(MySqlInstanceName),
    /// The deleter validates the ID against MySQL's INT UNSIGNED range.
    V6(DeploymentId),
}

#[derive(Clone, Copy)]
enum TableKind {
    Documents,
    Indexes,
    IndexesLatest,
    IndexesBackfillDeletes,
    Log(LogBucket),
}

struct TableLayout {
    name: String,
    columns: &'static [&'static str],
}

impl TableLayout {
    fn new(name: impl Into<String>, columns: &'static [&'static str]) -> Self {
        Self {
            name: name.into(),
            columns,
        }
    }
}

impl TableKind {
    fn layout(self) -> TableLayout {
        match self {
            Self::Documents => TableLayout::new("documents", DOCUMENT_KEY),
            Self::Indexes => TableLayout::new("indexes", V5_INDEX_KEY),
            Self::IndexesLatest => TableLayout::new("indexes_latest", V6_INDEX_KEY),
            Self::IndexesBackfillDeletes => {
                TableLayout::new("indexes_backfill_deletes", V6_INDEX_KEY)
            },
            Self::Log(bucket) => TableLayout::new(bucket.table_name(), V6_LOG_KEY),
        }
    }
}

#[derive(Clone)]
struct TableCursor {
    kind: TableKind,
    key: Option<Vec<Value>>,
}

impl TableCursor {
    fn new(kind: TableKind) -> Self {
        Self { kind, key: None }
    }
}

pub struct DeploymentDeletionCursor {
    deleter_id: Arc<()>,
    documents: Option<TableCursor>,
    indexes: VecDeque<TableCursor>,
}

pub struct DeploymentDeletionBatch {
    /// `None` when the document walk finished in an earlier batch.
    pub documents_deleted: Option<u64>,
    /// Rows deleted across index tables; `None` if all index walks finished
    /// earlier.
    pub indexes_deleted: Option<u64>,
    pub next_cursor: Option<DeploymentDeletionCursor>,
    /// Time spent in the slower of the two mutations, used to pace replication.
    pub delete_elapsed: Duration,
}

struct TableBatch {
    rows_deleted: u64,
    next_cursor: Option<TableCursor>,
    delete_elapsed: Duration,
}

pub struct DeploymentDeletionPool<RT: Runtime> {
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    version: PersistenceVersion,
}

impl<RT: Runtime> DeploymentDeletionPool<RT> {
    pub fn connect(
        cluster_url: &str,
        require_ssl: bool,
        runtime: RT,
        version: PersistenceVersion,
    ) -> anyhow::Result<Self> {
        let mut url: Url = cluster_url.parse().context("invalid MySQL cluster URL")?;
        anyhow::ensure!(
            !url.username().is_empty(),
            "MySQL cluster URL username must be set"
        );
        let db_name = url.path().trim_start_matches('/').to_string();
        anyhow::ensure!(
            !db_name.is_empty(),
            "MySQL cluster URL must contain a database name"
        );
        if require_ssl {
            url.query_pairs_mut().append_pair("require_ssl", "true");
        }
        url.query_pairs_mut()
            .append_pair("enable_cleartext_plugin", "true");
        let pool = Arc::new(ConvexMySqlPool::new(
            &url,
            *DATABASE_USE_PREPARED_STATEMENTS,
            true, /* require_leader */
            Some(runtime),
        )?);
        Ok(Self {
            pool,
            db_name,
            version,
        })
    }

    pub fn target(&self, name: MySqlInstanceName, id: DeploymentId) -> DeploymentDeletionTarget {
        match self.version {
            PersistenceVersion::V5 => DeploymentDeletionTarget::V5(name),
            PersistenceVersion::V6 => DeploymentDeletionTarget::V6(id),
        }
    }

    pub fn deleter(
        &self,
        target: DeploymentDeletionTarget,
    ) -> anyhow::Result<DeploymentDeleter<RT>> {
        let (tenant_column, tenant) = match (self.version, target) {
            (PersistenceVersion::V5, DeploymentDeletionTarget::V5(name)) => {
                ("instance_name", Value::from(name.raw))
            },
            (PersistenceVersion::V6, DeploymentDeletionTarget::V6(id)) => (
                "deployment_id",
                Value::from(PersistenceDeploymentId::try_from(id)?),
            ),
            (PersistenceVersion::V5, DeploymentDeletionTarget::V6(_))
            | (PersistenceVersion::V6, DeploymentDeletionTarget::V5(_)) => {
                anyhow::bail!(
                    "deployment deletion target does not match cluster version {:?}",
                    self.version
                )
            },
        };
        Ok(DeploymentDeleter {
            deleter_id: Arc::new(()),
            pool: self.pool.clone(),
            db_name: self.db_name.clone(),
            version: self.version,
            tenant_column,
            tenant,
        })
    }
}

pub struct DeploymentDeleter<RT: Runtime> {
    deleter_id: Arc<()>,
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    version: PersistenceVersion,
    tenant_column: &'static str,
    tenant: Value,
}

impl<RT: Runtime> DeploymentDeleter<RT> {
    pub async fn begin(&self) -> anyhow::Result<DeploymentDeletionCursor> {
        let mut conn = self
            .pool
            .acquire("begin_deployment_deletion", &self.db_name)
            .await?;
        let tenant_column = self.tenant_column;
        conn.exec_iter(
            &format!("UPDATE @db_name.leases SET ts = ? WHERE {tenant_column} = ?"),
            vec![i64::MAX.into(), self.tenant.clone()],
        )
        .await?;

        let max_timestamp = ConvexValue::Int64(Timestamp::MAX.into())
            .json_serialize()?
            .into_bytes();
        conn.exec_iter(
            &format!(
                "REPLACE INTO @db_name.persistence_globals ({tenant_column}, `key`, json_value) \
                 VALUES (?, ?, ?), (?, ?, ?)"
            ),
            vec![
                self.tenant.clone(),
                String::from(PersistenceGlobalKey::DocumentRetentionMinSnapshotTimestamp).into(),
                max_timestamp.clone().into(),
                self.tenant.clone(),
                String::from(PersistenceGlobalKey::IndexRetentionMinSnapshotTimestamp).into(),
                max_timestamp.into(),
            ],
        )
        .await?;

        let indexes = match self.version {
            PersistenceVersion::V5 => VecDeque::from([TableCursor::new(TableKind::Indexes)]),
            PersistenceVersion::V6 => {
                let mut indexes = VecDeque::from([
                    TableCursor::new(TableKind::IndexesLatest),
                    TableCursor::new(TableKind::IndexesBackfillDeletes),
                ]);
                // Lease revocation prevents writes into buckets created after this snapshot.
                let mut buckets = conn
                    .query_collect(
                        LIST_LOG_TABLES,
                        vec![(&self.db_name).into()],
                        16,
                        |row: Row| {
                            let name: String = row.get(0).context("missing log table name")?;
                            LogBucket::from_table_name(&name)
                        },
                    )
                    .await?;
                buckets.sort();
                indexes.extend(
                    buckets
                        .into_iter()
                        .map(|bucket| TableCursor::new(TableKind::Log(bucket))),
                );
                indexes
            },
        };
        Ok(DeploymentDeletionCursor {
            deleter_id: self.deleter_id.clone(),
            documents: Some(TableCursor::new(TableKind::Documents)),
            indexes,
        })
    }

    /// Deletes one document range and one index-table range per batch.
    pub async fn delete_batch(
        &self,
        cursor: &DeploymentDeletionCursor,
    ) -> anyhow::Result<DeploymentDeletionBatch> {
        // Cursor identity ensures begin() ran on this deleter.
        anyhow::ensure!(
            Arc::ptr_eq(&self.deleter_id, &cursor.deleter_id),
            "deletion cursor belongs to a different deleter"
        );
        let (documents, indexes) = futures::try_join!(
            self.delete_table_batch(cursor.documents.as_ref()),
            self.delete_table_batch(cursor.indexes.front()),
        )?;
        let mut remaining_indexes = cursor.indexes.clone();
        remaining_indexes.pop_front();
        if let Some(next) = indexes.next_cursor {
            remaining_indexes.push_front(next);
        }
        let next_cursor = if documents.next_cursor.is_none() && remaining_indexes.is_empty() {
            None
        } else {
            Some(DeploymentDeletionCursor {
                deleter_id: self.deleter_id.clone(),
                documents: documents.next_cursor,
                indexes: remaining_indexes,
            })
        };
        Ok(DeploymentDeletionBatch {
            documents_deleted: cursor.documents.as_ref().map(|_| documents.rows_deleted),
            indexes_deleted: cursor.indexes.front().map(|_| indexes.rows_deleted),
            next_cursor,
            delete_elapsed: cmp::max(documents.delete_elapsed, indexes.delete_elapsed),
        })
    }

    async fn delete_table_batch(&self, cursor: Option<&TableCursor>) -> anyhow::Result<TableBatch> {
        let Some(cursor) = cursor else {
            return Ok(TableBatch {
                rows_deleted: 0,
                next_cursor: None,
                delete_elapsed: Duration::ZERO,
            });
        };
        let connection_name = match cursor.kind {
            TableKind::Documents => "delete_deployment_documents",
            TableKind::Indexes
            | TableKind::IndexesLatest
            | TableKind::IndexesBackfillDeletes
            | TableKind::Log(_) => "delete_deployment_indexes",
        };
        let mut conn = self.pool.acquire(connection_name, &self.db_name).await?;
        match self.delete_range(&mut conn, cursor).await {
            // Maintenance reclaims entire log buckets independently of deployment deletion.
            Err(e)
                if matches!(cursor.kind, TableKind::Log(_))
                    && matches!(
                        e.downcast_ref::<mysql_async::Error>(),
                        Some(mysql_async::Error::Server(mysql_async::ServerError {
                            code: ER_NO_SUCH_TABLE,
                            ..
                        }))
                    ) =>
            {
                Ok(TableBatch {
                    rows_deleted: 0,
                    next_cursor: None,
                    delete_elapsed: Duration::ZERO,
                })
            },
            result => result,
        }
    }

    async fn delete_range(
        &self,
        conn: &mut MySqlConnection<'_, RT>,
        cursor: &TableCursor,
    ) -> anyhow::Result<TableBatch> {
        let layout = cursor.kind.layout();
        let table = &layout.name;
        let tenant_column = self.tenant_column;
        let columns = layout.columns.join(", ");
        let mut boundary_query = format!(
            "SELECT {columns} FROM @db_name.{table} FORCE INDEX FOR ORDER BY (PRIMARY) WHERE \
             {tenant_column} = ?"
        );
        let mut boundary_params = vec![self.tenant.clone()];
        if let Some(key) = &cursor.key {
            append_key_bound(
                &mut boundary_query,
                &mut boundary_params,
                layout.columns,
                key,
                CursorBound::After,
            );
        }
        boundary_query.push_str(&format!(
            " ORDER BY {tenant_column}, {columns} LIMIT 1 OFFSET {}",
            BATCH_SIZE - 1
        ));
        let next_key = conn
            .query_optional(&boundary_query, boundary_params)
            .await
            .with_context(|| format!("find {table} deletion boundary"))?
            .map(decode_key)
            .transpose()?;

        let mut delete_query = format!(
            "DELETE @db_name.{table} FROM @db_name.{table} FORCE INDEX (PRIMARY) WHERE \
             {tenant_column} = ?"
        );
        let mut delete_params = vec![self.tenant.clone()];
        if let Some(key) = &cursor.key {
            append_key_bound(
                &mut delete_query,
                &mut delete_params,
                layout.columns,
                key,
                CursorBound::After,
            );
        }
        if let Some(key) = &next_key {
            append_key_bound(
                &mut delete_query,
                &mut delete_params,
                layout.columns,
                key,
                CursorBound::Through,
            );
        }
        let started = Instant::now();
        let rows_deleted = conn
            .exec_iter(&delete_query, delete_params)
            .await
            .with_context(|| format!("delete {table} range"))?;
        Ok(TableBatch {
            rows_deleted,
            next_cursor: next_key.map(|key| TableCursor {
                key: Some(key),
                ..cursor.clone()
            }),
            delete_elapsed: started.elapsed(),
        })
    }

    /// Call only after `delete_batch()` returns no next cursor.
    pub async fn finish(&self) -> anyhow::Result<u64> {
        let mut conn = self
            .pool
            .acquire("finish_deployment_deletion", &self.db_name)
            .await?;
        let tenant_column = self.tenant_column;
        let globals_deleted = conn
            .exec_iter(
                &format!("DELETE FROM @db_name.persistence_globals WHERE {tenant_column} = ?"),
                vec![self.tenant.clone()],
            )
            .await?;
        conn.exec_iter(
            &format!("DELETE FROM @db_name.read_only WHERE {tenant_column} = ?"),
            vec![self.tenant.clone()],
        )
        .await?;
        // The lease is the existence marker, so retaining it until last makes retries
        // discoverable.
        conn.exec_iter(
            &format!("DELETE FROM @db_name.leases WHERE {tenant_column} = ?"),
            vec![self.tenant.clone()],
        )
        .await?;
        Ok(globals_deleted)
    }
}

fn decode_key(row: Row) -> anyhow::Result<Vec<Value>> {
    row.columns_ref()
        .iter()
        .enumerate()
        .map(|(i, column)| {
            // Text-protocol integers arrive as bytes. Binding those as binary literals
            // would change the numeric range comparisons.
            Ok(match column.column_type() {
                ColumnType::MYSQL_TYPE_LONG | ColumnType::MYSQL_TYPE_LONGLONG => Value::from(
                    row.get_opt::<i64, _>(i)
                        .context("missing integer key column")??,
                ),
                ColumnType::MYSQL_TYPE_STRING
                | ColumnType::MYSQL_TYPE_VAR_STRING
                | ColumnType::MYSQL_TYPE_VARCHAR => Value::from(
                    row.get_opt::<Vec<u8>, _>(i)
                        .context("missing binary key column")??,
                ),
                other => anyhow::bail!("unsupported deletion key column type: {other:?}"),
            })
        })
        .collect()
}

enum CursorBound {
    After,
    Through,
}

fn append_key_bound(
    query: &mut String,
    params: &mut Vec<Value>,
    columns: &[&str],
    key: &[Value],
    bound: CursorBound,
) {
    assert_eq!(columns.len(), key.len());
    let (strict, last) = match bound {
        CursorBound::After => (">", ">"),
        CursorBound::Through => ("<", "<="),
    };
    query.push_str(" AND (");
    // Expanded comparisons let MySQL seek the full composite primary key.
    for (i, (column, value)) in columns.iter().zip(key).enumerate() {
        if i + 1 == columns.len() {
            query.push_str(&format!("{column} {last} ?"));
            params.push(value.clone());
        } else {
            query.push_str(&format!("{column} {strict} ? OR ({column} = ? AND ("));
            params.extend([value.clone(), value.clone()]);
        }
    }
    query.push_str(&")".repeat(2 * columns.len() - 1));
}
