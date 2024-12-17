use std::{
    cell::Cell,
    cmp::Ordering,
    collections::{
        BTreeMap,
        VecDeque,
    },
    str::FromStr,
    sync::Arc,
    time::{
        Duration,
        SystemTime,
    },
};

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    errors::{
        report_error_sync,
        JsError,
    },
    execution_context::ExecutionContext,
    identity::InertIdentity,
    knobs,
    log_lines::{
        LogLine,
        LogLines,
    },
    log_streaming::{
        self,
        FunctionEventSource,
        LogEvent,
        LogSender,
        StructuredLogEvent,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        CursorMs,
        FunctionCaller,
        HttpActionRoute,
        ModuleEnvironment,
        TableName,
        TableStats,
        UdfIdentifier,
        UdfType,
    },
};
use float_next_after::NextAfter;
use http::{
    Method,
    StatusCode,
};
use isolate::{
    environment::helpers::validation::{
        ValidatedActionOutcome,
        ValidatedUdfOutcome,
    },
    HttpActionOutcome,
    HttpActionRequestHead,
    SyscallTrace,
    UdfOutcome,
};
use parking_lot::Mutex;
use serde_json::{
    json,
    Value as JsonValue,
};
use tokio::sync::oneshot;
use udf_metrics::{
    MetricName,
    MetricStore,
    MetricStoreConfig,
    MetricsWindow,
    Percentile,
    Timeseries,
    UdfMetricsError,
};
use url::Url;
use usage_tracking::{
    AggregatedFunctionUsageStats,
    CallType,
    FunctionUsageTracker,
    OccInfo,
    UsageCounter,
};
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    sha256::Sha256Digest,
    ConvexArray,
};

/// A function's execution is summarized by this structure and stored in the
/// UdfExecutionLog
#[derive(Debug, Clone)]
pub struct FunctionExecution {
    pub params: UdfParams,

    /// When we return the function result and log it. For cached
    // queries this can be long after the query was actually
    // executed.
    pub unix_timestamp: UnixTimestamp,

    /// When the function ran. For cached queries this can be long before
    // this execution is requested and is logged.
    pub execution_timestamp: UnixTimestamp,

    /// How this UDF was executed, with read-only or read-write permissions
    pub udf_type: UdfType,

    /// Log lines that the UDF emitted via the `Console` API.
    pub log_lines: LogLines,

    /// Which tables were read or written to in the UDF execution?
    pub tables_touched: WithHeapSize<BTreeMap<TableName, TableStats>>,

    /// Was this UDF execution computed from scratch or cached?
    pub cached_result: bool,
    /// How long (in seconds) did executing this UDF take?
    pub execution_time: f64,

    /// Who called this UDF?
    pub caller: FunctionCaller,

    /// What type of environment was this UDF run in? This can be `Invalid` if
    /// the user specified an invalid path for an action, and then we won't
    /// know whether the action was intended to run in V8 or Node.
    pub environment: ModuleEnvironment,

    /// What syscalls did this function execute?
    pub syscall_trace: SyscallTrace,

    /// Usage statistics for this instance
    pub usage_stats: AggregatedFunctionUsageStats,
    pub action_memory_used_mb: Option<u64>,

    /// The Convex NPM package version pushed with the module version executed.
    pub udf_server_version: Option<semver::Version>,

    /// The identity under which the udf was executed. Must be inert - since it
    /// can only be for logging purposes - not giving any authorization
    /// power.
    pub identity: InertIdentity,

    pub context: ExecutionContext,
}

impl HeapSize for FunctionExecution {
    fn heap_size(&self) -> usize {
        self.params.heap_size()
            + self.log_lines.heap_size()
            + self.tables_touched.heap_size()
            + self.syscall_trace.heap_size()
            + self.context.heap_size()
    }
}

impl FunctionExecution {
    fn identifier(&self) -> UdfIdentifier {
        match &self.params {
            UdfParams::Function { identifier, .. } => UdfIdentifier::Function(identifier.clone()),
            UdfParams::Http { identifier, .. } => UdfIdentifier::Http(identifier.clone()),
        }
    }

    fn event_source(
        &self,
        sub_function_path: Option<&CanonicalizedComponentFunctionPath>,
    ) -> FunctionEventSource {
        let cached = if self.udf_type == UdfType::Query {
            Some(self.cached_result)
        } else {
            None
        };
        let (component_path, udf_path) = match sub_function_path {
            Some(path) => (path.component.clone(), path.udf_path.to_string()),
            None => {
                let udf_id = self.params.identifier_str();
                let component_path = match &self.params {
                    UdfParams::Function { identifier, .. } => identifier.component.clone(),
                    // TODO(ENG-7612): Support HTTP actions in components.
                    UdfParams::Http { .. } => ComponentPath::root(),
                };
                (component_path, udf_id)
            },
        };

        FunctionEventSource {
            component_path,
            udf_path,
            udf_type: self.udf_type,
            module_environment: self.environment,
            cached,
            context: self.context.clone(),
        }
    }

    fn console_log_events_for_log_line(
        &self,
        log_line: &LogLine,
        sub_function_path: Option<&CanonicalizedComponentFunctionPath>,
    ) -> Vec<LogEvent> {
        match log_line {
            LogLine::Structured(log_line) => {
                vec![LogEvent {
                    timestamp: log_line.timestamp,
                    event: StructuredLogEvent::Console {
                        source: self.event_source(sub_function_path),
                        log_line: log_line.clone(),
                    },
                }]
            },
            LogLine::SubFunction { path, log_lines } => log_lines
                .iter()
                .flat_map(|log_line| self.console_log_events_for_log_line(log_line, Some(path)))
                .collect(),
        }
    }

