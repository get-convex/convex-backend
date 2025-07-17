use ::search::metrics::{
    SearchType,
    SEARCH_TYPE_LABEL,
};
use common::{
    identity::IDENTITY_LABEL,
    runtime::Runtime,
    types::Timestamp,
};
use errors::ErrorMetadata;
use keybroker::Identity;
use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_distribution_with_labels,
    register_convex_counter,
    register_convex_histogram,
    CancelableTimer,
    IntoLabel,
    StaticMetricLabel,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::{
    VMHistogram,
    VMHistogramVec,
};

use crate::{
    transaction::FinalTransaction,
    RetentionType,
    Transaction,
};

register_convex_histogram!(
    DOCUMENTS_SIZE_BYTES,
    "Total size of document store in bytes"
);
pub fn log_document_store_size(total_size: u64) {
    log_distribution(&DOCUMENTS_SIZE_BYTES, total_size as f64);
}

register_convex_histogram!(DOCUMENTS_KEYS_TOTAL, "Total number of document keys");
pub fn log_num_keys(num_keys: u64) {
    log_distribution(&DOCUMENTS_KEYS_TOTAL, num_keys as f64);
}

register_convex_histogram!(
    INDEXES_TO_BACKFILL_TOTAL,
    "Number of indexes needing backfill"
);
pub fn log_num_indexes_to_backfill(num_indexes: usize) {
    log_distribution(&INDEXES_TO_BACKFILL_TOTAL, num_indexes as f64);
}

register_convex_counter!(INDEXES_BACKFILLED_TOTAL, "Number of indexes backfilled");
pub fn log_index_backfilled() {
    log_counter(&INDEXES_BACKFILLED_TOTAL, 1);
}

register_convex_histogram!(
    DB_INDEX_BACKFILL_SECONDS,
    "Time for database indexes to backfill",
    &STATUS_LABEL
);
pub fn index_backfill_timer() -> CancelableTimer {
    CancelableTimer::new(&DB_INDEX_BACKFILL_SECONDS)
}

register_convex_histogram!(
    TABLET_DB_INDEX_BACKFILL_SECONDS,
    "Time for database indexes to backfill",
);
pub fn tablet_index_backfill_timer() -> Timer<VMHistogram> {
    Timer::new(&TABLET_DB_INDEX_BACKFILL_SECONDS)
}

register_convex_histogram!(
    DATABASE_WRITE_TX_READ_INTERVALS_TOTAL,
    "Number of read intervals in a write transaction"
);

register_convex_histogram!(
    DATABASE_WRITE_TX_WRITES_TOTAL,
    "Total size of writes in a write transaction"
);
register_convex_histogram!(
    DATABASE_WRITE_TX_NUM_WRITES_TOTAL,
    "Total number of writes in a write transaction"
);

register_convex_histogram!(
    DATABASE_USER_WRITE_TX_WRITES_TOTAL,
    "Size of writes to a user table in a transaction"
);

register_convex_histogram!(
    DATABASE_USER_WRITE_TX_NUM_WRITES_TOTAL,
    "Number of writes to a user table in a transaction"
);

register_convex_histogram!(
    DATABASE_SYSTEM_WRITE_TX_WRITES_TOTAL,
    "Size of writes to a system table in a transaction"
);

register_convex_histogram!(
    DATABASE_SYSTEM_WRITE_TX_NUM_WRITES_TOTAL,
    "Number of writes to a system table in a transaction"
);

pub fn log_write_tx(tx: &FinalTransaction) {
    log_distribution(
        &DATABASE_WRITE_TX_READ_INTERVALS_TOTAL,
        tx.reads.num_intervals() as f64,
    );

    let user_size = tx.writes.user_size();
    let system_size = tx.writes.system_size();

    // Combined
    log_distribution(
        &DATABASE_WRITE_TX_WRITES_TOTAL,
        (user_size.size + system_size.size) as f64,
    );
    log_distribution(
        &DATABASE_WRITE_TX_NUM_WRITES_TOTAL,
        (user_size.num_writes + system_size.num_writes) as f64,
    );

    // User tables
    log_distribution(&DATABASE_USER_WRITE_TX_WRITES_TOTAL, user_size.size as f64);
    log_distribution(
        &DATABASE_USER_WRITE_TX_NUM_WRITES_TOTAL,
        user_size.num_writes as f64,
    );

    // System tables
    log_distribution(
        &DATABASE_SYSTEM_WRITE_TX_WRITES_TOTAL,
        system_size.size as f64,
    );
    log_distribution(
        &DATABASE_SYSTEM_WRITE_TX_NUM_WRITES_TOTAL,
        user_size.num_writes as f64,
    );
}

register_convex_histogram!(
    DATABASE_READ_TX_READ_INTERVALS_TOTAL,
    "Number of read intervals in a read transaction"
);
pub fn log_read_tx<RT: Runtime>(tx: &Transaction<RT>) {
    log_distribution(
        &DATABASE_READ_TX_READ_INTERVALS_TOTAL,
        tx.reads.num_intervals() as f64,
    );
}

