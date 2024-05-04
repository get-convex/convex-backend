use std::{
    str::FromStr,
    time::SystemTime,
};

use anyhow::Context;
use async_trait::async_trait;
use axum::{
    body::Body,
    debug_handler,
    extract::{
        FromRequestParts,
        State,
    },
    response::IntoResponse,
    RequestPartsExt,
};
use common::{
    execution_context::{
        ExecutionContext,
        ExecutionId,
    },
    http::{
        extract::Json,
        ExtractClientVersion,
        HttpResponseError,
    },
    knobs::ACTION_USER_TIMEOUT,
    minitrace_helpers::{
        initialize_root_from_parent,
        EncodedSpan,
    },
    pause::PauseClient,
    runtime::UnixTimestamp,
    types::{
        AllowedVisibility,
        FunctionCaller,
        UdfIdentifier,
    },
    RequestId,
};
use errors::ErrorMetadata;
use http::HeaderMap;
use isolate::{
    ActionCallbacks,
    UdfArgsJson,
};
use keybroker::Identity;
use minitrace::future::FutureExt;
use model::file_storage::types::FileStorageEntry;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::{
    AuthenticationToken,
    UdfPath,
};
use usage_tracking::FunctionUsageTracker;
use value::{
    export::ValueFormat,
    id_v6::DeveloperDocumentId,
};
use vector::{
    VectorSearch,
    VectorSearchRequest,
};

use crate::{
    authentication::ExtractAuthenticationToken,
    parse::parse_udf_path,
    public_api::{
        export_value,
        UdfPostRequest,
        UdfResponse,
    },
    LocalAppState,
};

