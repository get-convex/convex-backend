use std::time::Duration;

use errors::ErrorMetadataAnyhowExt;
use metrics::{
    log_counter_with_tags,
    log_distribution,
    log_gauge,
    metric_tag_const_value,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    MetricTag,
    STATUS_LABEL,
};

register_convex_counter!(
    SCHEDULED_JOB_RESULT_TOTAL,
    "Count of schedule job results",
    &STATUS_LABEL
);
register_convex_histogram!(
    SCHEDULED_JOB_PREV_FAILURES_TOTAL,
    "Num previous failures retried before success",
);
pub fn log_scheduled_job_success(prev_failures: u32) {
    log_counter_with_tags(
        &SCHEDULED_JOB_RESULT_TOTAL,
        1,
        vec![MetricTag::STATUS_SUCCESS],
    );
    log_distribution(&SCHEDULED_JOB_PREV_FAILURES_TOTAL, prev_failures as f64);
}
pub fn log_scheduled_job_failure(e: &anyhow::Error) {
    let tag = e.metric_status_tag_value();
    log_counter_with_tags(
        &SCHEDULED_JOB_RESULT_TOTAL,
        1,
        vec![metric_tag_const_value("status", tag)],
    )
}

register_convex_gauge!(
    SCHEDULED_JOB_EXECUTION_LAG_SECONDS,
    "Schedule job execution lag"
);
pub fn log_scheduled_job_execution_lag(lag: Duration) {
    log_gauge(&SCHEDULED_JOB_EXECUTION_LAG_SECONDS, lag.as_secs_f64());
}
