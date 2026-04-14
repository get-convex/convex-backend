use std::ops::{
    Deref,
    DerefMut,
};

use anyhow::Context;
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
