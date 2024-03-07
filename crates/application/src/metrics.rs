use metrics::{
    log_counter_with_tags,
    log_distribution_with_tags,
    metric_tag_const,
    metric_tag_const_value,
    register_convex_counter,
    register_convex_histogram,
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
    let cache_tag = if is_cache_hit {
        "cache_status:hit"
    } else {
        "cache_status:miss"
    };

    log_counter_with_tags(
        &EXTERNAL_DEPS_PACKAGES_TOTAL,
        1,
        vec![metric_tag_const(cache_tag)],
    );
}

register_convex_histogram!(
    SOURCE_PACKAGE_SIZE_BYTES_TOTAL,
    "Size of source package in bytes",
    &["compressed"],
);
pub fn log_source_package_size_bytes_total(pkg_size: PackageSize) {
    let zipped_tag = metric_tag_const_value("compressed", "true");
    let unzipped_tag = metric_tag_const_value("compressed", "false");

    log_distribution_with_tags(
        &SOURCE_PACKAGE_SIZE_BYTES_TOTAL,
        pkg_size.zipped_size_bytes as f64,
        vec![zipped_tag],
    );
    log_distribution_with_tags(
        &SOURCE_PACKAGE_SIZE_BYTES_TOTAL,
        pkg_size.unzipped_size_bytes as f64,
        vec![unzipped_tag],
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
    SNAPSHOT_EXPORT_TIMER_SECONDS,
    "Time taken for a snapshot export",
    &STATUS_LABEL
);
pub fn export_timer() -> StatusTimer {
    StatusTimer::new(&SNAPSHOT_EXPORT_TIMER_SECONDS)
}
