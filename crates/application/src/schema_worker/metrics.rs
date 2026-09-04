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
    SCHEMA_VALIDATION_TIMER_SECONDS,
    "Time taken to validate a schema",
    &STATUS_LABEL
);
pub fn schema_validation_timer() -> StatusTimer {
    StatusTimer::new(&SCHEMA_VALIDATION_TIMER_SECONDS)
}

register_convex_counter!(
    SCHEMA_VALIDATION_DOCUMENTS_VALIDATED_TOTAL,
    "Number of documents validated against a schem"
);
pub fn log_document_validated() {
    log_counter(&SCHEMA_VALIDATION_DOCUMENTS_VALIDATED_TOTAL, 1);
}

register_convex_counter!(
    SCHEMA_VALIDATION_DOCUMENT_BYTES,
    "Total bytes of documents validated against a schema"
);
pub fn log_document_bytes(bytes: usize) {
    log_counter(&SCHEMA_VALIDATION_DOCUMENT_BYTES, bytes as u64);
}

register_convex_histogram!(
    SCHEMA_VALIDATION_WALK_TS_LAG_SECONDS,
    "How far a schema walk's last page timestamp is ahead of its starting timestamp"
);
pub fn log_walk_ts_lag(lag: Duration) {
    log_distribution(&SCHEMA_VALIDATION_WALK_TS_LAG_SECONDS, lag.as_secs_f64());
}
