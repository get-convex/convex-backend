use std::{
    cmp,
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
    errors::{
        report_error,
        JsError,
    },
    identity::InertIdentity,
    knobs::MAX_UDF_EXECUTION,
    log_lines::LogLines,
    log_streaming::{
        EventSource,
        FunctionEventSource,
        LogEvent,
        LogSender,
        LogTopic,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        CursorMs,
        FunctionCaller,
        HttpActionRoute,
        TableName,
        TableStats,
        UdfIdentifier,
        UdfType,
    },
};
use float_next_after::NextAfter;
use futures::channel::oneshot;
use http::{
    Method,
    StatusCode,
};
use isolate::{
    ActionOutcome,
    HttpActionOutcome,
    SyscallTrace,
    UdfOutcome,
};
use model::config::types::ModuleEnvironment;
use parking_lot::Mutex;
use request_context::RequestContext;
use serde::Deserialize;
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::CanonicalizedUdfPath;
use url::Url;
use usage_tracking::{
    AggregatedFunctionUsageStats,
    CallType,
    FunctionUsageTracker,
    UsageCounter,
};
use value::heap_size::{
    HeapSize,
    WithHeapSize,
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

    /// The Convex NPM package version pushed with the module version executed.
    pub udf_server_version: Option<semver::Version>,

    /// The identity under which the udf was executed. Must be inert - since it
    /// can only be for logging purposes - not giving any authorization
    /// power.
    pub identity: InertIdentity,

    pub context: RequestContext,
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
    /// Helper method to construct UDF execution for errors that occurred before
    /// execution and thus have no associated runtime information.
    pub fn for_error(
        udf_path: CanonicalizedUdfPath,
        udf_type: UdfType,
        unix_timestamp: UnixTimestamp,
        error: String,
        caller: FunctionCaller,
        udf_server_version: Option<semver::Version>,
        identity: InertIdentity,
        context: RequestContext,
    ) -> Self {
        FunctionExecution {
            params: UdfParams::Function {
                error: Some(JsError::from_message(error)),
                identifier: udf_path,
            },
            unix_timestamp,
            execution_timestamp: unix_timestamp,
            udf_type,
            log_lines: vec![].into(),
            tables_touched: WithHeapSize::default(),
            cached_result: false,
            execution_time: 0.0,
            caller,
            environment: ModuleEnvironment::Invalid,
            syscall_trace: SyscallTrace::new(),
            usage_stats: AggregatedFunctionUsageStats::default(),
            udf_server_version,
            identity,
            context,
        }
    }

    fn identifier(&self) -> UdfIdentifier {
        match &self.params {
            UdfParams::Function { identifier, .. } => UdfIdentifier::Function(identifier.clone()),
            UdfParams::Http { identifier, .. } => UdfIdentifier::Http(identifier.clone()),
        }
    }

    fn event_source(&self) -> FunctionEventSource {
        let udf_id = self.params.identifier_str();
        let cached = if self.udf_type == UdfType::Query {
            Some(self.cached_result)
        } else {
            None
        };

        FunctionEventSource {
            path: udf_id,
            udf_type: self.udf_type,
            cached,
            context: self.context.clone(),
        }
    }

    fn console_log_events(&self) -> Vec<LogEvent> {
        self.log_lines
            .iter()
            .map(|line| {
                let JsonValue::Object(payload) = json!({
                    "message": line.clone().to_pretty_string()
                }) else {
                    anyhow::bail!("could not create JSON object");
                };
                Ok::<_, anyhow::Error>(LogEvent {
                    topic: LogTopic::Console,
                    source: EventSource::Function(self.event_source()),
                    payload,
                    timestamp: self.unix_timestamp,
                })
            })
            .filter_map(|event| match event {
                Err(mut e) => {
                    tracing::error!("Dropping log event due to failed payload conversion: {e:?}");
                    report_error(&mut e);
                    None
                },
                Ok(ev) => Some(ev),
            })
            .collect()
    }

    fn udf_execution_record_log_events(&self) -> anyhow::Result<Vec<LogEvent>> {
        let execution_time: u64 = Duration::from_secs_f64(self.execution_time)
            .as_millis()
            .try_into()?;
        let (reason, status) = match self.params.err() {
            Some(err) => (json!(err.to_string()), "failure"),
            None => (JsonValue::Null, "success"),
        };

        let JsonValue::Object(payload) = json!({
            "status": status,
            "reason": reason,
            "executionTimeMs": execution_time,
            "databaseReadBytes": self.usage_stats.database_read_bytes,
            "databaseWriteBytes": self.usage_stats.database_write_bytes,
            "storageReadBytes": self.usage_stats.storage_read_bytes,
            "storageWriteBytes": self.usage_stats.storage_write_bytes,
        }) else {
            anyhow::bail!("could not create JSON object for UdfExecutionRecord");
        };

        let mut events = vec![LogEvent {
            topic: LogTopic::UdfExecutionRecord,
            timestamp: self.unix_timestamp,
            source: EventSource::Function(self.event_source()),
            payload,
        }];

        if let Some(err) = self.params.err() {
            events.push(LogEvent::construct_exception(
                err,
                self.unix_timestamp,
                EventSource::Function(self.event_source()),
                self.udf_server_version
                    .as_ref()
                    .map(|v| v.to_string())
                    .as_deref(),
                &self.identity,
            )?);
        }

        Ok(events)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionExecutionProgress {
    /// Log lines that the UDF emitted via the `Console` API.
    pub log_lines: LogLines,

    pub event_source: FunctionEventSource,
}

impl FunctionExecutionProgress {
    fn console_log_events(self, unix_timestamp: UnixTimestamp) -> Vec<LogEvent> {
        self.log_lines
            .into_iter()
            .map(|line| {
                let JsonValue::Object(payload) = json!({
                    "message": line.to_pretty_string()
                }) else {
                    anyhow::bail!("could not create JSON object");
                };
                Ok::<_, anyhow::Error>(LogEvent {
                    topic: LogTopic::Console,
                    source: EventSource::Function(self.event_source.clone()),
                    payload,
                    timestamp: unix_timestamp,
                })
            })
            .filter_map(|event| match event {
                Err(mut e) => {
                    tracing::error!("Dropping log event due to failed payload conversion: {e:?}");
                    report_error(&mut e);
                    None
                },
                Ok(ev) => Some(ev),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ActionCompletion {
    pub outcome: ActionOutcome,
    pub execution_time: Duration,
    pub environment: ModuleEnvironment,
    pub memory_in_mb: u64,
    pub context: RequestContext,
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
        /// Path of the UDF that was executed.
        identifier: CanonicalizedUdfPath,
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
            Self::Function { identifier, .. } => identifier.clone().strip().to_string(),
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
pub struct HttpActionStatusCode(StatusCode);

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

pub type Timeseries = Vec<(SystemTime, Option<f64>)>;

/// Integer in [0, 100].
pub type Percentile = usize;

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

#[derive(Debug)]
pub struct MetricsWindow {
    start: SystemTime,
    end: SystemTime,
    num_buckets: usize,
}

impl TryFrom<serde_json::Value> for MetricsWindow {
    type Error = anyhow::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        #[derive(Debug, Deserialize)]
        struct MetricsWindowInner {
            start: SystemTime,
            end: SystemTime,
            num_buckets: usize,
        }
        let parsed: MetricsWindowInner = serde_json::from_value(value)?;
        if parsed.end < parsed.start {
            anyhow::bail!(
                "Invalid query window: {:?} < {:?}",
                parsed.end,
                parsed.start
            );
        }
        if parsed.num_buckets == 0 {
            anyhow::bail!("Invalid query num_buckets: 0");
        }
        Ok(Self {
            start: parsed.start,
            end: parsed.end,
            num_buckets: parsed.num_buckets,
        })
    }
}

#[derive(Clone)]
pub struct FunctionExecutionLog<RT: Runtime> {
    inner: Arc<Mutex<Inner<RT>>>,
    usage_tracking: UsageCounter,
    rt: RT,
}

impl<RT: Runtime> HeapSize for FunctionExecutionLog<RT> {
    fn heap_size(&self) -> usize {
        self.inner.lock().heap_size() + self.usage_tracking.heap_size()
    }
}

impl<RT: Runtime> FunctionExecutionLog<RT> {
    pub fn new(rt: RT, usage_tracking: UsageCounter, log_manager: Arc<dyn LogSender>) -> Self {
        let inner = Inner {
            rt: rt.clone(),
            log: WithHeapSize::default(),
            log_waiters: vec![].into(),
            log_manager,
            metrics: Metrics::default(),
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            rt,
            usage_tracking,
        }
    }

    pub fn log_query(
        &self,
        outcome: UdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        was_cached: bool,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: FunctionUsageTracker,
        context: RequestContext,
    ) {
        let usage_stats = usage.gather_user_stats();
        let aggregated = usage_stats.aggregate();
        self.usage_tracking.track_call(
            UdfIdentifier::Function(outcome.udf_path.clone()),
            context.execution_id.clone(),
            if was_cached {
                CallType::CachedQuery
            } else {
                CallType::UncachedQuery
            },
            usage_stats,
        );
        if outcome.udf_path.is_system() {
            return;
        }
        let execution = FunctionExecution {
            params: UdfParams::Function {
                error: match outcome.result {
                    Ok(_) => None,
                    Err(e) => Some(e),
                },
                identifier: outcome.udf_path.clone(),
            },
            unix_timestamp: self.rt.unix_timestamp(),
            execution_timestamp: outcome.unix_timestamp,
            udf_type: UdfType::Query,
            log_lines: outcome.log_lines,
            tables_touched: tables_touched.into(),
            cached_result: was_cached,
            execution_time: execution_time.as_secs_f64(),
            caller,
            environment: ModuleEnvironment::Isolate,
            syscall_trace: outcome.syscall_trace,
            usage_stats: aggregated,
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context,
        };
        self.log_execution(execution, true);
    }

    // Set exclude_from_usage=true for any cases where we hit internal system errors
    // We still want these logs to show up in the UDF execution logs but not get
    // counted in user quota used for charging since it's not their fault.
    pub fn log_mutation(
        &self,
        outcome: UdfOutcome,
        tables_touched: BTreeMap<TableName, TableStats>,
        execution_time: Duration,
        caller: FunctionCaller,
        exclude_call_from_usage_tracking: bool,
        usage: FunctionUsageTracker,
        context: RequestContext,
    ) {
        let usage_stats = usage.gather_user_stats();
        let aggregated = usage_stats.aggregate();
        if !exclude_call_from_usage_tracking {
            self.usage_tracking.track_call(
                UdfIdentifier::Function(outcome.udf_path.clone()),
                context.execution_id.clone(),
                CallType::Mutation,
                usage_stats,
            );
        }
        if outcome.udf_path.is_system() {
            return;
        }
        let execution = FunctionExecution {
            params: UdfParams::Function {
                error: match outcome.result {
                    Ok(_) => None,
                    Err(e) => Some(e),
                },
                identifier: outcome.udf_path.clone(),
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
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context,
        };
        self.log_execution(execution, true);
    }

    pub fn log_http_action(
        &self,
        outcome: HttpActionOutcome,
        log_lines: LogLines,
        execution_time: Duration,
        caller: FunctionCaller,
        usage: FunctionUsageTracker,
        context: RequestContext,
    ) {
        let usage_stats = usage.gather_user_stats();
        let aggregated = usage_stats.aggregate();
        self.usage_tracking.track_call(
            UdfIdentifier::Http(outcome.route.clone()),
            context.execution_id.clone(),
            CallType::HttpAction {
                duration: execution_time,
                memory_in_mb: outcome.memory_in_mb,
            },
            usage_stats,
        );
        let execution = FunctionExecution {
            params: UdfParams::Http {
                result: match outcome.result {
                    Ok(v) => Ok(HttpActionStatusCode(v.status())),
                    Err(e) => Err(e),
                },
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
            syscall_trace: outcome.syscall_trace,
            usage_stats: aggregated,
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context,
        };
        self.log_execution(execution, /* send_console_events */ false);
    }

    // Set exclude_from_usage=true for any cases where we hit internal system errors
    // We still want these logs to show up in the UDF execution logs but not get
    // counted in user quota used for charging since it's not their fault.
    pub fn log_action(
        &self,
        completion: ActionCompletion,
        exclude_call_from_usage_tracking: bool,
        usage: FunctionUsageTracker,
    ) {
        let usage_stats = usage.gather_user_stats();
        let aggregated = usage_stats.aggregate();
        let udf_path = completion.outcome.udf_path.clone();
        if !exclude_call_from_usage_tracking {
            self.usage_tracking.track_call(
                UdfIdentifier::Function(udf_path.clone()),
                completion.context.execution_id.clone(),
                CallType::Action {
                    env: completion.environment.into(),
                    duration: completion.execution_time,
                    memory_in_mb: completion.memory_in_mb,
                },
                usage_stats,
            );
        }
        if udf_path.is_system() {
            return;
        }
        let outcome = completion.outcome;
        let log_lines = completion.log_lines;

        let execution = FunctionExecution {
            params: UdfParams::Function {
                error: match outcome.result {
                    Ok(_) => None,
                    Err(e) => Some(e),
                },
                identifier: outcome.udf_path.clone(),
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
            udf_server_version: outcome.udf_server_version,
            identity: outcome.identity,
            context: completion.context,
        };
        self.log_execution(execution, /* send_console_events */ false)
    }

    pub fn log_error(
        &self,
        udf_path: CanonicalizedUdfPath,
        udf_type: UdfType,
        unix_timestamp: UnixTimestamp,
        error: String,
        caller: FunctionCaller,
        udf_server_version: Option<semver::Version>,
        identity: InertIdentity,
        context: RequestContext,
    ) {
        let execution = FunctionExecution::for_error(
            udf_path,
            udf_type,
            unix_timestamp,
            error,
            caller,
            udf_server_version,
            identity,
            context,
        );
        self.log_execution(execution, true)
    }

    pub fn log_action_progress(
        &self,
        identifier: CanonicalizedUdfPath,
        unix_timestamp: UnixTimestamp,
        context: RequestContext,
        log_lines: LogLines,
    ) {
        if identifier.is_system() {
            return;
        }
        let event_source = FunctionEventSource {
            path: identifier.strip().to_string(),
            udf_type: UdfType::Action,
            cached: Some(false),
            context,
        };

        self.log_execution_progress(log_lines, event_source, unix_timestamp)
    }

    pub fn log_http_action_progress(
        &self,
        identifier: HttpActionRoute,
        unix_timestamp: UnixTimestamp,
        context: RequestContext,
        log_lines: LogLines,
    ) {
        let event_source = FunctionEventSource {
            path: identifier.to_string(),
            udf_type: UdfType::HttpAction,
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
            report_error(&mut e);
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
            report_error(&mut e);
        }
    }

    pub fn udf_rate(
        &self,
        identifier: UdfIdentifier,
        metric: UdfRate,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        let mut inner = self.inner.lock();
        let metrics = inner.metrics.udf.entry(identifier).or_default();
        let data = match metric {
            UdfRate::Invocations => &metrics.invocations,
            UdfRate::Errors => &metrics.errors,
            UdfRate::CacheHits => &metrics.cache_hits,
            UdfRate::CacheMisses => &metrics.cache_misses,
        };
        data.events_per_second(window)
    }

    pub fn cache_hit_percentage(
        &self,
        identifier: UdfIdentifier,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        let mut inner = self.inner.lock();
        let metrics = inner.metrics.udf.entry(identifier).or_default();
        metrics.cache_hit_percentage(window)
    }

    pub fn latency_percentiles(
        &self,
        identifier: UdfIdentifier,
        percentiles: Vec<Percentile>,
        window: MetricsWindow,
    ) -> anyhow::Result<BTreeMap<Percentile, Timeseries>> {
        let mut inner = self.inner.lock();
        let metrics = inner.metrics.udf.entry(identifier).or_default();
        metrics.latency_percentiles(percentiles, window)
    }

    pub fn table_rate(
        &self,
        name: TableName,
        metric: TableRate,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        let mut inner = self.inner.lock();
        let metrics = inner.metrics.table.entry(name).or_default();
        let data = match metric {
            TableRate::RowsRead => &metrics.rows_read,
            TableRate::RowsWritten => &metrics.rows_written,
        };
        data.summed_events_per_second(window)
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
                        .map(|(_, entry)| entry.clone())
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

    log: WithHeapSize<VecDeque<(CursorMs, FunctionExecution)>>,
    log_waiters: WithHeapSize<Vec<oneshot::Sender<()>>>,
    log_manager: Arc<dyn LogSender>,

    metrics: Metrics,
}

impl<RT: Runtime> HeapSize for Inner<RT> {
    fn heap_size(&self) -> usize {
        self.log.heap_size() + self.log_waiters.heap_size()
    }
}

impl<RT: Runtime> Inner<RT> {
    fn log_execution(
        &mut self,
        execution: FunctionExecution,
        send_console_events: bool,
    ) -> anyhow::Result<()> {
        self.metrics.append(&execution)?;
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
                report_error(&mut e);
            },
        }

        self.log_manager.send_logs(log_events);

        self.log.push_back((next_time, execution));
        while self.log.len() > *MAX_UDF_EXECUTION {
            self.log.pop_front();
        }
        for waiter in self.log_waiters.drain(..) {
            let _ = waiter.send(());
        }
        Ok(())
    }

    fn log_execution_progress(
        &mut self,
        log_lines: LogLines,
        event_source: FunctionEventSource,
        unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<()> {
        let progress = FunctionExecutionProgress {
            log_lines,
            event_source,
        };
        // TODO: this should ideally use a timestamp on the log lines themselves, but
        // for now use the start timestamp of the function
        let log_events = progress.console_log_events(unix_timestamp);
        self.log_manager.send_logs(log_events);
        // TODO: add these to the UDF execution log so we can stream them to the
        // dashboard
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
            let lower_bound = last_time.next_after(std::f64::INFINITY);
            if lower_bound > next_time {
                next_time = lower_bound;
            }
        }
        Ok(next_time)
    }
}

#[derive(Default)]
struct Metrics {
    udf: BTreeMap<UdfIdentifier, UdfMetrics>,
    table: BTreeMap<TableName, TableMetrics>,
}

impl Metrics {
    fn append(&mut self, row: &FunctionExecution) -> anyhow::Result<()> {
        let ts = row.unix_timestamp.as_system_time();
        self.udf
            .entry(row.identifier())
            .or_default()
            .append(ts, row)?;
        for (table_name, table_stats) in &row.tables_touched {
            self.table
                .entry(table_name.clone())
                .or_default()
                .append(ts, table_stats)?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct UdfMetrics {
    invocations: Series<()>,
    errors: Series<()>,

    cache_hits: Series<()>,
    cache_misses: Series<()>,

    execution_time: Series<Duration>,
}

impl UdfMetrics {
    fn append(&mut self, ts: SystemTime, row: &FunctionExecution) -> anyhow::Result<()> {
        self.invocations.append(ts)?;
        let is_err = match &row.params {
            UdfParams::Function { error, .. } => error.is_some(),
            UdfParams::Http { result, .. } => result.is_err(),
        };
        if is_err {
            self.errors.append(ts)?;
        }
        if row.cached_result {
            self.cache_hits.append(ts)?;
        } else {
            self.cache_misses.append(ts)?;
        }
        let execution_time = Duration::from_secs_f64(row.execution_time);
        self.execution_time.append_value(ts, execution_time)?;
        Ok(())
    }

    fn cache_hit_percentage(&self, window: MetricsWindow) -> anyhow::Result<Timeseries> {
        let mut hits = vec![0; window.num_buckets];
        let mut misses = vec![0; window.num_buckets];

        let start = self
            .cache_misses
            .bounded_start(self.cache_hits.bounded_start(window.start));
        for (ts, _) in self.cache_hits.range(start, window.end) {
            hits[window.bucket_index(ts)?] += 1;
        }
        for (ts, _) in self.cache_misses.range(start, window.end) {
            misses[window.bucket_index(ts)?] += 1;
        }
        hits.into_iter()
            .zip(misses)
            .enumerate()
            .map(|(i, (num_hits, num_misses))| {
                let num_reqs = num_hits + num_misses;
                // Emit a missing value if there are no requests for a bucket.
                let hit_percentage = if num_reqs == 0 {
                    None
                } else {
                    Some((num_hits as f64) / (num_reqs as f64) * 100.)
                };
                Ok((window.bucket_start(i)?, hit_percentage))
            })
            .collect()
    }

    fn latency_percentiles(
        &self,
        percentiles: Vec<Percentile>,
        window: MetricsWindow,
    ) -> anyhow::Result<BTreeMap<Percentile, Timeseries>> {
        let mut bucket_samples = vec![vec![]; window.num_buckets];
        for (ts, &latency) in self.execution_time.range(window.start, window.end) {
            bucket_samples[window.bucket_index(ts)?].push(latency);
        }
        for bucket_sample in &mut bucket_samples {
            bucket_sample.sort();
        }
        let mut out = BTreeMap::new();
        for percentile in percentiles {
            anyhow::ensure!(percentile <= 100);
            let timeseries = bucket_samples
                .iter()
                .enumerate()
                .map(|(i, bucket)| {
                    let metric = if bucket.is_empty() {
                        None
                    } else {
                        let ix = (((percentile as f64) / 100.) * (bucket.len() as f64)) as usize;
                        Some(bucket[ix].as_secs_f64())
                    };
                    Ok((window.bucket_start(i)?, metric))
                })
                .collect::<anyhow::Result<_>>()?;
            out.insert(percentile, timeseries);
        }
        Ok(out)
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
                                    let key = String::from(environment);
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

#[derive(Default)]
struct TableMetrics {
    rows_read: Series<u64>,
    rows_written: Series<u64>,
}

impl TableMetrics {
    fn append(&mut self, ts: SystemTime, stats: &TableStats) -> anyhow::Result<()> {
        self.rows_read.append_value(ts, stats.rows_read)?;
        self.rows_written.append_value(ts, stats.rows_written)?;
        Ok(())
    }
}

#[derive(Default)]
struct Series<V> {
    data: BTreeMap<SystemTime, Vec<V>>,
    low_watermark: Option<SystemTime>,
}

impl Series<()> {
    fn append(&mut self, ts: SystemTime) -> anyhow::Result<()> {
        self.append_value(ts, ())
    }
}

impl<V> Series<V> {
    /// Different Series can be truncated and have different starting points,
    /// so when comparing data across Series, bound the start time by
    /// bounded_start on both series.
    fn bounded_start(&self, start: SystemTime) -> SystemTime {
        if let Some(low_watermark) = self.low_watermark {
            cmp::max(start, low_watermark)
        } else {
            start
        }
    }

    fn range(&self, start: SystemTime, end: SystemTime) -> impl Iterator<Item = (SystemTime, &V)> {
        self.data
            .range(start..end)
            .flat_map(|(&k, vs)| vs.iter().map(move |v| (k, v)))
    }

    fn append_value(&mut self, ts: SystemTime, value: V) -> anyhow::Result<()> {
        self.data.entry(ts).or_default().push(value);
        if self.low_watermark.is_none() {
            self.low_watermark = Some(ts);
        }
        while self.data.len() > *MAX_UDF_EXECUTION {
            self.data.pop_first();
            self.low_watermark = Some(*self.data.first_key_value().expect("empty data too big?").0);
        }
        Ok(())
    }

    fn events_per_second(&self, window: MetricsWindow) -> anyhow::Result<Timeseries> {
        let mut bucket_counts = vec![0; window.num_buckets];
        for (ts, _) in self.range(window.start, window.end) {
            bucket_counts[window.bucket_index(ts)?] += 1;
        }
        let width = window.bucket_width()?.as_secs_f64();
        bucket_counts
            .into_iter()
            .enumerate()
            // Convert from a count to a number of events per second.
            .map(|(i, count)| Ok((window.bucket_start(i)?, Some((count as f64) / width))))
            .collect()
    }
}

impl Series<u64> {
    fn summed_events_per_second(&self, window: MetricsWindow) -> anyhow::Result<Timeseries> {
        let mut bucket_counts = vec![0; window.num_buckets];
        for (ts, &count) in self.range(window.start, window.end) {
            bucket_counts[window.bucket_index(ts)?] += count;
        }
        let width = window.bucket_width()?.as_secs_f64();
        bucket_counts
            .into_iter()
            .enumerate()
            // Convert from a count to a number of events per second.
            .map(|(i, count)| Ok((window.bucket_start(i)?, Some((count as f64) / width))))
            .collect()
    }
}

impl MetricsWindow {
    fn bucket_width(&self) -> anyhow::Result<Duration> {
        let interval_width = self
            .end
            .duration_since(self.start)
            .unwrap_or_else(|_| panic!("Invalid query window: {:?} < {:?}", self.end, self.start));
        Ok(interval_width / (self.num_buckets as u32))
    }

    fn bucket_index(&self, ts: SystemTime) -> anyhow::Result<usize> {
        if !(self.start <= ts && ts < self.end) {
            anyhow::bail!("{:?} not in [{:?}, {:?})", ts, self.start, self.end);
        }
        let since_start = ts.duration_since(self.start).unwrap();
        Ok((since_start.as_secs_f64() / self.bucket_width()?.as_secs_f64()) as usize)
    }

    fn bucket_start(&self, i: usize) -> anyhow::Result<SystemTime> {
        let bucket_start = self.start + self.bucket_width()? * (i as u32);
        if self.end < bucket_start {
            anyhow::bail!(
                "Invalid bucket index {} for {} buckets in [{:?}, {:?})",
                i,
                self.num_buckets,
                self.start,
                self.end
            );
        }
        Ok(bucket_start)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{
        Duration,
        SystemTime,
    };

    use super::{
        MetricsWindow,
        Series,
    };

    #[test]
    fn test_series() -> anyhow::Result<()> {
        let mut s: Series<()> = Series::default();

        let days_after_epoch = |n: u64| SystemTime::UNIX_EPOCH + Duration::from_secs(86400 * n);

        let t0 = days_after_epoch(30);
        s.append(t0)?;

        let t1 = days_after_epoch(32);
        s.append(t1)?;

        let window = MetricsWindow {
            start: days_after_epoch(29),
            end: days_after_epoch(31),
            num_buckets: 1,
        };
        let ts = s.events_per_second(window)?;
        assert_eq!(ts.len(), 1);
        let (ts, rate) = ts[0];
        assert_eq!(ts, days_after_epoch(29));
        assert_eq!(rate, Some(1. / (86400. * 2.)));

        let window = MetricsWindow {
            start: days_after_epoch(28),
            end: days_after_epoch(34),
            num_buckets: 1,
        };
        let ts = s.events_per_second(window)?;
        assert_eq!(ts.len(), 1);
        let (ts, rate) = ts[0];
        assert_eq!(ts, days_after_epoch(28));
        assert_eq!(rate, Some(2. / (86400. * 6.)));

        Ok(())
    }
}