register_convex_histogram!(
    DATABASE_SUBSCRIPTION_SECONDS,
    "Duration of a database subscription"
);
pub fn subscription_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_SUBSCRIPTION_SECONDS)
}

register_convex_histogram!(
    DATABASE_REFRESH_TOKEN_SECONDS,
    "time taken to refresh a database token"
);
pub fn refresh_token_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_REFRESH_TOKEN_SECONDS)
}

register_convex_histogram!(
    DATABASE_BOOTSTRAP_SHAPES_SECONDS,
    "Time taken to bootstrap shapes"
);
pub fn bootstrap_table_summaries_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_BOOTSTRAP_SHAPES_SECONDS)
}

register_convex_histogram!(DATABASE_LOAD_SECONDS, "Time to load the database");
pub fn load_database_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_LOAD_SECONDS)
}

register_convex_histogram!(
    DB_SNAPSHOT_TABLE_AND_INDEX_METADATA_LOAD_SECONDS,
    "Time to load table and index metadata"
);
pub fn load_table_and_index_metadata_timer() -> Timer<VMHistogram> {
    Timer::new(&DB_SNAPSHOT_TABLE_AND_INDEX_METADATA_LOAD_SECONDS)
}

register_convex_histogram!(
    DB_SNAPSHOT_LOAD_INDEXES_INTO_MEMORY_SECONDS,
    "Time to load indexes into memory"
);
pub fn load_indexes_into_memory_timer() -> Timer<VMHistogram> {
    Timer::new(&DB_SNAPSHOT_LOAD_INDEXES_INTO_MEMORY_SECONDS)
}

register_convex_histogram!(
    DB_SNAPSHOT_BOOTSTRAP_TABLE_REGISTRY_SECONDS,
    "Time to bootstrap table registry"
);
pub fn bootstrap_table_registry_timer() -> Timer<VMHistogram> {
    Timer::new(&DB_SNAPSHOT_BOOTSTRAP_TABLE_REGISTRY_SECONDS)
}

register_convex_histogram!(
    DB_SNAPSHOT_VERIFY_INVARIANTS_SECONDS,
    "Time to verify invariants when loading a DatabaseSnapshot"
);
pub fn verify_invariants_timer() -> Timer<VMHistogram> {
    Timer::new(&DB_SNAPSHOT_VERIFY_INVARIANTS_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_CLIENT_SECONDS,
    "Time taken to submit a commit",
    &[IDENTITY_LABEL],
);
/// Includes time waiting in queue for the committer thread.
pub fn commit_client_timer(identity: &Identity) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&DATABASE_COMMIT_CLIENT_SECONDS);
    timer.add_label(identity.tag());
    timer
}

register_convex_histogram!(DATABASE_COMMIT_QUEUE_SECONDS, "Time a commit is queued");
pub fn commit_queue_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_COMMIT_QUEUE_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_SECONDS,
    "Time taken for a database commit",
    &STATUS_LABEL
);
pub fn commit_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_ID_REUSE_SECONDS,
    "Time to check if IDs have been reused",
    &STATUS_LABEL
);
pub fn commit_id_reuse_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_ID_REUSE_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_IS_STALE_SECONDS,
    "Time to check if a commit is stale",
    &STATUS_LABEL
);
pub fn commit_is_stale_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_IS_STALE_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_PREPARE_WRITES_SECONDS,
    "Time to prepare writes",
    &STATUS_LABEL
);
pub fn commit_prepare_writes_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_PREPARE_WRITES_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_PERSISTENCE_WRITE_SECONDS,
    "Time to commit a persistence write",
    &STATUS_LABEL
);
pub fn commit_persistence_write_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_PERSISTENCE_WRITE_SECONDS)
}

register_convex_histogram!(
    DATABASE_COMMIT_APPLY_SECONDS,
    "Time to apply a commit",
    &STATUS_LABEL
);
pub fn commit_apply_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_APPLY_SECONDS)
}

register_convex_histogram!(
    DATABASE_CONFLICT_CHECKER_APPEND_SECONDS,
    "Time to update pending writes when validating a commit"
);
pub fn pending_writes_append_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_CONFLICT_CHECKER_APPEND_SECONDS)
}

register_convex_histogram!(
    DATABASE_APPLY_DOCUMENT_STORE_APPEND_SECONDS,
    "Time to apply updates to the document store log"
);
pub fn write_log_append_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_APPLY_DOCUMENT_STORE_APPEND_SECONDS)
}

register_convex_histogram!(
    DATABASE_WRITE_LOG_COMMIT_BYTES,
    "Total size of all write log entries for a commit"
);
pub fn write_log_commit_bytes(bytes: usize) {
    log_distribution(&DATABASE_WRITE_LOG_COMMIT_BYTES, bytes as f64);
}

