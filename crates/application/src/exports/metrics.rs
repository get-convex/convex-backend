use metrics::{
    register_convex_histogram,
    StatusTimer,
    STATUS_LABEL,
};

register_convex_histogram!(
    SNAPSHOT_EXPORT_TIMER_SECONDS,
    "Time taken for a snapshot export",
    &STATUS_LABEL
);
pub fn export_timer() -> StatusTimer {
    StatusTimer::new(&SNAPSHOT_EXPORT_TIMER_SECONDS)
}
