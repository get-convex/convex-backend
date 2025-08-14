use errors::ErrorMetadataAnyhowExt as _;
use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    StaticMetricLabel,
};

register_convex_counter!(
    SNAPSHOT_EXPORT_FAILED_TOTAL,
    "Number of snapshot export attempts that failed",
    &["status"]
);
pub fn log_export_failed(e: &anyhow::Error) {
    let status = e.metric_status_label_value();
    log_counter_with_labels(
        &SNAPSHOT_EXPORT_FAILED_TOTAL,
        1,
        vec![StaticMetricLabel::new("status", status)],
    );
}