register_convex_counter!(DATABASE_COMMIT_ROWS, "Number of commits to database");
pub fn commit_rows(num_rows: u64) {
    log_counter(&DATABASE_COMMIT_ROWS, num_rows);
}

register_convex_histogram!(
    DATABASE_SUBSCRIPTIONS_UPDATE_SECONDS,
    "Time to advance the SubscriptionManager's log"
);
pub fn subscriptions_update_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_SUBSCRIPTIONS_UPDATE_SECONDS)
}

register_convex_counter!(DATABASE_COMMITTER_FULL_TOTAL, "Committer queue full count");

pub fn committer_full_error() -> ErrorMetadata {
    log_counter(&DATABASE_COMMITTER_FULL_TOTAL, 1);

    ErrorMetadata::overloaded(
        "CommitterFullError",
        "Too many concurrent commits, backoff and try again",
    )
}

register_convex_counter!(
    SUBSCRIPTIONS_WORKER_FULL_TOTAL,
    "Count of subscription worker full errors"
);
pub fn subscriptions_worker_full_error() -> ErrorMetadata {
    log_counter(&SUBSCRIPTIONS_WORKER_FULL_TOTAL, 1);
    ErrorMetadata::overloaded(
        "SubscriptionsWorkerFullError",
        "Too many concurrent subscription messages, backoff and try again",
    )
}

register_convex_counter!(
    SHUTDOWN_TOTAL,
    "Count of errors caused due to the database shutting down"
);
pub fn shutdown_error() -> anyhow::Error {
    log_counter(&SHUTDOWN_TOTAL, 1);
    anyhow::anyhow!("Database Shutting Down")
        .context(ErrorMetadata::operational_internal_server_error())
}

register_convex_histogram!(BUMP_REPEATABLE_TS_SECONDS, "Time to bump max_repeatable_ts");
pub fn bump_repeatable_ts_timer() -> Timer<VMHistogram> {
    Timer::new(&BUMP_REPEATABLE_TS_SECONDS)
}

register_convex_histogram!(NEXT_COMMIT_TS_SECONDS, "Time to bump max_repeatable_ts");
pub fn next_commit_ts_seconds() -> Timer<VMHistogram> {
    Timer::new(&NEXT_COMMIT_TS_SECONDS)
}

register_convex_histogram!(
    LATEST_MIN_SNAPSHOT_SECONDS,
    "Time to get latest min_snapshot_ts"
);
pub fn latest_min_snapshot_timer() -> Timer<VMHistogram> {
    Timer::new(&LATEST_MIN_SNAPSHOT_SECONDS)
}

register_convex_histogram!(
    LATEST_MIN_DOCUMENT_SNAPSHOT_SECONDS,
    "Time to get latest min_document_snapshot_ts"
);
pub fn latest_min_document_snapshot_timer() -> Timer<VMHistogram> {
    Timer::new(&LATEST_MIN_DOCUMENT_SNAPSHOT_SECONDS)
}

register_convex_histogram!(
    RETENTION_ADVANCE_TIMER_SECONDS,
    "Time to advance retention min snapshot"
);
pub fn retention_advance_timestamp_timer() -> Timer<VMHistogram> {
    Timer::new(&RETENTION_ADVANCE_TIMER_SECONDS)
}

register_convex_histogram!(
    RETENTION_DELETE_SECONDS,
    "Time for retention to complete deletions"
);
pub fn retention_delete_timer() -> Timer<VMHistogram> {
    Timer::new(&RETENTION_DELETE_SECONDS)
}

register_convex_histogram!(
    RETENTION_DELETE_DOCUMENTS_SECONDS,
    "Time for retention to complete deletions"
);
pub fn retention_delete_documents_timer() -> Timer<VMHistogram> {
    Timer::new(&RETENTION_DELETE_DOCUMENTS_SECONDS)
}

register_convex_histogram!(
    RETENTION_DELETE_CHUNK_SECONDS,
    "Time for retention to delete one chunk"
);
pub fn retention_delete_chunk_timer() -> Timer<VMHistogram> {
    Timer::new(&RETENTION_DELETE_CHUNK_SECONDS)
}

register_convex_histogram!(
    RETENTION_DELETE_DOCUMENT_CHUNK_SECONDS,
    "Time for document retention to delete one chunk"
);
pub fn retention_delete_document_chunk_timer() -> Timer<VMHistogram> {
    Timer::new(&RETENTION_DELETE_DOCUMENT_CHUNK_SECONDS)
}

register_convex_histogram!(RETENTION_CURSOR_AGE_SECONDS, "Age of the retention cursor");
pub fn log_retention_cursor_age(age_secs: f64) {
    log_distribution(&RETENTION_CURSOR_AGE_SECONDS, age_secs)
}

register_convex_histogram!(
    RETENTION_CURSOR_LAG_SECONDS,
    "Lag between the retention cursor and the min index snapshot"
);
pub fn log_retention_cursor_lag(age_secs: f64) {
    log_distribution(&RETENTION_CURSOR_LAG_SECONDS, age_secs)
}

