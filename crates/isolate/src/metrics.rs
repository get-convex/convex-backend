use std::{
    borrow::Cow,
    sync::Arc,
    time::Duration,
};

use common::{
    components::ResolvedComponentFunctionPath,
    types::UdfType,
    version::Version,
};
use deno_core::v8;
use errors::ErrorMetadata;
use fastrace::{
    local::LocalSpan,
    Event,
};
use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_gauge,
    log_gauge_with_labels,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    CancelableTimer,
    IntoLabel,
    MetricLabel,
    StaticMetricLabel,
    StatusTimer,
    Timer,
    STATUS_LABEL,
};
use prometheus::{
    VMHistogram,
    VMHistogramVec,
};
use sync_types::CanonicalizedUdfPath;
use udf::{
    ActionOutcome,
    FunctionOutcome,
    HttpActionOutcome,
    UdfOutcome,
};

use crate::IsolateHeapStats;

register_convex_histogram!(
    UDF_EXECUTE_SECONDS,
    "Duration of an UDF execution",
    &["udf_type", "npm_version", "status"]
);
pub fn execute_timer(udf_type: &UdfType, npm_version: &Option<Version>) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_EXECUTE_SECONDS);
    t.add_label(udf_type.metric_label());
    t.add_label(match npm_version {
        Some(v) => StaticMetricLabel::new("npm_version", v.to_string()),
        None => StaticMetricLabel::new("npm_version", "none"),
    });
    t
}

register_convex_gauge!(
    ISOLATE_POOL_RUNNING_COUNT_INFO,
    "How many isolate workers are currently running work",
    &["pool_name", "client_id"]
);
pub fn log_pool_running_count(name: &'static str, count: usize, client_id: &str) {
    log_gauge_with_labels(
        &ISOLATE_POOL_RUNNING_COUNT_INFO,
        count as f64,
        vec![
            StaticMetricLabel::new("pool_name", name),
            MetricLabel::new("client_id", client_id),
        ],
    );
}

register_convex_gauge!(
    ISOLATE_POOL_MAX_INFO,
    "How many isolate workers can be running",
    &["pool_name"]
);
pub fn log_pool_max(name: &'static str, count: usize) {
    log_gauge_with_labels(
        &ISOLATE_POOL_MAX_INFO,
        count as f64,
        vec![StaticMetricLabel::new("pool_name", name)],
    );
}

register_convex_gauge!(
    ISOLATE_POOL_ALLOCATED_COUNT_INFO,
    "How many isolate workers have been allocated",
    &["pool_name"]
);
pub fn log_pool_allocated_count(name: &'static str, count: usize) {
    log_gauge_with_labels(
        &ISOLATE_POOL_ALLOCATED_COUNT_INFO,
        count as f64,
        vec![StaticMetricLabel::new("pool_name", name)],
    );
}

pub fn is_developer_ok(outcome: &FunctionOutcome) -> bool {
    match &outcome {
        FunctionOutcome::Query(UdfOutcome { result, .. }) => result.is_ok(),
        FunctionOutcome::Mutation(UdfOutcome { result, .. }) => result.is_ok(),
        FunctionOutcome::Action(ActionOutcome { result, .. }) => result.is_ok(),
        FunctionOutcome::HttpAction(HttpActionOutcome { result, .. }) => match result {
            // The developer might hit errors after beginning to stream the response that wouldn't
            // be captured here
            udf::HttpActionResult::Streamed => true,
            udf::HttpActionResult::Error(_) => false,
        },
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
    t.add_label(udf_type.metric_label());
    t
}

register_convex_histogram!(
    ISOLATE_SCHEDULER_STOLEN_WORKER_AGE_SECONDS,
    "The now - last_used_ts in seconds for the stolen worker",
);
pub fn log_worker_stolen(age: Duration) {
    log_distribution(
        &ISOLATE_SCHEDULER_STOLEN_WORKER_AGE_SECONDS,
        age.as_secs_f64(),
    );
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
    UDF_ISOLATE_LOAD_USER_MODULES_SECONDS,
    "Time to load all user modules for a request",
    &["udf_type", "is_dynamic", "status"],
);
pub fn eval_user_module_timer(udf_type: UdfType, is_dynamic: bool) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_ISOLATE_LOAD_USER_MODULES_SECONDS);
    t.add_label(udf_type.metric_label());
    t.add_label(StaticMetricLabel::new("is_dynamic", is_dynamic.as_label()));
    t
}

