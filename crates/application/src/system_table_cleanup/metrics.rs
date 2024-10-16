use metrics::{
    log_counter,
    log_counter_with_labels,
    prometheus::VMHistogram,
    register_convex_counter,
    register_convex_histogram,
    StaticMetricLabel,
    Timer,
};
use value::TableName;

register_convex_histogram!(
    SYSTEM_TABLE_CLEANUP_SECONDS,
    "Duration of system table cleanup"
);
pub fn system_table_cleanup_timer() -> Timer<VMHistogram> {
    Timer::new(&SYSTEM_TABLE_CLEANUP_SECONDS)
}

register_convex_counter!(
    SYSTEM_TABLE_CLEANUP_ROWS_TOTAL,
    "Number of rows cleaned up in system tables",
    &["table"]
);
pub fn log_system_table_cleanup_rows(table_name: &TableName, rows: usize) {
    log_counter_with_labels(
        &SYSTEM_TABLE_CLEANUP_ROWS_TOTAL,
        rows as u64,
        vec![StaticMetricLabel::new("table", table_name.to_string())],
    )
}

register_convex_counter!(
    EXPORT_TABLE_CLEANUP_ROWS_TOTAL,
    "Number of rows cleaned up in _exports table",
);
pub fn log_exports_s3_cleanup() {
    log_counter(&EXPORT_TABLE_CLEANUP_ROWS_TOTAL, 1)
}