register_convex_histogram!(
    DOCUMENT_RETENTION_CURSOR_AGE_SECONDS,
    "Age of the document retention cursor"
);
pub fn log_document_retention_cursor_age(age_secs: f64) {
    log_distribution(&DOCUMENT_RETENTION_CURSOR_AGE_SECONDS, age_secs)
}

register_convex_histogram!(
    DOCUMENT_RETENTION_CURSOR_LAG_SECONDS,
    "Lag between the retention cursor and the min document snapshot"
);
pub fn log_document_retention_cursor_lag(age_secs: f64) {
    log_distribution(&DOCUMENT_RETENTION_CURSOR_LAG_SECONDS, age_secs)
}

register_convex_counter!(
    RETENTION_MISSING_CURSOR_INFO,
    "Index retention has no cursor"
);
pub fn log_retention_no_cursor() {
    log_counter(&RETENTION_MISSING_CURSOR_INFO, 1)
}

register_convex_counter!(
    DOCUMENT_RETENTION_MISSING_CURSOR_INFO,
    "Document retention has no cursor"
);
pub fn log_document_retention_no_cursor() {
    log_counter(&DOCUMENT_RETENTION_MISSING_CURSOR_INFO, 1)
}

register_convex_counter!(
    RETENTION_SCANNED_DOCUMENT_TOTAL,
    "Count of documents scanned by retention",
    &["tombstone", "prev_rev"]
);
pub fn log_retention_scanned_document(is_tombstone: bool, has_prev_rev: bool) {
    log_counter_with_labels(
        &RETENTION_SCANNED_DOCUMENT_TOTAL,
        1,
        vec![
            StaticMetricLabel::new(
                "tombstone",
                if is_tombstone {
                    "is_tombstone"
                } else {
                    "is_document"
                },
            ),
            StaticMetricLabel::new(
                "prev_rev",
                if has_prev_rev {
                    "has_prev_rev"
                } else {
                    "no_prev_rev"
                },
            ),
        ],
    )
}

register_convex_counter!(
    DOCUMENT_RETENTION_SCANNED_DOCUMENT_TOTAL,
    "Count of documents scanned by retention",
    &["tombstone", "prev_rev"]
);
pub fn log_document_retention_scanned_document(is_tombstone: bool, has_prev_rev: bool) {
    log_counter_with_labels(
        &DOCUMENT_RETENTION_SCANNED_DOCUMENT_TOTAL,
        1,
        vec![
            StaticMetricLabel::new(
                "tombstone",
                if is_tombstone {
                    "is_tombstone"
                } else {
                    "is_document"
                },
            ),
            StaticMetricLabel::new(
                "prev_rev",
                if has_prev_rev {
                    "has_prev_rev"
                } else {
                    "no_prev_rev"
                },
            ),
        ],
    )
}

register_convex_counter!(
    RETENTION_EXPIRED_INDEX_ENTRY_TOTAL,
    "Number of index entries expired by retention",
    &["reason"]
);
pub fn log_retention_expired_index_entry(is_tombstone: bool, is_key_change_tombstone: bool) {
    log_counter_with_labels(
        &RETENTION_EXPIRED_INDEX_ENTRY_TOTAL,
        1,
        vec![StaticMetricLabel::new(
            "reason",
            if is_tombstone {
                if is_key_change_tombstone {
                    "key_change_tombstone"
                } else {
                    "tombstone"
                }
            } else {
                "overwritten"
            },
        )],
    )
}
register_convex_counter!(
    RETENTION_INDEX_ENTRIES_DELETED_TOTAL,
    "The total number of index entries persistence returns as having been actually deleted by \
     retention."
);
pub fn log_retention_index_entries_deleted(deleted_rows: usize) {
    log_counter(&RETENTION_INDEX_ENTRIES_DELETED_TOTAL, deleted_rows as u64)
}

register_convex_counter!(
    RETENTION_DOCUMENTS_DELETED_TOTAL,
    "The total number of documents persistence returns as having been actually deleted by \
     retention."
);
pub fn log_retention_documents_deleted(deleted_rows: usize) {
    log_counter(&RETENTION_DOCUMENTS_DELETED_TOTAL, deleted_rows as u64)
}

register_convex_counter!(
    RETENTION_TS_ADVANCED_TOTAL,
    "Number of times that min_snapshot timestamp was advanced",
    &["type"]
);
pub fn log_retention_ts_advanced(ty: RetentionType) {
    log_counter_with_labels(
        &RETENTION_TS_ADVANCED_TOTAL,
        1,
        vec![StaticMetricLabel::new(
            "type",
            match ty {
                RetentionType::Document => "document",
                RetentionType::Index => "index",
            },
        )],
    );
}