    fn console_log_events(&self) -> Vec<LogEvent> {
        self.log_lines
            .iter()
            .flat_map(|line| self.console_log_events_for_log_line(line, None))
            .collect()
    }

    fn udf_execution_record_log_events(&self) -> anyhow::Result<Vec<LogEvent>> {
        let execution_time = Duration::from_secs_f64(self.execution_time);

        let mut events = vec![LogEvent {
            timestamp: self.unix_timestamp,
            event: StructuredLogEvent::FunctionExecution {
                source: self.event_source(None),
                error: self.params.err().cloned(),
                execution_time,
                usage_stats: log_streaming::AggregatedFunctionUsageStats {
                    database_read_bytes: self.usage_stats.database_read_bytes,
                    database_write_bytes: self.usage_stats.database_write_bytes,
                    storage_read_bytes: self.usage_stats.storage_read_bytes,
                    storage_write_bytes: self.usage_stats.storage_write_bytes,
                    vector_index_read_bytes: self.usage_stats.vector_index_read_bytes,
                    vector_index_write_bytes: self.usage_stats.vector_index_write_bytes,
                    action_memory_used_mb: self.action_memory_used_mb,
                },
            },
        }];

        if let Some(err) = self.params.err() {
            events.push(LogEvent {
                timestamp: self.unix_timestamp,
                event: StructuredLogEvent::Exception {
                    error: err.clone(),
                    user_identifier: self.identity.user_identifier().cloned(),
                    source: self.event_source(None),
                    udf_server_version: self.udf_server_version.clone(),
                },
            });
        }

        Ok(events)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionExecutionProgress {
    /// Log lines that the UDF emitted via the `Console` API.
    pub log_lines: LogLines,

    pub event_source: FunctionEventSource,
    pub function_start_timestamp: UnixTimestamp,
}

impl HeapSize for FunctionExecutionProgress {
    fn heap_size(&self) -> usize {
        self.log_lines.heap_size() + self.event_source.heap_size()
    }
}

impl FunctionExecutionProgress {
    fn console_log_events_for_log_line(
        &self,
        log_line: &LogLine,
        sub_function_path: Option<&CanonicalizedComponentFunctionPath>,
    ) -> Vec<LogEvent> {
        match log_line {
            LogLine::Structured(log_line) => {
                let mut event_source = self.event_source.clone();
                if let Some(sub_function_path) = sub_function_path {
                    event_source.component_path = sub_function_path.component.clone();
                    event_source.udf_path = sub_function_path.udf_path.to_string();
                };
                vec![LogEvent {
                    timestamp: log_line.timestamp,
                    event: StructuredLogEvent::Console {
                        source: event_source,
                        log_line: log_line.clone(),
                    },
                }]
            },
            LogLine::SubFunction { path, log_lines } => log_lines
                .iter()
                .flat_map(|log_line| self.console_log_events_for_log_line(log_line, Some(path)))
                .collect(),
        }
    }

    fn console_log_events(self) -> Vec<LogEvent> {
        self.log_lines
            .iter()
            .flat_map(|line| self.console_log_events_for_log_line(line, None))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum FunctionExecutionPart {
    Completion(FunctionExecution),
    Progress(FunctionExecutionProgress),
}

impl HeapSize for FunctionExecutionPart {
    fn heap_size(&self) -> usize {
        match self {
            FunctionExecutionPart::Completion(i) => i.heap_size(),
            FunctionExecutionPart::Progress(i) => i.heap_size(),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ActionCompletion {
    pub outcome: ValidatedActionOutcome,
    pub execution_time: Duration,
    pub environment: ModuleEnvironment,
    pub memory_in_mb: u64,
    pub context: ExecutionContext,
    pub unix_timestamp: UnixTimestamp,
    pub caller: FunctionCaller,
    pub log_lines: LogLines,
}

impl ActionCompletion {
    pub fn log_lines(&self) -> &LogLines {
        &self.log_lines
    }
}

#[derive(Debug, Clone)]
pub enum UdfParams {
    Function {
        // Avoid storing the actual result because the json can be quite large.
        // Instead only store the error if there was one. If error is None, the
        // function succeeded.
        error: Option<JsError>,
        /// Path of the component and UDF that was executed.
        identifier: CanonicalizedComponentFunctionPath,
    },
    Http {
        result: Result<HttpActionStatusCode, JsError>,
        identifier: HttpActionRoute,
    },
}

impl HeapSize for UdfParams {
    fn heap_size(&self) -> usize {
        match self {
            UdfParams::Function { error, identifier } => error.heap_size() + identifier.heap_size(),
            UdfParams::Http { result, identifier } => result.heap_size() + identifier.heap_size(),
        }
    }
}

impl UdfParams {
    pub fn is_err(&self) -> bool {
        match self {
            UdfParams::Function { ref error, .. } => error.is_some(),
            UdfParams::Http { ref result, .. } => result.is_err(),
        }
    }

    fn err(&self) -> Option<&JsError> {
        match self {
            UdfParams::Function { error: Some(e), .. } => Some(e),
            UdfParams::Http { result: Err(e), .. } => Some(e),
            _ => None,
        }
    }

    pub fn identifier_str(&self) -> String {
        match self {
            Self::Function { identifier, .. } => identifier.udf_path.clone().strip().to_string(),
            Self::Http { identifier, .. } => identifier.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpActionRequest {
    url: Url,
    method: Method,
}

impl HeapSize for HttpActionRequest {
    fn heap_size(&self) -> usize {
        self.url.as_str().len()
    }
}

impl From<HttpActionRequest> for serde_json::Value {
    fn from(value: HttpActionRequest) -> Self {
        json!({
            "url": value.url.to_string(),
            "method": value.method.to_string()
        })
    }
}

#[derive(Debug, Clone)]
pub struct HttpActionStatusCode(pub StatusCode);

impl HeapSize for HttpActionStatusCode {
    fn heap_size(&self) -> usize {
        // StatusCode is a wrapper around u16
        0
    }
}

impl From<HttpActionStatusCode> for serde_json::Value {
    fn from(value: HttpActionStatusCode) -> Self {
        json!({
            "status": value.0.as_u16().to_string(),
        })
    }
}

pub enum TrackUsage {
    Track(FunctionUsageTracker),
    // We don't count usage for system errors since they're not the user's fault
    SystemError,
}

pub enum UdfRate {
    Invocations,
    Errors,
    CacheHits,
    CacheMisses,
}

impl FromStr for UdfRate {
    type Err = anyhow::Error;

    fn from_str(r: &str) -> anyhow::Result<Self> {
        let udf_rate = match r {
            "invocations" => UdfRate::Invocations,
            "errors" => UdfRate::Errors,
            "cacheHits" => UdfRate::CacheHits,
            "cacheMisses" => UdfRate::CacheMisses,
            _ => anyhow::bail!("Invalid UDF rate: {}", r),
        };
        Ok(udf_rate)
    }
}

pub enum TableRate {
    RowsRead,
    RowsWritten,
}

impl FromStr for TableRate {
    type Err = anyhow::Error;

    fn from_str(r: &str) -> anyhow::Result<Self> {
        let table_rate = match r {
            "rowsRead" => TableRate::RowsRead,
            "rowsWritten" => TableRate::RowsWritten,
            _ => anyhow::bail!("Invalid table rate: {}", r),
        };
        Ok(table_rate)
    }
}

#[derive(Clone)]
pub struct FunctionExecutionLog<RT: Runtime> {
    inner: Arc<Mutex<Inner<RT>>>,
    usage_tracking: UsageCounter,
    rt: RT,
}

impl<RT: Runtime> FunctionExecutionLog<RT> {
    pub fn new(rt: RT, usage_tracking: UsageCounter, log_manager: Arc<dyn LogSender>) -> Self {
        let base_ts = rt.system_time();
        let inner = Inner {
            rt: rt.clone(),
            num_execution_completions: 0,
            log: WithHeapSize::default(),
            log_waiters: vec![].into(),
            log_manager,
            metrics: MetricStore::new(
                base_ts,
                MetricStoreConfig {
                    bucket_width: *knobs::UDF_METRICS_BUCKET_WIDTH,
                    max_buckets: *knobs::UDF_METRICS_MAX_BUCKETS,
                    histogram_min_duration: *knobs::UDF_METRICS_MIN_DURATION,
                    histogram_max_duration: *knobs::UDF_METRICS_MAX_DURATION,
                    histogram_significant_figures: *knobs::UDF_METRICS_SIGNIFICANT_FIGURES,
                },
            ),
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            rt,
            usage_tracking,
        }
    }

    pub fn log_query(
        &self,
        outcome: &UdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        was_cached: bool,
        execution_time: Duration,
        caller: FunctionCaller,
        usage_tracking: FunctionUsageTracker,
        context: ExecutionContext,
    ) {
        self._log_query(
            outcome,
            tables_touched,
            was_cached,
            execution_time,
            caller,
            TrackUsage::Track(usage_tracking),
            context,
        )
    }

    pub fn log_query_system_error(
        &self,
        e: &anyhow::Error,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        identity: InertIdentity,
        start: tokio::time::Instant,
        caller: FunctionCaller,
        context: ExecutionContext,
    ) -> anyhow::Result<()> {
        // TODO: We currently synthesize a `UdfOutcome` for
        // an internal system error. If we decide we want to keep internal system errors
        // in the UDF execution log, we may want to plumb through stuff like log lines.
        let outcome = UdfOutcome::from_error(
            JsError::from_error_ref(e),
            path,
            arguments,
            identity,
            self.rt.clone(),
            None,
        )?;
        self._log_query(
            &outcome,
            BTreeMap::new(),
            false,
            start.elapsed(),
            caller,
            TrackUsage::SystemError,
            context,
        );
        Ok(())
    }

    #[minitrace::trace]
    fn _log_query(
        &self,
        outcome: &UdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        was_cached: bool,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: TrackUsage,
        context: ExecutionContext,
    ) {
        let aggregated = match usage {
            TrackUsage::Track(usage_tracker) => {
                let usage_stats = usage_tracker.gather_user_stats();
                let aggregated = usage_stats.aggregate();
                self.usage_tracking.track_call(
                    UdfIdentifier::Function(outcome.path.clone()),
                    context.execution_id.clone(),
                    context.request_id.clone(),
                    if was_cached {
                        CallType::CachedQuery
                    } else {
                        CallType::UncachedQuery
                    },
                    outcome.result.is_ok(),
                    usage_stats,
                );
                aggregated
            },
            TrackUsage::SystemError => AggregatedFunctionUsageStats::default(),
        };
        if outcome.path.is_system() {
            return;
        }
        let execution = FunctionExecution {
            params: UdfParams::Function {
                error: match &outcome.result {
                    Ok(_) => None,
                    Err(e) => Some(e.clone()),
                },
                identifier: outcome.path.clone(),
            },
            unix_timestamp: self.rt.unix_timestamp(),
            execution_timestamp: outcome.unix_timestamp,
            udf_type: UdfType::Query,
            log_lines: outcome.log_lines.clone(),
            tables_touched: tables_touched.into(),
            cached_result: was_cached,
            execution_time: execution_time.as_secs_f64(),
            caller,
            environment: ModuleEnvironment::Isolate,
            syscall_trace: outcome.syscall_trace.clone(),
            usage_stats: aggregated,
            action_memory_used_mb: None,
            udf_server_version: outcome.udf_server_version.clone(),
            identity: outcome.identity.clone(),
            context,
        };
        self.log_execution(execution, true);
    }

    pub fn log_mutation(
        &self,
        outcome: ValidatedUdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: FunctionUsageTracker,
        context: ExecutionContext,
    ) {
        self._log_mutation(
            outcome,
            tables_touched,
            execution_time,
            caller,
            TrackUsage::Track(usage),
            context,
            None,
        )
    }

    pub fn log_mutation_system_error(
        &self,
        e: &anyhow::Error,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        identity: InertIdentity,
        start: tokio::time::Instant,
        caller: FunctionCaller,
        context: ExecutionContext,
    ) -> anyhow::Result<()> {
        // TODO: We currently synthesize a `UdfOutcome` for
        // an internal system error. If we decide we want to keep internal system errors
        // in the UDF execution log, we may want to plumb through stuff like log lines.
        let outcome = ValidatedUdfOutcome::from_error(
            JsError::from_error_ref(e),
            path,
            arguments,
            identity,
            self.rt.clone(),
            None,
        )?;
        self._log_mutation(
            outcome,
            BTreeMap::new(),
            start.elapsed(),
            caller,
            TrackUsage::SystemError,
            context,
            None,
        );
        Ok(())
    }

    pub fn log_mutation_occ_error(
        &self,
        outcome: ValidatedUdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: FunctionUsageTracker,
        context: ExecutionContext,
        occ_info: OccInfo,
    ) {
        self._log_mutation(
            outcome,
            tables_touched,
            execution_time,
            caller,
            TrackUsage::Track(usage),
            context,
            Some(occ_info),
        );
    }

    fn _log_mutation(
        &self,
        outcome: ValidatedUdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: TrackUsage,
        context: ExecutionContext,
        occ_info: Option<OccInfo>,
    ) {
        let aggregated = match usage {
            TrackUsage::Track(usage_tracker) => {
                let usage_stats = usage_tracker.gather_user_stats();
                let aggregated = usage_stats.aggregate();
                self.usage_tracking.track_call(
                    UdfIdentifier::Function(outcome.path.clone()),
                    context.execution_id.clone(),
                    context.request_id.clone(),
                    CallType::Mutation { occ_info },
                    outcome.result.is_ok(),
                    usage_stats,
                );
                aggregated
            },
            TrackUsage::SystemError => AggregatedFunctionUsageStats::default(),
        };
        if outcome.path.udf_path.is_system() {
            return;
        }
        let execution = FunctionExecution {
            params: UdfParams::Function {
                error: match outcome.result {
                    Ok(_) => None,
                    Err(e) => Some(e),
                },
                identifier: outcome.path,
            },
            unix_timestamp: self.rt.unix_timestamp(),
            execution_timestamp: outcome.unix_timestamp,
            udf_type: UdfType::Mutation,
            log_lines: outcome.log_lines,
            tables_touched: tables_touched.into(),
            cached_result: false,
            execution_time: execution_time.as_secs_f64(),
            caller,
            environment: ModuleEnvironment::Isolate,
            syscall_trace: outcome.syscall_trace,
            usage_stats: aggregated,
            action_memory_used_mb: None,
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context,
        };
        self.log_execution(execution, true);
    }

    pub fn log_action(&self, completion: ActionCompletion, usage: FunctionUsageTracker) {
        self._log_action(completion, TrackUsage::Track(usage))
    }

    pub fn log_action_system_error(
        &self,
        e: &anyhow::Error,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        identity: InertIdentity,
        start: tokio::time::Instant,
        caller: FunctionCaller,
        log_lines: LogLines,
        context: ExecutionContext,
    ) -> anyhow::Result<()> {
        // Synthesize an `ActionCompletion` for system errors.
        let unix_timestamp = self.rt.unix_timestamp();
        let completion = ActionCompletion {
            outcome: ValidatedActionOutcome::from_system_error(
                path,
                arguments,
                identity,
                unix_timestamp,
                e,
            ),
            execution_time: start.elapsed(),
            environment: ModuleEnvironment::Invalid,
            memory_in_mb: 0,
            context,
            unix_timestamp: self.rt.unix_timestamp(),
            caller,
            log_lines,
        };
        self._log_action(completion, TrackUsage::SystemError);
        Ok(())
    }

    fn _log_action(&self, completion: ActionCompletion, usage: TrackUsage) {
        let outcome = completion.outcome;
        let log_lines = completion.log_lines;
        let aggregated = match usage {
            TrackUsage::Track(usage_tracker) => {
                let usage_stats = usage_tracker.gather_user_stats();
                let aggregated = usage_stats.aggregate();
                self.usage_tracking.track_call(
                    UdfIdentifier::Function(outcome.path.clone()),
                    completion.context.execution_id.clone(),
                    completion.context.request_id.clone(),
                    CallType::Action {
                        env: completion.environment,
                        duration: completion.execution_time,
                        memory_in_mb: completion.memory_in_mb,
                    },
                    outcome.result.is_ok(),
                    usage_stats,
                );
                aggregated
            },
            TrackUsage::SystemError => AggregatedFunctionUsageStats::default(),
        };
        if outcome.path.udf_path.is_system() {
            return;
        }
        let execution = FunctionExecution {
            params: UdfParams::Function {
                error: match outcome.result {
                    Ok(_) => None,
                    Err(e) => Some(e),
                },
                identifier: outcome.path,
            },
            unix_timestamp: self.rt.unix_timestamp(),
            execution_timestamp: outcome.unix_timestamp,
            udf_type: UdfType::Action,
            log_lines,
            tables_touched: WithHeapSize::default(),
            cached_result: false,
            execution_time: completion.execution_time.as_secs_f64(),
            caller: completion.caller,
            environment: completion.environment,
            syscall_trace: outcome.syscall_trace,
            usage_stats: aggregated,
            action_memory_used_mb: Some(completion.memory_in_mb),
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context: completion.context,
        };
        self.log_execution(execution, /* send_console_events */ false)
    }

    pub fn log_action_progress(
        &self,
        path: CanonicalizedComponentFunctionPath,
        unix_timestamp: UnixTimestamp,
        context: ExecutionContext,
        log_lines: LogLines,
        module_environment: ModuleEnvironment,
    ) {
        if path.is_system() {
            return;
        }
        let event_source = FunctionEventSource {
            component_path: path.component,
            udf_path: path.udf_path.strip().to_string(),
            udf_type: UdfType::Action,
            module_environment,
            cached: Some(false),
            context,
        };

        self.log_execution_progress(log_lines, event_source, unix_timestamp)
    }

    pub fn log_http_action(
        &self,
        outcome: HttpActionOutcome,
        result: Result<HttpActionStatusCode, JsError>,
        log_lines: LogLines,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: FunctionUsageTracker,
        context: ExecutionContext,
        response_sha256: Sha256Digest,
    ) {
        self._log_http_action(
            outcome,
            result,
            log_lines,
            execution_time,
            caller,
            TrackUsage::Track(usage),
            context,
            response_sha256,
        )
    }

    pub fn log_http_action_system_error(
        &self,
        error: &anyhow::Error,
        http_request: HttpActionRequestHead,
        identity: InertIdentity,
        start: tokio::time::Instant,
        caller: FunctionCaller,
        log_lines: LogLines,
        context: ExecutionContext,
        response_sha256: Sha256Digest,
    ) {
        let js_err = JsError::from_error_ref(error);
        let outcome = HttpActionOutcome::new(
            None,
            http_request,
            identity,
            self.rt.unix_timestamp(),
            isolate::HttpActionResult::Error(js_err.clone()),
            None,
            None,
        );
        self._log_http_action(
            outcome,
            Err(js_err),
            log_lines,
            start.elapsed(),
            caller,
            TrackUsage::SystemError,
            context,
            response_sha256,
        )
    }

    fn _log_http_action(
        &self,
        outcome: HttpActionOutcome,
        result: Result<HttpActionStatusCode, JsError>,
        log_lines: LogLines,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: TrackUsage,
        context: ExecutionContext,
        response_sha256: Sha256Digest,
    ) {
        let aggregated = match usage {
            TrackUsage::Track(usage_tracker) => {
                let usage_stats = usage_tracker.gather_user_stats();
                let aggregated = usage_stats.aggregate();
                self.usage_tracking.track_call(
                    UdfIdentifier::Http(outcome.route.clone()),
                    context.execution_id.clone(),
                    context.request_id.clone(),
                    CallType::HttpAction {
                        duration: execution_time,
                        memory_in_mb: outcome.memory_in_mb(),
                        response_sha256,
                    },
                    result.clone().is_ok_and(|code| code.0.as_u16() < 400),
                    usage_stats,
                );
                aggregated
            },
            TrackUsage::SystemError => AggregatedFunctionUsageStats::default(),
        };
        let execution = FunctionExecution {
            params: UdfParams::Http {
                result,
                identifier: outcome.route.clone(),
            },
            unix_timestamp: self.rt.unix_timestamp(),
            execution_timestamp: outcome.unix_timestamp,
            udf_type: UdfType::HttpAction,
            log_lines,
            tables_touched: WithHeapSize::default(),
            cached_result: false,
            execution_time: execution_time.as_secs_f64(),
            caller,
            environment: ModuleEnvironment::Isolate,
            usage_stats: aggregated,
            action_memory_used_mb: Some(outcome.memory_in_mb()),
            syscall_trace: outcome.syscall_trace,
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context,
        };
        self.log_execution(execution, /* send_console_events */ false);
    }

    pub fn log_http_action_progress(
        &self,
        identifier: HttpActionRoute,
        unix_timestamp: UnixTimestamp,
        context: ExecutionContext,
        log_lines: LogLines,
        module_environment: ModuleEnvironment,
    ) {
        let event_source = FunctionEventSource {
            // TODO(ENG-7612): Support HTTP actions in components.
            component_path: ComponentPath::root(),
            udf_path: identifier.to_string(),
            udf_type: UdfType::HttpAction,
            module_environment,
            cached: Some(false),
            context,
        };

        self.log_execution_progress(log_lines, event_source, unix_timestamp)
    }

    fn log_execution(&self, execution: FunctionExecution, send_console_events: bool) {
        if let Err(mut e) = self
            .inner
            .lock()
            .log_execution(execution, send_console_events)
        {
            report_error_sync(&mut e);
        }
    }

    fn log_execution_progress(
        &self,
        log_lines: LogLines,
        event_source: FunctionEventSource,
        timestamp: UnixTimestamp,
    ) {
        if let Err(mut e) =
            self.inner
                .lock()
                .log_execution_progress(log_lines, event_source, timestamp)
        {
            report_error_sync(&mut e);
        }
    }

    pub fn udf_rate(
        &self,
        identifier: UdfIdentifier,
        metric: UdfRate,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        let metrics = {
            let inner = self.inner.lock();
            inner.metrics.clone()
        };
        let name = match metric {
            UdfRate::Invocations => udf_invocations_metric(&identifier),
            UdfRate::Errors => udf_errors_metric(&identifier),
            UdfRate::CacheHits => udf_cache_hits_metric(&identifier),
            UdfRate::CacheMisses => udf_cache_misses_metric(&identifier),
        };
        let buckets = metrics
            .query_counter(&name, window.start..window.end)?
            .into_iter()
            .map(|bucket| (bucket.index, bucket.value))
            .collect();
        window.resample_counters(&metrics, buckets, true)
    }

    pub fn cache_hit_percentage(
        &self,
        identifier: UdfIdentifier,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        let metrics = {
            let inner = self.inner.lock();
            inner.metrics.clone()
        };
        let hits = metrics.query_counter(
            &udf_cache_hits_metric(&identifier),
            window.start..window.end,
        )?;
        let misses = metrics.query_counter(
            &udf_cache_misses_metric(&identifier),
            window.start..window.end,
        )?;

        // Merge the two timeseries by index, computing the hit percentage for each
        // bucket.
        let mut hits_iter = hits.into_iter().peekable();
        let mut misses_iter = misses.into_iter().peekable();
        let mut result = Vec::new();

        loop {
            let ordering = match (hits_iter.peek(), misses_iter.peek()) {
                (Some(hit), Some(miss)) => hit.index.cmp(&miss.index),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => break,
            };
            match ordering {
                Ordering::Less => {
                    let hit = hits_iter.next().unwrap();
                    result.push((hit.index, 100.));
                },
                Ordering::Equal => {
                    let hit = hits_iter.next().unwrap();
                    let miss = misses_iter.next().unwrap();
                    result.push((hit.index, hit.value / (hit.value + miss.value) * 100.));
                },
                Ordering::Greater => {
                    let miss = misses_iter.next().unwrap();
                    result.push((miss.index, 0.));
                },
            }
        }

        window.resample_counters(&metrics, result, false)
    }

    pub fn latency_percentiles(
        &self,
        identifier: UdfIdentifier,
        percentiles: Vec<Percentile>,
        window: MetricsWindow,
    ) -> anyhow::Result<BTreeMap<Percentile, Timeseries>> {
        let metrics = {
            let inner = self.inner.lock();
            inner.metrics.clone()
        };
        let buckets = metrics.query_histogram(
            &udf_execution_time_metric(&identifier),
            window.start..window.end,
        )?;
        window.resample_histograms(&metrics, buckets, &percentiles)
    }

    pub fn table_rate(
        &self,
        table_name: TableName,
        metric: TableRate,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        let metrics = {
            let inner = self.inner.lock();
            inner.metrics.clone()
        };
        let name = match metric {
            TableRate::RowsRead => table_rows_read_metric(&table_name),
            TableRate::RowsWritten => table_rows_written_metric(&table_name),
        };
        let buckets = metrics
            .query_counter(&name, window.start..window.end)?
            .into_iter()
            .map(|bucket| (bucket.index, bucket.value))
            .collect();
        window.resample_counters(&metrics, buckets, true)
    }

    pub fn udf_summary(
        &self,
        cursor: Option<CursorMs>,
    ) -> (Option<UdfMetricSummary>, Option<CursorMs>) {
        let inner = self.inner.lock();
        let new_cursor = inner.log.back().map(|(ts, _)| *ts);

        let first_entry_ix = inner.log.partition_point(|(ts, _)| Some(*ts) <= cursor);
        if first_entry_ix >= inner.log.len() {
            return (None, new_cursor);
        }
        let mut summary = UdfMetricSummary::default();
        for i in first_entry_ix..inner.log.len() {
            let (_, entry) = &inner.log[i];
            let FunctionExecutionPart::Completion(entry) = entry else {
                continue;
            };
            let function_summary = summary
                .function_calls
                .entry(entry.caller.clone())
                .or_default()
                .entry(entry.udf_type)
                .or_default()
                .entry(entry.environment)
                .or_default();
            let error_count = if entry.params.is_err() { 1 } else { 0 };
            let entry_duration = Duration::from_secs_f64(entry.execution_time);

            function_summary.invocations += 1;
            function_summary.errors += error_count;
            function_summary.execution_time += entry_duration;
            function_summary.syscalls.merge(&entry.syscall_trace);

            summary.invocations += 1;
            summary.errors += error_count;
            summary.execution_time += entry_duration;
        }

        (Some(summary), new_cursor)
    }

    pub async fn stream(&self, cursor: CursorMs) -> (Vec<FunctionExecution>, CursorMs) {
        loop {
            let rx = {
                let mut inner = self.inner.lock();
                let first_entry_ix = inner.log.partition_point(|(ts, _)| *ts <= cursor);
                if first_entry_ix < inner.log.len() {
                    let entries = (first_entry_ix..inner.log.len())
                        .map(|i| &inner.log[i])
                        .filter_map(|(_, entry)| match entry {
                            FunctionExecutionPart::Completion(completion) => {
                                Some(completion.clone())
                            },
                            _ => None,
                        })
                        .collect();
                    let (new_cursor, _) = inner.log.back().unwrap();
                    return (entries, *new_cursor);
                }
                let (tx, rx) = oneshot::channel();
                inner.log_waiters.push(tx);
                rx
            };
            let _ = rx.await;
        }
    }

    pub async fn stream_parts(&self, cursor: CursorMs) -> (Vec<FunctionExecutionPart>, CursorMs) {
        loop {
            let rx = {
                let mut inner = self.inner.lock();
                let first_entry_ix = inner.log.partition_point(|(ts, _)| *ts <= cursor);
                if first_entry_ix < inner.log.len() {
                    let entries = (first_entry_ix..inner.log.len())
                        .map(|i| &inner.log[i])
                        .map(|(_, entry)| match entry {
                            FunctionExecutionPart::Completion(c) => {
                                let with_stripped_log_lines = match c.udf_type {
                                    UdfType::Query | UdfType::Mutation => c.clone(),
                                    UdfType::Action | UdfType::HttpAction => {
                                        let mut cloned = c.clone();
                                        cloned.log_lines = vec![].into();
                                        cloned
                                    },
                                };
                                FunctionExecutionPart::Completion(with_stripped_log_lines)
                            },
                            FunctionExecutionPart::Progress(c) => {
                                FunctionExecutionPart::Progress(c.clone())
                            },
                        })
                        .collect();
                    let (new_cursor, _) = inner.log.back().unwrap();
                    return (entries, *new_cursor);
                }
                let (tx, rx) = oneshot::channel();
                inner.log_waiters.push(tx);
                rx
            };
            let _ = rx.await;
        }
    }

    pub fn latest_cursor(&self) -> CursorMs {
        let inner = self.inner.lock();
        if let Some((new_cursor, _)) = inner.log.back() {
            *new_cursor
        } else {
            0.0
        }
    }
}

struct Inner<RT: Runtime> {
    rt: RT,

    log: WithHeapSize<VecDeque<(CursorMs, FunctionExecutionPart)>>,
    num_execution_completions: usize,
    log_waiters: WithHeapSize<Vec<oneshot::Sender<()>>>,
    log_manager: Arc<dyn LogSender>,
    metrics: MetricStore,
}

impl<RT: Runtime> Inner<RT> {
    fn log_execution(
        &mut self,
        execution: FunctionExecution,
        send_console_events: bool,
    ) -> anyhow::Result<()> {
        if let Err(e) = self.log_execution_metrics(&execution) {
            Self::log_metrics_error(e);
        };
        let next_time = self.next_time()?;

        // Gather log lines
        let mut log_events = if send_console_events {
            execution.console_log_events()
        } else {
            vec![]
        };
        // Gather UDF execution record
        match execution.udf_execution_record_log_events() {
            Ok(records) => log_events.extend(records),
            Err(mut e) => {
                // Don't let failing to construct the UDF execution record block sending
                // the other log events
                tracing::error!("failed to create UDF execution record: {}", e);
                report_error_sync(&mut e);
            },
        }

        self.log_manager.send_logs(log_events);

        self.log
            .push_back((next_time, FunctionExecutionPart::Completion(execution)));
        self.num_execution_completions += 1;
        while self.num_execution_completions > *knobs::MAX_UDF_EXECUTION {
            let front = self.log.pop_front();
            if let Some((_, FunctionExecutionPart::Completion(_))) = front {
                self.num_execution_completions -= 1;
            }
        }
        for waiter in self.log_waiters.drain(..) {
            let _ = waiter.send(());
        }
        Ok(())
    }

    fn log_metrics_error(error: UdfMetricsError) {
        // Only log an error to tracing and/or Sentry at most once every 10 seconds per
        // thread.
        thread_local! {
            static LAST_LOGGED_ERROR: Cell<Option<SystemTime>> = const { Cell::new(None) };
        }
        let now = SystemTime::now();
        let should_log = LAST_LOGGED_ERROR.get().map_or(true, |last_logged| {
            now.duration_since(last_logged).unwrap_or(Duration::ZERO) >= Duration::from_secs(10)
        });
        if !should_log {
            return;
        }
        LAST_LOGGED_ERROR.set(Some(now));
        tracing::error!("Failed to log execution metrics: {}", error);
        if let UdfMetricsError::InternalError(mut e) = error {
            report_error_sync(&mut e);
        }
    }

    fn log_execution_metrics(
        &mut self,
        execution: &FunctionExecution,
    ) -> Result<(), UdfMetricsError> {
        let ts = execution.unix_timestamp.as_system_time();

        let identifier = execution.identifier();

        let name = udf_invocations_metric(&identifier);
        self.metrics.add_counter(&name, ts, 1.0)?;

        let is_err = match &execution.params {
            UdfParams::Function { error, .. } => error.is_some(),
            UdfParams::Http { result, .. } => result.is_err(),
        };
        if is_err {
            let name = udf_errors_metric(&identifier);
            self.metrics.add_counter(&name, ts, 1.0)?;
        }
        if execution.cached_result {
            let name = udf_cache_hits_metric(&identifier);
            self.metrics.add_counter(&name, ts, 1.0)?;
        } else {
            let name = udf_cache_misses_metric(&identifier);
            self.metrics.add_counter(&name, ts, 1.0)?;
        }

        let name = udf_execution_time_metric(&identifier);
        self.metrics
            .add_histogram(&name, ts, Duration::from_secs_f64(execution.execution_time))?;

        for (table_name, table_stats) in &execution.tables_touched {
            let name = table_rows_read_metric(table_name);
            self.metrics
                .add_counter(&name, ts, table_stats.rows_read as f32)?;
            let name = table_rows_written_metric(table_name);
            self.metrics
                .add_counter(&name, ts, table_stats.rows_written as f32)?;
        }
        Ok(())
    }

    fn log_execution_progress(
        &mut self,
        log_lines: LogLines,
        event_source: FunctionEventSource,
        function_start_timestamp: UnixTimestamp,
    ) -> anyhow::Result<()> {
        let next_time = self.next_time()?;
        let progress = FunctionExecutionProgress {
            log_lines,
            event_source,
            function_start_timestamp,
        };

        let log_events = progress.clone().console_log_events();
        self.log_manager.send_logs(log_events);
        self.log
            .push_back((next_time, FunctionExecutionPart::Progress(progress)));
        for waiter in self.log_waiters.drain(..) {
            let _ = waiter.send(());
        }
        Ok(())
    }

    fn next_time(&self) -> anyhow::Result<CursorMs> {
        let since_epoch = self
            .rt
            .system_time()
            .duration_since(SystemTime::UNIX_EPOCH)?;
        let mut next_time =
            (since_epoch.as_secs() as f64 * 1e3) + (since_epoch.subsec_nanos() as f64 * 1e-6);
        if let Some((last_time, _)) = self.log.back() {
            let lower_bound = last_time.next_after(f64::INFINITY);
            if lower_bound > next_time {
                next_time = lower_bound;
            }
        }
        Ok(next_time)
    }
}

#[derive(Default)]
pub struct UdfMetricSummary {
    // Aggregated metrics for backwards compatibility.
    pub invocations: u32,
    pub errors: u32,
    pub execution_time: Duration,

    pub function_calls:
        BTreeMap<FunctionCaller, BTreeMap<UdfType, BTreeMap<ModuleEnvironment, FunctionSummary>>>,
}

impl From<UdfMetricSummary> for JsonValue {
    fn from(value: UdfMetricSummary) -> Self {
        json!({
            "invocations": value.invocations,
            "errors": value.errors,
            "executionTime": value.execution_time.as_secs_f64(),

            "functionCalls": value
                .function_calls
                .into_iter()
                .map(|(caller, v)| {
                    let map1 = v.into_iter()
                        .map(|(udf_type, v)| {
                            let map2 = v.into_iter()
                                .map(|(environment, summary)| {
                                    let key = environment.to_string();
                                    let value = JsonValue::from(summary);
                                    (key, value)
                                })
                                .collect::<serde_json::Map<_, _>>();
                            (format!("{udf_type}"), JsonValue::Object(map2))
                        })
                        .collect::<serde_json::Map<_, _>>();
                    (format!("{caller}"), JsonValue::Object(map1))
                })
                .collect::<serde_json::Map<_, _>>(),
        })
    }
}

#[derive(Default)]
pub struct FunctionSummary {
    pub invocations: u32,
    pub errors: u32,
    pub execution_time: Duration,
    pub syscalls: SyscallTrace,
}

impl From<FunctionSummary> for JsonValue {
    fn from(value: FunctionSummary) -> Self {
        json!({
            "invocations": value.invocations,
            "errors": value.errors,
            "executionTime": value.execution_time.as_secs_f64(),
            "syscalls": JsonValue::from(value.syscalls),
        })
    }
}

fn udf_invocations_metric(identifier: &UdfIdentifier) -> MetricName {
    format!("udf:{}:invocations", identifier)
}

fn udf_errors_metric(identifier: &UdfIdentifier) -> MetricName {
    format!("udf:{}:errors", identifier)
}

fn udf_cache_hits_metric(identifier: &UdfIdentifier) -> MetricName {
    format!("udf:{}:cache_hits", identifier)
}

fn udf_cache_misses_metric(identifier: &UdfIdentifier) -> MetricName {
    format!("udf:{}:cache_misses", identifier)
}

fn udf_execution_time_metric(identifier: &UdfIdentifier) -> MetricName {
    format!("udf:{}:execution_time", identifier)
}

fn table_rows_read_metric(table_name: &TableName) -> MetricName {
    format!("table:{}:rows_read", table_name)
}

fn table_rows_written_metric(table_name: &TableName) -> MetricName {
    format!("table:{}:rows_written", table_name)
}
