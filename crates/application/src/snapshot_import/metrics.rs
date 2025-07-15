use std::time::Duration;

use metrics::{
    log_counter,
    log_distribution,
    register_convex_counter,
    register_convex_histogram,
    StatusTimer,
    STATUS_LABEL,
};

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

register_convex_counter!(
    SNAPSHOT_IMPORT_WORKER_DIED_TOTAL,
    "Number of times the snapshot import worker died",
);
pub fn log_snapshot_import_worker_died() {
    log_counter(&SNAPSHOT_IMPORT_WORKER_DIED_TOTAL, 1);
}
