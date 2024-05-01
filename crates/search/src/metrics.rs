use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_gauge_with_labels,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    IntoLabel,
    StaticMetricLabel,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::VMHistogram;

use crate::{
    query::{
        CompiledQuery,
        RevisionWithKeys,
    },
    scoring::Bm25StatisticsDiff,
    tantivy_query::SearchQueryResult,
    SearchFileType,
};

register_convex_histogram!(
    SEARCH_INDEX_INTO_TANTIVY_DOCUMENT_SECONDS,
    "Time taken to generate a Tantivy document"
);
pub fn index_into_tantivy_document_timer() -> Timer<VMHistogram> {
    Timer::new(&SEARCH_INDEX_INTO_TANTIVY_DOCUMENT_SECONDS)
}

register_convex_histogram!(
    SEARCH_INDEX_MANAGER_UPDATE_SECONDS,
    "Duration of a search index update",
    &STATUS_LABEL
);
pub fn index_manager_update_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_INDEX_MANAGER_UPDATE_SECONDS)
}

register_convex_histogram!(
    SEARCH_INDEX_UPDATE_SECONDS,
    "Duration of updating a single in-memory index",
    &STATUS_LABEL
);
pub fn index_update_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_INDEX_UPDATE_SECONDS)
}

register_convex_histogram!(SEARCH_TERM_TEXT_BYTES, "Size of search terms");
pub fn log_text_term(term: &str) {
    log_distribution(&SEARCH_TERM_TEXT_BYTES, term.len() as f64);
}

register_convex_histogram!(SEARCH_TERM_FILTER_BYTES, "Size of search filters");
pub fn log_filter_term(term: &[u8]) {
    log_distribution(&SEARCH_TERM_FILTER_BYTES, term.len() as f64);
}

register_convex_histogram!(
    SEARCH_INDEX_INTO_TERMS_SECONDS,
    "Time to process a document into terms"
);
pub fn index_into_terms_timer() -> Timer<VMHistogram> {
    Timer::new(&SEARCH_INDEX_INTO_TERMS_SECONDS)
}

register_convex_counter!(
    SEARCH_UPDATE_INDEX_CREATED_TOTAL,
    "Number of search indexes created"
);
pub fn log_index_created() {
    log_counter(&SEARCH_UPDATE_INDEX_CREATED_TOTAL, 1);
}

register_convex_counter!(
    SEARCH_UPDATE_INDEX_BACKFILLED_TOTAL,
    "Number of search indexes backfilled"
);
pub fn log_index_backfilled() {
    log_counter(&SEARCH_UPDATE_INDEX_BACKFILLED_TOTAL, 1);
}

register_convex_counter!(
    SEARCH_UPDATE_INDEX_ADVANCED_TOTAL,
    "Number of search indexes advanced in time"
);
pub fn log_index_advanced() {
    log_counter(&SEARCH_UPDATE_INDEX_ADVANCED_TOTAL, 1);
}
register_convex_counter!(
    SEARCH_UPDATE_INDEX_DELETED_TOTAL,
    "Number of search index deletions"
);
pub fn log_index_deleted() {
    log_counter(&SEARCH_UPDATE_INDEX_DELETED_TOTAL, 1);
}

register_convex_histogram!(
    SEARCH_INDEX_MANAGER_SEARCH_SECONDS,
    "Total search duration",
    &STATUS_LABEL
);
pub fn search_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_INDEX_MANAGER_SEARCH_SECONDS)
}

register_convex_histogram!(
    SEARCH_INDEX_MANAGER_RESULTS_TOTAL,
    "Number of results from the search index manager"
);

pub fn finish_search(timer: StatusTimer, revisions_with_keys: &RevisionWithKeys) {
    log_distribution(
        &SEARCH_INDEX_MANAGER_RESULTS_TOTAL,
        revisions_with_keys.len() as f64,
    );
    timer.finish();
}

