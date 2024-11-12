use std::time::Duration;

use metrics::{
    log_counter_with_labels,
    log_distribution,
    log_distribution_with_labels,
    log_gauge_with_labels,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
    STATUS_LABEL,
};
use model::source_packages::types::PackageSize;

register_convex_counter!(
    EXTERNAL_DEPS_PACKAGES_TOTAL,
    "Total pushes with external dependency packages",
    &["cache_status"],
);
pub fn log_external_deps_package(is_cache_hit: bool) {
    let cache_label = if is_cache_hit { "hit" } else { "miss" };

    log_counter_with_labels(
        &EXTERNAL_DEPS_PACKAGES_TOTAL,
        1,
        vec![StaticMetricLabel::new("cache_status", cache_label)],
    );
}

register_convex_histogram!(
    SOURCE_PACKAGE_SIZE_BYTES_TOTAL,
    "Size of source package in bytes",
    &["compressed"],
);
pub fn log_source_package_size_bytes_total(pkg_size: PackageSize) {
    let zipped_label = StaticMetricLabel::new("compressed", "true");
    let unzipped_label = StaticMetricLabel::new("compressed", "false");

    log_distribution_with_labels(
        &SOURCE_PACKAGE_SIZE_BYTES_TOTAL,
        pkg_size.zipped_size_bytes as f64,
        vec![zipped_label],
    );
    log_distribution_with_labels(
        &SOURCE_PACKAGE_SIZE_BYTES_TOTAL,
        pkg_size.unzipped_size_bytes as f64,
        vec![unzipped_label],
    );
}

register_convex_histogram!(
    SNAPSHOT_IMPORT_TIMER_SECONDS,
    "Time taken for a snapshot import",
    &STATUS_LABEL
);
pub fn snapshot_import_timer() -> StatusTimer {
    StatusTimer::new(&SNAPSHOT_IMPORT_TIMER_SECONDS)
}

register_convex_histogram!(
    SNAPSHOT_IMPORT_AGE_SECONDS,
    "Age of in-progress snapshot import",
);
pub fn log_snapshot_import_age(age: Duration) {
    log_distribution(&SNAPSHOT_IMPORT_AGE_SECONDS, age.as_secs_f64());
}

register_convex_histogram!(
    SNAPSHOT_EXPORT_TIMER_SECONDS,
    "Time taken for a snapshot export",
    &STATUS_LABEL
);
pub fn export_timer() -> StatusTimer {
    StatusTimer::new(&SNAPSHOT_EXPORT_TIMER_SECONDS)
}

pub struct AppWorkerStatus {
    name: &'static str,
}

impl Drop for AppWorkerStatus {
    fn drop(&mut self) {
        log_worker_status(false, self.name);
    }
}

register_convex_gauge!(
    APP_WORKER_IN_PROGRESS_TOTAL,
    "1 if a worker is working, 0 otherwise",
    &["worker"],
);
pub fn log_worker_starting(name: &'static str) -> AppWorkerStatus {
    log_worker_status(true, name);
    AppWorkerStatus { name }
}

fn log_worker_status(is_working: bool, name: &'static str) {
    log_gauge_with_labels(
        &APP_WORKER_IN_PROGRESS_TOTAL,
        if is_working { 1f64 } else { 0f64 },
        vec![StaticMetricLabel::new("worker", name)],
    )
}

register_convex_counter!(
    TABLE_SUMMARY_CHECKPOINT_TOTAL,
    "Number of table summary checkpoint writes",
    &["is_bootstrapping"],
);
pub fn log_table_summary_checkpoint(is_bootstrapping: bool) {
    log_counter_with_labels(
        &TABLE_SUMMARY_CHECKPOINT_TOTAL,
        1,
        vec![StaticMetricLabel::new(
            "is_bootstrapping",
            is_bootstrapping.to_string(),
        )],
    );
}

register_convex_histogram!(
    TABLE_SUMMARY_BOOTSTRAP_SECONDS,
    "Time to bootstrap table summary",
    &STATUS_LABEL
);
pub fn table_summary_bootstrap_timer() -> StatusTimer {
    StatusTimer::new(&TABLE_SUMMARY_BOOTSTRAP_SECONDS)
}
