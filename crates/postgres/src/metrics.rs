use std::ops::Deref;

use common::pool_stats::ConnectionPoolStats;
use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    CancelableTimer,
    StaticMetricLabel,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::{
    VMHistogram,
    VMHistogramVec,
};
use tokio_postgres::{
    types::FromSql,
    Row,
};

register_convex_histogram!(
    POSTGRES_LOAD_DOCUMENTS_SECONDS,
    "Time to load documents",
    &STATUS_LABEL
);
pub fn load_documents_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_LOAD_DOCUMENTS_SECONDS)
}

register_convex_counter!(
    POSTGRES_DOCUMENTS_LOADED_TOTAL,
    "Number of documents loaded"
);
pub fn finish_load_documents_timer(timer: StatusTimer, num_loaded: usize) {
    log_counter(&POSTGRES_DOCUMENTS_LOADED_TOTAL, num_loaded as u64);
    timer.finish();
}

register_convex_histogram!(
    POSTGRES_DOCUMENTS_MULTIGET_SECONDS,
    "Time to fetch documents at exact timestamps",
    &STATUS_LABEL
);
pub fn previous_revisions_of_documents_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_DOCUMENTS_MULTIGET_SECONDS)
}

register_convex_histogram!(
    POSTGRES_PREV_REVISIONS_SECONDS,
    "Time to fetch previous revisions",
    &STATUS_LABEL
);
pub fn prev_revisions_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_PREV_REVISIONS_SECONDS)
}

// There is no finish_index_timer since we use that from a stream that can
// potentially not be fully exhausted. We let the timer be dropped instead and
// do not labels with success or error.
register_convex_histogram!(POSTGRES_QUERY_INDEX_SECONDS, "Time to query the index");
pub fn query_index_timer() -> Timer<VMHistogram> {
    Timer::new(&POSTGRES_QUERY_INDEX_SECONDS)
}

pub struct QueryIndexStats {
    pub sql_statements: usize,
    pub rows_skipped_deleted: usize,
    pub rows_skipped_out_of_range: usize,
    pub rows_returned: usize,
    pub max_rows_buffered: usize,
}

impl QueryIndexStats {
    pub fn new() -> Self {
        Self {
            sql_statements: 0,
            rows_skipped_deleted: 0,
            rows_skipped_out_of_range: 0,
            rows_returned: 0,
            max_rows_buffered: 0,
        }
    }
}

register_convex_counter!(
    POSTGRES_QUERY_INDEX_SQL_STATEMENTS,
    "Number of index query SQL statements"
);
register_convex_histogram!(
    POSTGRES_QUERY_INDEX_SQL_PER_QUERY_STATEMENTS,
    "Number of index query SQL statements per query"
);
register_convex_counter!(
    POSTGRES_QUERY_INDEX_SKIPPED_DELETED_ROWS,
    "Number of index query rows skipped"
);
register_convex_histogram!(
    POSTGRES_QUERY_INDEX_SKIPPED_DELETED_PER_QUERY_ROWS,
    "Number of index query rows skipped per query"
);
register_convex_counter!(
    POSTGRES_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_ROWS,
    "Number of index query out-of-range rows skipped"
);
register_convex_histogram!(
    POSTGRES_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_PER_QUERY_ROWS,
    "Number of index query out-of-range rows skipped per query"
);
register_convex_counter!(
    POSTGRES_QUERY_INDEX_RETURNED_ROWS,
    "Number of index query rows returned"
);
register_convex_histogram!(
    POSTGRES_QUERY_INDEX_RETURNED_PER_QUERY_ROWS,
    "Number of index query rows returned per query"
);
register_convex_counter!(
    POSTGRES_QUERY_INDEX_MAX_BUFFERED_ROWS,
    "Number of index query buffered rows"
);
register_convex_histogram!(
    POSTGRES_QUERY_INDEX_MAX_BUFFERED_PER_QUERY_ROWS,
    "Number of index query buffered rows per query",
);
impl Drop for QueryIndexStats {
    fn drop(&mut self) {
        log_counter(
            &POSTGRES_QUERY_INDEX_SQL_STATEMENTS,
            self.sql_statements as u64,
        );
        log_distribution(
            &POSTGRES_QUERY_INDEX_SQL_PER_QUERY_STATEMENTS,
            self.sql_statements as f64,
        );
        log_counter(
            &POSTGRES_QUERY_INDEX_SKIPPED_DELETED_ROWS,
            self.rows_skipped_deleted as u64,
        );
        log_distribution(
            &POSTGRES_QUERY_INDEX_SKIPPED_DELETED_PER_QUERY_ROWS,
            self.rows_skipped_deleted as f64,
        );
        log_counter(
            &POSTGRES_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_ROWS,
            self.rows_skipped_out_of_range as u64,
        );
        log_distribution(
            &POSTGRES_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_PER_QUERY_ROWS,
            self.rows_skipped_out_of_range as f64,
        );
        log_counter(
            &POSTGRES_QUERY_INDEX_RETURNED_ROWS,
            self.rows_returned as u64,
        );
        log_distribution(
            &POSTGRES_QUERY_INDEX_RETURNED_PER_QUERY_ROWS,
            self.rows_returned as f64,
        );
        log_counter(
            &POSTGRES_QUERY_INDEX_MAX_BUFFERED_ROWS,
            self.max_rows_buffered as u64,
        );
        log_distribution(
            &POSTGRES_QUERY_INDEX_MAX_BUFFERED_PER_QUERY_ROWS,
            self.max_rows_buffered as f64,
        );
    }
}

