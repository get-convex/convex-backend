use std::str::FromStr;

use anyhow::Context;
use axum::{
    async_trait,
    extract::FromRequestParts,
};
use common::http::HttpResponseError;
use rand::distributions::{
    Alphanumeric,
    DistString,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::id_v6::DocumentIdV6;

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: RequestId,
    /// The id of the scheduled job that triggered this UDF, if any.
    pub parent_scheduled_job: Option<DocumentIdV6>,
}

impl RequestContext {
    pub fn only_id() -> Self {
        Self::new(None)
    }

    pub fn new(parent_scheduled_job: Option<DocumentIdV6>) -> Self {
        Self {
            request_id: RequestId::new(),
            parent_scheduled_job,
        }
    }

    pub fn new_from_parts(
        request_id: RequestId,
        parent_scheduled_job: Option<DocumentIdV6>,
    ) -> Self {
        Self {
            request_id,
            parent_scheduled_job,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct RequestId(String);

impl RequestId {
    pub fn new() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::thread_rng(), 16))
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

impl From<RequestContext> for pb::common::RequestContext {
    fn from(value: RequestContext) -> Self {
        pb::common::RequestContext {
            request_id: Some(value.request_id.to_string()),
            parent_scheduled_job: value.parent_scheduled_job.map(|id| id.into()),
        }
    }
}

impl TryFrom<pb::common::RequestContext> for RequestContext {
    type Error = anyhow::Error;

    fn try_from(value: pb::common::RequestContext) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: RequestId::from_str(&value.request_id.context("Missing request id")?)?,
            parent_scheduled_job: value.parent_scheduled_job.map(|s| s.parse()).transpose()?,
        })
    }
}

impl From<RequestContext> for JsonValue {
    fn from(value: RequestContext) -> Self {
        json!({
            "requestId": value.request_id.to_string(),
            "parentScheduledJob": value.parent_scheduled_job.map(|id| id.to_string()),
        })
    }
}
pub struct ExtractRequestContext(pub RequestContext);

#[async_trait]
impl<T> FromRequestParts<T> for ExtractRequestContext {
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _st: &T,
    ) -> Result<Self, Self::Rejection> {
        let request_id: RequestId = parts
            .headers
            .get("Convex-Request-Id")
            .map(|v| v.to_str())
            .transpose()
            .context("Request id must be a string")?
            .map(RequestId::from_str)
            .transpose()?
            // Only for backwards compatibility
            .unwrap_or(RequestId::new());
        let parent_job_id = parts
            .headers
            .get("Convex-Parent-Scheduled-Job")
            .map(|v| v.to_str())
            .transpose()
            .context("Parent scheduled job id must be a string")?
            .map(|s| s.parse())
            .transpose()
            .context("Invalid scheduled job id")?;

        Ok(Self(RequestContext::new_from_parts(
            request_id,
            parent_job_id,
        )))
    }
}
