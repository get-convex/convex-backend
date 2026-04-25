use std::ops::{
    Deref,
    DerefMut,
};

use anyhow::Context;
use errors::ErrorMetadata;
use serde::Serialize;
use serde_json::Value as JsonValue;
pub use sync_types::{
    SessionId,
    SessionRequestSeqNumber,
    Timestamp,
};
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

use crate::{
    components::CanonicalizedComponentFunctionPath,
    execution_context::{
        ClientIp,
        ClientUserAgent,
        RequestId,
    },
    runtime::UnixTimestamp,
};

/// List of user-space audit log lines from a Convex function execution.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct AuditLogLines(WithHeapSize<Vec<AuditLogLine>>);

#[derive(Clone, Debug, PartialEq)]
pub struct AuditLogLine {
    pub body: JsonValue,
    pub timestamp: UnixTimestamp,
    pub path: CanonicalizedComponentFunctionPath,
}

/// A resolved audit log line whose body has all sentinel objects replaced
/// with concrete values.
#[derive(Debug)]
pub struct ResolvedAuditLogLine(JsonValue);

impl ResolvedAuditLogLine {
    pub fn into_value(self) -> JsonValue {
        self.0
    }
}

#[derive(Serialize)]
pub struct AuditLogSentinelValues {
    request_id: RequestId,
    ip: Option<ClientIp>,
    user_agent: Option<ClientUserAgent>,
    now: UnixTimestamp,
}

impl AuditLogLine {
    /// Resolve all `{ "$var": "<name>" }` sentinel objects in the body,
    /// returning a [`ResolvedAuditLogLine`] with the substitutions applied.
    pub fn resolve_body(
        &self,
        sentinel_values: &AuditLogSentinelValues,
    ) -> anyhow::Result<ResolvedAuditLogLine> {
        let mut body = self.body.clone();
        resolve_vars(&mut body, sentinel_values)?;
        Ok(ResolvedAuditLogLine(body))
    }
}

/// Check if a JSON value is a `{ "$var": "<name>" }` sentinel and return the
/// var name if so.
fn as_var_sentinel(value: &JsonValue) -> Option<&str> {
    let obj = value.as_object()?;
    if obj.len() != 1 {
        return None;
    }
    obj.get("$var")?.as_str()
}

fn resolve_vars(
    value: &mut JsonValue,
    sentinel_values: &AuditLogSentinelValues,
) -> anyhow::Result<()> {
    let AuditLogSentinelValues {
        request_id,
        ip,
        user_agent,
        now,
    } = sentinel_values;
    if let Some(var_name) = as_var_sentinel(value) {
        match var_name {
            "requestId" => *value = serde_json::to_value(request_id)?,
            "ip" => *value = serde_json::to_value(ip)?,
            "userAgent" => *value = serde_json::to_value(user_agent)?,
            "now" => *value = serde_json::to_value(now.as_ms_since_epoch()?)?,
            _ => anyhow::bail!(ErrorMetadata::bad_request(
                "UnknownAuditLogVar",
                format!("Unknown audit log variable: \"{var_name}\""),
            )),
        }
        return Ok(());
    }
    match value {
        JsonValue::Object(map) => {
            for v in map.values_mut() {
                resolve_vars(v, sentinel_values)?;
            }
        },
        JsonValue::Array(arr) => {
            for v in arr.iter_mut() {
                resolve_vars(v, sentinel_values)?;
            }
        },
        _ => {},
    }
    Ok(())
}

impl Deref for AuditLogLines {
    type Target = WithHeapSize<Vec<AuditLogLine>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AuditLogLines {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<AuditLogLine>> for AuditLogLines {
    fn from(value: Vec<AuditLogLine>) -> Self {
        Self(value.into())
    }
}

impl IntoIterator for AuditLogLines {
    type IntoIter = <Vec<AuditLogLine> as IntoIterator>::IntoIter;
    type Item = AuditLogLine;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<AuditLogLine> for AuditLogLines {
    fn from_iter<T: IntoIterator<Item = AuditLogLine>>(iter: T) -> Self {
        Self(iter.into_iter().collect::<Vec<_>>().into())
    }
}

impl HeapSize for AuditLogLine {
    fn heap_size(&self) -> usize {
        self.body.heap_size() + self.path.heap_size() + self.timestamp.heap_size()
    }
}

impl From<AuditLogLine> for pb::outcome::AuditLogLine {
    fn from(value: AuditLogLine) -> Self {
        pb::outcome::AuditLogLine {
            body_json: Some(value.body.to_string()),
            timestamp: Some(value.timestamp.into()),
            component_path: Some(String::from(value.path.component)),
            udf_path: Some(String::from(value.path.udf_path)),
        }
    }
}

impl TryFrom<pb::outcome::AuditLogLine> for AuditLogLine {
    type Error = anyhow::Error;

    fn try_from(value: pb::outcome::AuditLogLine) -> Result<Self, Self::Error> {
        Ok(AuditLogLine {
            body: value.body_json.context("Missing body")?.parse()?,
            timestamp: value.timestamp.context("Missing timestamp")?.try_into()?,
            path: CanonicalizedComponentFunctionPath {
                component: value
                    .component_path
                    .context("Missing component path")?
                    .parse()?,
                udf_path: value.udf_path.context("Missing udf path")?.parse()?,
            },
        })
    }
}
