//! Usage / Databricks-backed analytics stubs.

use axum::{
    extract::Path,
    routing::get,
    Json,
    Router,
};
use orchestrator_api_types::stubs::StubUsageQueryResponse;

use crate::{
    auth::identity::AuthIdentity,
    errors::ApiResult,
    state::OrchestratorState,
    stub_data,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/teams/{team_id}/usage/team_usage_state",
            get(team_usage_state),
        )
        .route(
            "/teams/{team_id}/usage/current_billing_period",
            get(billing_period),
        )
        .route("/teams/{team_id}/usage/get_token_info", get(token_info))
        .route("/teams/{team_id}/usage/query", get(usage_query))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/usage/team_usage_state",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: { state: \"Default\" }")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn team_usage_state(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"state": "Default"})))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/usage/current_billing_period",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: synthetic 1-year billing window starting now")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn billing_period(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = crate::time::now_unix_ms() as f64;
    Ok(Json(serde_json::json!({
        "start": now,
        "end": now + 31_536_000_000.0_f64,
    })))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/usage/get_token_info",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: empty object")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn token_info(
    _auth: AuthIdentity,
    Path(_team_slug): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/usage/query",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = StubUsageQueryResponse, description = "stub: empty usage rows")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn usage_query(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<StubUsageQueryResponse>> {
    Ok(Json(stub_data::empty_usage()))
}
