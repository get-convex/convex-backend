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
    PaginatedDefaultEnvironmentVariablesResponse,
    PlatformDefaultEnvVar,
    UpdateDefaultEnvironmentVariablesArgs,
};

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
        .route(
            "/projects/{project_id}/list_default_environment_variables",
            get(list),
        )
        .route(
            "/projects/{project_id}/update_default_environment_variables",
            post(update),
        )
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}/list_default_environment_variables",
    params(("project_id" = i64, Path)),
    responses(
        (status = 200, body = PaginatedDefaultEnvironmentVariablesResponse),
    ),
    tag = "env_vars",
)]
pub(crate) async fn list(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<PaginatedDefaultEnvironmentVariablesResponse>> {
    let rows = state
        .storage
        .list_default_env_vars(project_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(PaginatedDefaultEnvironmentVariablesResponse {
        variables: rows
            .into_iter()
            .map(|v| PlatformDefaultEnvVar {
                name: v.name,
                value: v.value,
                deployment_types: v.deployment_types,
            })
            .collect(),
        cursor: None,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/update_default_environment_variables",
    params(("project_id" = i64, Path)),
    request_body = UpdateDefaultEnvironmentVariablesArgs,
    responses(
        (status = 200, description = "variables upserted"),
    ),
    tag = "env_vars",
)]
pub(crate) async fn update(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
    Json(args): Json<UpdateDefaultEnvironmentVariablesArgs>,
) -> ApiResult<StatusCode> {
    for v in args.variables {
        state
            .storage
            .upsert_default_env_var(project_id, &v.name, &v.value, &v.deployment_types)
            .await
            .map_err(ApiError::Internal)?;
    }
    Ok(StatusCode::OK)
}
