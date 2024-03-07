use std::time::Duration;

use application::function_log::UdfParams;
use axum::{
    extract::State,
    response::IntoResponse,
};
use common::http::{
    extract::{
        Json,
        Query,
    },
    HttpResponseError,
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
#[serde(rename_all = "camelCase")]
pub struct UdfExecutionJson {
    udf_type: String,
    identifier: String,
    log_lines: Vec<String>,
    timestamp: f64,
    cached_result: bool,
    execution_time: f64,
    success: Option<JsonValue>,
    error: Option<String>,
    request_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUdfExecutionResponse {
    entries: Vec<UdfExecutionJson>,
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
    let mut shutdown_rx = st.shutdown_rx.clone();
    futures::select_biased! {
        entries_future_r = entries_future.fuse() => {
            let (log_entries, new_cursor) = entries_future_r?;
            let entries = log_entries
                .into_iter()
                .map(|e| {
                    let json = match e.params {
                        UdfParams::Function { error, identifier } => {
                            let identifier: String = identifier.strip().into();
                            UdfExecutionJson {
                                udf_type: e.udf_type.to_string(),
                                identifier,
                                log_lines: e.log_lines
                                    .into_iter().map(|l| l.to_pretty_string()).collect(),
                                timestamp: e.unix_timestamp.as_secs_f64(),
                                cached_result: e.cached_result,
                                execution_time: e.execution_time,
                                success: None,
                                error: error.map(|e| e.to_string()),
                                request_id: e.request_id.to_string(),
                            }
                        },
                        UdfParams::Http{ result, identifier } => {
                            let identifier: String = identifier.to_string();
                            let (success, error) = match result {
                                Ok(v) => (Some(JsonValue::try_from(v)?), None),
                                Err(e) => (None, Some(e)),
                            };
                            UdfExecutionJson {
                                udf_type: e.udf_type.to_string(),
                                identifier,
                                log_lines: e.log_lines
                                    .into_iter().map(|l| l.to_pretty_string()).collect(),
                                timestamp: e.unix_timestamp.as_secs_f64(),
                                cached_result: e.cached_result,
                                execution_time: e.execution_time,
                                success,
                                error: error.map(|e| e.to_string()),
                                request_id: e.request_id.to_string(),
                            }
                        },
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
        _ = shutdown_rx.recv().fuse() => {
            // Return an error so the client reconnects after we come back up.
            Err(anyhow::anyhow!(ErrorMetadata::operational_internal_server_error()).context("Shutting down long poll request").into())
        },
    }
}
