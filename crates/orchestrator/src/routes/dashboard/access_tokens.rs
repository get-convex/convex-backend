use axum::{
    extract::{
        Path,
        State,
    },
    http::StatusCode,
    routing::{
        get,
        post,
    },
    Json,
    Router,
};
use orchestrator_api_types::dashboard::AccessTokenResponse;

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    state::OrchestratorState,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/teams/{team_id}/access_tokens", get(list_team_tokens))
        .route("/teams/{team_id}/app_access_tokens", get(list_team_tokens))
        .route(
            "/projects/{project_id}/access_tokens",
            get(list_project_tokens),
        )
        .route(
            "/projects/{project_id}/app_access_tokens",
            get(list_project_tokens),
        )
        .route(
            "/instances/{deployment_name}/access_tokens",
            get(list_deployment_tokens),
        )
        .route("/teams/delete_access_token", post(delete_access_token))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteArgs {
    pub id: String,
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/access_tokens",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = Vec<AccessTokenResponse>)),
    tag = "dashboard",
)]
pub(crate) async fn list_team_tokens(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<Vec<AccessTokenResponse>>> {
    let tokens = state
        .storage
        .list_access_tokens_by_team(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        tokens
            .into_iter()
            .map(|t| AccessTokenResponse {
                id: t.public_id,
                kind: t.kind.to_string(),
                name: t.name,
                creation_time: t.creation_time as f64,
                key_suffix: t.secret_suffix,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/projects/{project_id}/access_tokens",
    params(("project_id" = i64, Path)),
    responses((status = 200, body = Vec<AccessTokenResponse>)),
    tag = "dashboard",
)]
pub(crate) async fn list_project_tokens(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<Vec<AccessTokenResponse>>> {
    let tokens = state
        .storage
        .list_access_tokens_by_project(project_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        tokens
            .into_iter()
            .map(|t| AccessTokenResponse {
                id: t.public_id,
                kind: t.kind.to_string(),
                name: t.name,
                creation_time: t.creation_time as f64,
                key_suffix: t.secret_suffix,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/instances/{deployment_name}/access_tokens",
    params(("deployment_name" = String, Path)),
    responses((status = 200, body = Vec<AccessTokenResponse>), (status = 404)),
    tag = "dashboard",
)]
pub(crate) async fn list_deployment_tokens(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
) -> ApiResult<Json<Vec<AccessTokenResponse>>> {
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    let tokens = state
        .storage
        .list_access_tokens_by_deployment(d.id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        tokens
            .into_iter()
            .map(|t| AccessTokenResponse {
                id: t.public_id,
                kind: t.kind.to_string(),
                name: t.name,
                creation_time: t.creation_time as f64,
                key_suffix: t.secret_suffix,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/delete_access_token",
    request_body = DeleteArgs,
    responses((status = 200)),
    tag = "dashboard",
)]
pub(crate) async fn delete_access_token(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<DeleteArgs>,
) -> ApiResult<StatusCode> {
    state
        .storage
        .revoke_access_token(&args.id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}
