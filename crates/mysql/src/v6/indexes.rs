//! V6 index operations: the write path over `indexes_latest` and
//! `indexes_log_<n>`. Reads live on the `Reader` in `persistence.rs`;
//! `maintenance` keeps the log tables created. Every entry carries its
//! persistence index ID and its previous entry, so a write reads no table.

use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use common::{
    persistence::{
        ConflictStrategy,
        PersistenceIndexEntry,
    },
    runtime::Runtime,
    types::Timestamp,
    value::InternalDocumentId,
};
use fastrace::prelude::*;
use mysql_async::Value;

use super::{
    sql::{
        self,
        IndexKey,
        IndexRow,
        LogBucket,
        LogRow,
    },
    DeploymentId,
};
use crate::{
    chunks::{
        smart_chunks,
        ApproxSize,
    },
    connection::{
        MySqlConnection,
        MySqlTransaction,
    },
    metrics,
};

/// The rows a batch of index entries writes, grouped by the statement that
/// writes them.
#[derive(Default)]
pub(crate) struct IndexWriteBatch {
    /// Previous revisions, by the bucket of the commit that replaced them.
    log_rows: BTreeMap<LogBucket, Vec<LogRow>>,
    /// Live entries for keys that had no entry.
    inserts: Vec<IndexRow>,
    /// The exact previous entries of live entries under `Error`, removed
    /// before those entries are inserted as new.
    previous: Vec<IndexRow>,
    /// The exact previous entries tombstones remove.
    deletes: Vec<IndexRow>,
}

impl IndexWriteBatch {
    /// The commits that moved entries into the log, each as the document
    /// revision it wrote.
    pub(crate) fn replacement_commits(
        &self,
    ) -> impl Iterator<Item = (Timestamp, InternalDocumentId)> + '_ {
        self.log_rows
            .values()
            .flatten()
            .map(|row| (row.successor_ts, row.row.document_id))
    }
}

pub(crate) struct IndexEngine {
    deployment_id: DeploymentId,
}

impl IndexEngine {
    pub(crate) fn new(deployment_id: DeploymentId) -> Self {
        Self { deployment_id }
    }

