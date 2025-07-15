use errors::ErrorMetadataAnyhowExt;
use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
};

register_convex_histogram!(
    SNAPSHOT_EXPORT_TIMER_SECONDS,
    "Time taken for a snapshot export",
    &["instance_name", "status"]
);
pub fn export_timer(instance_name: &str) -> StatusTimer {
    let mut timer = StatusTimer::new(&SNAPSHOT_EXPORT_TIMER_SECONDS);
    timer.add_label(StaticMetricLabel::new(
        "instance_name",
        instance_name.to_owned(),
    ));
    timer
}

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
