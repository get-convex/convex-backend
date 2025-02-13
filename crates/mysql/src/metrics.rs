use common::pool_stats::ConnectionPoolStats;
use metrics::{
    log_counter_with_labels,
    log_distribution,
    log_distribution_with_labels,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    CancelableTimer,
    ProgressCounter,
    StaticMetricLabel,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use mysql_async::Row;
use prometheus::VMHistogramVec;

fn cluster_name_label(cluster_name: &str) -> StaticMetricLabel {
    StaticMetricLabel::new("cluster_name", cluster_name.to_owned())
}

register_convex_histogram!(
    MYSQL_WRITE_PERSISTENCE_GLOBAL_SECONDS,
    "Time to write persistence global",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn write_persistence_global_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_WRITE_PERSISTENCE_GLOBAL_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_LOAD_DOCUMENTS_SECONDS,
    "Time to load documents",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn load_documents_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_LOAD_DOCUMENTS_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_LOAD_DOCUMENTS_SKIPPED_WRONG_TABLE_TOTAL,
    "Number of documents skipped in memory because they belong to the wrong table",
    &["cluster_name"]
);
pub fn mysql_load_documents_skipped_wrong_table(num_skipped: usize, cluster_name: &str) {
    log_distribution_with_labels(
        &MYSQL_LOAD_DOCUMENTS_SKIPPED_WRONG_TABLE_TOTAL,
        num_skipped as f64,
        vec![cluster_name_label(cluster_name)],
    )
}

register_convex_counter!(
    MYSQL_DOCUMENTS_LOADED_TOTAL,
    "Number of documents loaded",
    &["cluster_name"]
);
pub fn finish_load_documents_timer(timer: StatusTimer, num_loaded: usize, cluster_name: &str) {
    log_counter_with_labels(
        &MYSQL_DOCUMENTS_LOADED_TOTAL,
        num_loaded as u64,
        vec![cluster_name_label(cluster_name)],
    );
    timer.finish();
}

register_convex_histogram!(
    MYSQL_DOCUMENTS_MULTIGET_SECONDS,
    "Time to fetch documents at exact timestamps",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn documents_multiget_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_DOCUMENTS_MULTIGET_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_PREV_REVISIONS_SECONDS,
    "Time to fetch previous revisions",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn prev_revisions_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_PREV_REVISIONS_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_counter!(
    MYSQL_PREV_REVISIONS_ROWS_READ_TOTAL,
    "Number of rows read to fetch previous revisions",
    &["cluster_name"]
);
pub fn log_prev_revisions_row_read(cluster_name: &str) {
    log_counter_with_labels(
        &MYSQL_PREV_REVISIONS_ROWS_READ_TOTAL,
        1,
        vec![cluster_name_label(cluster_name)],
    );
}

// There is no finish_index_timer since we use that from a stream that can
// potentially not be fully exhausted. We let the timer be dropped instead and
// do not tag with success or error.
register_convex_histogram!(
    MYSQL_QUERY_INDEX_SECONDS,
    "Time to query the index",
    &["cluster_name"]
);
pub fn query_index_timer(cluster_name: &str) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&MYSQL_QUERY_INDEX_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

pub struct QueryIndexStats<'a> {
    pub sql_statements: usize,
    // Rows read from MySQL.
    pub rows_read: usize,
    // Tombstones skipped.
    pub rows_skipped_deleted: usize,
    // Rows skipped due to long prefix.
    pub rows_skipped_out_of_range: usize,
    // Rows returned.
    pub rows_returned: usize,
    pub max_rows_buffered: usize,
    cluster_name: &'a str,
}

impl<'a> QueryIndexStats<'a> {
    pub fn new(cluster_name: &'a str) -> Self {
        Self {
            sql_statements: 0,
            rows_read: 0,
            rows_skipped_deleted: 0,
            rows_skipped_out_of_range: 0,
            rows_returned: 0,
            max_rows_buffered: 0,
            cluster_name,
        }
    }
}
register_convex_counter!(
    MYSQL_QUERY_INDEX_SQL_STATEMENTS,
    "Number of index query SQL statements",
    &["cluster_name"]
);
register_convex_histogram!(
    MYSQL_QUERY_INDEX_SQL_PER_QUERY_STATEMENTS,
    "Number of index query SQL statements per query",
    &["cluster_name"]
);
register_convex_counter!(
    MYSQL_QUERY_INDEX_READ_ROWS,
    "Number of index query rows read from the database",
    &["cluster_name"]
);
register_convex_counter!(
    MYSQL_QUERY_INDEX_SKIPPED_DELETED_ROWS,
    "Number of index query rows skipped",
    &["cluster_name"]
);
register_convex_histogram!(
    MYSQL_QUERY_INDEX_SKIPPED_DELETED_PER_QUERY_ROWS,
    "Number of index query rows skipped per query",
    &["cluster_name"]
);
register_convex_counter!(
    MYSQL_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_ROWS,
    "Number of index query out-of-range rows skipped",
    &["cluster_name"]
);
register_convex_histogram!(
    MYSQL_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_PER_QUERY_ROWS,
    "Number of index query out-of-range rows skipped per query",
    &["cluster_name"]
);
register_convex_counter!(
    MYSQL_QUERY_INDEX_RETURNED_ROWS,
    "Number of index query rows returned",
    &["cluster_name"]
);
register_convex_histogram!(
    MYSQL_QUERY_INDEX_RETURNED_PER_QUERY_ROWS,
    "Number of index query rows returned per query",
    &["cluster_name"]
);
register_convex_counter!(
    MYSQL_QUERY_INDEX_MAX_BUFFERED_ROWS,
    "Number of index query buffered rows",
    &["cluster_name"]
);
register_convex_histogram!(
    MYSQL_QUERY_INDEX_MAX_BUFFERED_PER_QUERY_ROWS,
    "Number of index query buffered rows per query",
    &["cluster_name"]
);

impl Drop for QueryIndexStats<'_> {
    fn drop(&mut self) {
        let labels = vec![cluster_name_label(self.cluster_name)];
        log_counter_with_labels(
            &MYSQL_QUERY_INDEX_SQL_STATEMENTS,
            self.sql_statements as u64,
            labels.clone(),
        );
        log_distribution_with_labels(
            &MYSQL_QUERY_INDEX_SQL_PER_QUERY_STATEMENTS,
            self.sql_statements as f64,
            labels.clone(),
        );
        log_counter_with_labels(
            &MYSQL_QUERY_INDEX_READ_ROWS,
            self.rows_read as u64,
            labels.clone(),
        );
        log_counter_with_labels(
            &MYSQL_QUERY_INDEX_SKIPPED_DELETED_ROWS,
            self.rows_skipped_deleted as u64,
            labels.clone(),
        );
        log_distribution_with_labels(
            &MYSQL_QUERY_INDEX_SKIPPED_DELETED_PER_QUERY_ROWS,
            self.rows_skipped_deleted as f64,
            labels.clone(),
        );
        log_counter_with_labels(
            &MYSQL_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_ROWS,
            self.rows_skipped_out_of_range as u64,
            labels.clone(),
        );
        log_distribution_with_labels(
            &MYSQL_QUERY_INDEX_SKIPPED_OUT_OF_RANGE_PER_QUERY_ROWS,
            self.rows_skipped_out_of_range as f64,
            labels.clone(),
        );
        log_counter_with_labels(
            &MYSQL_QUERY_INDEX_RETURNED_ROWS,
            self.rows_returned as u64,
            labels.clone(),
        );
        log_distribution_with_labels(
            &MYSQL_QUERY_INDEX_RETURNED_PER_QUERY_ROWS,
            self.rows_returned as f64,
            labels.clone(),
        );
        log_counter_with_labels(
            &MYSQL_QUERY_INDEX_MAX_BUFFERED_ROWS,
            self.max_rows_buffered as u64,
            labels.clone(),
        );
        log_distribution_with_labels(
            &MYSQL_QUERY_INDEX_MAX_BUFFERED_PER_QUERY_ROWS,
            self.max_rows_buffered as f64,
            labels,
        );
    }
}

