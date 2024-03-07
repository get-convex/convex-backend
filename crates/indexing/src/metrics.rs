use metrics::{
    log_counter_with_tags,
    metric_tag_const,
    register_convex_counter,
};

register_convex_counter!(
    TRANSACTION_INDEX_CACHE_HIT_TOTAL,
    "Count of transaction index cache reads, labeled with cache hits",
    &["hit"]
);

pub fn log_transaction_cache_query(hit: bool) {
    let label = if hit { "hit:true" } else { "hit:false" };
    log_counter_with_tags(
        &TRANSACTION_INDEX_CACHE_HIT_TOTAL,
        1,
        vec![metric_tag_const(label)],
    );
}
