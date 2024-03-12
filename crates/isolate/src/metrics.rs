use common::{
    types::UdfType,
    version::Version,
};
use deno_core::v8;
use errors::ErrorMetadata;
use metrics::{
    log_counter,
    log_counter_with_tags,
    log_distribution,
    log_gauge_with_tags,
    metric_tag,
    metric_tag_const,
    metric_tag_const_value,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    CancelableTimer,
    MetricTag,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::VMHistogram;
use sync_types::CanonicalizedUdfPath;

use crate::{
    environment::udf::outcome::UdfOutcome,
    ActionOutcome,
    FunctionOutcome,
    HttpActionOutcome,
};

register_convex_histogram!(
    UDF_EXECUTE_SECONDS,
    "Duration of an UDF execution",
    &["udf_type", "npm_version", "status"]
);
pub fn execute_timer(udf_type: &UdfType, npm_version: &Option<Version>) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_EXECUTE_SECONDS);
    t.add_tag(udf_type.metric_tag());
    t.add_tag(match npm_version {
        Some(v) => metric_tag(format!("npm_version:{v}")),
        None => metric_tag_const("npm_version:none"),
    });
    t
}

register_convex_gauge!(
    ISOLATE_POOL_RUNNING_COUNT_INFO,
    "How many isolate workers are currently running work",
    &["pool_name"]
);
pub fn log_pool_running_count(name: &'static str, count: usize) {
    log_gauge_with_tags(
        &ISOLATE_POOL_RUNNING_COUNT_INFO,
        count as f64,
        vec![metric_tag_const_value("pool_name", name)],
    );
}

register_convex_gauge!(
    ISOLATE_POOL_ALLOCATED_COUNT_INFO,
    "How many isolate workers have been allocated",
    &["pool_name"]
);
pub fn log_pool_allocated_count(name: &'static str, count: usize) {
    log_gauge_with_tags(
        &ISOLATE_POOL_ALLOCATED_COUNT_INFO,
        count as f64,
        vec![metric_tag_const_value("pool_name", name)],
    );
}

pub fn is_developer_ok(outcome: &FunctionOutcome) -> bool {
    match &outcome {
        FunctionOutcome::Query(UdfOutcome { result, .. }) => result.is_ok(),
        FunctionOutcome::Mutation(UdfOutcome { result, .. }) => result.is_ok(),
        FunctionOutcome::Action(ActionOutcome { result, .. }) => result.is_ok(),
        FunctionOutcome::HttpAction(HttpActionOutcome { result, .. }) => result.is_ok(),
    }
}

pub fn finish_execute_timer(timer: StatusTimer, outcome: &FunctionOutcome) {
    if is_developer_ok(outcome) {
        timer.finish();
    } else {
        timer.finish_developer_error();
    }
}

register_convex_counter!(UDF_EXECUTE_FULL_TOTAL, "UDF execution queue full count");
pub fn execute_full_error() -> ErrorMetadata {
    log_counter(&UDF_EXECUTE_FULL_TOTAL, 1);
    ErrorMetadata::overloaded(
        "ExecuteFullError",
        "Too many concurrent requests, backoff and try again.",
    )
}

register_convex_histogram!(
    UDF_SERVICE_REQUEST_SECONDS,
    "Time to service an UDF request",
    &["status", "udf_type"]
);
pub fn service_request_timer(udf_type: &UdfType) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_SERVICE_REQUEST_SECONDS);
    t.add_tag(udf_type.metric_tag());
    t
}

register_convex_histogram!(UDF_QUEUE_SECONDS, "UDF queue time");
pub fn queue_timer() -> Timer<VMHistogram> {
    Timer::new(&UDF_QUEUE_SECONDS)
}

pub enum RequestStatus {
    Success,
    DeveloperError,
    SystemError,
}

