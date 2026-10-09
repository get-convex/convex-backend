//! V6 index scans over deployment-scoped documents.
use std::{
    ops::Bound,
    sync::Arc,
};

use anyhow::Context;
use common::{
    cover,
    index::{
        IndexKeyBytes,
        MAX_INDEX_KEY_PREFIX_LEN,
    },
    interval::Interval,
    knobs::{
        MYSQL_FALLBACK_PAGE_SIZE,
        MYSQL_MAX_QUERY_BATCH_SIZE,
        MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE,
        MYSQL_MIN_QUERY_BATCH_SIZE,
    },
    persistence::{
        LatestDocument,
        RetentionValidator,
    },
    query::Order,
    runtime::Runtime,
    types::{
        IndexRef,
        Timestamp,
    },
    value::{
        InternalId,
        TabletId,
    },
};
use errors::ErrorMetadata;
use futures::Stream;
use futures_async_stream::try_stream_block;
use mysql_async::{
    Row,
    Value,
};

use super::{
    column,
    sql::{
        self,
        LogBucket,
        LogBucketBounds,
    },
    PersistenceDeploymentId,
};
use crate::{
    connection::is_message_too_large_error,
    document_encoding,
    metrics,
    ConvexMySqlPool,
};

pub(crate) struct IndexReader<'a, RT: Runtime> {
    pub(crate) pool: &'a ConvexMySqlPool<RT>,
    pub(crate) db_name: &'a str,
    pub(crate) deployment_id: PersistenceDeploymentId,
}

impl<'a, RT: Runtime> IndexReader<'a, RT> {
    /// What maintenance last published about the log buckets, read on the
    /// scan's connection right before the page that uses it.
    async fn log_bucket_bounds(
        &self,
        connection: &mut crate::connection::MySqlConnection<'_, RT>,
    ) -> anyhow::Result<LogBucketBounds> {
        let row: Row = connection
            .query_optional(sql::READ_LOG_BUCKET_BOUNDS, vec![])
            .await?
            .context("MySQL V6 log bucket maintenance state is missing")?;
        let created_through_ts: i64 = row.get_opt(0).context("created_through_ts")??;
        let oldest_kept_ts: i64 = row.get_opt(1).context("oldest_kept_ts")??;
        LogBucketBounds::from_state(created_through_ts, oldest_kept_ts)
    }

