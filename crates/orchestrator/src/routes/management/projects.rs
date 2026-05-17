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
use orchestrator_api_types::management::{
    PlatformCreateProjectArgs,
    PlatformCreateProjectResponse,
    PlatformProjectDetails,
};

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    ids::slugify,
    routes::helpers::project_to_platform,
    state::OrchestratorState,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/teams/{team_id}/create_project",
            post(create_project),
        )
        .route("/teams/{team_id}/list_projects", get(list_projects))
        .route("/projects/{project_id}", get(get_project))
        .route(
            "/teams/{team_id_or_slug}/projects/{project_slug}",
            get(get_project_by_slug),
        )
        .route("/projects/{project_id}/delete", post(delete_project))
}

#[utoipa::path(
    post,
    path = "/v1/teams/{team_id}/create_project",
    params(("team_id" = i64, Path)),
    request_body = PlatformCreateProjectArgs,
    responses(
        (status = 200, body = PlatformCreateProjectResponse),
        (status = 401),
        (status = 404, description = "team not found"),
    ),
    tag = "projects",
)]
pub(crate) async fn create_project(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<PlatformCreateProjectArgs>,
) -> ApiResult<Json<PlatformCreateProjectResponse>> {
    let _ = auth;
    let slug = args.slug.unwrap_or_else(|| slugify(&args.project_name));
    let project = state
        .storage
        .create_project(team_id, &args.project_name, &slug, false)
        .await
        .map_err(ApiError::Internal)?;
    let team = state
        .storage
        .get_team(team_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("team {team_id}")))?;
    Ok(Json(PlatformCreateProjectResponse {
        project_id: project.id as u64,
        project_slug: project.slug,
        team_slug: team.slug,
        deployment_name: None,
        url: None,
        admin_key: None,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id}/list_projects",
    params(("team_id" = i64, Path)),
    responses(
        (status = 200, body = Vec<PlatformProjectDetails>),
        (status = 401),
    ),
    tag = "projects",
)]
pub(crate) async fn list_projects(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<Vec<PlatformProjectDetails>>> {
    let rows = state
        .storage
        .list_projects(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(rows.iter().map(project_to_platform).collect()))
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}",
    params(("project_id" = i64, Path)),
    responses(
        (status = 200, body = PlatformProjectDetails),
        (status = 404, description = "project not found"),
    ),
    tag = "projects",
)]
pub(crate) async fn get_project(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<PlatformProjectDetails>> {
    let p = state
        .storage
        .get_project(project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("project".into()))?;
    Ok(Json(project_to_platform(&p)))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id_or_slug}/projects/{project_slug}",
    params(
        ("team_id_or_slug" = String, Path, description = "numeric team id or slug"),
        ("project_slug" = String, Path),
    ),
    responses(
        (status = 200, body = PlatformProjectDetails),
        (status = 404, description = "team or project not found"),
    ),
    tag = "projects",
)]
pub(crate) async fn get_project_by_slug(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path((team_id_or_slug, project_slug)): Path<(String, String)>,
) -> ApiResult<Json<PlatformProjectDetails>> {
    // Accept either numeric id or slug.
    let team_id = if let Ok(id) = team_id_or_slug.parse::<i64>() {
        id
    } else {
        state
            .storage
            .get_team_by_slug(&team_id_or_slug)
            .await
            .map_err(ApiError::Internal)?
            .ok_or_else(|| ApiError::NotFound(format!("team {team_id_or_slug}")))?
            .id
    };
    let p = state
        .storage
        .get_project_by_slug(team_id, &project_slug)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("project".into()))?;
    Ok(Json(project_to_platform(&p)))
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/delete",
    params(("project_id" = i64, Path)),
    responses(
        (status = 200, description = "project and dependents deleted"),
    ),
    tag = "projects",
)]
pub(crate) async fn delete_project(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<StatusCode> {
    crate::routes::helpers::cascade_delete_project(&state, project_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}
