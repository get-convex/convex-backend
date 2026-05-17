use axum::{
    extract::{
        Path,
        Query,
        State,
    },
    routing::get,
    Json,
    Router,
};
use orchestrator_api_types::dashboard::{
    AuditLogEvent,
    AuditLogPage,
};
use serde::Deserialize;

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    state::OrchestratorState,
    storage::AuditQuery,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new().route(
        "/teams/{team_id}/get_audit_log_events",
        get(get_audit_log_events),
    )
}

#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub(crate) struct AuditFilters {
    pub member_id: Option<i64>,
    pub action: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/get_audit_log_events",
    params(("team_id" = i64, Path), AuditFilters),
    responses((status = 200, body = AuditLogPage)),
    tag = "dashboard",
)]
pub(crate) async fn get_audit_log_events(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Query(filters): Query<AuditFilters>,
) -> ApiResult<Json<AuditLogPage>> {
    let q = AuditQuery {
        team_id,
        member_id: filters.member_id,
        action: filters.action,
        from: filters.from,
        to: filters.to,
        limit: filters.limit,
    };
    let entries = state.storage.query_audit(&q).await.map_err(ApiError::Internal)?;
    Ok(Json(AuditLogPage {
        events: entries
            .into_iter()
            .map(|e| AuditLogEvent {
                id: e.id as u64,
                team_id: e.team_id as u64,
                member_id: e.member_id.map(|m| m as u64),
                action: e.action,
                metadata: e.metadata,
                creation_time: e.creation_time as f64,
            })
            .collect(),
        cursor: None,
    }))
}
