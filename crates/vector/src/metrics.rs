use metrics::{
    log_counter,
    log_counter_with_tags,
    log_distribution,
    metric_tag,
    register_convex_counter,
    register_convex_histogram,
    MetricTag,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::VMHistogram;

use crate::{
    qdrant_index::QdrantVectorIndexType,
    query::CompiledVectorFilter,
    CompiledVectorSearch,
    VectorSearchQueryResult,
};

register_convex_histogram!(
    VECTOR_BOOTSTRAP_SECONDS,
    "Time taken to bootstrap search indexes",
    &STATUS_LABEL
);
pub fn bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_BOOTSTRAP_SECONDS)
}

register_convex_counter!(
    VECTOR_BOOTSTRAP_REVISIONS_TOTAL,
    "Number of revisions loaded during vector bootstrap"
);
register_convex_counter!(
    VECTOR_BOOTSTRAP_REVISIONS_BYTES,
    "Total size of revisions loaded during vector bootstrap"
);
pub fn finish_bootstrap(num_revisions: usize, bytes: usize, timer: StatusTimer) {
    log_counter(&VECTOR_BOOTSTRAP_REVISIONS_TOTAL, num_revisions as u64);
    log_counter(&VECTOR_BOOTSTRAP_REVISIONS_BYTES, bytes as u64);
    timer.finish();
}

register_convex_histogram!(
    VECTOR_INDEXES_BOOTSTRAP_SECONDS,
    "Time to bootstrap vector indexes",
);
pub fn bootstrap_vector_indexes_timer() -> Timer<VMHistogram> {
    Timer::new(&VECTOR_INDEXES_BOOTSTRAP_SECONDS)
}

pub enum IndexUpdateType {
    IndexMetadata,
    Document,
    None,
}

impl IndexUpdateType {
    fn tag(&self) -> &'static str {
        match self {
            IndexUpdateType::IndexMetadata => "index_metadata",
            IndexUpdateType::Document => "document",
            IndexUpdateType::None => "none",
        }
    }
}

register_convex_histogram!(
    VECTOR_INDEX_MANAGER_UPDATE_SECONDS,
    "Duration of a vector index update",
    &[STATUS_LABEL[0], "index_update_type"],
);
pub fn index_manager_update_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_INDEX_MANAGER_UPDATE_SECONDS)
}

pub fn finish_index_manager_update_timer(
    mut timer: StatusTimer,
    index_update_type: IndexUpdateType,
) {
    timer.add_tag(metric_tag(format!(
        "index_update_type:{}",
        index_update_type.tag()
    )));
    timer.finish();
}

register_convex_histogram!(
    VECTOR_SEARCH_SCHEMA_COMPILE_SECONDS,
    "Time to compile a search schema",
    &STATUS_LABEL
);
pub fn compile_timer() -> StatusTimer {
    StatusTimer::new(&VECTOR_SEARCH_SCHEMA_COMPILE_SECONDS)
}

register_convex_histogram!(
    VECTOR_SEARCH_SEARCHLIGHT_OVERFETCH_DELTA_TOTAL,
    "Size of the vector searchlight overfetch delta"
);
pub fn log_searchlight_overfetch_delta(overfetch_delta: usize) {
    log_distribution(
        &VECTOR_SEARCH_SEARCHLIGHT_OVERFETCH_DELTA_TOTAL,
        overfetch_delta as f64,
    );
}

register_convex_histogram!(
    VECTOR_SEARCH_SEARCHLIGHT_CLIENT_EXECUTE_SECONDS,
    "Time to execute a vector query against Searchlight",
    &[STATUS_LABEL[0], VECTOR_INDEX_TYPE_LABEL],
);
pub fn searchlight_client_execute_timer(vector_index_type: VectorIndexType) -> StatusTimer {
    let mut timer = StatusTimer::new(&VECTOR_SEARCH_SEARCHLIGHT_CLIENT_EXECUTE_SECONDS);
    timer.add_tag(vector_index_type_tag(vector_index_type));
    timer
}

register_convex_histogram!(
    VECTOR_SEARCH_SEARCHLIGHT_CLIENT_RESULTS_TOTAL,
    "Number of vector results from Searchlight"
);
pub fn finish_searchlight_client_execute(
    timer: StatusTimer,
    results: &Vec<VectorSearchQueryResult>,
) {
    log_distribution(
        &VECTOR_SEARCH_SEARCHLIGHT_CLIENT_RESULTS_TOTAL,
        results.len() as f64,
    );
    timer.finish();
}

register_convex_histogram!(
    VECTOR_SEARCH_NUM_DISCARDED_REVISIONS_TOTAL,
    "Number of vector discarded revisions"
);
pub fn log_num_discarded_revisions(discarded_revisions: usize) {
    log_distribution(
        &VECTOR_SEARCH_NUM_DISCARDED_REVISIONS_TOTAL,
        discarded_revisions as f64,
    );
}

register_convex_histogram!(
    VECTOR_SEARCH_NUMBER_OF_SEGMENTS_TOTAL,
    "Number of vector segments searched for a multi segment vector index"
);
pub fn log_num_segments_searched_total(num_segments: usize) {
    log_distribution(&VECTOR_SEARCH_NUMBER_OF_SEGMENTS_TOTAL, num_segments as f64);
}

fn log_vector_search_total(filter: &str) {
    log_counter_with_tags(
        &VECTOR_SEARCH_COMPILE_TOTAL,
        1,
        vec![metric_tag(format!("filter_type:{filter}"))],
    );
}

