use metrics::{
    log_counter_with_tags,
    log_gauge_with_tags,
    metric_tag,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    CancelableTimer,
    MetricTag,
    StatusTimer,
    STATUS_LABEL,
};

register_convex_counter!(
    ASYNC_LRU_CACHE_HIT_TOTAL,
    "Count of requests which had a result ready in the async lru cache",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_cache_hit(label: &str) {
    log_counter_with_tags(
        &ASYNC_LRU_CACHE_HIT_TOTAL,
        1,
        vec![async_lru_label_tag(label)],
    );
}

pub const ASYNC_LRU_LABEL: &str = "label";
pub fn async_lru_label_tag(label: &str) -> MetricTag {
    metric_tag(format!("{}:{}", ASYNC_LRU_LABEL, label))
}

register_convex_counter!(
    ASYNC_LRU_CACHE_WAITING_TOTAL,
    "Count of requests which waited on a result to become ready in the async lru cache",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_cache_waiting(label: &str) {
    log_counter_with_tags(
        &ASYNC_LRU_CACHE_WAITING_TOTAL,
        1,
        vec![async_lru_label_tag(label)],
    );
}

register_convex_counter!(
    ASYNC_LRU_CACHE_MISS_TOTAL,
    "Count of requests which had to load data as the async lru cache missed",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_cache_miss(label: &str) {
    log_counter_with_tags(
        &ASYNC_LRU_CACHE_MISS_TOTAL,
        1,
        vec![async_lru_label_tag(label)],
    );
}

register_convex_gauge!(
    ASYNC_LRU_SIZE_TOTAL,
    "Number of entries in an async LRU",
    &[ASYNC_LRU_LABEL],
);
pub fn log_async_lru_size(size: usize, label: &str) {
    log_gauge_with_tags(
        &ASYNC_LRU_SIZE_TOTAL,
        size as f64,
        vec![async_lru_label_tag(label)],
    )
}

register_convex_histogram!(
    ASYNC_LRU_COMPUTE_SECONDS,
    "Time to compute an arbitrary value in async lru",
    &[STATUS_LABEL[0], ASYNC_LRU_LABEL],
);
pub fn async_lru_compute_timer(label: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&ASYNC_LRU_COMPUTE_SECONDS);
    timer.add_tag(async_lru_label_tag(label));
    timer
}
register_convex_histogram!(
    ASYNC_LRU_GET_SECONDS,
    "Time taken for the async lru to obtain a value, including both cached and not cached results.",
    &[STATUS_LABEL[0], ASYNC_LRU_LABEL],
);
pub fn async_lru_get_timer(label: &str) -> CancelableTimer {
    let mut timer = CancelableTimer::new(&ASYNC_LRU_GET_SECONDS);
    timer.add_tag(async_lru_label_tag(label));
    timer
}