register_convex_histogram!(
    SEARCH_SCHEMA_COMPILE_SECONDS,
    "Time to compile a search schema",
    &STATUS_LABEL
);
pub fn compile_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_SCHEMA_COMPILE_SECONDS)
}

register_convex_counter!(
    SEARCH_SCHEMA_COMPILE_TEXT_TERMS_TOTAL,
    "Number of text terms in a compiled search query"
);
register_convex_counter!(
    SEARCH_SCHEMA_COMPILE_FILTER_TERMS_TOTAL,
    "Number of filter terms in a compiled search query"
);
pub fn log_compiled_query(query: &CompiledQuery) {
    log_counter(
        &SEARCH_SCHEMA_COMPILE_TEXT_TERMS_TOTAL,
        query.text_query.len() as u64,
    );
    log_counter(
        &SEARCH_SCHEMA_COMPILE_FILTER_TERMS_TOTAL,
        query.filter_conditions.len() as u64,
    );
}

register_convex_counter!(
    SEARCH_EXCEEDED_TOKEN_LIMIT_TOTAL,
    "The number of times a search query had more tokens than our limit"
);
pub fn log_search_token_limit_exceeded() {
    log_counter(&SEARCH_EXCEEDED_TOKEN_LIMIT_TOTAL, 1)
}

register_convex_histogram!(
    SEARCH_BM25_STATISTICS_DIFF_SECONDS,
    "Time to compute a BM25 diff",
    &STATUS_LABEL
);
pub fn bm25_statistics_diff_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_BM25_STATISTICS_DIFF_SECONDS)
}
register_convex_counter!(
    SEARCH_BM25_TERM_DOC_FREQ_DIFF_TOTAL,
    "Number of documents with a BM25 term diff"
);

register_convex_counter!(
    SEARCH_BM25_NUM_DOCS_DIFF_TOTAL,
    "Total number of documents in the BM25 diff"
);
register_convex_counter!(
    SEARCH_BM25_NUM_SEARCH_TERMS_DIFF_TOTAL,
    "Total number of tokens in the BM25 diff"
);
pub fn log_bm25_statistics_diff(timer: StatusTimer, diff: &Bm25StatisticsDiff) {
    for num_docs_with_term_diff in diff.term_statistics.values() {
        log_counter(
            &SEARCH_BM25_TERM_DOC_FREQ_DIFF_TOTAL,
            *num_docs_with_term_diff as u64,
        );
    }
    log_counter(
        &SEARCH_BM25_NUM_DOCS_DIFF_TOTAL,
        diff.num_documents_diff as u64,
    );
    log_counter(
        &SEARCH_BM25_NUM_SEARCH_TERMS_DIFF_TOTAL,
        diff.num_search_tokens_diff as u64,
    );
    timer.finish();
}

register_convex_histogram!(
    SEARCH_TOTAL_NUM_DOCUMENTS_AND_TOKENS_SECONDS,
    "Time to compute the total number of documents and tokens in memory"
);
pub fn total_num_documents_and_tokens_timer() -> Timer<VMHistogram> {
    Timer::new(&SEARCH_TOTAL_NUM_DOCUMENTS_AND_TOKENS_SECONDS)
}

register_convex_histogram!(
    SEARCH_NUM_DOCUMENTS_WITH_TERM_SECONDS,
    "Time to compute the number of documents containing a term"
);
pub fn num_documents_with_term_timer() -> Timer<VMHistogram> {
    Timer::new(&SEARCH_NUM_DOCUMENTS_WITH_TERM_SECONDS)
}

register_convex_histogram!(
    SEARCH_SEARCHLIGHT_CLIENT_EXECUTE_SECONDS,
    "Time to execute a query against Searchlight",
    &STATUS_LABEL
);
pub fn searchlight_client_execute_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_SEARCHLIGHT_CLIENT_EXECUTE_SECONDS)
}

