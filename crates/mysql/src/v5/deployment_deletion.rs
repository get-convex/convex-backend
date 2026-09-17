use std::{
    cmp,
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
    types::Timestamp,
    value::ConvexValue,
};
use mysql_async::{
    Row,
    Value,
};
use url::Url;

use crate::{
    connection::MySqlConnection,
    ConvexMySqlPool,
    MySqlInstanceName,
};

const BATCH_SIZE: usize = 2500;

const DOCUMENT_LOWER_BOUND: &str =
    " AND (ts > ? OR (ts = ? AND (table_id > ? OR (table_id = ? AND id > ?))))";
const DOCUMENT_UPPER_BOUND: &str =
    " AND (ts < ? OR (ts = ? AND (table_id < ? OR (table_id = ? AND id <= ?))))";
const INDEX_LOWER_BOUND: &str = " AND (index_id > ? OR (index_id = ? AND (key_prefix > ? OR \
                                 (key_prefix = ? AND (key_sha256 > ? OR (key_sha256 = ? AND ts > \
                                 ?))))))";
const INDEX_UPPER_BOUND: &str = " AND (index_id < ? OR (index_id = ? AND (key_prefix < ? OR \
                                 (key_prefix = ? AND (key_sha256 < ? OR (key_sha256 = ? AND ts <= \
                                 ?))))))";

#[derive(Clone)]
struct DocumentCursor {
    ts: i64,
    table_id: Vec<u8>,
    id: Vec<u8>,
}

impl DocumentCursor {
    fn from_row(row: Row) -> anyhow::Result<Self> {
        Ok(Self {
            ts: row.get(0).context("document cursor missing ts")?,
            table_id: row.get(1).context("document cursor missing table_id")?,
            id: row.get(2).context("document cursor missing id")?,
        })
    }

    fn params(&self) -> Vec<Value> {
        vec![
            self.ts.into(),
            self.ts.into(),
            self.table_id.clone().into(),
            self.table_id.clone().into(),
            self.id.clone().into(),
        ]
    }
}

#[derive(Clone)]
struct IndexCursor {
    index_id: Vec<u8>,
    key_prefix: Vec<u8>,
    key_sha256: Vec<u8>,
    ts: i64,
}

impl IndexCursor {
    fn from_row(row: Row) -> anyhow::Result<Self> {
        Ok(Self {
            index_id: row.get(0).context("index cursor missing index_id")?,
            key_prefix: row.get(1).context("index cursor missing key_prefix")?,
            key_sha256: row.get(2).context("index cursor missing key_sha256")?,
            ts: row.get(3).context("index cursor missing ts")?,
        })
    }

    fn params(&self) -> Vec<Value> {
        vec![
            self.index_id.clone().into(),
            self.index_id.clone().into(),
            self.key_prefix.clone().into(),
            self.key_prefix.clone().into(),
            self.key_sha256.clone().into(),
            self.key_sha256.clone().into(),
            self.ts.into(),
        ]
    }
}

/// Continuation state for deleting a deployment from multitenant MySQL.
///
/// Its representation is private so callers cannot depend on a persistence
/// version's physical primary keys.
pub struct DeploymentDeletionCursor {
    document_cursor: Option<DocumentCursor>,
    documents_done: bool,
    index_cursor: Option<IndexCursor>,
    indexes_done: bool,
}

/// Work completed by one deployment-deletion batch.
pub struct DeploymentDeletionBatch {
    /// `None` once the document walk had already completed before this batch.
    pub documents_deleted: Option<u64>,
    /// `None` once the index walk had already completed before this batch.
    pub indexes_deleted: Option<u64>,
    pub next_cursor: Option<DeploymentDeletionCursor>,
    /// Time spent in the slower of the two mutations, used to pace replication.
    pub delete_elapsed: Duration,
}

struct TableBatch<C> {
    rows_deleted: u64,
    next_cursor: Option<C>,
    delete_elapsed: Duration,
}

/// Connections shared by V5 multitenant deployment deletions in one database.
pub struct DeploymentDeletionPool<RT: Runtime> {
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
}

impl<RT: Runtime> DeploymentDeletionPool<RT> {
    pub fn connect(cluster_url: &str, require_ssl: bool, runtime: RT) -> anyhow::Result<Self> {
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
            url.query_pairs_mut()
                .append_pair("require_ssl", "true")
                .append_pair("verify_ca", "true");
        }
        url.query_pairs_mut()
            .append_pair("enable_cleartext_plugin", "true");
        let pool = Arc::new(ConvexMySqlPool::new(
            &url,
            *DATABASE_USE_PREPARED_STATEMENTS,
            true, /* require_leader */
            Some(runtime),
        )?);
        Ok(Self { pool, db_name })
    }

    pub fn deleter(&self, deployment_name: &str) -> DeploymentDeleter<RT> {
        DeploymentDeleter {
            pool: self.pool.clone(),
            db_name: self.db_name.clone(),
            instance_name: deployment_name.into(),
        }
    }
}

