use std::{
    fmt,
    fmt::Display,
    str::FromStr,
    time::Duration,
};

use serde::Deserialize;
use serde_json::{
    json,
    Value as JsonValue,
};
use value::heap_size::HeapSize;

use crate::{
    errors::JsError,
    execution_context::ExecutionContext,
    log_lines::LogLine,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        ModuleEnvironment,
        UdfType,
    },
};

/// Public worker for the LogManager.
pub trait LogSender: Send + Sync {
    fn send_logs(&self, logs: Vec<LogEvent>);
    fn shutdown(&self) -> anyhow::Result<()>;
}

pub struct NoopLogSender;

impl LogSender for NoopLogSender {
    fn send_logs(&self, _logs: Vec<LogEvent>) {}

    fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Structured log
#[derive(Debug, Clone)]
pub struct LogEvent {
    /// Rough timestamp of when this event was created, for the user's benefit.
    /// We provide no guarantees on the consistency of this timestamp across
    /// topics and log sources - it's best-effort.
    /// This timestamp is serialized to milliseconds.
    pub timestamp: UnixTimestamp,
    pub event: StructuredLogEvent,
}

/// User-facing UDF stats, that is logged in the UDF execution log
/// and might be used for debugging purposes.
///
/// TODO(sarah) this is nearly identical to the type in the `usage_tracking`
/// crate, but there's a dependency cycle preventing us from using it directly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AggregatedFunctionUsageStats {
    pub database_read_bytes: u64,
    pub database_write_bytes: u64,
    pub storage_read_bytes: u64,
    pub storage_write_bytes: u64,
    pub vector_index_read_bytes: u64,
    pub vector_index_write_bytes: u64,
    pub action_memory_used_mb: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum StructuredLogEvent {
    Verification,
    Console {
        source: FunctionEventSource,
        log_line: LogLine,
    },
    FunctionExecution {
        source: FunctionEventSource,
        error: Option<JsError>,
        execution_time: Duration,
        usage_stats: AggregatedFunctionUsageStats,
    },
    Exception {
        error: JsError,
        user_identifier: Option<sync_types::UserIdentifier>,
        source: FunctionEventSource,
        udf_server_version: Option<semver::Version>,
    },
    DeploymentAuditLog {
        action: String,
        metadata: serde_json::Map<String, JsonValue>,
    },
    // User-specified topics -- not yet implemented.
    // See here for more details: https://www.notion.so/Log-Streaming-in-Convex-19a1dfadd6924c33b29b2796b0f5b2e2
    // User {
    //     topic: String,
    //     payload: serde_json::Map<String, JsonValue>
    // },
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum LogEventFormatVersion {
    V1,
    V2,
}

impl FromStr for LogEventFormatVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::V1),
            "2" => Ok(Self::V2),
            v => anyhow::bail!("Invalid LogEventFormatVersion: {v}"),
        }
    }
}

impl Display for LogEventFormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1 => write!(f, "1"),
            Self::V2 => write!(f, "2"),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for LogEventFormatVersion {
    fn default() -> Self {
        Self::V2
    }
}

