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
use orchestrator_api_types::dashboard::{
    ListEnvironmentVariables,
    UpdateDefaultEnvVarsArgs,
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
            "/projects/{project_id}/environment_variables/list",
            get(list_env_vars),
        )
        .route(
            "/projects/{project_id}/environment_variables/update_batch",
            post(update_env_vars),
        )
}

#[utoipa::path(
    get,
    path = "/api/dashboard/projects/{project_id}/environment_variables/list",
    params(("project_id" = i64, Path)),
    responses((status = 200, body = ListEnvironmentVariables)),
    tag = "dashboard",
)]
pub(crate) async fn list_env_vars(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<ListEnvironmentVariables>> {
    let vars = state
        .storage
        .list_default_env_vars(project_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(ListEnvironmentVariables {
        variables: vars
            .into_iter()
            .map(|v| orchestrator_api_types::dashboard::EnvironmentVariable {
                name: v.name,
                value: v.value,
                deployment_types: v.deployment_types,
            })
            .collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/projects/{project_id}/environment_variables/update_batch",
    params(("project_id" = i64, Path)),
    request_body = UpdateDefaultEnvVarsArgs,
    responses((status = 200)),
    tag = "dashboard",
)]
pub(crate) async fn update_env_vars(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
    Json(args): Json<UpdateDefaultEnvVarsArgs>,
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