/// This is like `public_query_post`, except it allows calling internal
/// functions as well. This should not be used for any publicly accessible
/// endpoints, and should only be used to support Convex functions calling into
/// other Convex functions (i.e. actions calling into mutations)
#[minitrace::trace]
#[debug_handler]
pub async fn internal_query_post(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    ExtractExecutionContext(context): ExtractExecutionContext,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = parse_udf_path(&req.path)?;
    let udf_return = st
        .application
        .read_only_udf(
            context.request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            AllowedVisibility::All,
            FunctionCaller::Action {
                parent_scheduled_job: context.parent_scheduled_job,
            },
        )
        .await?;
    if req.format.is_some() {
        return Err(anyhow::anyhow!("req.format cannot be provided to action callbacks").into());
    }
    let value_format = Some(ValueFormat::ConvexEncodedJSON);
    let response = match udf_return.result {
        Ok(value) => UdfResponse::Success {
            value: export_value(value, value_format, client_version)?,
            log_lines: udf_return.log_lines,
        },
        Err(error) => {
            UdfResponse::nested_error(error, udf_return.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}

/// This is like `public_mutation_post`, except it allows calling internal
/// functions as well. This should not be used for any publicly accessible
/// endpoints, and should only be used to support Convex functions calling into
/// other Convex functions (i.e. actions calling into mutations)
#[minitrace::trace]
#[debug_handler]
pub async fn internal_mutation_post(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    ExtractExecutionContext(context): ExtractExecutionContext,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = parse_udf_path(&req.path)?;
    let udf_result = st
        .application
        .mutation_udf(
            context.request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            None,
            AllowedVisibility::All,
            FunctionCaller::Action {
                parent_scheduled_job: context.parent_scheduled_job,
            },
            PauseClient::new(),
        )
        .await?;
    if req.format.is_some() {
        return Err(anyhow::anyhow!("req.format cannot be provided to action callbacks").into());
    }
    let value_format = Some(ValueFormat::ConvexEncodedJSON);
    let response = match udf_result {
        Ok(write_return) => UdfResponse::Success {
            value: export_value(write_return.value, value_format, client_version)?,
            log_lines: write_return.log_lines,
        },
        Err(write_error) => UdfResponse::nested_error(
            write_error.error,
            write_error.log_lines,
            value_format,
            client_version,
        )?,
    };
    Ok(Json(response))
}

/// This is like `public_action_post`, except it allows calling internal
/// functions as well. This should not be used for any publicly accessible
/// endpoints, and should only be used to support Convex functions calling into
/// other Convex functions (i.e. actions calling into actions)
#[minitrace::trace]
#[debug_handler]
pub async fn internal_action_post(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    ExtractExecutionContext(context): ExtractExecutionContext,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = parse_udf_path(&req.path)?;
    let udf_result = st
        .application
        .action_udf(
            context.request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            AllowedVisibility::All,
            FunctionCaller::Action {
                parent_scheduled_job: context.parent_scheduled_job,
            },
        )
        .await?;
    if req.format.is_some() {
        return Err(anyhow::anyhow!("req.format cannot be provided to action callbacks").into());
    }
    let value_format = Some(ValueFormat::ConvexEncodedJSON);
    let response = match udf_result {
        Ok(action_return) => UdfResponse::Success {
            value: export_value(action_return.value, value_format, client_version)?,
            log_lines: action_return.log_lines,
        },
        Err(action_error) => UdfResponse::nested_error(
            action_error.error,
            action_error.log_lines,
            value_format,
            client_version,
        )?,
    };
    Ok(Json(response))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleJobRequest {
    udf_path: String,
    udf_args: UdfArgsJson,
    scheduled_ts: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleJobResponse {
    job_id: String,
}

#[debug_handler]
pub async fn schedule_job(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    ExtractExecutionContext(context): ExtractExecutionContext,
    Json(req): Json<ScheduleJobRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let scheduled_ts = UnixTimestamp::from_secs_f64(req.scheduled_ts);
    // User might have entered an invalid path, so this is a developer error.
    let udf_path = req.udf_path.parse::<UdfPath>().map_err(|e| {
        anyhow::anyhow!(ErrorMetadata::bad_request("InvalidUdfPath", e.to_string()))
    })?;
    let udf_args = req.udf_args.into_arg_vec();
    let job_id = st
        .application
        .runner()
        .schedule_job(identity, udf_path, udf_args, scheduled_ts, context)
        .await?;
    Ok(Json(ScheduleJobResponse {
        job_id: job_id.to_string(),
    }))
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelDeveloperJobRequest {
    pub id: String,
}

#[debug_handler]
pub async fn cancel_developer_job(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    Json(CancelDeveloperJobRequest { id }): Json<CancelDeveloperJobRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let virtual_doc_id = DeveloperDocumentId::from_str(&id).context(ErrorMetadata::bad_request(
        "InvalidArgument",
        "Invalid scheduled function ID",
    ))?;
    st.application
        .runner()
        .cancel_job(identity, virtual_doc_id)
        .await?;
    Ok(Json(json!(null)))
}

#[debug_handler]
pub async fn vector_search(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    ExtractActionName(action_name): ExtractActionName,
    ExtractExecutionContext(context): ExtractExecutionContext,
    Json(req): Json<VectorSearchRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let VectorSearchRequest { query } = req;
    let query = VectorSearch::try_from(query).map_err(|e| {
        let message = e.to_string();
        e.context(ErrorMetadata::bad_request("InvalidVectorQuery", message))
    })?;
    let (results, usage_stats) = st.application.vector_search(identity, query).await?;

    // This is a workaround. The correct way to track usage is to return in the
    // response, and then Node.js should aggregate it and then send it back to
    // the backend alongside the action result, which is how Funrun actions
    // work. Since we don't have that pipeline working in Node.js/Typescript, we
    // report vector usage directly here.
    if let Some(action_name) = action_name {
        let usage = FunctionUsageTracker::new();
        usage.add(usage_stats);
        st.application.usage_counter().track_function_usage(
            UdfIdentifier::Function(
                action_name
                    .parse()
                    .context(format!("Unexpected udf path format, got {action_name}"))?,
            ),
            // TODO(CX-6045) - have the action send the ExecutionId as a request header
            context.execution_id,
            usage.gather_user_stats(),
        );
    }

    let results: Vec<_> = results.into_iter().map(JsonValue::from).collect();
    Ok(Json(json!({ "results": results })))
}

#[debug_handler]
pub async fn storage_generate_upload_url(
    State(st): State<LocalAppState>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let url = st.application.storage_generate_upload_url().await?;
    Ok(Json(json!({ "url": url })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetParams {
    storage_id: String,
}

#[debug_handler]
pub async fn storage_get_url(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    Json(req): Json<GetParams>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let storage_id = req.storage_id.parse()?;
    let url = st
        .application
        .runner()
        .storage_get_url(identity, storage_id)
        .await?;
    Ok(Json(json!({ "url": url })))
}

#[debug_handler]
pub async fn storage_get_metadata(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    Json(req): Json<GetParams>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let storage_id = req.storage_id.parse()?;
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FileMetadataJson {
        storage_id: String,
        sha256: String,
        size: i64,
        content_type: Option<String>,
    }

    let file_metadata = st
        .application
        .runner()
        .storage_get_file_entry(identity, storage_id)
        .await?
        .map(
            |FileStorageEntry {
                 storage_id,
                 storage_key: _, // internal field that we shouldn't return in syscalls
                 sha256,
                 size,
                 content_type,
             }| {
                FileMetadataJson {
                    storage_id: storage_id.to_string(),
                    // TODO(CX-5533) use base64 for consistency.
                    sha256: sha256.as_hex(),
                    size,
                    content_type,
                }
            },
        );
    Ok(Json(file_metadata))
}

#[debug_handler]
pub async fn storage_delete(
    State(st): State<LocalAppState>,
    ExtractActionIdentity(identity): ExtractActionIdentity,
    Json(req): Json<GetParams>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let storage_id = req.storage_id.parse()?;
    st.application
        .runner()
        .storage_delete(identity, storage_id)
        .await?;
    Ok(Json(json!(null)))
}

pub static CONVEX_ACTIONS_CALLBACK_TOKEN: &str = "Convex-Action-Callback-Token";

async fn check_actions_token(
    st: &LocalAppState,
    headers: &HeaderMap,
) -> anyhow::Result<SystemTime> {
    let value = headers
        .get(CONVEX_ACTIONS_CALLBACK_TOKEN)
        .context("Missing callback token - is the call from actions?")?;

    let token = value
        .to_str()
        .context("Callback token must be an ASCII string")?;

    // Tokens are valid for 2x the action timeout, which should be more than enough
    // assuming the timeout measures in tens of seconds.
    let validity = 2 * *ACTION_USER_TIMEOUT;
    st.application
        .key_broker()
        .check_action_token(&token.to_owned(), validity)
}

fn get_encoded_span(headers: &HeaderMap) -> anyhow::Result<EncodedSpan> {
    Ok(EncodedSpan(
        headers
            .get("Convex-Encoded-Parent-Trace")
            .map(|value| value.to_str())
            .transpose()
            .context("Convex-Encoded-Parent-Trace must be a string")?
            .map(|s| s.to_string()),
    ))
}

pub async fn action_callbacks_middleware(
    State(st): State<LocalAppState>,
    req: http::request::Request<Body>,
    next: axum::middleware::Next<Body>,
) -> Result<impl IntoResponse, HttpResponseError> {
    // Validate we have an valid token in order to call any methods in this
    // actions_callback router.
    check_actions_token(&st, req.headers()).await?;

    let encoded_parent = get_encoded_span(req.headers())?;
    let root = initialize_root_from_parent(req.uri().path(), encoded_parent);

    let resp = next.run(req).in_span(root).await;
    Ok(resp)
}

// Similar to ExtractIdentity, but validates as of the action token issue time
// instead of the current time.
pub struct ExtractActionIdentity(pub Identity);

#[async_trait]
impl FromRequestParts<LocalAppState> for ExtractActionIdentity {
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        st: &LocalAppState,
    ) -> Result<Self, Self::Rejection> {
        let token: AuthenticationToken =
            parts.extract::<ExtractAuthenticationToken>().await?.into();

        // Validate the auth token based on when the action token was issued. This
        // prevents errors due to auth token expiring in the middle of long action.
        let issue_time = check_actions_token(st, &parts.headers).await?;
        Ok(Self(st.application.authenticate(token, issue_time).await?))
    }
}

pub struct ExtractActionName(pub Option<String>);

#[async_trait]
impl FromRequestParts<LocalAppState> for ExtractActionName {
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _st: &LocalAppState,
    ) -> Result<Self, Self::Rejection> {
        let action_name = parts
            .headers
            .get("Convex-Action-Function-Name")
            .map(|value| value.to_str())
            .transpose()
            .context("Convex-Action-Function-Name must be a string")?
            .map(|s| s.to_string());

        Ok(Self(action_name))
    }
}

pub struct ExtractExecutionContext(pub ExecutionContext);

#[async_trait]
impl<T> FromRequestParts<T> for ExtractExecutionContext {
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
        let execution_id: ExecutionId = parts
            .headers
            .get("Convex-Execution-Id")
            .map(|v| v.to_str())
            .transpose()
            .context("Execution id must be a string")?
            .map(ExecutionId::from_str)
            .transpose()?
            // For backwards compatibility
            .unwrap_or(ExecutionId::new());

        let is_root: bool = match parts.headers.get("Convex-Root-Request") {
            Some(v) => v.to_str().context("Convex-Root-Request must be a string")? == "true",
            None => false,
        };
        let parent_job_id = parts
            .headers
            .get("Convex-Parent-Scheduled-Job")
            .map(|v| v.to_str())
            .transpose()
            .context("Parent scheduled job id must be a string")?
            .map(|s| s.parse())
            .transpose()
            .context("Invalid scheduled job id")?;

        Ok(Self(ExecutionContext::new_from_parts(
            request_id,
            execution_id,
            parent_job_id,
            is_root,
        )))
    }
}

#[cfg(test)]
mod tests {

    use application::test_helpers::ApplicationTestExt;
    use axum::headers::authorization::Credentials;
    use common::runtime::Runtime;
    use http::Request;
    use hyper::Body;
    use runtime::prod::ProdRuntime;
    use serde_json::{
        json,
        Value as JsonValue,
    };

    use crate::{
        node_action_callbacks::ScheduleJobResponse,
        scheduling::CancelJobRequest,
        test_helpers::setup_backend_for_test,
    };

    #[convex_macro::prod_rt_test]
    async fn test_cancel_recursive_scheduled_job(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt.clone()).await?;
        let callback_token = backend.st.application.key_broker().issue_action_token();
        backend
            .st
            .application
            .load_udf_tests_modules_with_node()
            .await?;

        // Schedule a job
        let schedule_body = serde_json::to_vec(&json!({
            "udfPath": "node_actions:scheduleJob",
            "udfArgs": [{"name": "getCounter.js"}],
            "scheduledTs": Into::<i64>::into(rt.generate_timestamp()?) / 1_000_000_000,
        }))?;
        let req = Request::builder()
            .uri("/api/actions/schedule_job")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Content-Type", "application/json")
            .header("Convex-Action-Callback-Token", callback_token.clone())
            .body(schedule_body.clone().into())?;
        let ScheduleJobResponse { job_id } = backend.expect_success_and_result(req).await?;

        // Get the system document id
        let json_body = json!({
            "path":
                "_system/frontend/paginatedScheduledJobs.js",
            "args":json!({"paginationOpts": {"numItems": 10, "cursor": null}}),
            "format": "json",
        });
        let body = Body::from(serde_json::to_vec(&json_body)?);
        let req = Request::builder()
            .uri("/api/query")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Content-Type", "application/json")
            .body(body)?;
        let result: JsonValue = backend.expect_success_and_result(req).await?;
        let object = result.as_object().unwrap();
        assert_eq!(object["status"], "success");

        let jobs = object["value"]["page"].as_array().unwrap().clone();
        assert_eq!(jobs.len(), 1);
        let system_job_id = jobs[0]["_id"].as_str().unwrap().to_string();

        // Cancel the scheduled job
        let body = Body::from(serde_json::to_vec(&CancelJobRequest {
            id: job_id.clone(),
        })?);
        let req = Request::builder()
            .uri("/api/actions/cancel_job")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Content-Type", "application/json")
            .header("Convex-Action-Callback-Token", callback_token.clone())
            .body(body)?;
        backend.expect_success(req).await?;

        // Try to schedule a job as though we are a the currently running node action
        // that was just canceled
        let req = Request::builder()
            .uri("/api/actions/schedule_job")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Content-Type", "application/json")
            .header("Convex-Action-Callback-Token", callback_token.clone())
            .header("Convex-Parent-Scheduled-Job", system_job_id.clone())
            .body(schedule_body.into())?;
        backend.expect_success(req).await?;

        // Call an action A which calls an action B which schedules, as though A were
        // canceled.
        let action_body = serde_json::to_vec(&json!({
            "path": "node_actions:actionCallsAction",
            "args": [],
        }))?;
        let req = Request::builder()
            .uri("/api/actions/action")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Content-Type", "application/json")
            .header("Convex-Action-Callback-Token", callback_token)
            .header("Convex-Parent-Scheduled-Job", system_job_id)
            .body(action_body.into())?;
        backend.expect_success(req).await?;

        // Check that there are no more scheduled jobs
        let json_body = json!({
            "path":
                "_system/frontend/paginatedScheduledJobs.js",
            "args":json!({"paginationOpts": {"numItems": 10, "cursor": null}}),
            "format": "json",
        });
        let body = Body::from(serde_json::to_vec(&json_body)?);
        let req = Request::builder()
            .uri("/api/query")
            .method("POST")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Content-Type", "application/json")
            .body(body)?;
        let result: JsonValue = backend.expect_success_and_result(req).await?;
        let object = result.as_object().unwrap();
        assert_eq!(object["status"], "success");
        assert_eq!(object["value"]["page"], JsonValue::Array(vec![]));
        Ok(())
    }
}