register_convex_histogram!(
    UDF_ISOLATE_LOOKUP_SOURCE_SECONDS,
    "Time to load a single module's source",
    &["is_system", "status"],
);
pub fn lookup_source_timer(is_system: bool) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_ISOLATE_LOOKUP_SOURCE_SECONDS);
    t.add_label(StaticMetricLabel::new("is_system", is_system.as_label()));
    t
}

register_convex_histogram!(
    UDF_ISOLATE_COMPILE_MODULE_SECONDS,
    "Time to compile a single module's source",
    &["status", "cached"],
);
pub fn compile_module_timer(cached: bool) -> StatusTimer {
    let mut timer = StatusTimer::new(&UDF_ISOLATE_COMPILE_MODULE_SECONDS);
    timer.add_label(MetricLabel::new("cached", cached.as_label()));
    timer
}

register_convex_histogram!(
    UDF_ISOLATE_INSTANTIATE_MODULE_SECONDS,
    "Time to instantiate the top-level module",
    &["status"],
);
pub fn instantiate_module_timer() -> StatusTimer {
    StatusTimer::new(&UDF_ISOLATE_INSTANTIATE_MODULE_SECONDS)
}

register_convex_histogram!(
    UDF_ISOLATE_EVALUATE_MODULE_SECONDS,
    "Time to evaluate the top-level module",
    &["status"],
);
pub fn evaluate_module_timer() -> StatusTimer {
    StatusTimer::new(&UDF_ISOLATE_EVALUATE_MODULE_SECONDS)
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
    t.add_label(StaticMetricLabel::new("op", op_name.to_owned()));
    t
}

register_convex_counter!(
    ISOLATE_DIRECT_FUNCTION_CALL_TOTAL,
    "Number of calls to registered UDFs as js functions"
);
fn log_direct_function_call() {
    log_counter(&ISOLATE_DIRECT_FUNCTION_CALL_TOTAL, 1);
}

pub fn log_log_line(line: &str) {
    // We log a console.warn line containing this link when a function is called
    // directly. These are potentially problematic because it looks like arg and
    // return values are being validated, and a new isolate is running the UDF,
    // but actually the plain JS function is being called. If the non-isolated,
    // non-validated behavior is intended, the helper function should be explicit.
    if line.contains("https://docs.convex.dev/production/best-practices/#use-helper-functions-to-write-shared-code") {
        tracing::warn!("Direct function call detected: '{line}'");
        log_direct_function_call();
    }
}

register_convex_histogram!(
    UDF_SYSCALL_SECONDS,
    "Duration of UDF syscall",
    &["status", "syscall"]
);
pub fn syscall_timer(op_name: &str) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_SYSCALL_SECONDS);
    t.add_label(StaticMetricLabel::new("syscall", op_name.to_owned()));
    t
}

register_convex_histogram!(
    UDF_ASYNC_SYSCALL_SECONDS,
    "Duration of UDF async syscall",
    &["status", "syscall"]
);
pub fn async_syscall_timer(op_name: &str) -> StatusTimer {
    let mut t = StatusTimer::new(&UDF_ASYNC_SYSCALL_SECONDS);
    t.add_label(StaticMetricLabel::new("syscall", op_name.to_owned()));
    t
}

