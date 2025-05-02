use std::{
    fmt::{
        Display,
        Formatter,
    },
    str::FromStr,
};

use anyhow::Context;
use rand::Rng;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::types::SessionId;
use uuid::Uuid;
use value::{
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
    sha256,
};

use crate::{
    components::ComponentId,
    types::FunctionCaller,
};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ExecutionContext {
    pub request_id: RequestId,
    // A unique ID per entry in the function logs.
    // In contrast to the `RequestId`, there is a 1-1 relationship between a single
    // function ExecutionId.
    pub execution_id: ExecutionId,
    /// The id of the scheduled job that triggered this UDF, if any.
    pub parent_scheduled_job: Option<(ComponentId, DeveloperDocumentId)>,
    /// False if this function was called as part of a request (e.g. action
    /// calling a mutation) TODO: This is a stop gap solution. The richer
    /// version of this would be something like parent_execution_id:
    /// Option<ExecutionId>
    is_root: bool,
}

impl ExecutionContext {
    pub fn new(request_id: RequestId, caller: &FunctionCaller) -> Self {
        Self {
            request_id,
            execution_id: ExecutionId::new(),
            parent_scheduled_job: caller.parent_scheduled_job(),
            is_root: caller.is_root(),
        }
    }

    pub fn new_from_parts(
        request_id: RequestId,
        execution_id: ExecutionId,
        parent_scheduled_job: Option<(ComponentId, DeveloperDocumentId)>,
        is_root: bool,
    ) -> Self {
        Self {
            request_id,
            execution_id,
            parent_scheduled_job,
            is_root,
        }
    }

    pub fn is_root(&self) -> bool {
        self.is_root
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test() -> Self {
        Self {
            request_id: RequestId::new(),
            execution_id: ExecutionId::new(),
            parent_scheduled_job: None,
            is_root: true,
        }
    }

    pub fn add_sentry_tags(&self, scope: &mut sentry::Scope) {
        scope.set_tag("request_id", &self.request_id);
        scope.set_tag("execution_id", &self.execution_id);
    }
}

impl HeapSize for ExecutionContext {
    fn heap_size(&self) -> usize {
        self.request_id.heap_size()
            + self.execution_id.heap_size()
            + self
                .parent_scheduled_job
                .map_or(0, |(_, document_id)| document_id.heap_size())
            + self.is_root.heap_size()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct RequestId(String);

impl RequestId {
    pub fn new() -> Self {
        let bytes = rand::rng().random::<[u8; 8]>();
        Self(hex::encode(bytes))
    }

    // This produces a RequestId based off of information provided by a WS client
    // via our sync protocol.
    pub fn new_for_ws_session(session_id: SessionId, ws_request_counter: u32) -> Self {
        let mut hash =
            sha256::Sha256::hash(format!("{}|{ws_request_counter}", *session_id).as_bytes())
                .as_hex();
        hash.truncate(16);
        Self(hash)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<RequestId> for String {
    fn from(value: RequestId) -> Self {
        value.0
    }
}

impl FromStr for RequestId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RequestId(s.to_owned()))
    }
}

impl TryFrom<String> for RequestId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> anyhow::Result<Self> {
        Ok(RequestId(value))
    }
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl HeapSize for RequestId {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(RequestId)
    }
}

/// A unique ID per entry in the UDF logs. This can be group suboperations, like
/// database or storage calls, by the containing UDF.
///
/// In contrast to the `RequestId`, there is a 1-1 relationship between a single
/// UDF and its ExecutionId.
///
/// Execution ids are not meant to be human readable, but they must be globally
/// unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionId(Uuid);

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ExecutionId {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        "[a-f0-9]{32}"
            .prop_filter_map("Invalid Uuid", |s| s.parse().ok().map(Self))
            .boxed()
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for ExecutionId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(ExecutionId(value.parse()?))
    }
}

impl Display for ExecutionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for ExecutionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExecutionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|s| s.parse().map_err(serde::de::Error::custom))
    }
}

impl ExecutionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl HeapSize for ExecutionId {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl From<ExecutionContext> for pb::common::ExecutionContext {
    fn from(value: ExecutionContext) -> Self {
        let (parent_component_id, parent_document_id) = value.parent_scheduled_job.unzip();
        pb::common::ExecutionContext {
            request_id: Some(value.request_id.into()),
            execution_id: Some(value.execution_id.to_string()),
            parent_scheduled_job_component_id: parent_component_id
                .and_then(|id| id.serialize_to_string()),
            parent_scheduled_job: parent_document_id.map(Into::into),
            is_root: Some(value.is_root),
        }
    }
}

impl TryFrom<pb::common::ExecutionContext> for ExecutionContext {
    type Error = anyhow::Error;

    fn try_from(value: pb::common::ExecutionContext) -> Result<Self, Self::Error> {
        let parent_component_id = ComponentId::deserialize_from_string(
            value.parent_scheduled_job_component_id.as_deref(),
        )?;
        let parent_document_id = value.parent_scheduled_job.map(|s| s.parse()).transpose()?;
        Ok(Self {
            request_id: RequestId::from_str(&value.request_id.context("Missing request id")?)?,
            execution_id: match &value.execution_id {
                Some(e) => ExecutionId::from_str(e)?,
                None => ExecutionId::new(),
            },
            parent_scheduled_job: parent_document_id.map(|id| (parent_component_id, id)),
            is_root: value.is_root.unwrap_or_default(),
        })
    }
}

impl From<ExecutionContext> for JsonValue {
    fn from(value: ExecutionContext) -> Self {
        let (parent_component_id, parent_document_id) = value.parent_scheduled_job.unzip();
        json!({
            "requestId": String::from(value.request_id),
            "executionId": value.execution_id.to_string(),
            "isRoot": value.is_root,
            "parentScheduledJob": parent_document_id.map(|id| id.to_string()),
            "parentScheduledJobComponentId": parent_component_id.unwrap_or(ComponentId::Root).serialize_to_string(),
        })
    }
}