register_convex_counter!(
    OUTSIDE_RETENTION_TOTAL,
    "Number of snapshots out of retention min_snapshot_ts",
    &["optimistic", "leader"]
);
register_convex_histogram!(
    SNAPSHOT_AGE_SECONDS,
    "Age of snapshot during verification",
    &["optimistic", "leader"]
);
register_convex_histogram!(
    SNAPSHOT_BUFFER_VS_MIN_SNAPSHOT_SECONDS,
    "Time elapsed from snapshot and min_snapshot_ts",
    &["optimistic", "leader"]
);
pub fn log_snapshot_verification_age<RT: Runtime>(
    rt: &RT,
    snapshot: Timestamp,
    min_snapshot_ts: Timestamp,
    optimistic: bool,
    leader: bool,
) {
    let labels = vec![
        StaticMetricLabel::new("optimistic", optimistic.as_label()),
        StaticMetricLabel::new("leader", leader.as_label()),
    ];
    if snapshot < min_snapshot_ts {
        log_counter_with_labels(&OUTSIDE_RETENTION_TOTAL, 1, labels.clone());
    }
    if let Ok(current_timestamp) = rt.generate_timestamp() {
        log_distribution_with_labels(
            &SNAPSHOT_AGE_SECONDS,
            current_timestamp.secs_since_f64(snapshot),
            labels.clone(),
        );
    }
    log_distribution_with_labels(
        &SNAPSHOT_BUFFER_VS_MIN_SNAPSHOT_SECONDS,
        snapshot.secs_since_f64(min_snapshot_ts),
        labels,
    );
}

register_convex_histogram!(
    UDF_QUERY_USED_RESULTS_TOTAL,
    "Number of results used in a UDF index query"
);
register_convex_histogram!(
    UDF_QUERY_UNUSED_RESULTS_TOTAL,
    "Number of results unused in a UDF index query"
);
register_convex_histogram!(
    UDF_QUERY_PAGES_FETCHED_TOTAL,
    "Number of pages fetched in a UDF index query"
);
pub fn log_index_range(returned_results: usize, unused_results: usize, pages_fetched: usize) {
    log_distribution(&UDF_QUERY_USED_RESULTS_TOTAL, returned_results as f64);
    log_distribution(&UDF_QUERY_UNUSED_RESULTS_TOTAL, unused_results as f64);
    log_distribution(&UDF_QUERY_PAGES_FETCHED_TOTAL, pages_fetched as f64);
}

register_convex_counter!(
    DATABASE_READS_REFRESH_MISS_TOTAL,
    "Number of times refreshing reads fails because the write log is stale"
);
pub fn log_reads_refresh_miss() {
    log_counter(&DATABASE_READS_REFRESH_MISS_TOTAL, 1);
}

register_convex_histogram!(
    DATABASE_READS_REFRESH_AGE_SECONDS,
    "How old a given read set is compared to the timestamp of a request that wants to use it",
);
pub fn log_read_set_age(seconds: f64) {
    log_distribution(&DATABASE_READS_REFRESH_AGE_SECONDS, seconds);
}

register_convex_counter!(
    VIRTUAL_TABLE_GET_REQUESTS_TOTAL,
    "Number of times virtual table get path is called"
);
pub fn log_virtual_table_get() {
    log_counter(&VIRTUAL_TABLE_GET_REQUESTS_TOTAL, 1);
}

register_convex_counter!(
    VIRTUAL_TABLE_QUERY_REQUESTS_TOTAL,
    "Number of times virtual table query path is called"
);
pub fn log_virtual_table_query() {
    log_counter(&VIRTUAL_TABLE_QUERY_REQUESTS_TOTAL, 1);
}

register_convex_histogram!(
    SEARCH_AND_VECTOR_BOOTSTRAP_SECONDS,
    "Time taken to bootstrap text and vector indexes",
    &STATUS_LABEL
);
pub fn bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_AND_VECTOR_BOOTSTRAP_SECONDS)
}

register_convex_histogram!(
    SEARCH_AND_VECTOR_BOOTSTRAP_COMMITTER_UPDATE_SECONDS,
    "Time to update text and vector index bootstrap in the committer"
);
pub fn bootstrap_update_timer() -> Timer<VMHistogram> {
    Timer::new(&SEARCH_AND_VECTOR_BOOTSTRAP_COMMITTER_UPDATE_SECONDS)
}
register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_COMMITTER_UPDATE_REVISIONS_TOTAL,
    "Number of revisions loaded during text and vector bootstrap updates in the committer"
);
register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_COMMITTER_UPDATE_REVISIONS_BYTES,
    "Total size of revisions loaded during text and vector bootstrap updates in the committer"
);

pub fn finish_bootstrap_update(num_revisions: usize, bytes: usize) {
    log_counter(
        &SEARCH_AND_VECTOR_BOOTSTRAP_COMMITTER_UPDATE_REVISIONS_TOTAL,
        num_revisions as u64,
    );
    log_counter(
        &SEARCH_AND_VECTOR_BOOTSTRAP_COMMITTER_UPDATE_REVISIONS_BYTES,
        bytes as u64,
    );
}

