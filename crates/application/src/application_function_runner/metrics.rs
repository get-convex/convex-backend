use common::types::{
    ModuleEnvironment,
    UdfType,
};
use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_gauge_with_labels,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
    STATUS_LABEL,
};

pub enum UdfExecutorResult {
    Success,
    UserError,
    SystemError(&'static str),
}

register_convex_counter!(
    UDF_EXECUTOR_RESULT_TOTAL,
    "Number of queries against the module cache",
    &["udf_type", "result"]
);
pub fn log_udf_executor_result(udf_type: UdfType, result: UdfExecutorResult) {
    let result_value = match result {
        UdfExecutorResult::Success => "success",
        UdfExecutorResult::UserError => "user_error",
        UdfExecutorResult::SystemError(label) => label,
    };
    log_counter_with_labels(
        &UDF_EXECUTOR_RESULT_TOTAL,
        1,
        vec![
            udf_type.metric_label(),
            StaticMetricLabel::new("result", result_value),
        ],
    );
}

register_convex_counter!(
    APPLICATION_MUTATION_ALREADY_COMMITTED_TOTAL,
    "Count of mutations skipped because they were previously committed"
);
pub fn log_mutation_already_committed() {
    log_counter(&APPLICATION_MUTATION_ALREADY_COMMITTED_TOTAL, 1);
}

register_convex_histogram!(OCC_RETRIES_TOTAL, "Number of OCC retries for a commit");
pub fn log_occ_retries(count: usize) {
    log_distribution(&OCC_RETRIES_TOTAL, count as f64);
}

register_convex_histogram!(
    APPLICATION_MUTATION_SECONDS,
    "Time taken to execute a mutation",
    &STATUS_LABEL
);
pub fn mutation_timer() -> StatusTimer {
    StatusTimer::new(&APPLICATION_MUTATION_SECONDS)
}

pub enum OutstandingFunctionState {
    Running,
    Waiting,
}

register_convex_gauge!(
    APPLICATION_FUNCTION_RUNNER_OUTSTANDING_TOTAL,
    "The number of currently outstanding functions of a given type. Includes both running and \
     waiting functions",
    &["udf_type", "state", "env_type"]
);
pub fn log_outstanding_functions(
    total: usize,
    env: ModuleEnvironment,
    udf_type: UdfType,
    state: OutstandingFunctionState,
) {
    let state_label = StaticMetricLabel::new(
        "state",
        match state {
            OutstandingFunctionState::Running => "running",
            OutstandingFunctionState::Waiting => "waiting",
        },
    );
    log_gauge_with_labels(
        &APPLICATION_FUNCTION_RUNNER_OUTSTANDING_TOTAL,
        total as f64,
        vec![udf_type.metric_label(), state_label, env.metric_label()],
    )
}

register_convex_histogram!(
    APPLICATION_FUNCTION_RUNNER_TOTAL_SECONDS,
    "The total time it took to execute a function. This includes wait time and run time. The \
     metric is also logged for isolate client code path so we can compare apples to apples.",
    &[STATUS_LABEL[0], "udf_type", "env_type"]
);
pub fn function_total_timer(env: ModuleEnvironment, udf_type: UdfType) -> StatusTimer {
    let mut timer = StatusTimer::new(&APPLICATION_FUNCTION_RUNNER_TOTAL_SECONDS);
    timer.add_label(udf_type.metric_label());
    timer.add_label(env.metric_label());
    timer
}

trait ModuleEnvironmentExt {
    fn metric_label(&self) -> StaticMetricLabel;
}

impl ModuleEnvironmentExt for ModuleEnvironment {
    fn metric_label(&self) -> StaticMetricLabel {
        let value = match self {
            ModuleEnvironment::Isolate => "isolate",
            ModuleEnvironment::Node => "node",
            ModuleEnvironment::Invalid => "invalid",
        };
        StaticMetricLabel::new("env_type", value)
    }
}

register_convex_counter!(
    APPLICATION_FUNCTION_RUNNER_WAIT_TIMEOUT_TOTAL,
    "Total number with running a function has timed out due to instance concurrency limits.",
    &["udf_type", "env_type"],
);
pub fn log_function_wait_timeout(env: ModuleEnvironment, udf_type: UdfType) {
    log_counter_with_labels(
        &APPLICATION_FUNCTION_RUNNER_WAIT_TIMEOUT_TOTAL,
        1,
        vec![udf_type.metric_label(), env.metric_label()],
    );
}

register_convex_histogram!(
    APPLICATION_FUNCTION_RUNNER_WAIT_SECONDS,
    "The time a function waited for the semaphore.",
    &[STATUS_LABEL[0], "udf_type"]
);
pub fn function_waiter_timer(udf_type: UdfType) -> StatusTimer {
    let mut timer = StatusTimer::new(&APPLICATION_FUNCTION_RUNNER_WAIT_SECONDS);
    timer.add_label(udf_type.metric_label());
    timer
}

register_convex_histogram!(
    APPLICATION_FUNCTION_RUNNER_RUN_SECONDS,
    "The time a function took to run. This excludes the semaphore wait time.",
    &[STATUS_LABEL[0], "udf_type"]
);
pub fn function_run_timer(udf_type: UdfType) -> StatusTimer {
    let mut timer = StatusTimer::new(&APPLICATION_FUNCTION_RUNNER_RUN_SECONDS);
    timer.add_label(udf_type.metric_label());
    timer
}
