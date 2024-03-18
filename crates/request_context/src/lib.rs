#![feature(impl_trait_in_assoc_type)]

use std::str::FromStr;

use anyhow::Context;
use common::types::SessionId;
use rand::Rng;
use serde_json::{
    json,
    Value as JsonValue,
};
use uuid::Uuid;
use value::{
    id_v6::DocumentIdV6,
    sha256,
};
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: RequestId,
    // A unique ID per entry in the function logs.
    // In contrast to the `RequestId`, there is a 1-1 relationship between a single
    // function ExecutionId.
    pub execution_id: ExecutionId,
    /// The id of the scheduled job that triggered this UDF, if any.
    pub parent_scheduled_job: Option<DocumentIdV6>,
    /// False if this function was called as part of a request (e.g. action
    /// calling a mutation) TODO: This is a stop gap solution. The richer
    /// version of this would be something like parent_execution_id:
    /// Option<ExecutionId>
    is_root: bool,
}

impl RequestContext {
    pub fn new(parent_scheduled_job: Option<DocumentIdV6>) -> Self {
        Self {
            request_id: RequestId::new(),
            execution_id: ExecutionId::new(),
            parent_scheduled_job,
            is_root: true,
        }
    }

    pub fn new_from_parts(
        request_id: RequestId,
        execution_id: ExecutionId,
        parent_scheduled_job: Option<DocumentIdV6>,
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
}

#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct RequestId(String);

impl RequestId {
    pub fn new() -> Self {
        let bytes = rand::thread_rng().gen::<[u8; 8]>();
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
}

impl FromStr for RequestId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RequestId(s.to_string()))
    }
}

impl ToString for RequestId {
    fn to_string(&self) -> String {
        self.0.clone()
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

impl ToString for ExecutionId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl ExecutionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<RequestContext> for pb::common::RequestContext {
    fn from(value: RequestContext) -> Self {
        pb::common::RequestContext {
            request_id: Some(value.request_id.to_string()),
            execution_id: Some(value.execution_id.to_string()),
            parent_scheduled_job: value.parent_scheduled_job.map(|id| id.into()),
            is_root: Some(value.is_root),
        }
    }
}

impl TryFrom<pb::common::RequestContext> for RequestContext {
    type Error = anyhow::Error;

    fn try_from(value: pb::common::RequestContext) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: RequestId::from_str(&value.request_id.context("Missing request id")?)?,
            execution_id: match &value.execution_id {
                Some(e) => ExecutionId::from_str(e)?,
                None => ExecutionId::new(),
            },
            parent_scheduled_job: value.parent_scheduled_job.map(|s| s.parse()).transpose()?,
            is_root: value.is_root.unwrap_or_default(),
        })
    }
}

impl From<RequestContext> for JsonValue {
    fn from(value: RequestContext) -> Self {
        json!({
            "requestId": value.request_id.to_string(),
            "executionId": value.execution_id.to_string(),
            "isRoot": value.is_root,
            "parentScheduledJob": value.parent_scheduled_job.map(|id| id.to_string()),
        })
    }
}