register_convex_histogram!(
    SEARCH_SEARCHLIGHT_OVERFETCH_DELTA_TOTAL,
    "Size of the searchlight overfetch delta"
);
pub fn log_searchlight_overfetch_delta(overfetch_delta: usize) {
    log_distribution(
        &SEARCH_SEARCHLIGHT_OVERFETCH_DELTA_TOTAL,
        overfetch_delta as f64,
    );
}

register_convex_histogram!(
    SEARCH_NUM_DISCARDED_REVISIONS_TOTAL,
    "Number of discarded revisions"
);
pub fn log_num_discarded_revisions(discarded_revisions: usize) {
    log_distribution(
        &SEARCH_NUM_DISCARDED_REVISIONS_TOTAL,
        discarded_revisions as f64,
    );
}

register_convex_histogram!(
    SEARCH_SEARCHLIGHT_CLIENT_RESULTS_TOTAL,
    "Number of results from Searchlight"
);
pub fn finish_searchlight_client_execute(timer: StatusTimer, result: &SearchQueryResult) {
    log_distribution(
        &SEARCH_SEARCHLIGHT_CLIENT_RESULTS_TOTAL,
        result.results.len() as f64,
    );
    timer.finish();
}

register_convex_histogram!(
    SEARCH_MEMORY_QUERY_SECONDS,
    "Time to execute a search query against the memory index",
    &STATUS_LABEL
);
pub fn memory_query_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_MEMORY_QUERY_SECONDS)
}

register_convex_histogram!(
    SEARCH_INDEX_MEMORY_QUERY_RESULTS_TOTAL,
    "Number of results from querying the in-memory search index"
);
pub fn finish_memory_query(timer: StatusTimer, revisions_len: usize) {
    log_distribution(
        &SEARCH_INDEX_MEMORY_QUERY_RESULTS_TOTAL,
        revisions_len as f64,
    );
    timer.finish();
}
register_convex_histogram!(
    SEARCH_MEMORY_UPDATED_MATCHES_SECONDS,
    "Time to update matches in the memory search index",
    &STATUS_LABEL
);
pub fn updated_matches_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_MEMORY_UPDATED_MATCHES_SECONDS)
}

register_convex_histogram!(
    SEARCH_INDEX_READER_FOR_DIRECTORY_SECONDS,
    "Time to get a Tantivy IndexReader for a directory",
    &STATUS_LABEL
);
pub fn index_reader_for_directory_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_INDEX_READER_FOR_DIRECTORY_SECONDS)
}

register_convex_histogram!(
    SEARCH_QUERY_TANTIVY_SECONDS,
    "Total time to execute a query against Tantivy",
    &STATUS_LABEL
);
pub fn query_tantivy_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_QUERY_TANTIVY_SECONDS)
}

register_convex_histogram!(
    SEARCH_QUERY_TANTIVY_SEGMENTS_TOTAL,
    "Number of segments in the Tantivy index"
);
pub fn log_num_segments(num_segments: usize) {
    log_distribution(&SEARCH_QUERY_TANTIVY_SEGMENTS_TOTAL, num_segments as f64);
}

register_convex_histogram!(
    SEARCH_QUERY_TANTIVY_RESULTS_TOTAL,
    "Number of results from Tantivy"
);
pub fn finish_query_tantivy(timer: StatusTimer, revisions_len: usize) {
    log_distribution(&SEARCH_QUERY_TANTIVY_RESULTS_TOTAL, revisions_len as f64);
    timer.finish();
}

register_convex_histogram!(
    SEARCH_QUERY_TANTIVY_STATISTICS_SECONDS,
    "Time to query Tantivy statistics",
    &STATUS_LABEL
);
pub fn query_tantivy_statistics_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_QUERY_TANTIVY_STATISTICS_SECONDS)
}

