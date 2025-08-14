use metrics::{
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
