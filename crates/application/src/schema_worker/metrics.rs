use metrics::{
    log_counter,
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
