//! Common types representing database identifiers.
use std::{
    collections::BTreeMap,
    fmt::Display,
    str::FromStr,
};

use futures::{
    channel::mpsc,
    future::{
        BoxFuture,
        FutureExt,
    },
    select_biased,
    StreamExt,
};
use pb::funrun;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
pub use sync_types::{
    SessionId,
    SessionRequestSeqNumber,
    Timestamp,
};
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    remove_boolean,
    remove_int64,
    remove_nullable_object,
    remove_string,
    remove_vec_of_strings,
    ConvexObject,
    ConvexValue,
};

use crate::runtime::UnixTimestamp;

pub const TRUNCATED_LINE_SUFFIX: &str = " (truncated due to length)";
/// List of log lines from a Convex function execution.
pub type LogLines = WithHeapSize<Vec<LogLine>>;
pub type RawLogLines = WithHeapSize<Vec<String>>;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum LogLevel {
    Debug,
    Error,
    Warn,
    Info,
    Log,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => f.write_str("DEBUG"),
            LogLevel::Error => f.write_str("ERROR"),
            LogLevel::Warn => f.write_str("WARN"),
            LogLevel::Info => f.write_str("INFO"),
            LogLevel::Log => f.write_str("LOG"),
        }
    }
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = match s {
            "DEBUG" => LogLevel::Debug,
            "ERROR" => LogLevel::Error,
            "WARN" => LogLevel::Warn,
            "INFO" => LogLevel::Info,
            "LOG" => LogLevel::Log,
            _ => anyhow::bail!("Unknown log level"),
        };
        Ok(v)
    }
}

impl HeapSize for LogLevel {
    fn heap_size(&self) -> usize {
        0
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SystemLogMetadata {
    code: String,
}

impl HeapSize for SystemLogMetadata {
    fn heap_size(&self) -> usize {
        self.code.heap_size()
    }
}

impl TryFrom<ConvexObject> for SystemLogMetadata {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let code = remove_string(&mut fields, "code")?;
        Ok(SystemLogMetadata { code })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogLine {
    Unstructured(String),
    Structured {
        // Each of these is a string representation of an argument to `console.log`
        // or similar
        messages: WithHeapSize<Vec<String>>,
        level: LogLevel,
        is_truncated: bool,
        timestamp: UnixTimestamp,
        system_metadata: Option<SystemLogMetadata>,
    },
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for LogLine {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = LogLine>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        prop_oneof![
            any::<String>().prop_map(LogLine::Unstructured),
            (
                prop::collection::vec(any::<String>(), 1..4),
                any::<LogLevel>(),
                any::<bool>(),
                any::<u64>(),
                any::<Option<SystemLogMetadata>>()
            )
                .prop_map(
                    |(messages, level, is_truncated, timestamp_ms, system_metadata)| {
                        LogLine::Structured {
                            messages: messages.into(),
                            level,
                            is_truncated,
                            timestamp: UnixTimestamp::from_millis(timestamp_ms),
                            system_metadata,
                        }
                    }
                )
        ]
    }
}

impl LogLine {
    pub fn to_pretty_string(self) -> String {
        match self {
            LogLine::Unstructured(m) => m,
            LogLine::Structured {
                messages,
                level,
                is_truncated,
                timestamp: _timestamp,
                system_metadata: _system_metadata,
            } => {
                if is_truncated {
                    format!("[{level}] {}{TRUNCATED_LINE_SUFFIX}", messages.join(" "))
                } else {
                    format!("[{level}] {}", messages.join(" "))
                }
            },
        }
    }
}

impl HeapSize for LogLine {
    fn heap_size(&self) -> usize {
        match self {
            LogLine::Unstructured(m) => m.heap_size(),
            LogLine::Structured {
                messages,
                level,
                timestamp,
                is_truncated,
                system_metadata,
            } => {
                messages.heap_size()
                    + level.heap_size()
                    + timestamp.heap_size()
                    + is_truncated.heap_size()
                    + system_metadata.heap_size()
            },
        }
    }
}

impl TryFrom<ConvexValue> for LogLine {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        let result = match value {
            ConvexValue::String(s) => LogLine::Unstructured(s.into()),
            ConvexValue::Object(o) => {
                let mut fields = BTreeMap::from(o);
                let messages = remove_vec_of_strings(&mut fields, "messages")?;

                let is_truncated = remove_boolean(&mut fields, "is_truncated")?;

                let level = remove_string(&mut fields, "level")?;

                let timestamp = remove_int64(&mut fields, "timestamp")?;
                let system_metadata = remove_nullable_object(&mut fields, "system_metadata")?;

                LogLine::Structured {
                    messages: messages.clone().into(),
                    is_truncated,
                    level: LogLevel::from_str(&level)?,
                    timestamp: UnixTimestamp::from_millis(timestamp.try_into()?),
                    system_metadata,
                }
            },
            _ => anyhow::bail!("Invalid value type for log line"),
        };
        Ok(result)
    }
}

impl TryFrom<LogLine> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: LogLine) -> Result<Self, Self::Error> {
        Ok(ConvexValue::String(value.to_pretty_string().try_into()?))
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogLineJson {
    messages: Vec<String>,
    is_truncated: bool,
    timestamp: u64,
    level: String,
    system_metadata: Option<SystemLogMetadataJson>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemLogMetadataJson {
    code: String,
}

impl From<SystemLogMetadataJson> for SystemLogMetadata {
    fn from(value: SystemLogMetadataJson) -> SystemLogMetadata {
        SystemLogMetadata { code: value.code }
    }
}

impl From<SystemLogMetadata> for SystemLogMetadataJson {
    fn from(value: SystemLogMetadata) -> SystemLogMetadataJson {
        SystemLogMetadataJson { code: value.code }
    }
}

impl TryFrom<JsonValue> for SystemLogMetadata {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let json_val: SystemLogMetadataJson = serde_json::from_value(value)?;
        Ok(SystemLogMetadata::from(json_val))
    }
}

impl TryFrom<SystemLogMetadata> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(value: SystemLogMetadata) -> Result<Self, Self::Error> {
        Ok(serde_json::to_value(SystemLogMetadataJson::from(value))?)
    }
}

impl TryFrom<JsonValue> for LogLine {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::String(s) = value {
            return Ok(LogLine::Unstructured(s));
        }
        let log_line_json: LogLineJson = serde_json::from_value(value)?;
        Ok(LogLine::Structured {
            messages: log_line_json.messages.into(),
            is_truncated: log_line_json.is_truncated,
            timestamp: UnixTimestamp::from_millis(log_line_json.timestamp),
            level: log_line_json.level.parse()?,
            system_metadata: log_line_json.system_metadata.map(SystemLogMetadata::from),
        })
    }
}

