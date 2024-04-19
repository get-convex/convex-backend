use anyhow::{
    anyhow,
    Context,
};
use application::redaction::{
    RedactedJsError,
    RedactedLogLines,
};
use axum::{
    extract::State,
    response::IntoResponse,
};
use common::{
    http::{
        extract::{
            Json,
            Query,
        },
        ExtractClientVersion,
        ExtractRequestId,
        HttpResponseError,
    },
    pause::PauseClient,
    types::{
        AllowedVisibility,
        FunctionCaller,
    },
    version::ClientVersion,
};
use errors::ErrorMetadata;
use isolate::UdfArgsJson;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    export::ValueFormat,
    ConvexValue,
};

use crate::{
    admin::bad_admin_key_error,
    authentication::ExtractIdentity,
    parse::parse_udf_path,
    LocalAppState,
};

#[derive(Deserialize)]
pub struct UdfPostRequest {
    pub path: String,
    pub args: UdfArgsJson,

    pub format: Option<String>,
}

#[derive(Deserialize)]
pub struct UdfArgsQuery {
    path: String,
    args: UdfArgsJson,

    format: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum UdfResponse {
    #[serde(rename_all = "camelCase")]
    Success {
        value: JsonValue,

        #[serde(skip_serializing_if = "RedactedLogLines::is_empty")]
        log_lines: RedactedLogLines,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        error_message: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        error_data: Option<JsonValue>,

        #[serde(skip_serializing_if = "RedactedLogLines::is_empty")]
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

/// Executes an arbitrary query/mutation/action from its name.
pub async fn public_function_post(
    State(st): State<LocalAppState>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    // We ensure for now that the user is logged in
    // (this can be removed if this endpoint is used publicly one day)
    if !identity.is_admin() {
        return Result::Err(anyhow!(bad_admin_key_error(Some(st.instance_name.clone()))).into());
    }

    let udf_path = parse_udf_path(&req.path)?;
    let udf_result = st
        .application
        .any_udf(
            request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            AllowedVisibility::PublicOnly,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match udf_result {
        Ok(write_return) => UdfResponse::Success {
            value: export_value(write_return.value, value_format, client_version)?,
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
    let exported = match value_format {
        Some(value_format) => value.export(value_format),
        None => {
            if client_version.should_require_format_param() {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "RequiresFormatParam",
                    "`format` param is required for this API",
                ))
            }

            // Old clients default to the encoded format.
            JsonValue::from(value)
        },
    };

    Ok(exported)
}

#[minitrace::trace(properties = { "udf_type": "query"})]
pub async fn public_query_get(
    State(st): State<LocalAppState>,
    Query(req): Query<UdfArgsQuery>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = req.path.parse().context(ErrorMetadata::bad_request(
        "InvalidConvexFunction",
        format!("Failed to parse Convex function path: {}", req.path),
    ))?;
    let args = req.args.into_arg_vec();
    let udf_return = st
        .application
        .read_only_udf(
            request_id,
            udf_path,
            args,
            identity,
            AllowedVisibility::PublicOnly,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match udf_return.result {
        Ok(value) => UdfResponse::Success {
            value: export_value(value, value_format, client_version)?,
            log_lines: udf_return.log_lines,
        },
        Err(error) => {
            UdfResponse::error(error, udf_return.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}

#[minitrace::trace(properties = { "udf_type": "query"})]
pub async fn public_query_post(
    State(st): State<LocalAppState>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = req.path.parse().context(ErrorMetadata::bad_request(
        "InvalidConvexFunction",
        format!("Failed to parse Convex function path: {}", req.path),
    ))?;
    let udf_return = st
        .application
        .read_only_udf(
            request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            AllowedVisibility::PublicOnly,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match udf_return.result {
        Ok(value) => UdfResponse::Success {
            value: export_value(value, value_format, client_version)?,
            log_lines: udf_return.log_lines,
        },
        Err(error) => {
            UdfResponse::error(error, udf_return.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct QueryBatchArgs {
    queries: Vec<UdfPostRequest>,
}

#[derive(Serialize)]
pub struct QueryBatchResponse {
    results: Vec<UdfResponse>,
}

pub async fn public_query_batch_post(
    State(st): State<LocalAppState>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req_batch): Json<QueryBatchArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let mut results = vec![];
    let ts = *st.application.now_ts_for_reads();
    for req in req_batch.queries {
        let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
        let udf_path = parse_udf_path(&req.path)?;
        let udf_return = st
            .application
            .read_only_udf_at_ts(
                request_id.clone(),
                udf_path,
                req.args.into_arg_vec(),
                identity.clone(),
                ts,
                None,
                AllowedVisibility::PublicOnly,
                FunctionCaller::HttpApi(client_version.clone()),
            )
            .await?;
        let response = match udf_return.result {
            Ok(value) => UdfResponse::Success {
                value: export_value(value, value_format, client_version.clone())?,
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

#[minitrace::trace(properties = { "udf_type": "mutation"})]
pub async fn public_mutation_post(
    State(st): State<LocalAppState>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = parse_udf_path(&req.path)?;
    let udf_result = st
        .application
        .mutation_udf(
            request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            None,
            AllowedVisibility::PublicOnly,
            FunctionCaller::HttpApi(client_version.clone()),
            PauseClient::new(),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match udf_result {
        Ok(write_return) => UdfResponse::Success {
            value: export_value(write_return.value, value_format, client_version)?,
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

#[minitrace::trace(properties = { "udf_type": "action"})]
pub async fn public_action_post(
    State(st): State<LocalAppState>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<UdfPostRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let udf_path = parse_udf_path(&req.path)?;
    let action_result = st
        .application
        .action_udf(
            request_id,
            udf_path,
            req.args.into_arg_vec(),
            identity,
            AllowedVisibility::PublicOnly,
            FunctionCaller::HttpApi(client_version.clone()),
        )
        .await?;
    let value_format = req.format.as_ref().map(|f| f.parse()).transpose()?;
    let response = match action_result {
        Ok(action_return) => UdfResponse::Success {
            value: export_value(action_return.value, value_format, client_version)?,
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

#[cfg(test)]
mod tests {
    use application::test_helpers::ApplicationTestExt;
    use http::{
        Request,
        StatusCode,
    };
    use hyper::Body;
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
            .body(body)?;
        match expected {
            Ok(expected) => {
                let result: JsonValue = backend.expect_success_and_result(req).await?;
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
        http_format_tester(
            rt,
            "/api/query",
            "values:intQuery",
            json!({}),
            None,
            Err("RequiresFormatParam"),
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
        http_format_tester(
            rt,
            "/api/mutation",
            "values:intMutation",
            json!({}),
            None,
            Err("RequiresFormatParam"),
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
        http_format_tester(
            rt,
            "/api/action",
            "values:intAction",
            json!({}),
            None,
            Err("RequiresFormatParam"),
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
