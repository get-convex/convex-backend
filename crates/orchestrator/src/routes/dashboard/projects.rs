use axum::{
    extract::{
        Path,
        State,
    },
    http::StatusCode,
    routing::{
        get,
        post,
        put,
    },
    Json,
    Router,
};
use orchestrator_api_types::dashboard::{
    ProjectResponse,
    UpdateProjectArgs,
};

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    routes::helpers::project_to_response,
    state::OrchestratorState,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/teams/{team_id}/projects", get(list_projects))
        .route(
            "/teams/{team_id}/projects/{project_slug}",
            get(get_project_by_slug),
        )
        .route(
            "/projects/{project_id}",
            get(get_project).put(update_project),
        )
        .route("/delete_projects", post(delete_projects))
        .route("/projects/{project_id}/transfer", post(transfer_project))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/projects",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = Vec<ProjectResponse>)),
    tag = "dashboard",
)]
pub(crate) async fn list_projects(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<Vec<ProjectResponse>>> {
    let projects = state
        .storage
        .list_projects(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(projects.iter().map(project_to_response).collect()))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/projects/{project_slug}",
    params(("team_id" = i64, Path), ("project_slug" = String, Path)),
    responses((status = 200, body = ProjectResponse), (status = 404)),
    tag = "dashboard",
)]
pub(crate) async fn get_project_by_slug(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path((team_id, project_slug)): Path<(i64, String)>,
) -> ApiResult<Json<ProjectResponse>> {
    let p = state
        .storage
        .get_project_by_slug(team_id, &project_slug)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("project".into()))?;
    Ok(Json(project_to_response(&p)))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/projects/{project_id}",
    params(("project_id" = i64, Path)),
    responses((status = 200, body = ProjectResponse), (status = 404)),
    tag = "dashboard",
)]
pub(crate) async fn get_project(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<ProjectResponse>> {
    let p = state
        .storage
        .get_project(project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("project".into()))?;
    Ok(Json(project_to_response(&p)))
}

#[utoipa::path(
    put,
    path = "/api/dashboard/projects/{project_id}",
    params(("project_id" = i64, Path)),
    request_body = UpdateProjectArgs,
    responses((status = 200)),
    tag = "dashboard",
)]
pub(crate) async fn update_project(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
    Json(args): Json<UpdateProjectArgs>,
) -> ApiResult<StatusCode> {
    state
        .storage
        .update_project(project_id, args.name.as_deref(), args.slug.as_deref())
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/delete_projects",
    request_body = Vec<i64>,
    responses((status = 200)),
    tag = "dashboard",
)]
pub(crate) async fn delete_projects(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(ids): Json<Vec<i64>>,
) -> ApiResult<StatusCode> {
    for id in ids {
        crate::routes::helpers::cascade_delete_project(&state, id)
            .await
            .map_err(ApiError::Internal)?;
    }
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/projects/{project_id}/transfer",
    params(("project_id" = i64, Path)),
    responses((status = 501, description = "not implemented in v1")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn transfer_project(
    _auth: AuthIdentity,
    Path(_project_id): Path<i64>,
    Json(_args): Json<serde_json::Value>,
) -> ApiResult<StatusCode> {
    Err(ApiError::NotImplemented(
        "project transfer not implemented in v1".into(),
    ))
}

#[allow(dead_code)]
fn _put() -> axum::routing::MethodRouter<crate::state::OrchestratorState> {
    put(|| async { StatusCode::OK })
}