register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_TOTAL,
    "Number of revisions loaded during text and vector bootstrap"
);
register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_BYTES,
    "Total size of revisions loaded during text and vector bootstrap"
);
pub fn finish_bootstrap(num_revisions: usize, bytes: usize, timer: StatusTimer) {
    log_counter(
        &SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_TOTAL,
        num_revisions as u64,
    );
    log_counter(&SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_BYTES, bytes as u64);
    timer.finish();
}

pub enum SearchWriterLockWaiter {
    Compactor,
    Flusher,
}

const SEARCH_WRITER_WAITER_LABEL: &str = "waiter";

impl SearchWriterLockWaiter {
    fn tag(&self) -> StaticMetricLabel {
        let label = match self {
            SearchWriterLockWaiter::Compactor => "compactor",
            SearchWriterLockWaiter::Flusher => "flusher",
        };
        StaticMetricLabel::new(SEARCH_WRITER_WAITER_LABEL, label)
    }
}

register_convex_histogram!(
    SEARCH_WRITER_LOCK_WAIT_SECONDS,
    "The amount of time spent waiting for the writer lock to commit a vector/text index metadata \
     change",
    &[SEARCH_TYPE_LABEL, SEARCH_WRITER_WAITER_LABEL]
);
pub fn search_writer_lock_wait_timer(
    waiter: SearchWriterLockWaiter,
    search_type: SearchType,
) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&SEARCH_WRITER_LOCK_WAIT_SECONDS);
    timer.add_label(search_type.tag());
    timer.add_label(waiter.tag());
    timer
}

const MERGE_LABEL: &str = "merge_required";

pub enum SearchIndexMergeType {
    Unknown,
    Required,
    NotRequired,
}

impl SearchIndexMergeType {
    fn metric_label(&self) -> StaticMetricLabel {
        let label = match self {
            SearchIndexMergeType::Unknown => "unknown",
            SearchIndexMergeType::Required => "required",
            SearchIndexMergeType::NotRequired => "not_required",
        };
        StaticMetricLabel::new(MERGE_LABEL, label)
    }
}

register_convex_histogram!(
    SEARCH_COMPACTION_MERGE_COMMIT_SECONDS,
    "Time to merge deletes and commit after compaction",
    &[STATUS_LABEL[0], SEARCH_TYPE_LABEL, MERGE_LABEL],
);
pub fn search_compaction_merge_commit_timer(search_type: SearchType) -> StatusTimer {
    let mut timer = StatusTimer::new(&SEARCH_COMPACTION_MERGE_COMMIT_SECONDS);
    timer.add_label(search_type.tag());
    timer.add_label(SearchIndexMergeType::Unknown.metric_label());
    timer
}

register_convex_histogram!(
    SEARCH_FLUSH_MERGE_COMMIT_SECONDS,
    "Time to merge deletes and commit after flushing",
    &[STATUS_LABEL[0], SEARCH_TYPE_LABEL, MERGE_LABEL],
);
pub fn search_flush_merge_commit_timer(search_type: SearchType) -> StatusTimer {
    let mut timer = StatusTimer::new(&SEARCH_FLUSH_MERGE_COMMIT_SECONDS);
    timer.add_label(search_type.tag());
    timer.add_label(SearchIndexMergeType::Unknown.metric_label());
    timer
}

pub fn finish_search_index_merge_timer(mut timer: StatusTimer, merge_type: SearchIndexMergeType) {
    timer.replace_label(
        SearchIndexMergeType::Unknown.metric_label(),
        merge_type.metric_label(),
    );
    timer.finish();
}

