use common::{
    runtime::Runtime,
    types::Timestamp,
};
use errors::ErrorMetadata;
use metrics::{
    log_counter,
    log_counter_with_tags,
    log_distribution,
    log_distribution_with_tags,
    log_gauge,
    log_gauge_with_tags,
    metric_tag_const,
    metric_tag_const_value,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::VMHistogram;

use crate::{
    transaction::FinalTransaction,
    Transaction,
};

register_convex_gauge!(
    DOCUMENTS_SIZE_BYTES,
    "Total size of document store in bytes"
);
pub fn log_document_store_size(total_size: usize) {
    log_gauge(&DOCUMENTS_SIZE_BYTES, total_size as f64);
}

register_convex_gauge!(DOCUMENTS_KEYS_TOTAL, "Total number of document keys");
pub fn log_num_keys(num_keys: usize) {
    log_gauge(&DOCUMENTS_KEYS_TOTAL, num_keys as f64);
}

register_convex_gauge!(
    INDEXES_TO_BACKFILL_TOTAL,
    "Number of indexes needing backfill"
);
pub fn log_num_indexes_to_backfill(num_indexes: usize) {
    log_gauge(&INDEXES_TO_BACKFILL_TOTAL, num_indexes as f64);
}

register_convex_counter!(INDEXES_BACKFILLED_TOTAL, "Number of indexes backfilled");
pub fn log_index_backfilled() {
    log_counter(&INDEXES_BACKFILLED_TOTAL, 1);
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
    DB_SNAPSHOT_VIRTUAL_TABLE_METADATA_LOAD_SECONDS,
    "Time to load virtual table metadata"
);
pub fn load_virtual_table_metadata_timer() -> Timer<VMHistogram> {
    Timer::new(&DB_SNAPSHOT_VIRTUAL_TABLE_METADATA_LOAD_SECONDS)
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
    "Time taken to submit a commit"
);
/// Includes time waiting in queue for the committer thread.
pub fn commit_client_timer() -> Timer<VMHistogram> {
    Timer::new(&DATABASE_COMMIT_CLIENT_SECONDS)
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
    DATABASE_COMMIT_VALIDATE_INDEX_WRITE_SECONDS,
    "Time to validate an index write",
    &STATUS_LABEL
);
pub fn commit_validate_index_write_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_COMMIT_VALIDATE_INDEX_WRITE_SECONDS)
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
        "Too many concurrent commits, backoff and try again",
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
    RETENTION_DELETE_CHUNK_SECONDS,
    "Time for retention to delete one chunk"
);
pub fn retention_delete_chunk_timer() -> Timer<VMHistogram> {
    Timer::new(&RETENTION_DELETE_CHUNK_SECONDS)
}

register_convex_gauge!(RETENTION_CURSOR_AGE_SECONDS, "Age of the retention cursor");
pub fn log_retention_cursor_age(age_secs: f64) {
    log_gauge(&RETENTION_CURSOR_AGE_SECONDS, age_secs)
}

