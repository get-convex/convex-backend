use metrics::{
    log_counter,
    register_convex_counter,
};

register_convex_counter!(
    MIGRATION_WORKER_FAILED_TOTAL,
    "Number of times a migration worker failed"
);
pub fn log_migration_worker_failed() {
    log_counter(&MIGRATION_WORKER_FAILED_TOTAL, 1)
}
