//! Discord, Vercel marketplace, WorkOS / SSO, OAuth-app stubs.

use axum::{
    http::StatusCode,
    routing::{
        get,
        post,
    },
    Json,
    Router,
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
        .route("/discord/accounts", get(empty_list))
        .route("/discord/login_url", get(empty_url))
        .route("/discord/authorize", post(ok))
        .route("/discord/unlink", post(ok))
        .route("/vercel/potential_teams", get(empty_list))
        .route(
            "/vercel/potential_teams/{proposed_team_id}/join",
            post(not_configured),
        )
        .route("/teams/{team_id}/oauth_apps", get(empty_list))
        .route("/teams/{team_id}/oauth_apps/check", post(ok_obj))
        .route("/teams/{team_id}/oauth_apps/register", post(not_configured))
        .route(
            "/teams/{team_id}/oauth_apps/{client_id}/update",
            post(not_configured),
        )
        .route(
            "/teams/{team_id}/oauth_apps/{client_id}/delete",
            post(not_configured),
        )
        .route(
            "/teams/{team_id}/oauth_apps/{client_id}/regenerate_secret",
            post(not_configured),
        )
        .route("/workos/get_or_provision_workos_environment", post(not_configured))
        .route("/workos/delete_environment", post(not_configured))
        .route("/workos/disconnect_workos_team", post(not_configured))
        .route("/workos/invite_team_member", post(not_configured))
        .route("/workos/provision_associated_workos_team", post(not_configured))
        .route("/workos/delete_project_environment", post(not_configured))
        .route("/workos/check_project_environment_health", post(ok_obj))
        .route("/workos/available_workos_team_emails", get(empty_list))
        .route(
            "/deployments/{deployment_name}/workos_environment",
            get(empty_object),
        )
        .route(
            "/deployments/{deployment_name}/has_associated_workos_team",
            get(false_object),
        )
        .route(
            "/deployments/{deployment_name}/workos_environment_health",
            get(empty_object),
        )
        .route("/teams/{team_id}/workos_integration", get(empty_object))
        .route(
            "/teams/{team_id}/workos_invitation_eligible_emails",
            get(empty_list),
        )
        .route("/teams/{team_id}/workos_team_health", get(empty_object))
        .route(
            "/projects/{project_id}/workos_environments",
            get(empty_list).post(not_configured),
        )
        .route(
            "/projects/{project_id}/workos_environments/{client_id}",
            get(empty_object).post(not_configured),
        )
        .route("/teams/{team_id}/enable_sso", post(not_configured))
        .route("/teams/{team_id}/get_sso", get(empty_object))
        .route("/teams/{team_id}/disable_sso", post(not_configured))
        .route("/teams/{team_id}/update_sso", post(not_configured))
        .route(
            "/teams/{team_id}/generate_sso_configuration_link",
            post(not_configured),
        )
}

// Discord, Vercel, WorkOS, OAuth-app stubs. Many handlers below back several
// routes (a single `empty_list` handles `/discord/accounts`,
// `/vercel/potential_teams`, `/teams/{id}/oauth_apps`, etc.); see the router
// definition above for the full set of bindings.

#[utoipa::path(
    get,
    path = "/api/dashboard/discord/accounts",
    responses((status = 200, description = "stub: empty list. Same handler backs vercel/oauth_apps/workos list endpoints.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_list(
    _auth: AuthIdentity,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(Vec::new()))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/deployments/{deployment_name}/workos_environment",
    params(("deployment_name" = String, Path)),
    responses((status = 200, description = "stub: empty JSON object. Backs several workos_*/get_sso reads.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_object(
    _auth: AuthIdentity,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/discord/login_url",
    responses((status = 200, description = "stub: { url: \"\" }")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_url(
    _auth: AuthIdentity,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"url": ""})))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/deployments/{deployment_name}/has_associated_workos_team",
    params(("deployment_name" = String, Path)),
    responses((status = 200, description = "stub: { value: false }")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn false_object(
    _auth: AuthIdentity,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"value": false})))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/discord/authorize",
    responses((status = 200, description = "stub no-op. Also backs /discord/unlink.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn ok(
    _auth: AuthIdentity,
) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/oauth_apps/check",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: empty object. Also backs /workos/check_project_environment_health.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn ok_obj(
    _auth: AuthIdentity,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/oauth_apps/register",
    params(("team_id" = i64, Path)),
    responses((status = 501, description = "stub: feature is Cloud-only. Backs every WorkOS/SSO write + Vercel join.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn not_configured(
    _auth: AuthIdentity,
) -> ApiResult<StatusCode> {
    Err(ApiError::NotImplemented(
        "this feature requires Convex Cloud and is not available in self-hosted deployments"
            .into(),
    ))
}