register_convex_counter!(
    RETENTION_SCANNED_DOCUMENT_TOTAL,
    "Count of documents scanned by retention",
    &["tombstone", "prev_rev"]
);
pub fn log_retention_scanned_document(is_tombstone: bool, has_prev_rev: bool) {
    log_counter_with_tags(
        &RETENTION_SCANNED_DOCUMENT_TOTAL,
        1,
        vec![
            metric_tag_const_value(
                "tombstone",
                if is_tombstone {
                    "is_tombstone"
                } else {
                    "is_document"
                },
            ),
            metric_tag_const_value(
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
    log_counter_with_tags(
        &RETENTION_EXPIRED_INDEX_ENTRY_TOTAL,
        1,
        vec![metric_tag_const_value(
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
    let tags = vec![
        if optimistic {
            metric_tag_const("optimistic:true")
        } else {
            metric_tag_const("optimistic:false")
        },
        if leader {
            metric_tag_const("leader:true")
        } else {
            metric_tag_const("leader:false")
        },
    ];
    if snapshot < min_snapshot_ts {
        log_counter_with_tags(&OUTSIDE_RETENTION_TOTAL, 1, tags.clone());
    }
    if let Ok(current_timestamp) = rt.generate_timestamp() {
        log_distribution_with_tags(
            &SNAPSHOT_AGE_SECONDS,
            current_timestamp.secs_since_f64(snapshot),
            tags.clone(),
        );
    }
    log_distribution_with_tags(
        &SNAPSHOT_BUFFER_VS_MIN_SNAPSHOT_SECONDS,
        snapshot.secs_since_f64(min_snapshot_ts),
        tags,
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

pub struct DatabaseWorkerStatus {
    name: &'static str,
}

impl Drop for DatabaseWorkerStatus {
    fn drop(&mut self) {
        log_worker_status(false, self.name);
    }
}

register_convex_gauge!(
    DATABASE_WORKER_IN_PROGRESS_TOTAL,
    "1 if a worker is working, 0 otherwise",
    &["worker"],
);
pub fn log_worker_starting(name: &'static str) -> DatabaseWorkerStatus {
    log_worker_status(true, name);
    DatabaseWorkerStatus { name }
}

fn log_worker_status(is_working: bool, name: &'static str) {
    log_gauge_with_tags(
        &DATABASE_WORKER_IN_PROGRESS_TOTAL,
        if is_working { 1f64 } else { 0f64 },
        vec![metric_tag_const_value("worker", name)],
    )
}

register_convex_histogram!(
    SEARCH_AND_VECTOR_BOOTSTRAP_SECONDS,
    "Time taken to bootstrap search and vector indexes",
    &STATUS_LABEL
);
pub fn bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_AND_VECTOR_BOOTSTRAP_SECONDS)
}

register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_TOTAL,
    "Number of revisions loaded during vector bootstrap"
);
register_convex_counter!(
    SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_BYTES,
    "Total size of revisions loaded during vector bootstrap"
);
pub fn finish_bootstrap(num_revisions: usize, bytes: usize, timer: StatusTimer) {
    log_counter(
        &SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_TOTAL,
        num_revisions as u64,
    );
    log_counter(&SEARCH_AND_VECTOR_BOOTSTRAP_REVISIONS_BYTES, bytes as u64);
    timer.finish();
}

pub mod search {

    use metrics::{
        log_counter,
        log_distribution,
        register_convex_counter,
        register_convex_histogram,
        StatusTimer,
        STATUS_LABEL,
    };
    use search::{
        TantivyDocument,
        TantivySearchIndexSchema,
    };

    register_convex_histogram!(
        DATABASE_SEARCH_BUILD_ONE_SECONDS,
        "Time to build one search index",
        &STATUS_LABEL
    );
    pub fn build_one_timer() -> StatusTimer {
        StatusTimer::new(&DATABASE_SEARCH_BUILD_ONE_SECONDS)
    }

    register_convex_histogram!(
        DATABASE_SEARCH_DOCUMENT_INDEXED_SEARCH_BYTES,
        "Size of search fields in search index"
    );
    register_convex_histogram!(
        DATABASE_SEARCH_DOCUMENT_INDEXED_FILTER_BYTES,
        "Size of filter fields in search index"
    );
    pub fn log_document_indexed(schema: &TantivySearchIndexSchema, document: &TantivyDocument) {
        let lengths = schema.document_lengths(document);
        log_distribution(
            &DATABASE_SEARCH_DOCUMENT_INDEXED_SEARCH_BYTES,
            lengths.search_field as f64,
        );
        for (_, filter_len) in lengths.filter_fields {
            log_distribution(
                &DATABASE_SEARCH_DOCUMENT_INDEXED_FILTER_BYTES,
                filter_len as f64,
            );
        }
    }

    register_convex_histogram!(
        DATABASE_SEARCH_DOCUMENTS_PER_INDEX_TOTAL,
        "Number of documents per search index",
    );
    pub fn log_documents_per_index(count: usize) {
        log_distribution(&DATABASE_SEARCH_DOCUMENTS_PER_INDEX_TOTAL, count as f64);
    }

    register_convex_histogram!(
        DATABASE_SEARCH_ITERATOR_NEXT_SECONDS,
        "Time to fetch the next document in a search query iterator",
        &STATUS_LABEL
    );
    pub fn iterator_next_timer() -> StatusTimer {
        StatusTimer::new(&DATABASE_SEARCH_ITERATOR_NEXT_SECONDS)
    }

    register_convex_histogram!(
        SEARCH_BOOTSTRAP_SECONDS,
        "Time taken to bootstrap search indexes",
        &STATUS_LABEL
    );
    pub fn bootstrap_timer() -> StatusTimer {
        StatusTimer::new(&SEARCH_BOOTSTRAP_SECONDS)
    }

    register_convex_counter!(
        SEARCH_BOOTSTRAP_REVISIONS_TOTAL,
        "Number of revisions loaded during search bootstrap"
    );
    register_convex_counter!(
        SEARCH_BOOTSTRAP_REVISIONS_BYTES,
        "Total size of revisions loaded during search bootstrap"
    );
    pub fn finish_bootstrap(num_revisions: usize, bytes: usize, timer: StatusTimer) {
        log_counter(&SEARCH_BOOTSTRAP_REVISIONS_TOTAL, num_revisions as u64);
        log_counter(&SEARCH_BOOTSTRAP_REVISIONS_BYTES, bytes as u64);
        timer.finish();
    }
}
register_convex_histogram!(
    DATABASE_VECTOR_AND_SEARCH_BOOTSTRAP_SECONDS,
    "Time to bootstrap vector and search indexes",
    &STATUS_LABEL
);
pub fn search_and_vector_bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&DATABASE_VECTOR_AND_SEARCH_BOOTSTRAP_SECONDS)
}

pub mod vector {
    use metrics::{
        log_distribution,
        metric_tag,
        register_convex_histogram,
        CancelableTimer,
        MetricTag,
        StatusTimer,
        Timer,
        STATUS_LABEL,
    };
    use prometheus::VMHistogramVec;

    register_convex_histogram!(
        DATABASE_VECTOR_BUILD_ONE_SECONDS,
        "Time to build one vector index",
        &STATUS_LABEL,
    );
    pub fn build_one_timer() -> StatusTimer {
        StatusTimer::new(&DATABASE_VECTOR_BUILD_ONE_SECONDS)
    }

    register_convex_histogram!(
        DATABASE_VECTOR_DOCUMENTS_PER_INDEX_TOTAL,
        "Number of documents per vector index",
    );
    pub fn log_documents_per_index(count: u64) {
        log_distribution(&DATABASE_VECTOR_DOCUMENTS_PER_INDEX_TOTAL, count as f64);
    }

    register_convex_histogram!(
        DATABASE_VECTOR_DOCUMENTS_PER_SEGMENT_TOTAL,
        "Number of documents per vector index segment",
    );
    pub fn log_documents_per_segment(count: u64) {
        log_distribution(&DATABASE_VECTOR_DOCUMENTS_PER_SEGMENT_TOTAL, count as f64);
    }

    register_convex_histogram!(
        DATABASE_VECTOR_DOCUMENTS_PER_NEW_SEGMENT_TOTAL,
        "Number of documents in a newly built vector index segment",
    );
    pub fn log_documents_per_new_segment(count: u32) {
        log_distribution(
            &DATABASE_VECTOR_DOCUMENTS_PER_NEW_SEGMENT_TOTAL,
            count as f64,
        );
    }

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

    const COMPACTION_REASON_LABEL: &str = "compaction_reason";

    pub enum CompactionReason {
        Unknown,
        SmallSegments,
        LargeSegments,
        Deletes,
    }

    impl CompactionReason {
        fn metric_tag(&self) -> MetricTag {
            let label = match self {
                CompactionReason::Unknown => "unknown",
                CompactionReason::SmallSegments => "small",
                CompactionReason::LargeSegments => "large",
                CompactionReason::Deletes => "deletes",
            };
            metric_tag(format!("{}:{}", COMPACTION_REASON_LABEL, label))
        }
    }

    register_convex_histogram!(
        VECTOR_COMPACTION_BUILD_ONE_SECONDS,
        "Time to run a single vector compaction",
        &[STATUS_LABEL[0], COMPACTION_REASON_LABEL],
    );
    pub fn vector_compaction_build_one_timer() -> StatusTimer {
        let mut timer = StatusTimer::new(&VECTOR_COMPACTION_BUILD_ONE_SECONDS);
        timer.add_tag(CompactionReason::Unknown.metric_tag());
        timer
    }

    pub fn finish_compaction_timer(mut timer: StatusTimer, reason: CompactionReason) {
        timer.replace_tag(CompactionReason::Unknown.metric_tag(), reason.metric_tag());
        timer.finish();
    }

    register_convex_histogram!(
        VECTOR_COMPACTION_COMPACTED_SEGMENTS_TOTAL,
        "Total number of compacted segments",
    );
    pub fn log_vector_compaction_total_segments(total_segments: usize) {
        log_distribution(
            &VECTOR_COMPACTION_COMPACTED_SEGMENTS_TOTAL,
            total_segments as f64,
        );
    }

    register_convex_histogram!(
        VECTOR_COMPACTION_COMPACTED_SEGMENT_NUM_VECTORS_TOTAL,
        "Size of the newly generated compacted segment",
    );
    pub fn log_vector_compaction_compacted_segment_num_vectors_total(total_vectors: u32) {
        log_distribution(
            &VECTOR_COMPACTION_COMPACTED_SEGMENT_NUM_VECTORS_TOTAL,
            total_vectors as f64,
        );
    }

    pub enum VectorWriterLockWaiter {
        Compactor,
        Flusher,
    }

    const VECTOR_WRITER_WAITER_LABEL: &str = "waiter";

    impl VectorWriterLockWaiter {
        fn tag(&self) -> MetricTag {
            let label = match self {
                VectorWriterLockWaiter::Compactor => "compactor",
                VectorWriterLockWaiter::Flusher => "flusher",
            };
            metric_tag(format!("{}:{}", VECTOR_WRITER_WAITER_LABEL, label))
        }
    }

    register_convex_histogram!(
        VECTOR_WRITER_LOCK_WAIT_SECONDS,
        "The amount of time spent waiting for the writer lock to commit a vector index metadata \
         change",
        &[VECTOR_WRITER_WAITER_LABEL]
    );
    pub fn vector_writer_lock_wait_timer(waiter: VectorWriterLockWaiter) -> Timer<VMHistogramVec> {
        let mut timer = Timer::new_tagged(&VECTOR_WRITER_LOCK_WAIT_SECONDS);
        timer.add_tag(waiter.tag());
        timer
    }

    const MERGE_LABEL: &str = "merge_required";

    pub enum VectorIndexMergeType {
        Unknown,
        Required,
        NotRequired,
    }

    impl VectorIndexMergeType {
        fn metric_tag(&self) -> MetricTag {
            let label = match self {
                VectorIndexMergeType::Unknown => "unknown",
                VectorIndexMergeType::Required => "required",
                VectorIndexMergeType::NotRequired => "not_required",
            };
            metric_tag(format!("{}:{}", MERGE_LABEL, label))
        }
    }

    register_convex_histogram!(
        VECTOR_COMPACTION_MERGE_COMMIT_SECONDS,
        "Time to merge deletes and commit after compaction",
        &[STATUS_LABEL[0], MERGE_LABEL],
    );
    pub fn vector_compaction_merge_commit_timer() -> StatusTimer {
        let mut timer = StatusTimer::new(&VECTOR_COMPACTION_MERGE_COMMIT_SECONDS);
        timer.add_tag(VectorIndexMergeType::Unknown.metric_tag());
        timer
    }

    register_convex_histogram!(
        VECTOR_FLUSH_MERGE_COMMIT_SECONDS,
        "Time to merge deletes and commit after flushing",
        &[STATUS_LABEL[0], MERGE_LABEL],
    );
    pub fn vector_flush_merge_commit_timer() -> StatusTimer {
        let mut timer = StatusTimer::new(&VECTOR_COMPACTION_MERGE_COMMIT_SECONDS);
        timer.add_tag(VectorIndexMergeType::Unknown.metric_tag());
        timer
    }

    pub fn finish_vector_index_merge_timer(
        mut timer: StatusTimer,
        merge_type: VectorIndexMergeType,
    ) {
        timer.replace_tag(
            VectorIndexMergeType::Unknown.metric_tag(),
            merge_type.metric_tag(),
        );
        timer.finish();
    }
}
