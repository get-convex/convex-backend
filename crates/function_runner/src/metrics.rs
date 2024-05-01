use metrics::{
    log_counter_with_labels,
    log_distribution_with_labels,
    register_convex_counter,
    register_convex_histogram,
    MetricLabel,
    StaticMetricLabel,
    Timer,
};
use prometheus::{
    VMHistogram,
    VMHistogramVec,
};

fn cache_metric_labels<'a>(table_name: &'a str, instance_name: &'a str) -> Vec<MetricLabel<'a>> {
    vec![
        MetricLabel::new("table", table_name),
        MetricLabel::new("instance_name", instance_name),
    ]
}

register_convex_counter!(
    MEMORY_INDEX_CACHE_GET_TOTAL,
    "Number of funrun index cache gets (hits + misses)",
    &["table", "instance_name"]
);
pub fn log_funrun_index_cache_get(table_name: &str, instance_name: &str) {
    log_counter_with_labels(
        &MEMORY_INDEX_CACHE_GET_TOTAL,
        1,
        cache_metric_labels(table_name, instance_name),
    );
}

register_convex_histogram!(
    MEMORY_INDEX_CACHE_LOAD_INDEX_SECONDS,
    "Time to load an in-memory index for funrun",
    &["table", "instance_name"]
);
pub fn load_index_timer(table_name: &str, instance_name: &str) -> Timer<VMHistogramVec> {
    let mut t = Timer::new_with_labels(&MEMORY_INDEX_CACHE_LOAD_INDEX_SECONDS);
    t.add_label(StaticMetricLabel::new("table", table_name.to_owned()));
    t.add_label(StaticMetricLabel::new(
        "instance_name",
        instance_name.to_owned(),
    ));
    t
}

register_convex_histogram!(
    MEMORY_INDEX_CACHE_LOADED_ROWS,
    "Number of rows loaded for an index",
    &["table", "instance_name"]
);
pub fn log_funrun_index_load_rows(rows: u64, table_name: &str, instance_name: &str) {
    log_distribution_with_labels(
        &MEMORY_INDEX_CACHE_LOADED_ROWS,
        rows as f64,
        cache_metric_labels(table_name, instance_name),
    );
}

register_convex_histogram!(
    FUNCTION_RUNNER_BEGIN_TX_SECONDS,
    "Time to begin a transaction",
);
pub fn begin_tx_timer() -> Timer<VMHistogram> {
    Timer::new(&FUNCTION_RUNNER_BEGIN_TX_SECONDS)
}