pub fn finish_service_request_timer(timer: StatusTimer, status: RequestStatus) {
    match status {
        RequestStatus::Success => {
            timer.finish();
        },
        RequestStatus::DeveloperError => {
            timer.finish_developer_error();
        },
        RequestStatus::SystemError => (),
    };
}

register_convex_histogram!(
    UDF_ISOLATE_BUILD_SECONDS,
    "Time to build isolate context",
    &STATUS_LABEL
);
pub fn context_build_timer() -> StatusTimer {
    StatusTimer::new(&UDF_ISOLATE_BUILD_SECONDS)
}

register_convex_histogram!(
    UDF_ISOLATE_ARGUMENTS_BYTES,
    "Size of isolate arguments in bytes"
);
pub fn log_argument_length(args: &str) {
    log_distribution(&UDF_ISOLATE_ARGUMENTS_BYTES, args.len() as f64);
}

register_convex_histogram!(UDF_ISOLATE_RESULT_BYTES, "Size of isolate results in bytes");
pub fn log_result_length(result: &str) {
    log_distribution(&UDF_ISOLATE_RESULT_BYTES, result.len() as f64);
}

register_convex_histogram!(UDF_OP_SECONDS, "Duration of UDF op", &["status", "op"]);
pub fn op_timer(op_name: &str) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_OP_SECONDS);
    t.add_tag(metric_tag(format!("op:{op_name}")));
    t
}

register_convex_histogram!(
    UDF_SYSCALL_SECONDS,
    "Duration of UDF syscall",
    &["status", "syscall"]
);
pub fn syscall_timer(op_name: &str) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_SYSCALL_SECONDS);
    t.add_tag(metric_tag(format!("syscall:{op_name}")));
    t
}

register_convex_histogram!(
    UDF_ASYNC_SYSCALL_SECONDS,
    "Duration of UDF async syscall",
    &["status", "syscall"]
);
pub fn async_syscall_timer(op_name: &str) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_ASYNC_SYSCALL_SECONDS);
    t.add_tag(metric_tag(format!("syscall:{op_name}")));
    t
}

register_convex_counter!(
    UDF_UNAWAITED_OP_TOTAL,
    "Count of async syscalls/ops still pending when a function resolves",
    &["environment"],
);
pub fn log_unawaited_pending_op(count: usize, environment: &'static str) {
    log_counter_with_tags(
        &UDF_UNAWAITED_OP_TOTAL,
        count as u64,
        vec![metric_tag_const_value("environment", environment)],
    );
}

register_convex_counter!(
    FUNCTION_LIMIT_WARNING_TOTAL,
    "Count of functions that exceeded some limit warning level",
    &["limit", "system_udf_path"]
);
pub fn log_function_limit_warning(
    limit_name: &'static str,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) {
    let tags = match system_udf_path {
        Some(udf_path) => vec![
            metric_tag_const_value("limit", limit_name),
            metric_tag(format!("system_udf_path:{udf_path}")),
        ],
        None => vec![
            metric_tag_const_value("limit", limit_name),
            metric_tag_const_value("system_udf_path", "none"),
        ],
    };
    log_counter_with_tags(&FUNCTION_LIMIT_WARNING_TOTAL, 1, tags);
}

register_convex_counter!(
    UDF_SOURCE_MAP_FAILURE_TOTAL,
    "Number of source map failures"
);
pub fn log_source_map_failure(exception_message: &str, e: &anyhow::Error) {
    tracing::error!("Failed to extract error from {exception_message:?}: {e}");
    log_counter(&UDF_SOURCE_MAP_FAILURE_TOTAL, 1);
}

register_convex_counter!(UDF_USER_TIMEOUT_TOTAL, "Number of UDF user timeouts");
pub fn log_user_timeout() {
    log_counter(&UDF_USER_TIMEOUT_TOTAL, 1);
}

register_convex_counter!(UDF_SYSTEM_TIMEOUT_TOTAL, "Number of UDF system timeouts");
pub fn log_system_timeout() {
    log_counter(&UDF_SYSTEM_TIMEOUT_TOTAL, 1);
}