/// Structured log
impl LogEvent {
    pub fn default_for_verification<RT: Runtime>(runtime: &RT) -> anyhow::Result<Self> {
        Ok(Self {
            event: StructuredLogEvent::Verification,
            timestamp: runtime.unix_timestamp(),
        })
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn sample_exception<RT: Runtime>(runtime: &RT) -> anyhow::Result<Self> {
        use sync_types::UserIdentifier;

        let source = FunctionEventSource {
            context: ExecutionContext::new_for_test(),
            path: "test".to_string(),
            udf_type: UdfType::Action,
            module_environment: ModuleEnvironment::Isolate,
            cached: None,
        };
        Ok(Self {
            timestamp: runtime.unix_timestamp(),
            event: StructuredLogEvent::Exception {
                error: JsError::from_frames_for_test(
                    "test_message",
                    vec!["test_frame_1", "test_frame_2"],
                ),
                user_identifier: Some(UserIdentifier("test|user".to_string())),
                source,
                udf_server_version: Some(semver::Version::new(1, 5, 1)),
            },
        })
    }

    pub fn to_json_map(
        self,
        format: LogEventFormatVersion,
    ) -> anyhow::Result<serde_json::Map<String, JsonValue>> {
        let ms = self.timestamp.as_ms_since_epoch()?;
        let value = match format {
            LogEventFormatVersion::V1 => match self.event {
                StructuredLogEvent::Verification => {
                    json!({
                        "_timestamp": ms,
                        "_topic":  "_verification",
                        "message": "Convex connection test"
                    })
                },
                StructuredLogEvent::Console { source, log_line } => {
                    json!({
                        "_timestamp": ms,
                        "_topic":  "_console",
                        "_functionPath": source.path,
                        "_functionType": source.udf_type,
                        "_functionCached": source.cached,
                        "message": log_line.to_pretty_string()
                    })
                },
                StructuredLogEvent::FunctionExecution {
                    source,
                    error,
                    execution_time,
                    usage_stats,
                } => {
                    let (reason, status) = match error {
                        Some(err) => (json!(err.to_string()), "failure"),
                        None => (JsonValue::Null, "success"),
                    };
                    let execution_time_ms = execution_time.as_millis();
                    json!({
                        "_timestamp": ms,
                        "_topic":  "_execution_record",
                        "_functionPath": source.path,
                        "_functionType": source.udf_type,
                        "_functionCached": source.cached,
                        "status": status,
                        "reason": reason,
                        "executionTimeMs": execution_time_ms,
                        "databaseReadBytes": usage_stats.database_read_bytes,
                        "databaseWriteBytes": usage_stats.database_write_bytes,
                        "storageReadBytes": usage_stats.storage_read_bytes,
                        "storageWriteBytes": usage_stats.storage_write_bytes,
                    })
                },
                StructuredLogEvent::Exception {
                    error,
                    user_identifier,
                    source,
                    udf_server_version,
                } => {
                    let message = error.message;
                    let frames: Option<Vec<String>> = error
                        .frames
                        .as_ref()
                        .map(|frames| frames.0.iter().map(|frame| frame.to_string()).collect());
                    json!({
                        "_timestamp": ms,
                        "_topic":  "_exception",
                        "_functionPath": source.path,
                        "_functionType": source.udf_type,
                        "_functionCached": source.cached,
                        "message": message,
                        "frames": frames,
                        "udfServerVersion": udf_server_version,
                        "userIdentifier": user_identifier,
                    })
                },
                StructuredLogEvent::DeploymentAuditLog { action, metadata } => {
                    json!({
                        "_timestamp": ms,
                        "_topic":  "_audit_log",
                        "action": action,
                        "actionMetadata": metadata
                    })
                },
            },
            LogEventFormatVersion::V2 => match self.event {
                StructuredLogEvent::Verification => {
                    json!({
                        "timestamp": ms,
                        "topic": "verification",
                        "message": "Convex connection test"
                    })
                },
                StructuredLogEvent::Console { source, log_line } => {
                    let function_source = source.to_json_map();
                    match log_line {
                        LogLine::Structured {
                            messages,
                            level,
                            timestamp,
                            is_truncated,
                            system_metadata,
                        } => {
                            let timestamp_ms = timestamp.as_ms_since_epoch()?;
                            json!({
                                "timestamp": timestamp_ms,
                                "topic": "console",
                                "function": function_source,
                                "log_level": level.to_string(),
                                "message": messages.join(" "),
                                "is_truncated": is_truncated,
                                "system_code": system_metadata.map(|s| s.code)

                            })
                        },
                    }
                },
                StructuredLogEvent::FunctionExecution {
                    source,
                    error,
                    execution_time,
                    usage_stats,
                } => {
                    let function_source = source.to_json_map();
                    let (status, error_message) = match error {
                        Some(error) => ("failure", Some(error.to_string())),
                        None => ("success", None),
                    };
                    json!({
                        "timestamp": ms,
                        "topic": "function_execution",
                        "function": function_source,
                        "execution_time_ms": execution_time.as_millis(),
                        "status": status,
                        "error_message": error_message,
                        "usage": {
                            "database_read_bytes": usage_stats.database_read_bytes,
                            "database_write_bytes": usage_stats.database_write_bytes,
                            "file_storage_read_bytes": usage_stats.storage_read_bytes,
                            "file_storage_write_bytes": usage_stats.storage_write_bytes,
                            "vector_storage_read_bytes": usage_stats.vector_index_read_bytes,
                            "vector_storage_write_bytes": usage_stats.vector_index_write_bytes,
                            "action_memory_used_mb": usage_stats.action_memory_used_mb
                        }
                    })
                },
                StructuredLogEvent::Exception {
                    error,
                    user_identifier,
                    source,
                    udf_server_version,
                } => {
                    let message = error.message;
                    let frames: Option<Vec<String>> = error
                        .frames
                        .as_ref()
                        .map(|frames| frames.0.iter().map(|frame| frame.to_string()).collect());
                    json!({
                        "_timestamp": ms,
                        "_topic":  "_exception",
                        "_functionPath": source.path,
                        "_functionType": source.udf_type,
                        "_functionCached": source.cached,
                        "message": message,
                        "frames": frames,
                        "udfServerVersion": udf_server_version,
                        "userIdentifier": user_identifier,
                    })
                },
                StructuredLogEvent::DeploymentAuditLog { action, metadata } => {
                    json!({
                        "timestamp": ms,
                        "topic": "audit_log",
                        "audit_log_action": action,
                        // stringified JSON to avoid
                        "audit_log_metadata": serde_json::to_string(&JsonValue::Object(metadata))?
                    })
                },
            },
        };
        let JsonValue::Object(fields) = value else {
            unreachable!();
        };
        Ok(fields)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum LogTopic {
    /// Topic for logs generated by `console.*` events. This is considered a
    /// `SystemLogTopic` since the topic is generated by the backend.
    Console,
    /// Topic for verification logs. These are issued on sink startup and are
    /// used to test that the backend can authenticate with the sink.
    Verification,
    /// Topic that records UDF executions and provides information on the
    /// execution.
    UdfExecutionRecord,
    /// Topic for deployment audit logs. These are issued when developers
    /// interact with a deployment.
    DeploymentAuditLog,
    /// Topic for exceptions. These happen when a UDF raises an exception from
    /// JS
    Exception,
    /// User-specified topics which are emitted via the client-side UDF
    /// capability See here for more details: https://www.notion.so/Log-Streaming-in-Convex-19a1dfadd6924c33b29b2796b0f5b2e2
    User(String),
}

impl TryFrom<LogTopic> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(value: LogTopic) -> Result<Self, Self::Error> {
        let topic = match value {
            LogTopic::Console => "_console".to_string(),
            LogTopic::Verification => "_verification".to_string(),
            LogTopic::UdfExecutionRecord => "_execution_record".to_string(),
            LogTopic::DeploymentAuditLog => "_audit_log".to_string(),
            LogTopic::Exception => "_exception".to_string(),
            LogTopic::User(s) => s,
        };
        Ok(JsonValue::String(topic))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum EventSource {
    Function(FunctionEventSource),
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct FunctionEventSource {
    pub context: ExecutionContext,
    pub path: String,
    pub udf_type: UdfType,
    pub module_environment: ModuleEnvironment,
    // Only queries can be cached, so this is only Some for queries. This is important
    // information to transmit to the client to distinguish from logs users explicitly created
    // and logs that we created for by redoing a query when its readset changes.
    pub cached: Option<bool>,
}

impl FunctionEventSource {
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test() -> Self {
        Self {
            context: ExecutionContext::new_for_test(),
            path: "path/to/file:myFunction".to_string(),
            udf_type: UdfType::Mutation,
            module_environment: ModuleEnvironment::Isolate,
            cached: None,
        }
    }

    pub fn to_json_map(&self) -> serde_json::Map<String, JsonValue> {
        let udf_type = match self.udf_type {
            UdfType::Query => "query",
            UdfType::Mutation => "mutation",
            UdfType::Action => "action",
            UdfType::HttpAction => "http_action",
        };
        let JsonValue::Object(fields) = json!({
            "path": self.path,
            "type": udf_type,
            "cached": self.cached,
            "request_id": self.context.request_id.to_string(),
        }) else {
            unreachable!()
        };
        fields
    }
}

impl HeapSize for FunctionEventSource {
    fn heap_size(&self) -> usize {
        self.path.heap_size() + self.udf_type.heap_size() + self.cached.heap_size()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{
        json,
        Value as JsonValue,
    };

    use crate::{
        execution_context::ExecutionContext,
        log_lines::{
            LogLevel,
            LogLine,
        },
        log_streaming::{
            FunctionEventSource,
            LogEvent,
            LogEventFormatVersion,
            StructuredLogEvent,
        },
        runtime::UnixTimestamp,
        types::{
            ModuleEnvironment,
            UdfType,
        },
    };

    #[test]
    fn test_serialization_of_console_log_event() -> anyhow::Result<()> {
        let timestamp = UnixTimestamp::from_millis(1000);
        let context = ExecutionContext::new_for_test();
        let request_id = context.request_id.clone();
        let event = LogEvent {
            timestamp,
            event: StructuredLogEvent::Console {
                source: FunctionEventSource {
                    context,
                    path: "test:test".to_string(),
                    udf_type: UdfType::Query,
                    module_environment: ModuleEnvironment::Isolate,
                    cached: Some(true),
                },
                log_line: LogLine::new_developer_log_line(
                    LogLevel::Log,
                    vec!["my test log".to_string()],
                    timestamp,
                ),
            },
        };

        // Test serialization
        let fields: serde_json::Map<String, JsonValue> =
            event.to_json_map(LogEventFormatVersion::default())?;
        let value = serde_json::to_value(&fields)?;
        assert_eq!(
            value,
            json!({
                "topic": "console",
                "timestamp": 1000,
                "function": json!({
                    "path": "test:test",
                    "type": "query",
                    "cached": true,
                    "request_id": request_id.to_string()
                }),
                "log_level": "LOG",
                "message": "my test log",
                "is_truncated": false,
                "system_code": JsonValue::Null
            })
        );
        Ok(())
    }
}
