use std::str::FromStr;

use anyhow::Context;
use derive_more::Display;
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
    ConvexObject,
    ConvexValue,
    JsonPackedValue,
    Size,
};

use crate::{
    components::ComponentId,
    types::FunctionCaller,
};

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub invocation_metadata: Option<ConvexObject>,
}

impl ExecutionContext {
    pub fn new(request_id: RequestId, caller: &FunctionCaller) -> Self {
        Self::new_with_metadata(request_id, caller, None)
    }

    pub fn new_with_metadata(
        request_id: RequestId,
        caller: &FunctionCaller,
        invocation_metadata: Option<ConvexObject>,
    ) -> Self {
        Self {
            request_id,
            execution_id: ExecutionId::new(),
            parent_scheduled_job: caller.parent_scheduled_job(),
            is_root: caller.is_root(),
            invocation_metadata,
        }
    }

    pub fn new_from_parts(
        request_id: RequestId,
        execution_id: ExecutionId,
        parent_scheduled_job: Option<(ComponentId, DeveloperDocumentId)>,
        is_root: bool,
        invocation_metadata: Option<ConvexObject>,
    ) -> Self {
        Self {
            request_id,
            execution_id,
            parent_scheduled_job,
            is_root,
            invocation_metadata,
        }
    }

    pub fn is_root(&self) -> bool {
        self.is_root
    }

    pub fn merged_invocation_metadata(
        &self,
        invocation_metadata: Option<ConvexObject>,
    ) -> Option<ConvexObject> {
        merge_invocation_metadata(self.invocation_metadata.clone(), invocation_metadata)
    }

    pub fn add_sentry_tags(&self, scope: &mut sentry::Scope) {
        scope.set_tag("request_id", &self.request_id);
        scope.set_tag("execution_id", self.execution_id);
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
            + self.invocation_metadata.as_ref().map_or(0, |metadata| {
                metadata
                    .iter()
                    .map(|(field_name, value)| field_name.heap_size() + value.size())
                    .sum::<usize>()
            })
    }
}

fn merge_invocation_metadata(
    parent: Option<ConvexObject>,
    override_metadata: Option<ConvexObject>,
) -> Option<ConvexObject> {
    match (parent, override_metadata) {
        (None, None) => None,
        (Some(parent), None) => Some(parent),
        (None, Some(override_metadata)) => Some(override_metadata),
        (Some(parent), Some(override_metadata)) => Some(
            parent
                .shallow_merge(override_metadata)
                .expect("Invocation metadata should always shallow-merge"),
        ),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Display, Serialize, Deserialize)]
#[display("{_0}")]
#[serde(transparent)]
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

impl HeapSize for RequestId {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Display, Serialize, Deserialize)]
#[display("{_0}")]
#[serde(transparent)]
pub struct ExecutionId(Uuid);

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
            invocation_metadata_json_packed_value: value
                .invocation_metadata
                .map(|metadata| {
                    JsonPackedValue::pack(ConvexValue::Object(metadata))
                        .as_str()
                        .to_owned()
                }),
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
            invocation_metadata: value
                .invocation_metadata_json_packed_value
                .map(JsonPackedValue::from_network)
                .transpose()?
                .map(|metadata| metadata.unpack())
                .transpose()?
                .map(ConvexObject::try_from)
                .transpose()?,
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
            "invocationMetadata": value.invocation_metadata.map(JsonValue::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde_json::json;
    use value::{
        ConvexObject,
        ConvexValue,
    };

    use super::{
        ExecutionContext,
        ExecutionId,
        JsonValue,
        RequestId,
    };

    fn convex_object(value: JsonValue) -> ConvexObject {
        ConvexObject::try_from(ConvexValue::try_from(value).unwrap()).unwrap()
    }

    #[test]
    fn merged_invocation_metadata_shallow_overrides_top_level_keys() {
        let context = ExecutionContext::new_from_parts(
            RequestId::from_str("request-123").unwrap(),
            ExecutionId::from_str("f4e8dbe9-071f-430f-8e76-2fda4b529d15").unwrap(),
            None,
            true,
            Some(convex_object(json!({
                "correlationId": "corr_123",
                "origin": "nuxt",
            }))),
        );

        let merged = context
            .merged_invocation_metadata(Some(convex_object(json!({
                "origin": "mcp",
                "phase": "draft",
            }))))
            .unwrap();

        assert_eq!(
            JsonValue::from(ConvexValue::Object(merged)),
            json!({
                "correlationId": "corr_123",
                "origin": "mcp",
                "phase": "draft",
            }),
        );
    }

    #[test]
    fn execution_context_proto_round_trips_invocation_metadata() {
        let context = ExecutionContext::new_from_parts(
            RequestId::from_str("request-456").unwrap(),
            ExecutionId::from_str("56ef8d74-2681-46a4-8b11-a86fcdbdb51e").unwrap(),
            None,
            false,
            Some(convex_object(json!({
                "correlationId": "corr_456",
                "origin": "browser",
                "tenantHint": "acme",
            }))),
        );

        let round_tripped =
            ExecutionContext::try_from(pb::common::ExecutionContext::from(context.clone()))
                .unwrap();

        assert_eq!(round_tripped, context);
    }
}
