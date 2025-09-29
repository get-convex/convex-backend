use application::{
    api::ExecuteQueryTimestamp,
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
};
use axum::{
    debug_handler,
    extract::{
        FromRef,
        State,
    },
    response::IntoResponse,
};
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    http::{
        extract::{
            Json,
            Path,
            Query,
        },
        ExtractClientVersion,
        ExtractRequestId,
        ExtractResolvedHostname,
        HttpResponseError,
    },
    types::FunctionCaller,
    version::ClientVersion,
};
use errors::ErrorMetadata;
use isolate::UdfArgsJson;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::Timestamp;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use value::{
    export::ValueFormat,
    ConvexValue,
};

use crate::{
    args_structs::UdfPostRequestWithComponent,
    authentication::ExtractAuthenticationToken,
    parse::{
        parse_export_path,
        parse_udf_path,
    },
    RouterState,
};

#[derive(Deserialize, Debug, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UdfPostRequest {
    pub path: String,
    #[schema(value_type = Object)]
    pub args: UdfArgsJson,

    pub format: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ts {
    pub ts: SerializedTs,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UdfPostWithTsRequest {
    pub path: String,
    #[schema(value_type = Object)]
    pub args: UdfArgsJson,
    pub ts: SerializedTs,

    pub format: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SerializedTs(String);

impl From<Timestamp> for SerializedTs {
    fn from(ts: Timestamp) -> Self {
        let n: u64 = ts.into();
        let bytes = base64::encode(n.to_le_bytes());
        SerializedTs(bytes)
    }
}
impl TryFrom<SerializedTs> for Timestamp {
    type Error = anyhow::Error;

    fn try_from(value: SerializedTs) -> anyhow::Result<Self> {
        let bytes = base64::decode(value.0)?;
        let array: [u8; 8] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Wrong number of bytes for SerializedTs."))?;
        let n = u64::from_le_bytes(array);
        Timestamp::try_from(n)
    }
}

#[derive(Deserialize)]
pub struct UdfArgsQuery {
    pub path: String,
    pub args: UdfArgsJson,

    pub format: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum UdfResponse {
    #[serde(rename_all = "camelCase")]
    Success {
        value: JsonValue,

        #[serde(skip_serializing_if = "RedactedLogLines::is_empty")]
        #[schema(value_type = Vec<String>)]
        log_lines: RedactedLogLines,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        error_message: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        error_data: Option<JsonValue>,

        #[serde(skip_serializing_if = "RedactedLogLines::is_empty")]
        #[serde(default = "RedactedLogLines::empty")]
        #[schema(value_type = Vec<String>)]
        log_lines: RedactedLogLines,
    },
}

impl UdfResponse {
    pub fn nested_error(
        error: RedactedJsError,
        log_lines: RedactedLogLines,
        value_format: Option<ValueFormat>,
        client_version: ClientVersion,
    ) -> anyhow::Result<Self> {
        Self::_error(
            error.nested_to_string(),
            error,
            log_lines,
            value_format,
            client_version,
        )
    }

    pub fn error(
        error: RedactedJsError,
        log_lines: RedactedLogLines,
        value_format: Option<ValueFormat>,
        client_version: ClientVersion,
    ) -> anyhow::Result<Self> {
        Self::_error(
            format!("{error}"),
            error,
            log_lines,
            value_format,
            client_version,
        )
    }

    fn _error(
        error_message: String,
        error: RedactedJsError,
        log_lines: RedactedLogLines,
        value_format: Option<ValueFormat>,
        client_version: ClientVersion,
    ) -> anyhow::Result<Self> {
        Ok(Self::Error {
            error_message,
            error_data: error
                .custom_data_if_any()
                .map(|value| export_value(value, value_format, client_version))
                .transpose()?,
            log_lines,
        })
    }
}

/// Execute any function
///
/// Execute a query, mutation, or action function by name.
#[utoipa::path(
    post,
    path = "/function",
    request_body = UdfPostRequestWithComponent,
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
pub async fn public_function_post(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequestWithComponent>,
) -> Result<impl IntoResponse, HttpResponseError> {
    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;

    let component = req.component_path(&identity)?;
    let udf_path = parse_udf_path(&req.path)?;
    let component_function_path = CanonicalizedComponentFunctionPath {
        component,
        udf_path,
    };
    let udf_result = st
        .api
        .execute_any_function(
            &host,
            request_id,
            identity,
            component_function_path,
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match udf_result {
        Ok(write_return) => UdfResponse::Success {
            value: export_value(write_return.value.unpack(), value_format, client_version)?,
            log_lines: write_return.log_lines,
        },
        Err(write_error) => UdfResponse::error(
            write_error.error,
            write_error.log_lines,
            value_format,
            client_version,
        )?,
    };
    Ok(Json(response))
}

#[derive(Deserialize, ToSchema)]
pub struct UdfPostRequestArgsOnly {
    #[schema(value_type = Object)]
    pub args: UdfArgsJson,
    pub format: Option<String>,
}

/// Execute function by URL path
///
/// Execute a query, mutation, or action function by path in URL.
#[utoipa::path(
    post,
    path = "/run/{*functionIdentifier}",
    params(
        ("functionIdentifier" = String, Path, description = "Function path like messages/list")
    ),
    request_body = UdfPostRequestArgsOnly,
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
pub async fn public_function_post_with_path(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    Path(path): Path<String>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequestArgsOnly>,
) -> Result<impl IntoResponse, HttpResponseError> {
    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;

    let bad_request_error = || {
        anyhow::anyhow!(ErrorMetadata::bad_request(
            "MissingIdentifier",
            "Path or function name not provided in path, e.g. /api/run/messages/list",
        ))
    };
    println!("{path:?}");

    // messages/list -> ["messages", "list"]
    let mut path_parts = path
        .as_str()
        .split('/')
        .map(|p| urlencoding::decode(p).map_err(|_e| bad_request_error()))
        .try_collect::<Vec<_>>()?;
    println!("{path_parts:?}");
    if path_parts.len() < 2 {
        return Err(bad_request_error().into());
    }
    let function_name = path_parts.pop().ok_or_else(bad_request_error)?;
    let udf_path_str = format!("{}:{}", path_parts.join("/"), function_name);
    let udf_path = parse_udf_path(&udf_path_str)?;
    let udf_result = st
        .api
        .execute_any_function(
            &host,
            request_id,
            identity,
            CanonicalizedComponentFunctionPath {
                // Only functions exported at the root can be called through this endpoint
                component: ComponentPath::root(),
                udf_path,
            },
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    // Default to ConvexCleanJSON if no format is provided.
    let value_format = match req.format.as_ref().map(|f| f.parse()).transpose()? {
        Some(format) => Some(format),
        None => Some(ValueFormat::ConvexCleanJSON),
    };
    let response = match udf_result {
        Ok(write_return) => UdfResponse::Success {
            value: export_value(write_return.value.unpack(), value_format, client_version)?,
            log_lines: write_return.log_lines,
        },
        Err(write_error) => UdfResponse::error(
            write_error.error,
            write_error.log_lines,
            value_format,
            client_version,
        )?,
    };
    Ok(Json(response))
}

pub fn export_value(
    value: ConvexValue,
    value_format: Option<ValueFormat>,
    client_version: ClientVersion,
) -> anyhow::Result<JsonValue> {
    let format = match value_format {
        Some(value_format) => value_format,
        None => client_version.default_format(),
    };

    Ok(value.export(format))
}

/// Execute query (GET)
///
/// Execute a query function via GET request.
#[utoipa::path(
    get,
    path = "/query",
    params(
        ("path" = String, Query, description = "Function path"),
        ("args" = String, Query, description = "Function arguments as JSON string"),
        ("format" = Option<String>, Query, description = "Response format")
    ),
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
#[fastrace::trace(properties = { "udf_type": "query"})]
pub async fn public_query_get(
    State(st): State<RouterState>,
    Query(req): Query<UdfArgsQuery>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
) -> Result<impl IntoResponse, HttpResponseError> {
    let export_path = parse_export_path(&req.path)?;
    let journal = None;
    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;
    let query_result = st
        .api
        .execute_public_query(
            &host,
            request_id,
            identity,
            export_path,
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
            ExecuteQueryTimestamp::Latest,
            journal,
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let log_lines = query_result.log_lines;
    let response = match query_result.result {
        Ok(value) => UdfResponse::Success {
            value: export_value(value.unpack(), value_format, client_version)?,
            log_lines,
        },
        Err(error) => UdfResponse::error(error, log_lines, value_format, client_version)?,
    };
    Ok(Json(response))
}

/// Execute query (POST)
///
/// Execute a query function via POST request.
#[utoipa::path(
    post,
    path = "/query",
    request_body = UdfPostRequest,
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
#[fastrace::trace(properties = { "udf_type": "query"})]
pub async fn public_query_post(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = parse_export_path(&req.path)?;
    let journal = None;
    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;
    let query_return = st
        .api
        .execute_public_query(
            &host,
            request_id,
            identity,
            udf_path,
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
            ExecuteQueryTimestamp::Latest,
            journal,
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match query_return.result {
        Ok(value) => UdfResponse::Success {
            value: export_value(value.unpack(), value_format, client_version)?,
            log_lines: query_return.log_lines,
        },
        Err(error) => {
            UdfResponse::error(error, query_return.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}

/// Get latest timestamp
///
/// Get the latest timestamp for queries.
#[utoipa::path(
    post,
    path = "/query_ts",
    responses((status = 200, body = Ts)),
)]
#[debug_handler]
pub async fn public_get_query_ts(
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    State(st): State<RouterState>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let ts = *(st.api.latest_timestamp(&host, request_id).await?);
    Ok(Json(Ts { ts: ts.into() }))
}

/// Execute query at timestamp
///
/// Execute a query function at a specific timestamp.
#[utoipa::path(
    post,
    path = "/query_at_ts",
    request_body = UdfPostWithTsRequest,
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
#[fastrace::trace(properties = { "udf_type": "query"})]
pub async fn public_query_at_ts_post(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostWithTsRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let export_path = parse_export_path(&req.path)?;
    let journal = None;
    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;
    let ts = Timestamp::try_from(req.ts)?;
    let query_return = st
        .api
        .execute_public_query(
            &host,
            request_id,
            identity,
            export_path,
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
            ExecuteQueryTimestamp::At(ts),
            journal,
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match query_return.result {
        Ok(value) => UdfResponse::Success {
            value: export_value(value.unpack(), value_format, client_version)?,
            log_lines: query_return.log_lines,
        },
        Err(error) => {
            UdfResponse::error(error, query_return.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}

#[derive(Deserialize, ToSchema)]
pub struct QueryBatchArgs {
    queries: Vec<UdfPostRequest>,
}

#[derive(Serialize, ToSchema)]
pub struct QueryBatchResponse {
    results: Vec<UdfResponse>,
}

/// Execute query batch
///
/// Execute multiple query functions in a batch.
#[utoipa::path(
    post,
    path = "/query_batch",
    request_body = QueryBatchArgs,
    responses((status = 200, body = QueryBatchResponse)),
)]
#[debug_handler]
pub async fn public_query_batch_post(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req_batch): Json<QueryBatchArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let mut results = vec![];
    // All queries execute at the same timestamp.
    let ts = st.api.latest_timestamp(&host, request_id.clone()).await?;
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;
    for req in req_batch.queries {
        let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
        let export_path = parse_export_path(&req.path)?;
        let udf_return = st
            .api
            .execute_public_query(
                &host,
                request_id.clone(),
                identity.clone(),
                export_path,
                req.args.into_serialized_args()?,
                FunctionCaller::HttpApi(client_version.clone()),
                ExecuteQueryTimestamp::At(*ts),
                None,
            )
            .await?;
        let response = match udf_return.result {
            Ok(value) => UdfResponse::Success {
                value: export_value(value.unpack(), value_format, client_version.clone())?,
                log_lines: udf_return.log_lines,
            },
            Err(error) => UdfResponse::error(
                error,
                udf_return.log_lines,
                value_format,
                client_version.clone(),
            )?,
        };
        results.push(response);
    }
    Ok(Json(QueryBatchResponse { results }))
}

/// Execute mutation
///
/// Execute a mutation function.
#[utoipa::path(
    post,
    path = "/mutation",
    request_body = UdfPostRequest,
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
#[fastrace::trace(properties = { "udf_type": "mutation"})]
pub async fn public_mutation_post(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let export_path = parse_export_path(&req.path)?;
    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;
    let udf_result = st
        .api
        .execute_public_mutation(
            &host,
            request_id,
            identity,
            export_path,
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
            None,
            None,
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match udf_result {
        Ok(write_return) => UdfResponse::Success {
            value: export_value(write_return.value.unpack(), value_format, client_version)?,
            log_lines: write_return.log_lines,
        },
        Err(write_error) => UdfResponse::error(
            write_error.error,
            write_error.log_lines,
            value_format,
            client_version,
        )?,
    };
    Ok(Json(response))
}

/// Execute action
///
/// Execute an action function.
#[utoipa::path(
    post,
    path = "/action",
    request_body = UdfPostRequest,
    responses((status = 200, body = UdfResponse)),
)]
#[debug_handler]
#[fastrace::trace(properties = { "udf_type": "action"})]
pub async fn public_action_post(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractAuthenticationToken(auth_token): ExtractAuthenticationToken,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let export_path = parse_export_path(&req.path)?;

    // NOTE: We could coalesce authenticating and executing the query into one
    // rpc but we keep things simple by reusing the same method as the sync worker.
    // Round trip latency between Usher and Backend is much smaller than between
    // client and Usher.
    let identity = st
        .api
        .authenticate(&host, request_id.clone(), auth_token)
        .await?;
    let action_result = st
        .api
        .execute_public_action(
            &host,
            request_id,
            identity,
            export_path,
            req.args.into_serialized_args()?,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match action_result {
        Ok(action_return) => UdfResponse::Success {
            value: export_value(action_return.value.unpack(), value_format, client_version)?,
            log_lines: action_return.log_lines,
        },
        Err(action_error) => UdfResponse::error(
            action_error.error,
            action_error.log_lines,
            value_format,
            client_version,
        )?,
    };
    Ok(Json(response))
}

// The public (stable, no auth required) API of a deployment.
pub fn public_api_router<S>() -> OpenApiRouter<S>
where
    RouterState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(public_query_get))
        .routes(utoipa_axum::routes!(public_query_post))
        .routes(utoipa_axum::routes!(public_get_query_ts))
        .routes(utoipa_axum::routes!(public_query_at_ts_post))
        .routes(utoipa_axum::routes!(public_query_batch_post))
        .routes(utoipa_axum::routes!(public_mutation_post))
        .routes(utoipa_axum::routes!(public_action_post))
        .routes(utoipa_axum::routes!(public_function_post))
        .routes(utoipa_axum::routes!(public_function_post_with_path))
}

#[cfg(test)]
mod tests {
    use application::test_helpers::ApplicationTestExt;
    use axum::body::Body;
    use http::{
        Request,
        StatusCode,
    };
    use runtime::prod::ProdRuntime;
    use serde_json::{
        json,
        Value as JsonValue,
    };

    use crate::test_helpers::setup_backend_for_test;

    async fn http_format_tester(
        rt: ProdRuntime,
        uri: &'static str,
        udf: &'static str,
        args: JsonValue,
        format: Option<&'static str>,
        expected: Result<JsonValue, &'static str>,
    ) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        backend.st.application.load_udf_tests_modules().await?;
        let mut json_body = json!({
            "path": udf,
            "args": args,
        });
        if let Some(format) = format {
            json_body["format"] = format.into();
        }
        let body = Body::from(serde_json::to_vec(&json_body)?);
        let req = Request::builder()
            .uri(uri)
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Host", "localhost")
            .body(body)?;
        match expected {
            Ok(expected) => {
                let result: JsonValue = backend.expect_success(req).await?;
                assert_eq!(
                    result,
                    json!({
                        "status": "success",
                        "value": expected,
                    })
                );
            },
            Err(expected) => {
                backend
                    .expect_error(req, StatusCode::BAD_REQUEST, expected)
                    .await?;
            },
        };
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_query_default(rt: ProdRuntime) -> anyhow::Result<()> {
        // The default format is clean JSON
        http_format_tester(
            rt,
            "/api/query",
            "values:intQuery",
            json!({}),
            None,
            Ok(json!("1")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_query_clean_json(rt: ProdRuntime) -> anyhow::Result<()> {
        http_format_tester(
            rt,
            "/api/query",
            "values:intQuery",
            json!({}),
            Some("json"),
            Ok(json!("1")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_mutation_default(rt: ProdRuntime) -> anyhow::Result<()> {
        // The default format is clean JSON
        http_format_tester(
            rt,
            "/api/mutation",
            "values:intMutation",
            json!({}),
            None,
            Ok(json!("1")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_mutation_clean_json(rt: ProdRuntime) -> anyhow::Result<()> {
        http_format_tester(
            rt,
            "/api/mutation",
            "values:intMutation",
            json!({}),
            Some("json"),
            Ok(json!("1")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_action_default(rt: ProdRuntime) -> anyhow::Result<()> {
        // The default format is clean JSON
        http_format_tester(
            rt,
            "/api/action",
            "values:intAction",
            json!({}),
            None,
            Ok(json!("1")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_action_clean_json(rt: ProdRuntime) -> anyhow::Result<()> {
        http_format_tester(
            rt,
            "/api/action",
            "values:intAction",
            json!({}),
            Some("json"),
            Ok(json!("1")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_query_with_arg(rt: ProdRuntime) -> anyhow::Result<()> {
        http_format_tester(
            rt,
            "/api/query",
            "args_validation:stringArg",
            json!({"arg": "val"}),
            Some("json"),
            Ok(json!("val")),
        )
        .await
    }

    #[convex_macro::prod_rt_test]
    async fn test_http_query_legacy_list_args(rt: ProdRuntime) -> anyhow::Result<()> {
        http_format_tester(
            rt,
            "/api/query",
            "args_validation:stringArg",
            json!([{"arg": "val"}]),
            Some("json"),
            Ok(json!("val")),
        )
        .await
    }
}