impl TryFrom<LogLine> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(value: LogLine) -> Result<Self, Self::Error> {
        match value {
            LogLine::Unstructured(m) => Ok(JsonValue::String(m)),
            LogLine::Structured {
                messages,
                level,
                is_truncated,
                timestamp,
                system_metadata,
            } => {
                let log_line_json = LogLineJson {
                    messages: messages.into(),
                    is_truncated,
                    timestamp: timestamp.as_ms_since_epoch()?,
                    level: level.to_string(),
                    system_metadata: system_metadata.map(SystemLogMetadataJson::from),
                };
                Ok(serde_json::to_value(log_line_json)?)
            },
        }
    }
}

impl From<LogLine> for funrun::LogLine {
    fn from(value: LogLine) -> Self {
        match value {
            LogLine::Unstructured(m) => funrun::LogLine {
                line: Some(funrun::log_line::Line::Unstructured(m)),
            },
            LogLine::Structured {
                messages,
                level,
                is_truncated,
                timestamp,
                system_metadata,
            } => funrun::LogLine {
                line: Some(funrun::log_line::Line::Structured(
                    funrun::StructuredLogLine {
                        messages: messages.into(),
                        level: level.to_string(),
                        is_truncated,
                        timestamp: Some(timestamp.into()),
                        system_metadata: system_metadata
                            .map(|m| funrun::SystemLogMetadata { code: m.code }),
                    },
                )),
            },
        }
    }
}

impl TryFrom<funrun::LogLine> for LogLine {
    type Error = anyhow::Error;

    fn try_from(value: funrun::LogLine) -> Result<Self, Self::Error> {
        let result = match value.line {
            Some(line) => match line {
                funrun::log_line::Line::Unstructured(u) => LogLine::Unstructured(u),
                funrun::log_line::Line::Structured(s) => LogLine::Structured {
                    messages: s.messages.into(),
                    is_truncated: s.is_truncated,
                    level: s.level.parse()?,
                    timestamp: s
                        .timestamp
                        .ok_or_else(|| anyhow::anyhow!("Missing timestamp"))?
                        .try_into()?,
                    system_metadata: s
                        .system_metadata
                        .map(|m| SystemLogMetadata { code: m.code }),
                },
            },
            None => LogLine::Unstructured("".to_string()),
        };
        Ok(result)
    }
}

pub async fn run_function_and_collect_log_lines<Outcome>(
    get_outcome: BoxFuture<'_, Outcome>,
    mut log_line_receiver: mpsc::UnboundedReceiver<LogLine>,
    on_log_line: impl Fn(LogLine),
) -> (Outcome, LogLines) {
    let mut full_log_lines = vec![];
    let mut fused_get_outcome = get_outcome.fuse();
    let outcome = loop {
        select_biased! {
            outcome = fused_get_outcome => {
                break outcome;
            },
            log_line = log_line_receiver.select_next_some() =>  {
                on_log_line(log_line.clone());
                full_log_lines.push(log_line);
            }
        }
    };
    let remaining_log_lines: Vec<LogLine> = log_line_receiver.collect().await;
    for log_line in remaining_log_lines {
        on_log_line(log_line.clone());
        full_log_lines.push(log_line);
    }
    (outcome, full_log_lines.into())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;
    use value::{
        testing::assert_roundtrips,
        ConvexValue,
    };

    use crate::log_lines::LogLine;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_structured_round_trips(log_line in any::<LogLine>()) {
            let pretty = log_line.clone().to_pretty_string();
            let val = LogLine::try_from(ConvexValue::try_from(log_line).unwrap()).unwrap();
            assert_eq!(val, LogLine::Unstructured(pretty));
        }

        #[test]
        fn test_json_round_trips(log_line in any::<LogLine>()) {
            assert_roundtrips::<LogLine, JsonValue>(log_line);
        }
    }
}
