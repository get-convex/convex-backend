//! Cloud backup endpoints — self-hosted users use the backend's existing
//! backup APIs instead.

use axum::{
    extract::Path,
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
        .route(
            "/deployments/{deployment_id}/request_cloud_backup",
            post(not_implemented),
        )
        .route(
            "/deployments/{deployment_id}/restore_from_cloud_backup",
            post(not_implemented),
        )
        .route(
            "/deployments/{deployment_id}/list_cloud_backups",
            get(empty_list),
        )
        .route(
            "/cloud_backups/{cloud_backup_id}/delete",
            post(not_implemented),
        )
        .route("/cloud_backups/{cloud_backup_id}", get(not_implemented_obj))
        .route(
            "/cloud_backups/{cloud_backup_id}/cancel",
            post(not_implemented),
        )
        .route(
            "/deployments/{deployment_id}/get_periodic_backup_config",
            get(empty_object),
        )
        .route(
            "/deployments/{deployment_id}/configure_periodic_backup",
            post(not_implemented),
        )
        .route(
            "/deployments/{deployment_id}/disable_periodic_backup",
            post(not_implemented),
        )
}

// All cloud-backup endpoints are stubs in self-hosted: operators use the
// backend's `/api/snapshot_export` flow instead. Each unique handler is
// annotated with its primary route below; alias routes (e.g. /restore_from_*,
// /cloud_backups/*/delete, etc.) share the same response shape.

#[utoipa::path(
    post,
    path = "/api/dashboard/deployments/{deployment_id}/request_cloud_backup",
    params(("deployment_id" = String, Path)),
    responses((status = 501, description = "stub: use the backend's /api/snapshot_export flow. Also bound to /restore_from_cloud_backup, /cloud_backups/{id}/delete, /cloud_backups/{id}/cancel, /configure_periodic_backup, /disable_periodic_backup.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn not_implemented(
    _auth: AuthIdentity,
    Path(_id): Path<String>,
) -> ApiResult<StatusCode> {
    Err(ApiError::NotImplemented(
        "use the backend's /api/snapshot_export endpoints for self-hosted backups".into(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/cloud_backups/{cloud_backup_id}",
    params(("cloud_backup_id" = String, Path)),
    responses((status = 501, description = "stub: use the backend's /api/snapshot_export flow")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn not_implemented_obj(
    _auth: AuthIdentity,
    Path(_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::NotImplemented(
        "use the backend's /api/snapshot_export endpoints for self-hosted backups".into(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/deployments/{deployment_id}/list_cloud_backups",
    params(("deployment_id" = String, Path)),
    responses((status = 200, description = "stub: empty list")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_list(
    _auth: AuthIdentity,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(Vec::new()))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/deployments/{deployment_id}/get_periodic_backup_config",
    params(("deployment_id" = String, Path)),
    responses((status = 200, description = "stub: empty config object")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_object(
    _auth: AuthIdentity,
    Path(_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}
