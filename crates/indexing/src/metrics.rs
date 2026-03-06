use metrics::{
    log_counter,
    log_counter_with_labels,
    register_convex_counter,
    register_convex_histogram,
    IntoLabel,
    StaticMetricLabel,
    StatusTimer,
};

register_convex_counter!(
    TRANSACTION_INDEX_CACHE_HIT_TOTAL,
    "Count of transaction index cache reads, labeled with cache hits",
    &["hit"]
);

pub fn log_transaction_cache_query(hit: bool) {
    log_counter_with_labels(
        &TRANSACTION_INDEX_CACHE_HIT_TOTAL,
        1,
        vec![StaticMetricLabel::new("hit", hit.as_label())],
    );
}

register_convex_counter!(
    TRANSACTION_INDEX_CACHE_CLEARED_TOTAL,
    "Count of times transaction cache was cleared"
);
pub fn log_index_cache_cleared() {
    log_counter(&TRANSACTION_INDEX_CACHE_CLEARED_TOTAL, 1);
}

register_convex_histogram!(
    INDEX_PAGE_SECONDS,
    "Time to execute IndexReader::index_page in seconds",
    &["source", "status"]
);
pub fn index_page_timer(source: &'static str) -> StatusTimer {
    let mut t = StatusTimer::new(&INDEX_PAGE_SECONDS);
    t.add_label(StaticMetricLabel::new("source", source));
    t
}
