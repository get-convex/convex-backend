use metrics::{
    register_convex_histogram,
    CancelableTimer,
    StatusTimer,
    STATUS_LABEL,
};
use vector::{
    vector_index_type_label,
    VectorIndexType,
    VECTOR_INDEX_TYPE_LABEL,
};

register_convex_histogram!(
    TEXT_QUERY_TOKENS_SEARCHER_LATENCY_SECONDS,
    "The amount of time it took to query for text token matches on searchlight (in Searcher)",
    &STATUS_LABEL,
);
pub(crate) fn text_query_tokens_searcher_latency_seconds() -> StatusTimer {
    StatusTimer::new(&TEXT_QUERY_TOKENS_SEARCHER_LATENCY_SECONDS)
}

register_convex_histogram!(
    TEXT_QUERY_BM25_SEARCHER_LATENCY_SECONDS,
    "The amount of time it took to query for bm25 stats from searcher (searchlight)",
    &STATUS_LABEL,
);
pub(crate) fn text_query_bm25_searcher_latency_seconds() -> StatusTimer {
    StatusTimer::new(&TEXT_QUERY_BM25_SEARCHER_LATENCY_SECONDS)
}

register_convex_histogram!(
    TEXT_QUERY_POSTING_LISTS_SEARCHER_LATENCY_SECONDS,
    "The amount of time it took to query for posting lists in searcher (searchlight)",
    &STATUS_LABEL,
);
pub(crate) fn text_query_posting_lists_searcher_latency_seconds() -> StatusTimer {
    StatusTimer::new(&TEXT_QUERY_POSTING_LISTS_SEARCHER_LATENCY_SECONDS)
}

register_convex_histogram!(
    TEXT_COMPACTION_SEARCHER_LATENCY_SECONDS,
    "The amount of time it took to run a text index compaction in searcher (searchlight)",
    &STATUS_LABEL,
);
pub(crate) fn text_compaction_searcher_latency_seconds() -> StatusTimer {
    StatusTimer::new(&TEXT_COMPACTION_SEARCHER_LATENCY_SECONDS)
}

register_convex_histogram!(
    TEXT_QUERY_TERM_ORDINALS_SEARCHER_LATENCY_SECONDS,
    "The amount of time it took to query for term ordinals for values in Searcher (searchlight)",
    &STATUS_LABEL,
);
pub(crate) fn text_query_term_ordinals_searcher_timer() -> StatusTimer {
    StatusTimer::new(&TEXT_QUERY_TERM_ORDINALS_SEARCHER_LATENCY_SECONDS)
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
    timer.add_label(vector_index_type_label(vector_index_type));
    timer
}
