use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_gauge,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
    STATUS_LABEL,
};
register_convex_histogram!(
    CACHE_GET_SECONDS,
    "Time taken for a UDF cache read",
    &["status", "cache_status"]
);
pub fn get_timer() -> StatusTimer {
    let mut t = StatusTimer::new(&CACHE_GET_SECONDS);
    // Start with the error tag until the application calls
    // `succeed_udf_read_timer`, which replaces it with the success tag. This
    // way the success case is the deliberate one, and we'll default to
    // accidentally logging errors over successes.
    t.add_label(StaticMetricLabel::new("cache_status", "unknown"));
    t
}

pub fn succeed_get_timer(mut timer: StatusTimer, is_cache_hit: bool) {
    if is_cache_hit {
        timer.replace_label(
            StaticMetricLabel::new("cache_status", "unknown"),
            StaticMetricLabel::new("cache_status", "hit"),
        );
    } else {
        timer.replace_label(
            StaticMetricLabel::new("cache_status", "unknown"),
            StaticMetricLabel::new("cache_status", "miss"),
        );
    }
    timer.finish();
}

register_convex_histogram!(
    CACHE_SUCCESS_ATTEMPTS_TOTAL,
    "Number of attempts needed on a successful cache fetch"
);

pub fn log_success(num_attempts: usize) {
    log_distribution(&CACHE_SUCCESS_ATTEMPTS_TOTAL, num_attempts as f64);
}

register_convex_counter!(
    CACHE_PLAN_READY_TOTAL,
    "Number of times a cache entry was already ready"
);
pub fn log_plan_ready() {
    log_counter(&CACHE_PLAN_READY_TOTAL, 1);
}

register_convex_counter!(
    CACHE_PLAN_PEER_TIMEOUT_TOTAL,
    "Number of times a peer was found to have timed out when computing a cache result"
);
pub fn log_plan_peer_timeout() {
    log_counter(&CACHE_PLAN_PEER_TIMEOUT_TOTAL, 1);
}

register_convex_counter!(
    CACHE_PLAN_WAIT_TOTAL,
    "Number of times an execution plans to wait for a cache result"
);
pub fn log_plan_wait() {
    log_counter(&CACHE_PLAN_WAIT_TOTAL, 1);
}
pub enum GoReason {
    NoCacheResult,
    PeerTimestampTooNew,
}
register_convex_counter!(
    CACHE_PLAN_GO_TOTAL,
    "Number of times an execution plans to compute the cache result",
    &["reason"]
);
pub fn log_plan_go(reason: GoReason) {
    let label = match reason {
        GoReason::NoCacheResult => StaticMetricLabel::new("reason", "no_cache_result"),
        GoReason::PeerTimestampTooNew => StaticMetricLabel::new("reason", "peer_timestamp_too_new"),
    };
    log_counter_with_labels(&CACHE_PLAN_GO_TOTAL, 1, vec![label]);
}

register_convex_counter!(
    CACHE_PERFORM_PEER_TIMEOUT_TOTAL,
    "Number of times a waiting execution determined that a peer timed out"
);
pub fn log_perform_wait_peer_timeout() {
    log_counter(&CACHE_PERFORM_PEER_TIMEOUT_TOTAL, 1);
}

register_convex_counter!(
    CACHE_PERFORM_SELF_TIMEOUT_TOTAL,
    "Number of times an execution determined its own cache computation timed out"
);
pub fn log_perform_wait_self_timeout() {
    log_counter(&CACHE_PERFORM_SELF_TIMEOUT_TOTAL, 1);
}
register_convex_counter!(
    CACHE_PERFORM_GO_TOTAL,
    "Number of times an execution begins computing a cache result",
    &STATUS_LABEL
);
pub fn log_perform_go(is_ok: bool) {
    log_counter_with_labels(
        &CACHE_PERFORM_GO_TOTAL,
        1,
        vec![StaticMetricLabel::status(is_ok)],
    );
}

register_convex_counter!(
    CACHE_TS_TOO_OLD_TOTAL,
    "Number of times a cache entry disregarded as it is too new for the requested timestamp"
);
pub fn log_validate_ts_too_old() {
    log_counter(&CACHE_TS_TOO_OLD_TOTAL, 1);
}

register_convex_counter!(
    CACHE_DROP_CACHE_RESULT_TOO_OLD_TOTAL,
    "Number of times a cache result is dropped as it is older than the existing entry"
);
pub fn log_drop_cache_result_too_old() {
    log_counter(&CACHE_DROP_CACHE_RESULT_TOO_OLD_TOTAL, 1);
}

register_convex_counter!(
    CACHE_VALIDATE_REFRESH_FAILED_TOTAL,
    "Number of times a cache entry couldn't be refreshed during validation"
);
pub fn log_validate_refresh_failed() {
    log_counter(&CACHE_VALIDATE_REFRESH_FAILED_TOTAL, 1);
}

register_convex_counter!(
    CACHE_VALIDATE_SYSTEM_TIME_TOO_OLD_TOTAL,
    "Number of times a cache entry's system time was too old"
);
pub fn log_validate_system_time_too_old() {
    log_counter(&CACHE_VALIDATE_SYSTEM_TIME_TOO_OLD_TOTAL, 1);
}
register_convex_counter!(
    CACHE_VALIDATE_SYSTEM_TIME_IN_THE_FUTURE_TOTAL,
    "Number of times a cache entry's system time was in the future"
);
pub fn log_validate_system_time_in_the_future() {
    log_counter(&CACHE_VALIDATE_SYSTEM_TIME_IN_THE_FUTURE_TOTAL, 1);
}

register_convex_gauge!(CACHE_SIZE_BYTES, "Size of the cache in bytes");
pub fn log_cache_size(size: usize) {
    log_gauge(&CACHE_SIZE_BYTES, size as f64)
}