register_convex_histogram!(
    POSTGRES_GET_CONNECTION_SECONDS,
    "Time to get Postgres connection",
    &STATUS_LABEL
);
pub fn get_connection_timer() -> CancelableTimer {
    CancelableTimer::new(&POSTGRES_GET_CONNECTION_SECONDS)
}

register_convex_histogram!(
    POSTGRES_CONNECTION_LIFETIME_SECONDS,
    "Time a postgres connection was used for",
    &["name"]
);
pub fn connection_lifetime_timer(name: &'static str) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&POSTGRES_CONNECTION_LIFETIME_SECONDS);
    timer.add_label(StaticMetricLabel::new("name", name));
    timer
}

register_convex_counter!(
    POSTGRES_POISONED_CONNECTIONS,
    "Number of times connections were poisoned",
);
pub fn log_poisoned_connection() {
    POSTGRES_POISONED_CONNECTIONS.inc();
}

register_convex_histogram!(
    POSTGRES_POOL_ACTIVE_CONNECTIONS,
    "Number of active connections",
    &["cluster_name"]
);
register_convex_gauge!(
    POSTGRES_POOL_MAX_CONNECTIONS,
    "The maximum number of active connections for the lifetime of the pool",
    &["cluster_name"]
);
pub fn new_connection_pool_stats(cluster_name: &str) -> ConnectionPoolStats {
    ConnectionPoolStats::new(
        &POSTGRES_POOL_ACTIVE_CONNECTIONS,
        &POSTGRES_POOL_MAX_CONNECTIONS,
        vec![StaticMetricLabel::new(
            "cluster_name",
            cluster_name.to_owned(),
        )],
    )
}

register_convex_histogram!(
    POSTGRES_QUERY_INDEX_SQL_PREPARE_SECONDS,
    "Time to prepare query index SQL",
    &STATUS_LABEL
);
pub fn query_index_sql_prepare_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_QUERY_INDEX_SQL_PREPARE_SECONDS)
}

register_convex_histogram!(
    POSTGRES_QUERY_INDEX_SQL_EXECUTE_SECONDS,
    "Time to execute query index SQL",
    &STATUS_LABEL
);
pub fn query_index_sql_execute_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_QUERY_INDEX_SQL_EXECUTE_SECONDS)
}

register_convex_histogram!(
    POSTGRES_RETENTION_VALIDATE_SECONDS,
    "Time to validate retention",
    &STATUS_LABEL
);
pub fn retention_validate_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_RETENTION_VALIDATE_SECONDS)
}

register_convex_histogram!(
    POSTGRES_INSERT_CHUNK_SECONDS,
    "Time to insert a chunk of documents",
    &STATUS_LABEL
);
pub fn insert_document_chunk_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_INSERT_CHUNK_SECONDS)
}

register_convex_histogram!(
    POSTGRES_INSERT_ONE_SECONDS,
    "Time to insert one document",
    &STATUS_LABEL
);
pub fn insert_one_document_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_INSERT_ONE_SECONDS)
}