register_convex_counter!(
    RECREATE_ISOLATE_TOTAL,
    "Number of times an isolate is recreated",
    &["reason"]
);
pub fn log_recreate_isolate(reason: &'static str) {
    log_counter_with_tags(
        &RECREATE_ISOLATE_TOTAL,
        1,
        vec![metric_tag_const_value("reason", reason)],
    )
}

register_convex_counter!(
    ISOLATE_REQUEST_CANCELED_TOTAL,
    "Number of times an isolate execution have exited due to cancellation",
);
pub fn log_isolate_request_cancelled() {
    log_counter(&ISOLATE_REQUEST_CANCELED_TOTAL, 1)
}

register_convex_counter!(
    PROMISE_HANDLER_ADDED_AFTER_REJECT_TOTAL,
    "Number of times a promise handler was added after rejection"
);
pub fn log_promise_handler_added_after_reject() {
    log_counter(&PROMISE_HANDLER_ADDED_AFTER_REJECT_TOTAL, 1);
}

register_convex_histogram!(ISOLATE_USED_HEAP_SIZE_BYTES, "Isolate used heap size");
register_convex_histogram!(ISOLATE_HEAP_SIZE_LIMIT_BYTES, "Isolate heap size limit");
register_convex_histogram!(
    ISOLATE_TOTAL_AVAILABLE_SIZE_BYTES,
    "Isolate total available size"
);
register_convex_histogram!(ISOLATE_TOTAL_HEAP_SIZE_BYTES, "Isolate total heap size");
register_convex_histogram!(
    ISOLATE_TOTAL_HEAP_SIZE_EXECUTABLE_BYTES,
    "Isolate total executable heap size "
);
register_convex_histogram!(ISOLATE_EXTERNAL_MEMORY_BYTES, "Isolate external memory");
register_convex_histogram!(ISOLATE_TOTAL_PHYSICAL_SIZE_BYTES, "Isolate physical size");
register_convex_histogram!(ISOLATE_MALLOCED_MEMORY_BYTES, "Isolate malloc'd memory");
register_convex_histogram!(
    ISOLATE_PEAK_MALLOCED_MEMORY_BYTES,
    "Isolate peak malloc'd memory"
);
register_convex_histogram!(
    ISOLATE_TOTAL_GLOBAL_HANDLES_SIZE_BYTES,
    "Isolate size of all global handles"
);
register_convex_histogram!(
    ISOLATE_NATIVE_CONTEXT_TOTAL,
    "Isolate number of native contexts"
);
register_convex_histogram!(
    ISOLATE_DETACHED_CONTEXT_TOTAL,
    "Isolate number of detached contexts"
);
/// Heap statistics currently logged before building the Context and running the
/// UDF, to detect leaks between UDFs.
pub fn log_heap_statistics(stats: &v8::HeapStatistics) {
    log_distribution(&ISOLATE_USED_HEAP_SIZE_BYTES, stats.used_heap_size() as f64);
    log_distribution(
        &ISOLATE_HEAP_SIZE_LIMIT_BYTES,
        stats.heap_size_limit() as f64,
    );
    log_distribution(
        &ISOLATE_TOTAL_AVAILABLE_SIZE_BYTES,
        stats.total_available_size() as f64,
    );
    log_distribution(
        &ISOLATE_TOTAL_HEAP_SIZE_BYTES,
        stats.total_heap_size() as f64,
    );
    log_distribution(
        &ISOLATE_TOTAL_HEAP_SIZE_EXECUTABLE_BYTES,
        stats.total_heap_size_executable() as f64,
    );
    log_distribution(
        &ISOLATE_EXTERNAL_MEMORY_BYTES,
        stats.external_memory() as f64,
    );
    log_distribution(
        &ISOLATE_TOTAL_PHYSICAL_SIZE_BYTES,
        stats.total_physical_size() as f64,
    );
    log_distribution(
        &ISOLATE_MALLOCED_MEMORY_BYTES,
        stats.malloced_memory() as f64,
    );
    log_distribution(
        &ISOLATE_PEAK_MALLOCED_MEMORY_BYTES,
        stats.peak_malloced_memory() as f64,
    );
    log_distribution(
        &ISOLATE_TOTAL_GLOBAL_HANDLES_SIZE_BYTES,
        stats.total_global_handles_size() as f64,
    );

    log_distribution(
        &ISOLATE_NATIVE_CONTEXT_TOTAL,
        stats.number_of_native_contexts() as f64,
    );
    log_distribution(
        &ISOLATE_DETACHED_CONTEXT_TOTAL,
        stats.number_of_detached_contexts() as f64,
    );
}