register_convex_counter!(
    VECTOR_SEARCH_COMPILE_TOTAL,
    "Number of vector searches that are compiled",
    &["filter_type"]
);
register_convex_histogram!(
    VECTOR_SEARCH_COMPILE_FILTER_IN_TOTAL,
    "Number of terms in an IN vector search filter",
);
register_convex_histogram!(
    VECTOR_SEARCH_VECTOR_LENGTH_TOTAL,
    "The size of the vector being searched by vector search",
);
pub fn log_compiled_query(query: &CompiledVectorSearch) {
    if query.filter_conditions.is_empty() {
        log_vector_search_total("none");
    } else if query.filter_conditions.len() == 1 {
        for filter in query.filter_conditions.values() {
            match filter {
                CompiledVectorFilter::Eq(_) => log_vector_search_total("eq"),
                CompiledVectorFilter::In(vec) => {
                    log_vector_search_total("in");
                    log_distribution(&VECTOR_SEARCH_COMPILE_FILTER_IN_TOTAL, vec.len() as f64);
                },
            }
        }
    } else {
        log_vector_search_total("multifield");
    }
    log_distribution(
        &VECTOR_SEARCH_VECTOR_LENGTH_TOTAL,
        query.vector.len() as f64,
    )
}

register_convex_histogram!(
    VECTOR_INDEX_MANAGER_SEARCH_SECONDS,
    "Total vector search duration",
    &[STATUS_LABEL[0], VECTOR_INDEX_TYPE_LABEL],
);
pub fn search_timer() -> StatusTimer {
    let mut timer = StatusTimer::new(&VECTOR_INDEX_MANAGER_SEARCH_SECONDS);
    timer.add_tag(vector_index_type_tag(VectorIndexType::Unknown));
    timer
}

register_convex_histogram!(
    VECTOR_INDEX_MANAGER_RESULTS_TOTAL,
    "Number of results from the vector index manager"
);

pub fn finish_search(
    mut timer: StatusTimer,
    results: &Vec<VectorSearchQueryResult>,
    vector_index_type: VectorIndexType,
) {
    log_distribution(&VECTOR_INDEX_MANAGER_RESULTS_TOTAL, results.len() as f64);
    timer.add_tag(vector_index_type_tag(vector_index_type));
    timer.finish();
}

register_convex_counter!(
    VECTOR_UPDATE_INDEX_CREATED_TOTAL,
    "Number of vector indexes created"
);
pub fn log_index_created() {
    log_counter(&VECTOR_UPDATE_INDEX_CREATED_TOTAL, 1);
}

register_convex_counter!(
    VECTOR_UPDATE_INDEX_BACKFILLED_TOTAL,
    "Number of vector indexes backfilled"
);
pub fn log_index_backfilled() {
    log_counter(&VECTOR_UPDATE_INDEX_BACKFILLED_TOTAL, 1);
}

register_convex_counter!(
    VECTOR_UPDATE_INDEX_ADVANCED_TOTAL,
    "Number of vector indexes advanced in time"
);
pub fn log_index_advanced() {
    log_counter(&VECTOR_UPDATE_INDEX_ADVANCED_TOTAL, 1);
}
register_convex_counter!(
    VECTOR_UPDATE_INDEX_DELETED_TOTAL,
    "Number of vector index deletions"
);
pub fn log_index_deleted() {
    log_counter(&VECTOR_UPDATE_INDEX_DELETED_TOTAL, 1);
}

const QDRANT_VECTOR_INDEX_TYPE: &str = "index_type";

impl QdrantVectorIndexType {
    fn metric_tag(&self) -> MetricTag {
        let index_string = match self {
            QdrantVectorIndexType::Plain => "plain",
            QdrantVectorIndexType::HNSW => "hnsw",
        };
        metric_tag(format!("{QDRANT_VECTOR_INDEX_TYPE}:{index_string}"))
    }
}

register_convex_histogram!(
    QDRANT_SEGMENT_MEMORY_BUILD_SECONDS,
    "The amount of time it takes to build the appendable memory qdrant segment",
    &STATUS_LABEL,
);
pub fn qdrant_segment_memory_build_timer() -> StatusTimer {
    StatusTimer::new(&QDRANT_SEGMENT_MEMORY_BUILD_SECONDS)
}
register_convex_histogram!(
    QDRANT_SEGMENT_DISK_BUILD_SECONDS,
    "The amount of time it takes to build the hnsw indexed immutable qdrant segment",
    &[STATUS_LABEL[0], QDRANT_VECTOR_INDEX_TYPE],
);
pub fn qdrant_segment_disk_build_timer(disk_index_type: QdrantVectorIndexType) -> StatusTimer {
    let mut timer = StatusTimer::new(&QDRANT_SEGMENT_DISK_BUILD_SECONDS);
    timer.add_tag(disk_index_type.metric_tag());
    timer
}

#[derive(Clone, Copy, Debug)]
pub enum VectorIndexType {
    SingleSegment,
    MultiSegment,
    Unknown,
}

pub const VECTOR_INDEX_TYPE_LABEL: &str = "vector_index_type";
pub fn vector_index_type_tag(vector_index_type: VectorIndexType) -> MetricTag {
    let type_str = match vector_index_type {
        VectorIndexType::SingleSegment => "single_segment",
        VectorIndexType::MultiSegment => "multi_segment",
        VectorIndexType::Unknown => "unknown",
    };
    metric_tag(format!("{VECTOR_INDEX_TYPE_LABEL}:{type_str}"))
}
