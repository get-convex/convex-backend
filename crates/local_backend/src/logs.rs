use std::time::Duration;

use anyhow::Context;
use application::function_log::{
    FunctionExecution,
    FunctionExecutionPart,
    UdfParams,
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
        HttpResponseError,
    },
    version::ClientType,
    RequestId,
};
use errors::ErrorMetadata;
use futures::FutureExt;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;

use crate::{
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
pub struct StreamUdfExecutionQueryArgs {
    cursor: f64,
}

#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum FunctionExecutionJson {
    #[serde(rename_all = "camelCase")]
    Completion {
        udf_type: String,
        component_path: Option<String>,
        identifier: String,
        log_lines: Vec<JsonValue>,
        timestamp: f64,
        cached_result: bool,
        execution_time: f64,
        success: Option<JsonValue>,
        error: Option<String>,
        request_id: String,
        execution_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        udf_type: String,
        component_path: Option<String>,
        identifier: String,
        timestamp: f64,
        log_lines: Vec<JsonValue>,
        request_id: String,
        execution_id: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUdfExecutionResponse {
    entries: Vec<FunctionExecutionJson>,
    new_cursor: f64,
}

pub async fn stream_udf_execution(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(query_args): Query<StreamUdfExecutionQueryArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let entries_future = st
        .application
        .stream_udf_execution(identity, query_args.cursor);
    let mut zombify_rx = st.zombify_rx.clone();
    futures::select_biased! {
        entries_future_r = entries_future.fuse() => {
            let (log_entries, new_cursor) = entries_future_r?;
            let entries = log_entries
                .into_iter()
                .map(|e| execution_to_json(e, false))
                .try_collect()?;
            let response = StreamUdfExecutionResponse {
                entries,
                new_cursor,
            };
            Ok(Json(response))
        },
        _ = tokio::time::sleep(Duration::from_secs(60)).fuse() => {
            let response = StreamUdfExecutionResponse {
                entries: vec![],
                new_cursor: query_args.cursor,
            };
            Ok(Json(response))
        },
        _ = zombify_rx.recv().fuse() => {
            // Return an error so the client reconnects after we come back up.
            Err(anyhow::anyhow!(ErrorMetadata::operational_internal_server_error()).context("Shutting down long poll request").into())
        },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamFunctionLogsResponse {
    entries: Vec<FunctionExecutionJson>,
    new_cursor: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamFunctionLogs {
    cursor: f64,
    session_id: Option<String>,
    client_request_counter: Option<u32>,
}
// Streams log lines + function completion events.
// Log lines can either appear in the completion (mutations, queries) or as
// separate messages (actions, HTTP actions), but will only appear once.
//
// If (session_id, client_request_counter) is provided, the results will be
// filtered to events from the root execution of the corresponding request.
pub async fn stream_function_logs(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Query(query_args): Query<StreamFunctionLogs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let entries_future = st
        .application
        .stream_function_logs(identity, query_args.cursor);
    let mut zombify_rx = st.zombify_rx.clone();
    let request_id = match (query_args.session_id, query_args.client_request_counter) {
        (Some(session_id), Some(client_request_counter)) => Some(RequestId::new_for_ws_session(
            session_id.parse().context("Invalid session ID")?,
            client_request_counter,
        )),
        _ => None,
    };
    // As of writing, this endpoint is only used by the CLI and dashboard, both of
    // which support either unstructured `string` log lines or structured log
    // lines.
    let supports_structured_log_lines = match client_version.client() {
        ClientType::CLI => true,
        ClientType::Dashboard => true,
        ClientType::NPM
        | ClientType::Actions
        | ClientType::Python
        | ClientType::Rust
        | ClientType::StreamingImport
        | ClientType::AirbyteExport
        | ClientType::FivetranImport
        | ClientType::FivetranExport
        | ClientType::Unrecognized(_) => false,
    };
    futures::select_biased! {
        entries_future_r = entries_future.fuse() => {
            let (log_entries, new_cursor) = entries_future_r?;
            let entries = log_entries
                .into_iter()
                .filter(|e| {
                    let Some(request_id_filter) = request_id.as_ref() else {
                        return true
                    };
                    match e {
                        FunctionExecutionPart::Completion(c) => {
                            &c.context.request_id == request_id_filter && c.context.is_root()
                        },
                        FunctionExecutionPart::Progress(c) => {
                            &c.event_source.context.request_id == request_id_filter
                                && c.event_source.context.is_root()
                        }
                    }
                })
                .map(|e| {
                    let json = match e {
                        FunctionExecutionPart::Completion(c) => {
                            execution_to_json(c, supports_structured_log_lines)?
                        },
                        FunctionExecutionPart::Progress(c) => {
                            FunctionExecutionJson::Progress {
                                udf_type: c.event_source.udf_type.to_string(),
                                component_path: c.event_source.component_path.serialize(),
                                identifier: c.event_source.udf_path,
                                timestamp: c.function_start_timestamp.as_secs_f64(),
                                log_lines: c.log_lines
                                    .into_iter()
                                    .map(|l| l.to_json(supports_structured_log_lines, false))
                                    .try_collect()?,
                                request_id: c.event_source.context.request_id.to_string(),
                                execution_id: c.event_source.context.execution_id.to_string()
                            }
                        }
                    };

                    Ok(json)
                })
                .collect::<anyhow::Result<_>>()?;
            let response = StreamUdfExecutionResponse {
                entries,
                new_cursor,
            };
            Ok(Json(response))
        },
        _ = tokio::time::sleep(Duration::from_secs(60)).fuse() => {
            let response = StreamUdfExecutionResponse {
                entries: vec![],
                new_cursor: query_args.cursor,
            };
            Ok(Json(response))
        },
        _ = zombify_rx.recv().fuse() => {
            // Return an error so the client reconnects after we come back up.
            Err(anyhow::anyhow!(ErrorMetadata::operational_internal_server_error()).context("Shutting down long poll request").into())
        },
    }
}

fn execution_to_json(
    execution: FunctionExecution,
    supports_structured_log_lines: bool,
) -> anyhow::Result<FunctionExecutionJson> {
    let json = match execution.params {
        UdfParams::Function { error, identifier } => {
            let component_path = identifier.component.serialize();
            let identifier: String = identifier.udf_path.strip().into();
            FunctionExecutionJson::Completion {
                udf_type: execution.udf_type.to_string(),
                component_path,
                identifier,
                log_lines: execution
                    .log_lines
                    .into_iter()
                    .map(|l| l.to_json(supports_structured_log_lines, false))
                    .try_collect()?,
                timestamp: execution.unix_timestamp.as_secs_f64(),
                cached_result: execution.cached_result,
                execution_time: execution.execution_time,
                success: None,
                error: error.map(|e| e.to_string()),
                request_id: execution.context.request_id.to_string(),
                execution_id: execution.context.execution_id.to_string(),
            }
        },
        UdfParams::Http { result, identifier } => {
            let identifier: String = identifier.to_string();
            let (success, error) = match result {
                Ok(v) => (Some(JsonValue::from(v)), None),
                Err(e) => (None, Some(e)),
            };
            FunctionExecutionJson::Completion {
                udf_type: execution.udf_type.to_string(),
                component_path: None,
                identifier,
                log_lines: execution
                    .log_lines
                    .into_iter()
                    .map(|l| l.to_json(supports_structured_log_lines, false))
                    .try_collect()?,
                timestamp: execution.unix_timestamp.as_secs_f64(),
                cached_result: execution.cached_result,
                execution_time: execution.execution_time,
                success,
                error: error.map(|e| e.to_string()),
                request_id: execution.context.request_id.to_string(),
                execution_id: execution.context.execution_id.to_string(),
            }
        },
    };
    Ok(json)
}