    /// Sorts a batch of entries into the rows each statement writes. An entry
    /// without a persistence index ID predates ID assignment, which V6 does
    /// not store. An entry replacing a newer revision is out of order, which
    /// only a backfill racing live writes produces; V6 cannot order them yet,
    /// so it fails rather than corrupt the index. V6 rejects `Overwrite` until
    /// index backfill defines how to order it against tombstones and live
    /// writes.
    pub(crate) fn plan_index_writes(
        &self,
        index_updates: &[PersistenceIndexEntry],
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<IndexWriteBatch> {
        anyhow::ensure!(
            conflict_strategy == ConflictStrategy::Error || index_updates.is_empty(),
            "MySQL V6 index writes with `Overwrite` are unimplemented until index backfill \
             supports them"
        );
        let mut batch = IndexWriteBatch::default();
        for update in index_updates {
            let index_id = update.index.persistence_index_id().with_context(|| {
                format!(
                    "MySQL V6 requires a persistence index ID to write index {}",
                    update.index.id()
                )
            })?;
            let key = IndexKey::from_key(update.key.to_vec());
            let row = |ts, document_id| IndexRow {
                deployment_id: self.deployment_id,
                index_id,
                key: key.clone(),
                ts,
                document_id,
            };
            if let Some(prev) = update.prev {
                anyhow::ensure!(
                    prev.ts < update.ts,
                    "MySQL V6 does not support out-of-order index writes yet (index backfill is \
                     unimplemented)"
                );
                batch
                    .log_rows
                    .entry(LogBucket::from_successor_ts(update.ts))
                    .or_default()
                    .push(LogRow {
                        row: row(prev.ts, prev.document_id),
                        successor_ts: update.ts,
                    });
            }
            match (update.value, update.prev) {
                (Some(document_id), None) => batch.inserts.push(row(update.ts, document_id)),
                // The previous entry is removed by exact identity and the new
                // one inserted as new, so a wrong `prev` fails instead of
                // fabricating history.
                (Some(document_id), Some(prev)) => {
                    batch.previous.push(row(prev.ts, prev.document_id));
                    batch.inserts.push(row(update.ts, document_id));
                },
                (None, Some(prev)) => batch.deletes.push(row(prev.ts, prev.document_id)),
                (None, None) => anyhow::bail!(
                    "MySQL V6 tombstone for an index key with no previous entry (index backfill \
                     is unimplemented)"
                ),
            }
        }
        Ok(batch)
    }

    /// Writes a batch as a few multi-row statements per chunk. Under `Error`
    /// the previous entries a batch replaces are removed by exact identity
    /// first, so a previous entry that is missing, or is not the one the batch
    /// names, fails the commit before any history is written.
    pub(crate) async fn write_index_batch(
        &self,
        tx: &mut MySqlTransaction<'_>,
        batch: &IndexWriteBatch,
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        self.delete_exact(
            tx,
            &batch.previous,
            "MySQL V6 index write replaces an entry that is not there",
            "previous_chunk_write",
            cluster_name,
        )
        .await?;
        for (bucket, rows) in &batch.log_rows {
            for chunk in smart_chunks(rows) {
                let timer = metrics::insert_index_chunk_timer(cluster_name);
                async {
                    tx.exec_drop(
                        &sql::insert_log_chunk(*bucket, chunk.len()),
                        chunk.iter().flat_map(LogRow::params).collect(),
                    )
                    .await
                    .with_context(|| {
                        format!("MySQL V6 index write into log bucket {}", bucket.value())
                    })
                }
                .in_span(chunk_span("log_chunk_write", chunk))
                .await?;
                timer.finish();
            }
        }
        for chunk in smart_chunks(&batch.inserts) {
            let timer = metrics::insert_index_chunk_timer(cluster_name);
            async {
                tx.exec_drop(
                    &sql::insert_latest_chunk(chunk.len()),
                    chunk.iter().flat_map(IndexRow::params).collect(),
                )
                .await
            }
            .in_span(chunk_span("insert_chunk_write", chunk))
            .await?;
            timer.finish();
        }
        self.delete_exact(
            tx,
            &batch.deletes,
            "MySQL V6 tombstone for an index key whose previous entry is not there",
            "delete_chunk_write",
            cluster_name,
        )
        .await
    }

    /// Removes exactly `rows`. Every row has to be found: a count short of the
    /// chunk means a previous entry the batch named is missing or differs from
    /// what is stored.
    async fn delete_exact(
        &self,
        tx: &mut MySqlTransaction<'_>,
        rows: &[IndexRow],
        missing: &str,
        span_kind: &str,
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        for chunk in smart_chunks(rows) {
            let timer = metrics::insert_index_chunk_timer(cluster_name);
            let affected = async {
                tx.exec_iter(
                    &sql::delete_latest_chunk(chunk.len()),
                    chunk.iter().flat_map(IndexRow::delete_params).collect(),
                )
                .await
            }
            .in_span(chunk_span(span_kind, chunk))
            .await?;
            anyhow::ensure!(affected == chunk.len() as u64, "{missing}");
            timer.finish();
        }
        Ok(())
    }
}

/// The span one chunk statement runs in, carrying the properties V5 records.
fn chunk_span(kind: &str, rows: &[impl ApproxSize]) -> Span {
    Span::enter_with_local_parent(format!("write_index_batch::{kind}")).with_properties(|| {
        [
            ("chunk_length", rows.len().to_string()),
            (
                "chunk_bytes",
                rows.iter()
                    .map(ApproxSize::approx_size)
                    .sum::<usize>()
                    .to_string(),
            ),
        ]
    })
}

pub(crate) async fn list_log_buckets<RT: Runtime>(
    connection: &mut MySqlConnection<'_, RT>,
    db_name: &str,
) -> anyhow::Result<BTreeSet<LogBucket>> {
    let buckets = connection
        .query_collect(
            sql::LIST_LOG_TABLES,
            vec![Value::Bytes(db_name.as_bytes().to_vec())],
            16,
            |row| {
                let table_name: String = row.get_opt(0).context("row[0]")??;
                LogBucket::from_table_name(&table_name)
            },
        )
        .await?;
    Ok(buckets.into_iter().collect())
}