/// Lease-free administrative deletion for one V5 multitenant deployment.
pub struct DeploymentDeleter<RT: Runtime> {
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    instance_name: MySqlInstanceName,
}

impl<RT: Runtime> DeploymentDeleter<RT> {
    /// Prevents future persistence and retention work before rows are removed.
    pub async fn begin(&self) -> anyhow::Result<()> {
        let mut conn = self
            .pool
            .acquire("begin_deployment_deletion", &self.db_name)
            .await?;
        conn.exec_iter(
            "UPDATE @db_name.leases SET ts = ? WHERE instance_name = ?",
            vec![i64::MAX.into(), (&self.instance_name.raw).into()],
        )
        .await?;

        let max_timestamp = ConvexValue::Int64(Timestamp::MAX.into())
            .json_serialize()?
            .into_bytes();
        conn.exec_iter(
            "REPLACE INTO @db_name.persistence_globals (instance_name, `key`, json_value) VALUES \
             (?, ?, ?), (?, ?, ?)",
            vec![
                (&self.instance_name.raw).into(),
                String::from(PersistenceGlobalKey::DocumentRetentionMinSnapshotTimestamp).into(),
                max_timestamp.clone().into(),
                (&self.instance_name.raw).into(),
                String::from(PersistenceGlobalKey::IndexRetentionMinSnapshotTimestamp).into(),
                max_timestamp.into(),
            ],
        )
        .await?;
        Ok(())
    }

    /// Deletes at most one primary-key range from each large table.
    pub async fn delete_batch(
        &self,
        cursor: Option<&DeploymentDeletionCursor>,
    ) -> anyhow::Result<DeploymentDeletionBatch> {
        let documents_done = cursor.is_some_and(|cursor| cursor.documents_done);
        let indexes_done = cursor.is_some_and(|cursor| cursor.indexes_done);
        let delete_documents = !documents_done;
        let delete_indexes = !indexes_done;
        let document_cursor = cursor.and_then(|cursor| cursor.document_cursor.as_ref());
        let index_cursor = cursor.and_then(|cursor| cursor.index_cursor.as_ref());

        let document_batch = async {
            if !delete_documents {
                return Ok(TableBatch {
                    rows_deleted: 0,
                    next_cursor: None,
                    delete_elapsed: Duration::ZERO,
                });
            }
            let mut conn = self
                .pool
                .acquire("delete_deployment_documents", &self.db_name)
                .await?;
            delete_document_batch(&mut conn, &self.instance_name, document_cursor).await
        };
        let index_batch = async {
            if !delete_indexes {
                return Ok(TableBatch {
                    rows_deleted: 0,
                    next_cursor: None,
                    delete_elapsed: Duration::ZERO,
                });
            }
            let mut conn = self
                .pool
                .acquire("delete_deployment_indexes", &self.db_name)
                .await?;
            delete_index_batch(&mut conn, &self.instance_name, index_cursor).await
        };
        let (document_batch, index_batch) = futures::try_join!(document_batch, index_batch)?;
        let documents_done = documents_done || document_batch.next_cursor.is_none();
        let indexes_done = indexes_done || index_batch.next_cursor.is_none();
        let delete_elapsed = cmp::max(document_batch.delete_elapsed, index_batch.delete_elapsed);
        let next_cursor = if documents_done && indexes_done {
            None
        } else {
            Some(DeploymentDeletionCursor {
                document_cursor: document_batch.next_cursor,
                documents_done,
                index_cursor: index_batch.next_cursor,
                indexes_done,
            })
        };
        Ok(DeploymentDeletionBatch {
            documents_deleted: delete_documents.then_some(document_batch.rows_deleted),
            indexes_deleted: delete_indexes.then_some(index_batch.rows_deleted),
            next_cursor,
            delete_elapsed,
        })
    }