register_convex_histogram!(
    SEARCH_QUERY_TANTIVY_SEARCH_SECONDS,
    "Time to collect Tantivy search results",
    &STATUS_LABEL
);
pub fn query_tantivy_search_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_QUERY_TANTIVY_SEARCH_SECONDS)
}

register_convex_histogram!(
    SEARCH_QUERY_TANTIVY_FAST_FIELD_SECONDS,
    "Time to query Tantivy fast fields",
    &STATUS_LABEL
);
pub fn query_tantivy_fast_field_timer() -> StatusTimer {
    StatusTimer::new(&SEARCH_QUERY_TANTIVY_FAST_FIELD_SECONDS)
}

register_convex_histogram!(
    SEARCH_QUERY_READS_OVERLAPS_SECONDS,
    "Time to compute if a read query overlaps with a document"
);
pub fn query_reads_overlaps_timer() -> Timer<VMHistogram> {
    Timer::new(&SEARCH_QUERY_READS_OVERLAPS_SECONDS)
}

register_convex_counter!(
    SEARCH_QUERY_READS_OVERLAPS_TOTAL,
    "Number of query reads and whether or not they overlapped a document",
    &["overlaps"]
);
pub fn log_query_reads_outcome(overlaps: bool) {
    log_counter_with_labels(
        &SEARCH_QUERY_READS_OVERLAPS_TOTAL,
        1,
        vec![StaticMetricLabel::new("overlaps", overlaps.as_label())],
    );
}

register_convex_histogram!(
    VECTOR_COMPACTION_COMPACT_SECONDS_TOTAL,
    "The amount of time spent actually compacting segments",
    &STATUS_LABEL,
);
pub fn vector_compact_seconds_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_COMPACTION_COMPACT_SECONDS_TOTAL)
}

register_convex_histogram!(
    VECTOR_COMPACTION_FETCH_SEGMENTS_SECONDS_TOTAL,
    "The amount of time spent fetching segments to compact",
    &STATUS_LABEL,
);
pub fn vector_compact_fetch_segments_seconds_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_COMPACTION_FETCH_SEGMENTS_SECONDS_TOTAL)
}

register_convex_histogram!(
    VECTOR_COMPACTION_CONSTRUCT_SEGMENTS_SECONDS_TOTAL,
    "The amount of time spent compacting segments after they've been fetched",
    &STATUS_LABEL,
);
pub fn vector_compact_construct_segment_seconds_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_COMPACTION_CONSTRUCT_SEGMENTS_SECONDS_TOTAL)
}

register_convex_histogram!(
    VECTOR_COMPACTION_VECTORS_IN_SEGMENT_TOTAL,
    "The total number of vectors in the newly created compacted segment"
);
pub fn log_vectors_in_compacted_segment_total(num_vectors: u32) {
    log_distribution(
        &VECTOR_COMPACTION_VECTORS_IN_SEGMENT_TOTAL,
        num_vectors as f64,
    );
}

register_convex_histogram!(
    VECTOR_COMPACTION_COMPACTED_SEGMENT_SIZE_BYTES,
    "The total number of vectors in the newly created compacted segment"
);
pub fn log_compacted_segment_size_bytes(size_bytes: u64) {
    log_distribution(
        &VECTOR_COMPACTION_COMPACTED_SEGMENT_SIZE_BYTES,
        size_bytes as f64,
    );
}

pub const SEARCH_FILE_TYPE: &str = "search_file_type";

impl SearchFileType {
    pub fn metric_label(&self) -> StaticMetricLabel {
        let search_type_str = match self {
            SearchFileType::VectorSegment => "vector_segment",
            SearchFileType::VectorDeletedBitset => "vector_deleted_bitset",
            SearchFileType::VectorIdTracker => "vector_id_tracker",
            SearchFileType::Text => "text",
            SearchFileType::TextIdTracker => "text_id_tracker",
            SearchFileType::TextDeletedBitset => "text_deleted_bitset",
            SearchFileType::TextDeletedTerms => "text_deleted_terms",
            SearchFileType::FragmentedVectorSegment => "fragmented_vector_segment",
        };
        StaticMetricLabel::new(SEARCH_FILE_TYPE, search_type_str)
    }
}

