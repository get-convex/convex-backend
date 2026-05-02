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
    execution_context::{
        ClientIp,
        ClientUserAgent,
        ExecutionContext,
        RequestId,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
};

/// List of user-space audit log lines from a Convex function execution.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct AuditLogLines(WithHeapSize<Vec<AuditLogLine>>);

impl AuditLogLines {
    pub fn resolve_bodies(&self, vars: &AuditLogVars) -> anyhow::Result<ResolvedAuditLogLines> {
        let logs = self
            .0
            .iter()
            .map(|log| log.resolve_body(vars))
            .collect::<anyhow::Result<Vec<ResolvedAuditLogLine>>>()?;
        let timestamp = vars.now;
        Ok(ResolvedAuditLogLines { logs, timestamp })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuditLogLine {
    pub body: JsonValue,
}

#[derive(Clone)]
pub struct ResolvedAuditLogLines {
    pub logs: Vec<ResolvedAuditLogLine>,
    /// This timestamp is only used when emitting to log streams
    pub timestamp: UnixTimestamp,
}

/// A resolved audit log line whose body has all sentinel objects replaced
/// with concrete values.
#[derive(Debug, Clone)]
pub struct ResolvedAuditLogLine(JsonValue);

impl ResolvedAuditLogLine {
    pub fn into_value(self) -> JsonValue {
        self.0
    }
}

#[derive(Serialize)]
pub struct AuditLogVars {
    request_id: RequestId,
    ip: Option<ClientIp>,
    user_agent: Option<ClientUserAgent>,
    now: UnixTimestamp,
}

impl AuditLogVars {
    pub fn from_context(context: ExecutionContext, rt: &impl Runtime) -> Self {
        AuditLogVars {
            ip: context.request_metadata.ip,
            request_id: context.request_id,
            now: rt.unix_timestamp(),
            user_agent: context.request_metadata.user_agent,
        }
    }
}

impl AuditLogLine {
    /// Resolve all `{ "$var": "<name>" }` sentinel objects in the body,
    /// returning a [`ResolvedAuditLogLine`] with the substitutions applied.
    pub fn resolve_body(&self, vars: &AuditLogVars) -> anyhow::Result<ResolvedAuditLogLine> {
        let mut body = self.body.clone();
        resolve_vars(&mut body, vars)?;
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

fn resolve_vars(value: &mut JsonValue, vars: &AuditLogVars) -> anyhow::Result<()> {
    let AuditLogVars {
        request_id,
        ip,
        user_agent,
        now,
    } = vars;
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
                resolve_vars(v, vars)?;
            }
        },
        JsonValue::Array(arr) => {
            for v in arr.iter_mut() {
                resolve_vars(v, vars)?;
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
        self.body.heap_size()
    }
}

impl From<AuditLogLine> for pb::outcome::AuditLogLine {
    fn from(value: AuditLogLine) -> Self {
        pb::outcome::AuditLogLine {
            body_json: Some(value.body.to_string()),
        }
    }
}

impl TryFrom<pb::outcome::AuditLogLine> for AuditLogLine {
    type Error = anyhow::Error;

    fn try_from(value: pb::outcome::AuditLogLine) -> Result<Self, Self::Error> {
        Ok(AuditLogLine {
            body: value.body_json.context("Missing body")?.parse()?,
        })
    }
}
