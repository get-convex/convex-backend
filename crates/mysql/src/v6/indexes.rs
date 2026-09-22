//! `Scanning` writes record deletion timestamps in `indexes_backfill_deletes`.
//! After backfill insertion finishes, reconciliation removes `indexes_latest`
//! rows whose timestamps precede their keys' latest deletion timestamps.
//!
//! `Scanning` writes, backfill chunks, and reconciliation pages acquire
//! `indexes_latest` record locks in primary-key order. `Scanning` tombstones
//! participate in the sorted upsert so inserts and deletions follow the same
//! lock order.

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Bound,
};

use anyhow::Context;
use common::{
    knobs::INDEX_RETENTION_DELETE_CHUNK,
    persistence::{
        ConflictStrategy,
        IndexBackfillEntry,
        PersistenceIndexEntry,
    },
    runtime::Runtime,
    types::{
        IndexRef,
        IndexWriteMode,
        PersistenceIndexId,
        Timestamp,
    },
    value::InternalDocumentId,
};
use fastrace::prelude::*;
use mysql_async::Value;

use super::{
    sql::{
        self,
        sort_by_latest_primary_key,
        BackfillDelete,
        IndexKey,
        IndexRow,
        LogBucket,
        LogRow,
        SqlKey,
    },
    DeploymentId,
};
use crate::{
    chunks::{
        fill_chunks,
        ApproxSize,
    },
    connection::{
        MySqlConnection,
        MySqlTransaction,
    },
    metrics,
};

#[derive(Default)]
pub(crate) struct IndexWriteBatch {
    log_rows: BTreeMap<LogBucket, Vec<LogRow>>,
    scan_complete_inserts: Vec<IndexRow>,
    scan_complete_previous: Vec<IndexRow>,
    scan_complete_deletes: Vec<IndexRow>,
    scanning_ops: Vec<ScanningOp>,
}

enum ScanningOp {
    Live(IndexRow),
    Tombstone(IndexRow),
}

impl ScanningOp {
    fn row(&self) -> &IndexRow {
        match self {
            Self::Live(row) | Self::Tombstone(row) => row,
        }
    }

    fn tombstone(&self) -> Option<&IndexRow> {
        match self {
            Self::Tombstone(row) => Some(row),
            Self::Live(_) => None,
        }
    }
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

