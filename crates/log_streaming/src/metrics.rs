use metrics::{
    log_counter,
    log_counter_with_labels,
    register_convex_counter,
    StaticMetricLabel,
};

register_convex_counter!(
    DATADOG_SINK_LOGS_RECEIVED_TOTAL,
    "Number of received logs by the Datadog sink",
    &["version"],
);
pub fn datadog_sink_logs_received(count: usize, version: &'static str) {
    log_counter_with_labels(
        &DATADOG_SINK_LOGS_RECEIVED_TOTAL,
        count as u64,
        vec![StaticMetricLabel::new("version", version)],
    );
}

register_convex_counter!(
    DATADOG_SINK_LOGS_SENT_TOTAL,
    "Number of logs sent out successfully by Datadog sink",
    &["version"],
);
pub fn datadog_sink_logs_sent(count: usize, version: &'static str) {
    log_counter_with_labels(
        &DATADOG_SINK_LOGS_SENT_TOTAL,
        count as u64,
        vec![StaticMetricLabel::new("version", version)],
    );
}

register_convex_counter!(
    WEBHOOK_SINK_LOGS_RECEIVED_TOTAL,
    "Number of received logs by the Webhook sink",
);
pub fn webhook_sink_logs_received(count: usize) {
    log_counter(&WEBHOOK_SINK_LOGS_RECEIVED_TOTAL, count as u64);
}

register_convex_counter!(
    WEBHOOK_SINK_LOGS_SENT_TOTAL,
    "Number of logs sent out successfully by Webhook sink",
);
pub fn webhook_sink_logs_sent(count: usize) {
    log_counter(&WEBHOOK_SINK_LOGS_SENT_TOTAL, count as u64);
}

register_convex_counter!(
    LOG_MANAGER_LOGS_RECEIVED_TOTAL,
    "Number of logs received by log manager",
);
pub fn log_manager_logs_received(count: usize) {
    log_counter(&LOG_MANAGER_LOGS_RECEIVED_TOTAL, count as u64);
}

register_convex_counter!(
    LOG_EVENT_DROPPED_OVERFLOW_ERROR_TOTAL,
    "Number of log events dropped due to an overflowing LogManager channel"
);
pub fn log_event_dropped_overflow_error(count: usize) {
    log_counter(&LOG_EVENT_DROPPED_OVERFLOW_ERROR_TOTAL, count as u64)
}

register_convex_counter!(
    LOG_EVENT_DROPPED_DISCONNECTED_ERROR_TOTAL,
    "Number of log events dropped due to a disconnected LogManager channel"
);
pub fn log_event_dropped_disconnected_error(count: usize) {
    log_counter(&LOG_EVENT_DROPPED_DISCONNECTED_ERROR_TOTAL, count as u64)
}

register_convex_counter!(
    LOG_EVENT_TOTAL,
    "Total number of log events sent through LogManager channel",
);
pub fn log_event_total(count: usize) {
    log_counter(&LOG_EVENT_TOTAL, count as u64)
}

register_convex_counter!(
    AXIOM_SINK_LOGS_RECEIVED_TOTAL,
    "Number of received logs by the Axiom sink",
    &["version"],
);
pub fn axiom_sink_logs_received(count: usize, version: &'static str) {
    log_counter_with_labels(
        &AXIOM_SINK_LOGS_RECEIVED_TOTAL,
        count as u64,
        vec![StaticMetricLabel::new("version", version)],
    );
}

register_convex_counter!(
    AXIOM_SINK_LOGS_SENT_TOTAL,
    "Number of logs sent out successfully by Axiom sink",
    &["version"],
);
pub fn axiom_sink_logs_sent(count: usize, version: &'static str) {
    log_counter_with_labels(
        &AXIOM_SINK_LOGS_SENT_TOTAL,
        count as u64,
        vec![StaticMetricLabel::new("version", version)],
    );
}

register_convex_counter!(
    SENTRY_SINK_LOGS_RECEIVED_TOTAL,
    "Number of received logs by the Sentry sink",
);
pub fn sentry_sink_logs_received(count: usize) {
    log_counter(&SENTRY_SINK_LOGS_RECEIVED_TOTAL, count as u64);
}

register_convex_counter!(
    SENTRY_SINK_LOGS_SENT_TOTAL,
    "Number of logs sent out successfully by Sentry sink",
);
pub fn sentry_sink_logs_sent(count: usize) {
    log_counter(&SENTRY_SINK_LOGS_SENT_TOTAL, count as u64);
}
