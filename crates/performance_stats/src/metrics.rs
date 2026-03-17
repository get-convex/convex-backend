use metrics::{
    log_distribution_with_labels,
    register_convex_histogram,
    StaticMetricLabel,
};

use crate::performance::JemallocStats;

register_convex_histogram!(
    APPLICATION_PROCESS_MEMORY_BYTES,
    "Process-level memory statistics in bytes",
    &["component"]
);
pub fn log_process_level_stats(memory_usage: usize, jemalloc: &JemallocStats) {
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        memory_usage as f64,
        vec![StaticMetricLabel::new("component", "memory_usage")],
    );
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        jemalloc.allocated as f64,
        vec![StaticMetricLabel::new("component", "jemalloc_allocated")],
    );
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        jemalloc.active as f64,
        vec![StaticMetricLabel::new("component", "jemalloc_active")],
    );
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        jemalloc.metadata as f64,
        vec![StaticMetricLabel::new("component", "jemalloc_metadata")],
    );
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        jemalloc.resident as f64,
        vec![StaticMetricLabel::new("component", "jemalloc_resident")],
    );
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        jemalloc.mapped as f64,
        vec![StaticMetricLabel::new("component", "jemalloc_mapped")],
    );
    log_distribution_with_labels(
        &APPLICATION_PROCESS_MEMORY_BYTES,
        jemalloc.retained as f64,
        vec![StaticMetricLabel::new("component", "jemalloc_retained")],
    );
}