    /// `ScanComplete` entries must name their persisted predecessor.
    /// `Scanning` entries supply history through `prev` while backfill is
    /// still inserting predecessors.
    pub(crate) fn plan_index_writes(
        &self,
        index_updates: &[PersistenceIndexEntry],
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<IndexWriteBatch> {
        anyhow::ensure!(
            conflict_strategy == ConflictStrategy::Error || index_updates.is_empty(),
            "MySQL V6 index writes with `Overwrite` are unimplemented: index backfill uses \
             `write_index_backfill`"
        );
        let mut batch = IndexWriteBatch::default();
        for update in index_updates {
            let index_id = persistence_index_id(update.index)?;
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
                    "MySQL V6 live index write at {} supersedes a revision at {}, which is not \
                     earlier",
                    update.ts,
                    prev.ts
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
            match (update.value, update.prev, update.mode) {
                (Some(document_id), None, IndexWriteMode::ScanComplete) => batch
                    .scan_complete_inserts
                    .push(row(update.ts, document_id)),
                (Some(document_id), Some(prev), IndexWriteMode::ScanComplete) => {
                    batch
                        .scan_complete_previous
                        .push(row(prev.ts, prev.document_id));
                    batch
                        .scan_complete_inserts
                        .push(row(update.ts, document_id));
                },
                (None, Some(prev), IndexWriteMode::ScanComplete) => batch
                    .scan_complete_deletes
                    .push(row(prev.ts, prev.document_id)),
                (Some(document_id), _, IndexWriteMode::Scanning) => batch
                    .scanning_ops
                    .push(ScanningOp::Live(row(update.ts, document_id))),
                (None, Some(prev), IndexWriteMode::Scanning) => batch
                    .scanning_ops
                    .push(ScanningOp::Tombstone(row(update.ts, prev.document_id))),
                (None, None, _) => {
                    anyhow::bail!("MySQL V6 tombstone for an index key with no previous entry")
                },
            }
        }
        sort_by_latest_primary_key(&mut batch.scan_complete_inserts, |row| row);
        sort_by_latest_primary_key(&mut batch.scan_complete_previous, |row| row);
        sort_by_latest_primary_key(&mut batch.scan_complete_deletes, |row| row);
        // Stable sorting preserves the last operation for each key.
        sort_by_latest_primary_key(&mut batch.scanning_ops, ScanningOp::row);
        Ok(batch)
    }

    pub(crate) fn plan_backfill_rows(
        &self,
        entries: &[IndexBackfillEntry],
    ) -> anyhow::Result<Vec<IndexRow>> {
        let mut rows = entries
            .iter()
            .map(|entry| {
                Ok(IndexRow {
                    deployment_id: self.deployment_id,
                    index_id: persistence_index_id(entry.index)?,
                    key: IndexKey::from_key(entry.key.to_vec()),
                    ts: entry.ts,
                    document_id: entry.document_id,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        sort_by_latest_primary_key(&mut rows, |row| row);
        rows.dedup_by(|a, b| a.latest_primary_key() == b.latest_primary_key());
        Ok(rows)
    }

    pub(crate) async fn write_backfill_chunk(
        &self,
        tx: &mut MySqlTransaction<'_>,
        rows: &[IndexRow],
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        self.write_latest_chunk(
            tx,
            rows,
            sql::insert_backfill_latest_chunk,
            "backfill_chunk_write",
            cluster_name,
        )
        .await
    }

    pub(crate) async fn write_index_batch(
        &self,
        tx: &mut MySqlTransaction<'_>,
        batch: &IndexWriteBatch,
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        self.delete_exact(
            tx,
            &batch.scan_complete_previous,
            "MySQL V6 index write replaces an entry that is not there",
            "previous_chunk_write",
            cluster_name,
        )
        .await?;
        let markers: Vec<&IndexRow> = batch
            .scanning_ops
            .iter()
            .filter_map(ScanningOp::tombstone)
            .collect();
        for chunk in fill_chunks(&markers) {
            let timer = metrics::insert_index_chunk_timer(cluster_name);
            async {
                tx.query_drop(
                    &sql::insert_backfill_marker_chunk(chunk.len()),
                    chunk
                        .iter()
                        .flat_map(|row| row.backfill_marker_params())
                        .collect(),
                )
                .await
            }
            .in_span(chunk_span("marker_chunk_write", chunk))
            .await?;
            timer.finish();
        }
        // Upserting tombstones before deleting them makes every `Scanning`
        // operation acquire its record lock in primary-key order.
        let scanning: Vec<IndexRow> = batch
            .scanning_ops
            .iter()
            .map(|op| op.row().clone())
            .collect();
        for chunk in fill_chunks(&scanning) {
            self.write_latest_chunk(
                tx,
                chunk,
                sql::upsert_latest_chunk,
                "upsert_chunk_write",
                cluster_name,
            )
            .await?;
        }
        let tombstones: Vec<IndexRow> = batch
            .scanning_ops
            .chunk_by(|a, b| a.row().latest_primary_key() == b.row().latest_primary_key())
            .filter_map(|key_ops| key_ops.last()?.tombstone())
            .cloned()
            .collect();
        self.delete_exact(
            tx,
            &tombstones,
            "MySQL V6 tombstone's own row is not there",
            "tombstone_chunk_write",
            cluster_name,
        )
        .await?;
        for (bucket, rows) in &batch.log_rows {
            for chunk in fill_chunks(rows) {
                let timer = metrics::insert_index_chunk_timer(cluster_name);
                async {
                    tx.query_drop(
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
        for chunk in fill_chunks(&batch.scan_complete_inserts) {
            self.write_latest_chunk(
                tx,
                chunk,
                sql::insert_latest_chunk,
                "insert_chunk_write",
                cluster_name,
            )
            .await?;
        }
        self.delete_exact(
            tx,
            &batch.scan_complete_deletes,
            "MySQL V6 tombstone for an index key whose previous entry is not there",
            "delete_chunk_write",
            cluster_name,
        )
        .await
    }

    pub(crate) async fn read_backfill_markers_page<RT: Runtime>(
        &self,
        connection: &mut MySqlConnection<'_, RT>,
        index: PersistenceIndexId,
        after: Bound<SqlKey>,
    ) -> anyhow::Result<Vec<BackfillDelete>> {
        let (query, params) = sql::backfill_markers_page(
            self.deployment_id,
            index,
            after,
            *INDEX_RETENTION_DELETE_CHUNK,
        );
        let deployment_id = self.deployment_id;
        connection
            .query_collect(&query, params, *INDEX_RETENTION_DELETE_CHUNK, move |row| {
                BackfillDelete::from_row(deployment_id, index, &row)
            })
            .await
    }

    pub(crate) async fn delete_stale(
        &self,
        tx: &mut MySqlTransaction<'_>,
        deletes: &[BackfillDelete],
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        for chunk in fill_chunks(deletes) {
            let timer = metrics::insert_index_chunk_timer(cluster_name);
            async {
                tx.query_drop(
                    &sql::delete_stale_chunk(chunk.len()),
                    chunk.iter().flat_map(BackfillDelete::params).collect(),
                )
                .await
            }
            .in_span(chunk_span("reconcile_chunk_write", chunk))
            .await?;
            timer.finish();
        }
        Ok(())
    }

    async fn write_latest_chunk(
        &self,
        tx: &mut MySqlTransaction<'_>,
        rows: &[IndexRow],
        chunk_sql: fn(usize) -> String,
        span_kind: &str,
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        let timer = metrics::insert_index_chunk_timer(cluster_name);
        async {
            tx.query_drop(
                &chunk_sql(rows.len()),
                rows.iter().flat_map(IndexRow::params).collect(),
            )
            .await
        }
        .in_span(chunk_span(span_kind, rows))
        .await?;
        timer.finish();
        Ok(())
    }

    async fn delete_exact(
        &self,
        tx: &mut MySqlTransaction<'_>,
        rows: &[IndexRow],
        missing: &str,
        span_kind: &str,
        cluster_name: &str,
    ) -> anyhow::Result<()> {
        for chunk in fill_chunks(rows) {
            let timer = metrics::insert_index_chunk_timer(cluster_name);
            let affected = async {
                tx.query_iter(
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

fn persistence_index_id(index: IndexRef) -> anyhow::Result<PersistenceIndexId> {
    index.persistence_index_id().with_context(|| {
        format!(
            "MySQL V6 requires a persistence index ID to write index {}",
            index.id()
        )
    })
}

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
