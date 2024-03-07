use metrics::{
    register_convex_histogram,
    CancelableTimer,
    StatusTimer,
    STATUS_LABEL,
};
use vector::{
    vector_index_type_tag,
    VectorIndexType,
    VECTOR_INDEX_TYPE_LABEL,
};

register_convex_histogram!(
    SEARCHLIGHT_QUERY_SECONDS,
    "Duration of a Searchlight query",
    &STATUS_LABEL
);
pub fn query_timer() -> StatusTimer {
    StatusTimer::new(&SEARCHLIGHT_QUERY_SECONDS)
}

register_convex_histogram!(
    SEARCHLIGHT_VECTOR_COMPACTION_PREFETCH_SECONDS,
    "The amount of time it takes to prefetch a new segment produced by vector compaction",
    &STATUS_LABEL
);
pub fn vector_compaction_prefetch_timer() -> StatusTimer {
    StatusTimer::new(&SEARCHLIGHT_VECTOR_COMPACTION_PREFETCH_SECONDS)
}

// Unlike vector_query_timer, this metric excludes the time to fetch and open
// segments. Instead it's close to the query time in the qdrant segment, but
// still includes some translation to/from qdrant/convex types and ids.
register_convex_histogram!(
    SEARCHLIGHT_VECTOR_SCHEMA_QUERY_SECONDS,
    "Duration of a Searchlight schema vector query",
    &STATUS_LABEL
);
pub fn vector_schema_query_timer() -> StatusTimer {
    StatusTimer::new(&SEARCHLIGHT_VECTOR_SCHEMA_QUERY_SECONDS)
}

register_convex_histogram!(
    SEARCHLIGHT_VECTOR_QUERY_SECONDS,
    "Duration of a Searchlight vector query",
    &[STATUS_LABEL[0], VECTOR_INDEX_TYPE_LABEL],
);
pub fn vector_query_timer(vector_index_type: VectorIndexType) -> CancelableTimer {
    let mut timer = CancelableTimer::new(&SEARCHLIGHT_VECTOR_QUERY_SECONDS);
    timer.add_tag(vector_index_type_tag(vector_index_type));
    timer
}