register_convex_histogram!(UDF_FETCH_SECONDS, "Duration of UDF fetch", &STATUS_LABEL);
pub fn udf_fetch_timer() -> StatusTimer {
    StatusTimer::new(&UDF_FETCH_SECONDS)
}

register_convex_histogram!(CREATE_ISOLATE_SECONDS, "Time to create a new isolate");
pub fn create_isolate_timer() -> Timer<prometheus::VMHistogram> {
    Timer::new(&CREATE_ISOLATE_SECONDS)
}

register_convex_histogram!(
    CONCURRENCY_PERMIT_ACQUIRE_SECONDS,
    "Time to acquire a concurrency permit. High latency indicate that isolate threads are \
     oversubscribed and spend time waiting for CPU instead of waiting on async work",
    &STATUS_LABEL
);
pub fn concurrency_permit_acquire_timer() -> CancelableTimer {
    CancelableTimer::new(&CONCURRENCY_PERMIT_ACQUIRE_SECONDS)
}

register_convex_counter!(UDF_FETCH_TOTAL, "Number of UDF fetches", &STATUS_LABEL);
register_convex_counter!(UDF_FETCH_BYTES_TOTAL, "Number of bytes fetched in UDFs");
pub fn finish_udf_fetch_timer(t: StatusTimer, success: Result<usize, ()>) {
    let status_tag = if let Ok(size) = success {
        t.finish();
        log_counter(&UDF_FETCH_BYTES_TOTAL, size as u64);
        MetricTag::STATUS_SUCCESS
    } else {
        MetricTag::STATUS_ERROR
    };
    log_counter_with_tags(&UDF_FETCH_TOTAL, 1, vec![status_tag]);
}

// Analyze counters
register_convex_counter!(
    SOURCE_MAP_MISSING_TOTAL,
    "Number of times source map is missing during a UDF or HTTP analysis"
);
pub fn log_source_map_missing() {
    log_counter(&SOURCE_MAP_MISSING_TOTAL, 1);
}

register_convex_counter!(
    SOURCE_MAP_TOKEN_LOOKUP_FAILED_TOTAL,
    "Number of times source map exists but token lookup yields an invalid value during a UDF or \
     HTTP analysis"
);
pub fn log_source_map_token_lookup_failed() {
    log_counter(&SOURCE_MAP_TOKEN_LOOKUP_FAILED_TOTAL, 1);
}

register_convex_counter!(
    SOURCE_MAP_ORIGIN_IN_SEPARATE_MODULE_TOTAL,
    "Number of times the origin of a V8 Function is in a separate module during a UDF or HTTP \
     analysis"
);
pub fn log_source_map_origin_in_separate_module() {
    log_counter(&SOURCE_MAP_ORIGIN_IN_SEPARATE_MODULE_TOTAL, 1);
}

register_convex_histogram!(MODULE_LOAD_SECONDS, "Time to loado modules");
pub fn module_load_timer() -> Timer<VMHistogram> {
    Timer::new(&MODULE_LOAD_SECONDS)
}