register_convex_histogram!(
    MYSQL_GET_CONNECTION_SECONDS,
    "Time to get a connection",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn get_connection_timer(cluster_name: &str) -> CancelableTimer {
    let mut timer = CancelableTimer::new(&MYSQL_GET_CONNECTION_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_BEGIN_TRANSACTION_SECONDS,
    "Time to get a connection",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn begin_transaction_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_BEGIN_TRANSACTION_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_CONNECTION_LIFETIME_SECONDS,
    "Time a mysql connection was used for",
    &["name", "cluster_name"]
);
pub fn connection_lifetime_timer(name: &'static str, cluster_name: &str) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&MYSQL_CONNECTION_LIFETIME_SECONDS);
    timer.add_label(StaticMetricLabel::new("name", name));
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_POOL_ACTIVE_CONNECTIONS,
    "Number of active connections",
    &["cluster_name"]
);
register_convex_gauge!(
    MYSQL_POOL_MAX_CONNECTIONS,
    "The maximum number of active connections for the lifetime of the pool",
    &["cluster_name"]
);
pub fn new_connection_pool_stats(cluster_name: &str) -> ConnectionPoolStats {
    ConnectionPoolStats::new(
        &MYSQL_POOL_ACTIVE_CONNECTIONS,
        &MYSQL_POOL_MAX_CONNECTIONS,
        vec![StaticMetricLabel::new(
            "cluster_name",
            cluster_name.to_owned(),
        )],
    )
}

register_convex_histogram!(
    MYSQL_QUERY_INDEX_SQL_PREPARE_SECONDS,
    "Time ot prepare index query SQL",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn query_index_sql_prepare_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_QUERY_INDEX_SQL_PREPARE_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_QUERY_INDEX_SQL_EXECUTE_SECONDS,
    "Time to execute index query SQL",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn query_index_sql_execute_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_QUERY_INDEX_SQL_EXECUTE_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_RETENTION_VALIDATE_SECONDS,
    "Time to validate retention",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn retention_validate_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_RETENTION_VALIDATE_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_INSERT_CHUNK_SECONDS,
    "Time to insert a chunk of doucments",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn insert_document_chunk_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_INSERT_CHUNK_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_INSERT_INDEX_CHUNK_SECONDS,
    "Time to insert an index chunk",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn insert_index_chunk_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_INSERT_INDEX_CHUNK_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(MYSQL_WRITE_BYTES, "Number of bytes written in MySQL writes");
pub fn log_write_bytes(size: usize) {
    log_distribution(&MYSQL_WRITE_BYTES, size as f64);
}

register_convex_histogram!(
    MYSQL_WRITE_DOCUMENTS,
    "Number of documents written in MySQL writes",
);
pub fn log_write_documents(size: usize) {
    log_distribution(&MYSQL_WRITE_DOCUMENTS, size as f64);
}

register_convex_histogram!(
    MYSQL_LEASE_ACQUIRE_SECONDS,
    "Time to acquire a lease",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn lease_acquire_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_LEASE_ACQUIRE_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_LEASE_PRECOND_SECONDS,
    "Time to check lease preconditions",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn lease_precond_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_LEASE_PRECOND_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_histogram!(
    MYSQL_COMMIT_SECONDS,
    "Time to commit a transaction",
    &[STATUS_LABEL[0], "cluster_name"]
);
pub fn commit_timer(cluster_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&MYSQL_COMMIT_SECONDS);
    timer.add_label(cluster_name_label(cluster_name));
    timer
}

register_convex_counter!(
    MYSQL_QUERY_TOTAL,
    "Total number of queries executed",
    &["name", "cluster_name"]
);
pub fn log_query(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&MYSQL_QUERY_TOTAL, 1, labels)
}

register_convex_counter!(
    MYSQL_QUERY_RESULT_TOTAL,
    "Total number of query results",
    &["name", "cluster_name"]
);
register_convex_counter!(
    MYSQL_QUERY_RESULT_BYTES,
    "Total size of query results",
    &["name", "cluster_name"]
);

pub fn log_query_result(row: &Row, labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&MYSQL_QUERY_RESULT_TOTAL, 1, labels.clone());
    let mut total_data_size = 0;
    for i in 0..row.len() {
        // Only counts size from BLOBs because the interface doesn't allow
        // generic parsing. All JsonValues are BLOBs though so this is almost
        // everything.
        let col_bytes: Option<Result<Vec<u8>, _>> = row.get_opt(i);
        if let Some(Ok(col_bytes)) = col_bytes {
            total_data_size += col_bytes.len();
        }
    }
    log_counter_with_labels(&MYSQL_QUERY_RESULT_BYTES, total_data_size as u64, labels);
}

register_convex_counter!(
    MYSQL_QUERY_UNFINISHED_PROGRESS_TOTAL,
    "Estimated number of query results not consumed before dropping the stream",
    &["name", "cluster_name"]
);

pub fn query_progress_counter(size_hint: usize, labels: Vec<StaticMetricLabel>) -> ProgressCounter {
    ProgressCounter::new(&MYSQL_QUERY_UNFINISHED_PROGRESS_TOTAL, size_hint, labels)
}

register_convex_counter!(
    MYSQL_EXECUTE_TOTAL,
    "Total number of MySQL executions",
    &["name", "cluster_name"]
);
pub fn log_execute(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&MYSQL_EXECUTE_TOTAL, 1, labels)
}

register_convex_counter!(
    MYSQL_TRANSACTION_TOTAL,
    "Total number of transactions",
    &["name", "cluster_name"]
);
pub fn log_transaction(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&MYSQL_TRANSACTION_TOTAL, 1, labels)
}

pub const LARGE_STATEMENT_THRESHOLD: usize = 1 << 14; // 16KB

register_convex_counter!(
    MYSQL_LARGE_STATEMENT_TOTAL,
    "Number of MySQL statements large enough to pin the connection",
    &["name", "cluster_name"]
);
pub fn log_large_statement(labels: Vec<StaticMetricLabel>) {
    log_counter_with_labels(&MYSQL_LARGE_STATEMENT_TOTAL, 1, labels)
}