    /// Removes metadata after both large-table walks have completed.
    pub async fn finish(&self) -> anyhow::Result<u64> {
        let mut conn = self
            .pool
            .acquire("finish_deployment_deletion", &self.db_name)
            .await?;
        let globals_deleted = conn
            .exec_iter(
                "DELETE FROM @db_name.persistence_globals WHERE instance_name = ?",
                vec![(&self.instance_name.raw).into()],
            )
            .await?;
        conn.exec_iter(
            "DELETE FROM @db_name.read_only WHERE instance_name = ?",
            vec![(&self.instance_name.raw).into()],
        )
        .await?;
        // The lease is the existence marker, so retaining it until last makes retries
        // discoverable.
        conn.exec_iter(
            "DELETE FROM @db_name.leases WHERE instance_name = ?",
            vec![(&self.instance_name.raw).into()],
        )
        .await?;
        Ok(globals_deleted)
    }
}

async fn delete_document_batch<RT: Runtime>(
    conn: &mut MySqlConnection<'_, RT>,
    instance_name: &MySqlInstanceName,
    cursor: Option<&DocumentCursor>,
) -> anyhow::Result<TableBatch<DocumentCursor>> {
    let mut boundary_query = String::from(
        "SELECT ts, table_id, id FROM @db_name.documents FORCE INDEX FOR ORDER BY (PRIMARY) WHERE \
         instance_name = ?",
    );
    let mut boundary_params = vec![(&instance_name.raw).into()];
    if let Some(cursor) = cursor {
        boundary_query.push_str(DOCUMENT_LOWER_BOUND);
        boundary_params.extend(cursor.params());
    }
    boundary_query.push_str(&format!(
        " ORDER BY instance_name, ts, table_id, id LIMIT 1 OFFSET {}",
        BATCH_SIZE - 1
    ));
    let next_cursor = conn
        .query_optional(&boundary_query, boundary_params)
        .await
        .context("find document deletion boundary")?
        .map(DocumentCursor::from_row)
        .transpose()?;

    let mut delete_query = String::from(
        "DELETE @db_name.documents FROM @db_name.documents FORCE INDEX (PRIMARY) WHERE \
         instance_name = ?",
    );
    let mut delete_params = vec![(&instance_name.raw).into()];
    if let Some(cursor) = cursor {
        delete_query.push_str(DOCUMENT_LOWER_BOUND);
        delete_params.extend(cursor.params());
    }
    if let Some(next_cursor) = &next_cursor {
        delete_query.push_str(DOCUMENT_UPPER_BOUND);
        delete_params.extend(next_cursor.params());
    }
    let started = Instant::now();
    let rows_deleted = conn
        .exec_iter(&delete_query, delete_params)
        .await
        .context("delete document range")?;
    Ok(TableBatch {
        rows_deleted,
        next_cursor,
        delete_elapsed: started.elapsed(),
    })
}

async fn delete_index_batch<RT: Runtime>(
    conn: &mut MySqlConnection<'_, RT>,
    instance_name: &MySqlInstanceName,
    cursor: Option<&IndexCursor>,
) -> anyhow::Result<TableBatch<IndexCursor>> {
    let mut boundary_query = String::from(
        "SELECT index_id, key_prefix, key_sha256, ts FROM @db_name.indexes FORCE INDEX FOR ORDER \
         BY (PRIMARY) WHERE instance_name = ?",
    );
    let mut boundary_params = vec![(&instance_name.raw).into()];
    if let Some(cursor) = cursor {
        boundary_query.push_str(INDEX_LOWER_BOUND);
        boundary_params.extend(cursor.params());
    }
    boundary_query.push_str(&format!(
        " ORDER BY instance_name, index_id, key_prefix, key_sha256, ts LIMIT 1 OFFSET {}",
        BATCH_SIZE - 1
    ));
    let next_cursor = conn
        .query_optional(&boundary_query, boundary_params)
        .await
        .context("find index deletion boundary")?
        .map(IndexCursor::from_row)
        .transpose()?;

    let mut delete_query = String::from(
        "DELETE @db_name.indexes FROM @db_name.indexes FORCE INDEX (PRIMARY) WHERE instance_name \
         = ?",
    );
    let mut delete_params = vec![(&instance_name.raw).into()];
    if let Some(cursor) = cursor {
        delete_query.push_str(INDEX_LOWER_BOUND);
        delete_params.extend(cursor.params());
    }
    if let Some(next_cursor) = &next_cursor {
        delete_query.push_str(INDEX_UPPER_BOUND);
        delete_params.extend(next_cursor.params());
    }
    let started = Instant::now();
    let rows_deleted = conn
        .exec_iter(&delete_query, delete_params)
        .await
        .context("delete index range")?;
    Ok(TableBatch {
        rows_deleted,
        next_cursor,
        delete_elapsed: started.elapsed(),
    })
}