register_convex_counter!(
    UDF_UNAWAITED_OP_TOTAL,
    "Count of async syscalls/ops still pending when a function resolves",
    &["environment"],
);
pub fn log_unawaited_pending_op(count: usize, environment: &'static str) {
    log_counter_with_labels(
        &UDF_UNAWAITED_OP_TOTAL,
        count as u64,
        vec![StaticMetricLabel::new("environment", environment)],
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
    let labels = match system_udf_path {
        Some(udf_path) => vec![
            StaticMetricLabel::new("limit", limit_name),
            StaticMetricLabel::new("system_udf_path", udf_path.to_string()),
        ],
        None => vec![
            StaticMetricLabel::new("limit", limit_name),
            StaticMetricLabel::new("system_udf_path", "none"),
        ],
    };
    log_counter_with_labels(&FUNCTION_LIMIT_WARNING_TOTAL, 1, labels);
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
    log_counter_with_labels(
        &RECREATE_ISOLATE_TOTAL,
        1,
        vec![StaticMetricLabel::new("reason", reason)],
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

register_convex_counter!(
    PROMISE_REJECTED_AFTER_RESOLVED_TOTAL,
    "Number of times a promise was rejected after it was resolved"
);
pub fn log_promise_rejected_after_resolved() {
    log_counter(&PROMISE_REJECTED_AFTER_RESOLVED_TOTAL, 1);
}

register_convex_counter!(
    PROMISE_RESOLVED_AFTER_RESOLVED_TOTAL,
    "Number of times a promise was resolved after it was resolved"
);
pub fn log_promise_resolved_after_resolved() {
    log_counter(&PROMISE_RESOLVED_AFTER_RESOLVED_TOTAL, 1);
}

register_convex_histogram!(ISOLATE_USED_HEAP_SIZE_BYTES, "Isolate used heap size");
register_convex_histogram!(ISOLATE_HEAP_SIZE_LIMIT_BYTES, "Isolate heap size limit");
register_convex_histogram!(ISOLATE_AVAILABLE_SIZE_BYTES, "Isolate available size");
register_convex_histogram!(ISOLATE_HEAP_SIZE_BYTES, "Isolate heap size");
register_convex_histogram!(
    ISOLATE_HEAP_SIZE_EXECUTABLE_BYTES,
    "Isolate executable heap size "
);
register_convex_histogram!(ISOLATE_EXTERNAL_MEMORY_BYTES, "Isolate external memory");
register_convex_histogram!(ISOLATE_PHYSICAL_SIZE_BYTES, "Isolate physical size");
register_convex_histogram!(ISOLATE_MALLOCED_MEMORY_BYTES, "Isolate malloc'd memory");
register_convex_histogram!(
    ISOLATE_PEAK_MALLOCED_MEMORY_BYTES,
    "Isolate peak malloc'd memory"
);
register_convex_histogram!(
    ISOLATE_GLOBAL_HANDLES_SIZE_BYTES,
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
        &ISOLATE_AVAILABLE_SIZE_BYTES,
        stats.total_available_size() as f64,
    );
    log_distribution(&ISOLATE_HEAP_SIZE_BYTES, stats.total_heap_size() as f64);
    log_distribution(
        &ISOLATE_HEAP_SIZE_EXECUTABLE_BYTES,
        stats.total_heap_size_executable() as f64,
    );
    log_distribution(
        &ISOLATE_EXTERNAL_MEMORY_BYTES,
        stats.external_memory() as f64,
    );
    log_distribution(
        &ISOLATE_PHYSICAL_SIZE_BYTES,
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
        &ISOLATE_GLOBAL_HANDLES_SIZE_BYTES,
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

register_convex_gauge!(
    ISOLATE_TOTAL_USED_HEAP_SIZE_BYTES,
    "Total isolate used heap size across all isolates"
);
register_convex_gauge!(
    ISOLATE_TOTAL_HEAP_SIZE_BYTES,
    "Total isolate heap size across all isolates"
);
register_convex_gauge!(
    ISOLATE_TOTAL_HEAP_SIZE_EXECUTABLE_BYTES,
    "Total isolate executable heap siz across all isolates "
);
register_convex_gauge!(
    ISOLATE_TOTAL_EXTERNAL_MEMORY_BYTES,
    "Total isolate external memory across all isolates"
);
register_convex_gauge!(
    ISOLATE_TOTAL_PHYSICAL_SIZE_BYTES,
    "Total isolate physical size across all isolates"
);
register_convex_gauge!(
    ISOLATE_TOTAL_MALLOCED_MEMORY_BYTES,
    "Total isolate malloc'd memory across all isolates"
);

pub fn log_aggregated_heap_stats(stats: &IsolateHeapStats) {
    log_gauge(
        &ISOLATE_TOTAL_USED_HEAP_SIZE_BYTES,
        stats.v8_used_heap_size as f64,
    );
    log_gauge(
        &ISOLATE_TOTAL_HEAP_SIZE_BYTES,
        stats.v8_total_heap_size as f64,
    );
    log_gauge(
        &ISOLATE_TOTAL_HEAP_SIZE_EXECUTABLE_BYTES,
        stats.v8_total_heap_size_executable as f64,
    );
    log_gauge(
        &ISOLATE_TOTAL_EXTERNAL_MEMORY_BYTES,
        stats.v8_external_memory_bytes as f64,
    );
    log_gauge(
        &ISOLATE_TOTAL_PHYSICAL_SIZE_BYTES,
        stats.v8_total_physical_size as f64,
    );
    log_gauge(
        &ISOLATE_TOTAL_MALLOCED_MEMORY_BYTES,
        stats.v8_malloced_memory as f64,
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

register_convex_histogram!(CREATE_CONTEXT_SECONDS, "Time to create a new V8 context");
pub fn create_context_timer() -> Timer<prometheus::VMHistogram> {
    Timer::new(&CREATE_CONTEXT_SECONDS)
}

register_convex_histogram!(
    CREATE_CODE_CACHE_SECONDS,
    "Time to create a code cache for a module",
    &STATUS_LABEL
);
pub fn create_code_cache_timer() -> StatusTimer {
    StatusTimer::new(&CREATE_CODE_CACHE_SECONDS)
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

register_convex_counter!(
    CONCURRENCY_PERMIT_TOTAL_HOLD_TIME_SECONDS,
    "The total time concurrency limit was held for ",
    &["client_id"]
);
pub fn log_concurrency_permit_used(client_id: Arc<String>, duration: Duration) {
    let duration_ms = duration
        .as_millis()
        .try_into()
        .expect("Hold duration is too long {}");
    // This is fairly high cardinality but also super important metric.
    if duration_ms > 0 {
        log_counter_with_labels(
            &CONCURRENCY_PERMIT_TOTAL_HOLD_TIME_SECONDS,
            duration_ms,
            vec![StaticMetricLabel::new("client_id", client_id.to_string())],
        );
    }
}

register_convex_counter!(UDF_FETCH_TOTAL, "Number of UDF fetches", &STATUS_LABEL);
register_convex_counter!(UDF_FETCH_BYTES_TOTAL, "Number of bytes fetched in UDFs");
pub fn finish_udf_fetch_timer(t: StatusTimer, success: Result<usize, ()>) {
    let status_label = if let Ok(size) = success {
        t.finish();
        log_counter(&UDF_FETCH_BYTES_TOTAL, size as u64);
        StaticMetricLabel::STATUS_SUCCESS
    } else {
        StaticMetricLabel::STATUS_ERROR
    };
    log_counter_with_labels(&UDF_FETCH_TOTAL, 1, vec![status_label]);
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

register_convex_histogram!(MODULE_LOAD_SECONDS, "Time to load modules", &["source"]);
pub fn module_load_timer(source: &'static str) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&MODULE_LOAD_SECONDS);
    timer.add_label(MetricLabel::new_const("source", source));
    timer
}

register_convex_counter!(
    ISOLATE_OUT_OF_MEMORY_TOTAL,
    "Number of times isolate ran out of memory during function execution"
);
pub fn log_isolate_out_of_memory() {
    log_counter(&ISOLATE_OUT_OF_MEMORY_TOTAL, 1);
}

pub fn record_component_function_path(component_function_path: &ResolvedComponentFunctionPath) {
    LocalSpan::add_event(Event::new("component_function_path").with_properties(|| {
        let mut labels = vec![(
            Cow::Borrowed("udf_path"),
            Cow::Owned(component_function_path.udf_path.to_string()),
        )];
        if let Some(component_path) = &component_function_path.component_path {
            labels.push((
                Cow::Borrowed("component"),
                Cow::Owned(component_path.to_string()),
            ));
        }
        labels
    }));
}

register_convex_counter!(
    HTTP_ACTION_WITH_UNKNOWN_IDENTITY_TOTAL,
    "Number of HTTP actions that were called with an unknown identity",
);

pub fn log_http_action_with_unknown_identity() {
    log_counter(&HTTP_ACTION_WITH_UNKNOWN_IDENTITY_TOTAL, 1);
}

register_convex_counter!(
    RUN_UDF_TOTAL,
    "Number of times that UDFs invoke nested UDFs",
    &[
        "outer_type",
        "inner_type",
        "outer_observed_identity",
        "inner_observed_identity"
    ]
);

pub fn log_run_udf(
    outer_type: UdfType,
    inner_type: UdfType,
    outer_observed_identity: bool,
    inner_observed_identity: bool,
) {
    log_counter_with_labels(
        &RUN_UDF_TOTAL,
        1,
        vec![
            StaticMetricLabel::new("outer_type", outer_type.to_lowercase_string()),
            StaticMetricLabel::new("inner_type", inner_type.to_lowercase_string()),
            StaticMetricLabel::new(
                "outer_observed_identity",
                outer_observed_identity.as_label(),
            ),
            StaticMetricLabel::new(
                "inner_observed_identity",
                inner_observed_identity.as_label(),
            ),
        ],
    );
}

register_convex_counter!(
    COMPONENT_GET_USER_IDENTITY_TOTAL,
    "Number of times that components call getUserIdentity()",
    &["has_user_identity"]
);

pub fn log_component_get_user_identity(has_user_identity: bool) {
    log_counter_with_labels(
        &COMPONENT_GET_USER_IDENTITY_TOTAL,
        1,
        vec![StaticMetricLabel::new(
            "has_user_identity",
            has_user_identity.as_label(),
        )],
    );
}