    pub(crate) fn scan(
        self,
        index: IndexRef,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: Interval,
        order: Order,
        size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> impl Stream<Item = anyhow::Result<(IndexKeyBytes, LatestDocument)>> + 'a {
        try_stream_block!({
            retention_validator.optimistic_validate_snapshot(read_timestamp)?;
            let _timer = metrics::query_index_timer(self.pool.cluster_name());
            let mut stats = metrics::QueryIndexStats::new(self.pool.cluster_name());
            let (mut lower, mut upper) = sql::to_sql_bounds(interval.clone());
            let snapshot_bucket = LogBucket::from_successor_ts(read_timestamp);
            let persistence_index_id = index.persistence_index_id().with_context(|| {
                format!(
                    "MySQL V6 requires a persistence index ID to read index {}",
                    index.id()
                )
            })?;
            // The size hint makes the common case one query. Later pages grow to
            // correct for tombstones, long prefixes and a wrong hint, unless a
            // fallback pinned the size.
            let mut page_size =
                size_hint.clamp(*MYSQL_MIN_QUERY_BATCH_SIZE, *MYSQL_MAX_QUERY_BATCH_SIZE);
            let mut fallback = false;
            let mut buffered_prefix = None;
            let mut buffered = Vec::new();
            loop {
                let mut connection = self.pool.acquire("v6_index_scan", self.db_name).await?;
                let bounds = self.log_bucket_bounds(&mut connection).await?;
                stats.sql_statements += 1;
                let buckets = bounds.covering(snapshot_bucket);
                let prepare_timer =
                    metrics::query_index_sql_prepare_timer(self.pool.cluster_name());
                let (query, params) = sql::index_query(
                    self.deployment_id,
                    persistence_index_id,
                    read_timestamp,
                    lower.clone(),
                    upper.clone(),
                    order,
                    page_size,
                    &buckets,
                );
                prepare_timer.finish();
                let execute_timer =
                    metrics::query_index_sql_execute_timer(self.pool.cluster_name());
                let rows = match connection
                    .query_collect(&query, params, page_size, Ok)
                    .await
                {
                    Ok(rows) => rows,
                    Err(ref error)
                        if let Some(server_error) = is_message_too_large_error(error) =>
                    {
                        anyhow::ensure!(
                            page_size > 1,
                            "Failed to load index rows with minimum page size `1`: {}",
                            server_error.message
                        );
                        let fallback_size = usize::try_from(*MYSQL_FALLBACK_PAGE_SIZE)?;
                        if page_size <= fallback_size {
                            tracing::warn!(
                                "Falling back to page size `1` due to repeated server error: {}",
                                server_error.message
                            );
                            page_size = 1;
                        } else {
                            tracing::warn!(
                                "Falling back to page size `{fallback_size}` due to server error: \
                                 {}",
                                server_error.message
                            );
                            page_size = fallback_size;
                        }
                        fallback = true;
                        continue;
                    },
                    Err(error) => return Err(error),
                };
                execute_timer.finish();
                // Check after reading the page to include concurrent displacements.
                // Below the floor, any later commit may have displaced a revision
                // into an omitted bucket. `write` requires a document revision for
                // every displacement, so the documents table provides this check
                // even after the bucket is dropped.
                if snapshot_bucket < bounds.floor {
                    stats.sql_statements += 1;
                    let displaced_since_snapshot = connection
                        .query_optional(
                            sql::HAS_COMMIT_BETWEEN,
                            vec![
                                self.deployment_id.into(),
                                Value::Int(i64::from(read_timestamp)),
                                Value::Int(i64::from(bounds.floor.start_ts()?)),
                            ],
                        )
                        .await?
                        .is_some();
                    if displaced_since_snapshot {
                        cover!(super::coverage::OUT_OF_RETENTION);
                        return Err(out_of_retention_error(
                            read_timestamp,
                            format!(
                                "a later commit displaced revisions into a log bucket below the \
                                 maintenance floor {}",
                                bounds.floor.value()
                            ),
                        ));
                    }
                }
                drop(connection);
                let retention_validate_timer =
                    metrics::retention_validate_timer(self.pool.cluster_name());
                retention_validator
                    .validate_snapshot(read_timestamp)
                    .await?;
                retention_validate_timer.finish();
                let rows_loaded = rows.len();
                let mut cursor = None;
                for row in rows {
                    stats.rows_read += 1;
                    let prefix = column::bytes(&row, 1)?.to_vec();
                    let suffix_hash = column::bytes(&row, 2)?.to_vec();
                    cursor = Some(sql::SqlKey {
                        prefix: prefix.clone(),
                        suffix_hash,
                    });
                    if buffered_prefix.as_ref().is_some_and(|p| p != &prefix) {
                        buffered.sort_by(|a: &(IndexKeyBytes, LatestDocument), b| a.0.cmp(&b.0));
                        if order == Order::Desc {
                            buffered.reverse();
                        }
                        for result in buffered.drain(..) {
                            yield result;
                        }
                    }
                    buffered_prefix = Some(prefix.clone());
                    // A prefix shorter than the limit is the whole key, so no other
                    // row can share it: yield now instead of reading the next page
                    // to find where this prefix's run ends.
                    let complete_key = prefix.len() < MAX_INDEX_KEY_PREFIX_LEN;

                    let mut key = prefix;
                    if let Some(suffix) = column::maybe_bytes(&row, 3)? {
                        cover!(super::coverage::LONG_KEY_SUFFIX);
                        key.extend_from_slice(suffix);
                    }
                    let key = IndexKeyBytes(key);
                    if !interval.contains(&key) {
                        stats.rows_skipped_out_of_range += 1;
                        continue;
                    }
                    let ts: i64 = row.get_opt(4).context("row[4]")??;
                    let ts = Timestamp::try_from(ts)?;
                    let table_id = TabletId(InternalId::try_from(column::bytes(&row, 5)?)?);
                    anyhow::ensure!(table_id == tablet_id);
                    let encoded = column::maybe_bytes(&row, 7)?
                        .with_context(|| format!("Dangling index reference for {key:?} {ts:?}"))?;
                    let document =
                        document_encoding::decode(encoded, table_id)?.with_context(|| {
                            format!("Index reference to deleted document {key:?} {ts:?}")
                        })?;
                    let prev_ts: Option<i64> = row.get_opt(8).context("row[8]")??;
                    buffered.push((
                        key,
                        LatestDocument {
                            ts,
                            value: document,
                            prev_ts: prev_ts.map(Timestamp::try_from).transpose()?,
                        },
                    ));
                    stats.rows_returned += 1;
                    stats.max_rows_buffered = stats.max_rows_buffered.max(buffered.len());
                    if complete_key {
                        buffered_prefix = None;
                        for result in buffered.drain(..) {
                            yield result;
                        }
                    }
                }
                if rows_loaded < page_size {
                    break;
                }
                let cursor = cursor.context("full V6 index page has no cursor")?;
                cover!(super::coverage::SCAN_RESUMED);
                match order {
                    Order::Asc => lower = Bound::Excluded(cursor),
                    Order::Desc => upper = Bound::Excluded(cursor),
                }
                if page_size < *MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE && !fallback {
                    page_size = (page_size * 2).min(*MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE);
                }
            }
            buffered.sort_by(|a: &(IndexKeyBytes, LatestDocument), b| a.0.cmp(&b.0));
            if order == Order::Desc {
                buffered.reverse();
            }
            for result in buffered {
                yield result;
            }
        })
    }
}

fn out_of_retention_error(read_timestamp: Timestamp, reason: String) -> anyhow::Error {
    anyhow::anyhow!(ErrorMetadata::out_of_retention()).context(format!(
        "V6 index snapshot {read_timestamp} is outside the retained log buckets: {reason}"
    ))
}