register_convex_histogram!(
    SEARCH_UPLOAD_ARCHIVE_SECONDS,
    "Amount of time it takes to upload an archive",
    &[STATUS_LABEL[0], SEARCH_FILE_TYPE],
);
pub fn upload_archive_timer(search_file_type: SearchFileType) -> StatusTimer {
    let mut timer = StatusTimer::new(&SEARCH_UPLOAD_ARCHIVE_SECONDS);
    timer.add_label(search_file_type.metric_label());
    timer
}

#[derive(Clone, Copy)]
pub enum SearchType {
    Vector,
    Text,
}

pub const SEARCH_TYPE_LABEL: [&str; 1] = ["search_type"];
pub fn search_type_label(search_type: SearchType) -> StaticMetricLabel {
    let type_str = match search_type {
        SearchType::Vector => "vector",
        SearchType::Text => "text",
    };
    StaticMetricLabel::new("search_type", type_str)
}

register_convex_counter!(
    SEARCHLIGHT_ASYNC_LRU_CACHE_HIT_TOTAL,
    "Count of requests which had a result ready in the archive cache",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_cache_hit(label: &str) {
    log_counter_with_labels(
        &SEARCHLIGHT_ASYNC_LRU_CACHE_HIT_TOTAL,
        1,
        vec![async_lru_label(label)],
    );
}

pub const ASYNC_LRU_LABEL: &str = "label";
pub fn async_lru_label(label: &str) -> StaticMetricLabel {
    StaticMetricLabel::new(ASYNC_LRU_LABEL, label.to_owned())
}

register_convex_counter!(
    SEARCHLIGHT_ASYNC_LRU_CACHE_WAITING_TOTAL,
    "Count of requests which waited on a result to become ready in the archive cache",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_cache_waiting(label: &str) {
    log_counter_with_labels(
        &SEARCHLIGHT_ASYNC_LRU_CACHE_WAITING_TOTAL,
        1,
        vec![async_lru_label(label)],
    );
}

register_convex_counter!(
    SEARCHLIGHT_ASYNC_LRU_CACHE_MISS_TOTAL,
    "Count of requests which had to fetch the archive as the cache missed",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_cache_miss(label: &str) {
    log_counter_with_labels(
        &SEARCHLIGHT_ASYNC_LRU_CACHE_MISS_TOTAL,
        1,
        vec![async_lru_label(label)],
    );
}

register_convex_gauge!(
    SEARCHLIGHT_ASYNC_LRU_SIZE_TOTAL,
    "Number of entries in a searchlight async LRU",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_size(size: usize, label: &str) {
    log_gauge_with_labels(
        &SEARCHLIGHT_ASYNC_LRU_SIZE_TOTAL,
        size as f64,
        vec![async_lru_label(label)],
    )
}

register_convex_histogram!(
    VECTOR_PREFETCH_SECONDS,
    "Time to prefetch a vector segment in searchlight once it reaches the front of the queue",
    &STATUS_LABEL,
);
pub fn vector_prefetch_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_PREFETCH_SECONDS)
}

register_convex_counter!(
    VECTOR_PREFETCH_REJECTIONS_TOTAL,
    "Count of number of rejected prefetch requests due to the queue being full",
);
pub fn log_vector_prefetch_rejection() {
    log_counter(&VECTOR_PREFETCH_REJECTIONS_TOTAL, 1);
}

register_convex_counter!(
    VECTOR_PREFETCH_EXPIRATIONS_TOTAL,
    "Count of number of expired prefetch requests due codel queue expiration",
);
pub fn log_vector_prefetch_expiration() {
    log_counter(&VECTOR_PREFETCH_EXPIRATIONS_TOTAL, 1);
}