register_convex_histogram!(
    POSTGRES_INSERT_INDEX_CHUNK_SECONDS,
    "Time to insert an index chunk",
    &STATUS_LABEL
);
pub fn insert_index_chunk_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_INSERT_INDEX_CHUNK_SECONDS)
}

register_convex_histogram!(
    POSTGRES_INSERT_ONE_INDEX_SECONDS,
    "Time to insert one index",
    &STATUS_LABEL
);
pub fn insert_one_index_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_INSERT_ONE_INDEX_SECONDS)
}

register_convex_histogram!(
    POSTGRES_WRITE_BYTES,
    "Number of bytes written in Postgres writes"
);
pub fn log_write_bytes(size: usize) {
    log_distribution(&POSTGRES_WRITE_BYTES, size as f64);
}

register_convex_histogram!(
    POSTGRES_WRITE_DOCUMENTS,
    "Number of documents written in Postgres writes",
);
pub fn log_write_documents(size: usize) {
    log_distribution(&POSTGRES_WRITE_DOCUMENTS, size as f64);
}

register_convex_histogram!(
    POSTGRES_LEASE_ACQUIRE_SECONDS,
    "Time to acquire a lease",
    &STATUS_LABEL
);
pub fn lease_acquire_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_LEASE_ACQUIRE_SECONDS)
}

register_convex_histogram!(
    POSTGRES_ADVISORY_LEASE_CHECK_SECONDS,
    "Time to check lease is still held at the start of a transaction",
    &STATUS_LABEL
);
pub fn lease_check_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_ADVISORY_LEASE_CHECK_SECONDS)
}

register_convex_histogram!(
    POSTGRES_LEASE_PRECOND_SECONDS,
    "Time to check lease precondition",
    &STATUS_LABEL
);
pub fn lease_precond_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_LEASE_PRECOND_SECONDS)
}

register_convex_histogram!(
    POSTGRES_COMMIT_SECONDS,
    "Postgres commit duration",
    &STATUS_LABEL
);
pub fn commit_timer() -> StatusTimer {
    StatusTimer::new(&POSTGRES_COMMIT_SECONDS)
}

register_convex_counter!(
    POSTGRES_QUERY_TOTAL,
    "Number of Postgres queries",
    &["name"]
);
pub fn log_query(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&POSTGRES_QUERY_TOTAL, 1, labels)
}

struct RawSqlValue<'a>(&'a [u8]);

impl<'a> FromSql<'a> for RawSqlValue<'a> {
    fn from_sql(
        _ty: &tokio_postgres::types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(raw))
    }

    fn accepts(_ty: &tokio_postgres::types::Type) -> bool {
        true
    }
}

impl<'a> Deref for RawSqlValue<'a> {
    type Target = &'a [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

register_convex_counter!(
    POSTGRES_QUERY_RESULT_TOTAL,
    "Number of times a query result was fetched",
    &["name"]
);
register_convex_counter!(
    POSTGRES_QUERY_RESULT_BYTES,
    "Total size of Postgres query result",
    &["name"]
);
pub fn log_query_result(row: &Row, labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&POSTGRES_QUERY_RESULT_TOTAL, 1, labels.clone());
    let mut total_data_size = 0;
    for i in 0..row.len() {
        let col_bytes: Option<RawSqlValue> = row.get(i);
        if let Some(col_bytes) = col_bytes {
            total_data_size += col_bytes.len();
        }
    }
    log_counter_with_labels(&POSTGRES_QUERY_RESULT_BYTES, total_data_size as u64, labels);
}

register_convex_counter!(
    POSTGRES_EXECUTE_TOTAL,
    "Total Postgres executions",
    &["name"]
);
pub fn log_execute(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&POSTGRES_EXECUTE_TOTAL, 1, labels)
}

register_convex_counter!(
    POSTGRES_TRANSACTION_TOTAL,
    "Total Postgres transactions",
    &["name"]
);
pub fn log_transaction(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&POSTGRES_TRANSACTION_TOTAL, 1, labels)
}

register_convex_counter!(
    POSTGRES_IMPORT_BATCH_ROWS,
    "Number of rows batch-imported into a Postgres database",
    &["target"]
);

pub fn log_import_batch_rows(rows: usize, target: &'static str) {
    log_counter_with_labels(
        &POSTGRES_IMPORT_BATCH_ROWS,
        rows as u64,
        vec![StaticMetricLabel::new("target", target)],
    )
}
