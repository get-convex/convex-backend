use metrics::{
    register_convex_histogram,
    StatusTimer,
    STATUS_LABEL,
};

register_convex_histogram!(
    USAGE_TRACKING_WORKER_RUN_SECONDS,
    "Duration of the usage tracking worker run",
    &STATUS_LABEL,
);

pub fn usage_gauges_tracking_worker_timer() -> StatusTimer {
    StatusTimer::new(&USAGE_TRACKING_WORKER_RUN_SECONDS)
}
