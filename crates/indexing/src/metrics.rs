use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    IntoLabel,
    StaticMetricLabel,
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
