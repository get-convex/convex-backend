use std::time::Duration;

use metrics::{
    log_counter,
    log_distribution,
    log_distribution_with_labels,
    register_convex_counter,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
};
use model::source_packages::types::PackageSize;

register_convex_histogram!(
    NODE_EXECUTOR_TOTAL_SECONDS,
    "Duration of Node executor",
    &["status", "method"]
);
pub fn node_executor(method: &'static str) -> StatusTimer {
    let mut t = StatusTimer::new(&NODE_EXECUTOR_TOTAL_SECONDS);
    t.add_label(StaticMetricLabel::new("method", method));
    t
}

register_convex_histogram!(
    NODE_EXECUTOR_DOWNLOAD_SECONDS,
    "Download duration of Node executor"
);
pub fn log_download_time(elapsed: Duration) {
    log_distribution(&NODE_EXECUTOR_DOWNLOAD_SECONDS, elapsed.as_secs_f64());
}

register_convex_histogram!(
    NODE_EXECUTOR_IMPORT_SECONDS,
    "Import duration of Node executor"
);
pub fn log_import_time(elapsed: Duration) {
    log_distribution(&NODE_EXECUTOR_IMPORT_SECONDS, elapsed.as_secs_f64());
}

register_convex_histogram!(
    NODE_EXECUTOR_UDF_SECONDS,
    "UDF execution time in Node executor"
);
pub fn log_udf_time(elapsed: Duration) {
    log_distribution(&NODE_EXECUTOR_UDF_SECONDS, elapsed.as_secs_f64());
}

register_convex_histogram!(NODE_EXECUTOR_OVERHEAD_SECONDS, "Overhead of Node executor");
pub fn log_overhead(elapsed: Duration) {
    log_distribution(&NODE_EXECUTOR_OVERHEAD_SECONDS, elapsed.as_secs_f64());
}

register_convex_histogram!(
    NODE_EXECUTOR_LAMBDA_TOTAL_SECONDS,
    "Node executor total duration"
);
pub fn log_total_executor_time(elapsed: Duration) {
    log_distribution(&NODE_EXECUTOR_LAMBDA_TOTAL_SECONDS, elapsed.as_secs_f64());
}

register_convex_counter!(
    NODE_EXECUTOR_COLD_START_TOTAL,
    "Number of cold starts in the Node executor"
);
register_convex_counter!(
    NODE_EXECUTOR_NON_LAMBDA_RESPONSE_TOTAL,
    "Number of non-lambda responses in the Node executor"
);
pub fn log_function_execution(cold_start: Option<bool>) {
    match cold_start {
        Some(cold_start) => {
            let value = if cold_start { 1 } else { 0 };
            log_counter(&NODE_EXECUTOR_COLD_START_TOTAL, value);
        },
        None => {
            // If cold_start is not set, the error didn't come up from the function
            // executor.
            log_counter(&NODE_EXECUTOR_NON_LAMBDA_RESPONSE_TOTAL, 1);
        },
    }
}

register_convex_counter!(
    NODE_SOURCE_MAP_MISSING_TOTAL,
    "Number of times source map is missing during a UDF or HTTP analysis"
);
pub fn log_node_source_map_missing() {
    log_counter(&NODE_SOURCE_MAP_MISSING_TOTAL, 1);
}

register_convex_counter!(
    NODE_SOURCE_MAP_TOKEN_LOOKUP_FAILED_TOTAL,
    "Number of times source map exists but token lookup yields an invalid value during a UDF or \
     HTTP analysis"
);
pub fn log_node_source_map_token_lookup_failed() {
    log_counter(&NODE_SOURCE_MAP_TOKEN_LOOKUP_FAILED_TOTAL, 1);
}

register_convex_histogram!(
    EXTERNAL_DEPS_SIZE_BYTES_TOTAL,
    "Size of external deps",
    &["compressed"],
);
pub fn log_external_deps_size_bytes_total(pkg_size: PackageSize) {
    let zipped_label = StaticMetricLabel::new("compressed", "true");
    let unzipped_label = StaticMetricLabel::new("compressed", "false");

    log_distribution_with_labels(
        &EXTERNAL_DEPS_SIZE_BYTES_TOTAL,
        pkg_size.zipped_size_bytes as f64,
        vec![zipped_label],
    );
    log_distribution_with_labels(
        &EXTERNAL_DEPS_SIZE_BYTES_TOTAL,
        pkg_size.unzipped_size_bytes as f64,
        vec![unzipped_label],
    );
}