register_convex_histogram!(
    DOCUMENTS_PER_NEW_SEARCH_SEGMENT_TOTAL,
    "Total number of documents in a newly built search index segment.",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_documents_per_new_search_segment(count: u64, search_type: SearchType) {
    log_distribution_with_labels(
        &DOCUMENTS_PER_NEW_SEARCH_SEGMENT_TOTAL,
        count as f64,
        vec![search_type.tag()],
    );
}

register_convex_histogram!(
    DOCUMENTS_PER_SEARCH_SEGMENT_TOTAL,
    "Total number of documents in a specific search segment, including documents that were added \
     to the segment but are deleted and excluded from any search results",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_documents_per_search_segment(count: u64, search_type: SearchType) {
    log_distribution_with_labels(
        &DOCUMENTS_PER_SEARCH_SEGMENT_TOTAL,
        count as f64,
        vec![search_type.tag()],
    );
}

register_convex_histogram!(
    NON_DELETED_DOCUMENTS_PER_SEARCH_SEGMENT_TOTAL,
    "Total number of non-deleted documents in a specific search segment, excluding documents that \
     were added to the segment but are deleted and excluded from any search results",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_non_deleted_documents_per_search_segment(count: u64, search_type: SearchType) {
    log_distribution_with_labels(
        &NON_DELETED_DOCUMENTS_PER_SEARCH_SEGMENT_TOTAL,
        count as f64,
        vec![search_type.tag()],
    );
}

register_convex_histogram!(
    DOCUMENTS_PER_SEARCH_INDEX_TOTAL,
    "Total number of documents across all segments in a search index, including documents that \
     were added to the index but are deleted and excluded from any search results",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_documents_per_search_index(count: u64, search_type: SearchType) {
    log_distribution_with_labels(
        &DOCUMENTS_PER_SEARCH_INDEX_TOTAL,
        count as f64,
        vec![search_type.tag()],
    );
}
register_convex_histogram!(
    NON_DELETED_DOCUMENTS_PER_SEARCH_INDEX_TOTAL,
    "Total number of non-deleted documents across all segments in a search index segment, \
     excluding documents that were added to the index but are deleted and excluded from any \
     search results",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_non_deleted_documents_per_search_index(count: u64, search_type: SearchType) {
    log_distribution_with_labels(
        &NON_DELETED_DOCUMENTS_PER_SEARCH_INDEX_TOTAL,
        count as f64,
        vec![search_type.tag()],
    );
}

const COMPACTION_REASON_LABEL: &str = "compaction_reason";

#[derive(Debug)]
pub enum CompactionReason {
    SmallSegments,
    LargeSegments,
    Deletes,
}

impl CompactionReason {
    fn metric_label(&self) -> StaticMetricLabel {
        let label = match self {
            CompactionReason::SmallSegments => "small",
            CompactionReason::LargeSegments => "large",
            CompactionReason::Deletes => "deletes",
        };
        StaticMetricLabel::new(COMPACTION_REASON_LABEL, label)
    }
}

register_convex_histogram!(
    COMPACTION_BUILD_ONE_SECONDS,
    "Time to run a single vector/text index compaction",
    &[STATUS_LABEL[0], COMPACTION_REASON_LABEL, SEARCH_TYPE_LABEL],
);
pub fn compaction_build_one_timer(
    search_type: SearchType,
    reason: CompactionReason,
) -> StatusTimer {
    let mut timer = StatusTimer::new(&COMPACTION_BUILD_ONE_SECONDS);
    timer.add_label(search_type.tag());
    timer.add_label(reason.metric_label());
    timer
}

register_convex_histogram!(
    COMPACTION_COMPACTED_SEGMENTS_TOTAL,
    "Total number of compacted segments",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_compaction_total_segments(total_segments: usize, search_type: SearchType) {
    log_distribution_with_labels(
        &COMPACTION_COMPACTED_SEGMENTS_TOTAL,
        total_segments as f64,
        vec![search_type.tag()],
    );
}

register_convex_histogram!(
    COMPACTION_COMPACTED_SEGMENT_NUM_DOCUMENTS_TOTAL,
    "The number of documents in the newly generated compacted segment",
    &[SEARCH_TYPE_LABEL],
);
pub fn log_compaction_compacted_segment_num_documents_total(
    total_vectors: u64,
    search_type: SearchType,
) {
    log_distribution_with_labels(
        &COMPACTION_COMPACTED_SEGMENT_NUM_DOCUMENTS_TOTAL,
        total_vectors as f64,
        vec![search_type.tag()],
    );
}

register_convex_histogram!(
    DATABASE_SEARCH_INDEX_BUILD_ONE_SECONDS,
    "Time to build one (multisegment) search index",
    &[STATUS_LABEL[0], SEARCH_TYPE_LABEL],
);
pub fn build_one_search_index_timer(search_type: SearchType) -> StatusTimer {
    let mut timer = StatusTimer::new(&DATABASE_SEARCH_INDEX_BUILD_ONE_SECONDS);
    timer.add_label(search_type.tag());
    timer
}

register_convex_histogram!(
    DATABASE_VECTOR_AND_SEARCH_BOOTSTRAP_SECONDS,
    "Time to bootstrap vector and text indexes",
    &STATUS_LABEL
);
pub fn search_and_vector_bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_VECTOR_AND_SEARCH_BOOTSTRAP_SECONDS)
}

register_convex_histogram!(
    DATABASE_TABLE_SUMMARY_FINISH_BOOTSTRAP_SECONDS,
    "Time to finish table summary bootstrap",
    &STATUS_LABEL
);
pub fn table_summary_finish_bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_TABLE_SUMMARY_FINISH_BOOTSTRAP_SECONDS)
}

register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_DOCUMENTS_SKIPPED_TOTAL,
    "Number of documents skipped during vector and text index bootstrap",
);
pub fn log_document_skipped() {
    log_counter(&SEARCH_AND_VECTOR_BOOTSTRAP_DOCUMENTS_SKIPPED_TOTAL, 1);
}

pub mod search {

    use metrics::{
        register_convex_histogram,
        StatusTimer,
        STATUS_LABEL,
    };

    register_convex_histogram!(
        DATABASE_SEARCH_ITERATOR_NEXT_SECONDS,
        "Time to fetch the next document in a search query iterator",
        &STATUS_LABEL
    );
    pub fn iterator_next_timer() -> StatusTimer {
        StatusTimer::new(&DATABASE_SEARCH_ITERATOR_NEXT_SECONDS)
    }
}

pub mod vector {
    use metrics::{
        register_convex_histogram,
        CancelableTimer,
        StatusTimer,
        STATUS_LABEL,
    };

    register_convex_histogram!(
        DATABASE_VECTOR_SEARCH_QUERY_SECONDS,
        "Time to run a single vector search, not including retries due to bootstrapping",
        &STATUS_LABEL
    );
    pub fn vector_search_timer() -> StatusTimer {
        StatusTimer::new(&DATABASE_VECTOR_SEARCH_QUERY_SECONDS)
    }

    register_convex_histogram!(
        DATABASE_VECTOR_SEARCH_WITH_RETRIES_QUERY_SECONDS,
        "Time to run a vector search, including retries",
        &STATUS_LABEL
    );
    pub fn vector_search_with_retries_timer() -> CancelableTimer {
        CancelableTimer::new(&DATABASE_VECTOR_SEARCH_WITH_RETRIES_QUERY_SECONDS)
    }
}

register_convex_counter!(
    DATABASE_NONEMPTY_COMPONENT_EXPORTS_TOTAL,
    "Nonempty component definition loaded from database"
);
pub fn log_nonempty_component_exports() {
    log_counter(&DATABASE_NONEMPTY_COMPONENT_EXPORTS_TOTAL, 1);
}
register_convex_histogram!(
    DOCUMENT_DELTAS_READ_DOCUMENTS,
    "Total number of rows read in a document_deltas call",
);
pub fn log_document_deltas_read_documents(num: usize) {
    log_distribution(&DOCUMENT_DELTAS_READ_DOCUMENTS, num as f64);
}
register_convex_histogram!(
    DOCUMENT_DELTAS_RETURNED_DOCUMENTS,
    "Total number of documents returned by a document_deltas call",
);
pub fn log_document_deltas_returned_documents(num: usize) {
    log_distribution(&DOCUMENT_DELTAS_RETURNED_DOCUMENTS, num as f64);
}
register_convex_histogram!(
    LIST_SNAPSHOT_PAGE_DOCUMENTS,
    "Total number of documents in a returned SnapshotPage",
);
pub fn log_list_snapshot_page_documents(num_docs: usize) {
    log_distribution(&LIST_SNAPSHOT_PAGE_DOCUMENTS, num_docs as f64);
}

register_convex_histogram!(
    SUBSCRIPTION_INVALIDATION_UPDATES,
    "Number of subscriptions invalidated when advancing the log",
);
pub fn log_subscriptions_invalidated(num: usize) {
    log_distribution(&SUBSCRIPTION_INVALIDATION_UPDATES, num as f64);
}

register_convex_histogram!(
    SUBSCRIPTION_LOG_ITERATE_SECONDS,
    "Time to iterate over the write log when advancing subscriptions",
);
pub fn subscriptions_log_iterate_timer() -> Timer<VMHistogram> {
    Timer::new(&SUBSCRIPTION_LOG_ITERATE_SECONDS)
}

register_convex_histogram!(
    SUBSCRIPTION_PROCESS_WRITE_LOG_ENTRY_SECONDS,
    "Time to process one write log entry when advancing subscriptions",
);
pub fn subscription_process_write_log_entry_timer() -> Timer<VMHistogram> {
    Timer::new(&SUBSCRIPTION_PROCESS_WRITE_LOG_ENTRY_SECONDS)
}

register_convex_histogram!(
    SUBSCRIPTION_LOG_INVALIDATE_SECONDS,
    "Time to invalidate segsstiptions when edvancing rh_ log",
);
pub fn subscriptions_invalidate_timer() -> Timer<VMHistogram> {
    Timer::new(&SUBSCRIPTION_LOG_INVALIDATE_SECONDS)
}

register_convex_histogram!(
    SUBSCRIPTION_LOG_ENFORCE_RETENTION_SECONDS,
    "Time to enforce retention policy when advancing subscriptions",
);
pub fn subscriptions_log_enforce_retention_timer() -> Timer<VMHistogram> {
    Timer::new(&SUBSCRIPTION_LOG_ENFORCE_RETENTION_SECONDS)
}

register_convex_histogram!(
    SUBSCRIPTION_LOG_PROCESSED_COMMITS,
    "Total number of commits in the write log processed during one advance_log",
);
pub fn log_subscriptions_log_processed_commits(log_len: usize) {
    log_distribution(&SUBSCRIPTION_LOG_PROCESSED_COMMITS, log_len as f64);
}

register_convex_histogram!(
    SUBSCRIPTION_LOG_PROCESSED_WRITES,
    "Total number of writes in the write log processed during one advance_log",
);
pub fn log_subscriptions_log_processed_writes(num_writes: usize) {
    log_distribution(&SUBSCRIPTION_LOG_PROCESSED_WRITES, num_writes as f64);
}
