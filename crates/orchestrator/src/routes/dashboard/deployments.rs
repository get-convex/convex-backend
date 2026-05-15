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
    DeploymentResponse,
    GetDeploymentAuthDashboardResponse,
    RegisterDeploymentArgs,
};

use crate::{
    auth::{
        identity::AuthIdentity,
        tokens::{
            sha256_hex,
            suffix_of,
        },
    },
    errors::{
        ApiError,
        ApiResult,
    },
    ids::random_id,
    routes::helpers::deployment_to_response,
    state::OrchestratorState,
    storage::{
        access_tokens::NewAccessToken,
        deployments::NewDeployment,
        AccessTokenKind,
        DeploymentClass,
        DeploymentType,
    },
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/teams/{team_id}/deployments/{deployment_id}",
            get(get_deployment_by_id),
        )
        .route(
            "/instances/{deployment_name}/auth",
            post(get_deployment_auth_dashboard),
        )
        .route("/deployments/register", post(register_deployment))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/deployments/{deployment_id}",
    params(("team_id" = i64, Path), ("deployment_id" = i64, Path)),
    responses(
        (status = 200, body = DeploymentResponse),
        (status = 404),
    ),
    tag = "dashboard",
)]
pub(crate) async fn get_deployment_by_id(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path((_team_id, deployment_id)): Path<(i64, i64)>,
) -> ApiResult<Json<DeploymentResponse>> {
    let d = state
        .storage
        .get_deployment(deployment_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    Ok(Json(deployment_to_response(&d)))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/instances/{deployment_name}/auth",
    params(("deployment_name" = String, Path)),
    responses(
        (status = 200, body = GetDeploymentAuthDashboardResponse),
        (status = 403),
        (status = 404),
    ),
    tag = "dashboard",
)]
pub(crate) async fn get_deployment_auth_dashboard(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
) -> ApiResult<Json<GetDeploymentAuthDashboardResponse>> {
    let _ = auth;
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    let admin_key = crate::routes::deployment_internal::ephemeral_admin_key(&state, &d)
        .await
        .ok_or(ApiError::Forbidden)?;
    Ok(Json(GetDeploymentAuthDashboardResponse {
        admin_key,
        url: d.url,
    }))
}

/// Register a pre-provisioned backend with the orchestrator. Used by
/// operators running in `--provisioner external` mode (the default).
///
/// The orchestrator stores the deployment metadata + a SHA-256 hash of the
/// admin key plus its visible suffix; the plaintext admin key is **not**
/// retained. After registration, dashboard and CLI lookups that hit the
/// admin key path return the value the operator passed here.
#[utoipa::path(
    post,
    path = "/api/dashboard/deployments/register",
    request_body = RegisterDeploymentArgs,
    responses(
        (status = 201, body = DeploymentResponse),
        (status = 400, description = "invalid args (missing previewIdentifier, unknown deployment type, or duplicate deployment name)"),
        (status = 403, description = "caller is not a team member"),
        (status = 404, description = "project not found"),
    ),
    tag = "dashboard",
)]
pub(crate) async fn register_deployment(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<RegisterDeploymentArgs>,
) -> ApiResult<(StatusCode, Json<DeploymentResponse>)> {
    let member_id = auth.require_member()?;

    // Caller must belong to the project's team.
    let project_id = i64::try_from(args.project_id)
        .map_err(|_| ApiError::BadRequest(format!("invalid projectId {}", args.project_id)))?;
    let project = state
        .storage
        .get_project(project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("project {project_id}")))?;
    if state
        .storage
        .get_team_role(project.team_id, member_id)
        .await
        .map_err(ApiError::Internal)?
        .is_none()
    {
        return Err(ApiError::Forbidden);
    }

    let dt: DeploymentType = args.deployment_type.parse().map_err(|_| {
        ApiError::BadRequest(format!("unknown deployment type {}", args.deployment_type))
    })?;
    if matches!(dt, DeploymentType::Preview) && args.preview_identifier.is_none() {
        return Err(ApiError::BadRequest(
            "previewIdentifier is required when deploymentType is `preview`".into(),
        ));
    }

    // Reject re-registering an existing deployment by name; the operator
    // should delete + re-create rather than silently overwriting state.
    if state
        .storage
        .get_deployment_by_name(&args.deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .is_some()
    {
        return Err(ApiError::BadRequest(format!(
            "deployment {} already registered",
            args.deployment_name
        )));
    }

    // The operator's admin key is the literal value backends accept; we store
    // it verbatim in `instance_secret` so dashboard auth lookups can return
    // it without minting an ephemeral key.
    let new = state
        .storage
        .create_deployment(NewDeployment {
            project_id: project.id,
            name: &args.deployment_name,
            deployment_type: dt,
            deployment_class: DeploymentClass::Standard,
            region: args.region.as_deref(),
            url: &args.url,
            site_url: &args.site_url,
            backend_pid: None,
            backend_port: 0,
            creator_id: Some(member_id),
            preview_identifier: args.preview_identifier.as_deref(),
            instance_secret: &args.admin_key,
        })
        .await
        .map_err(ApiError::Internal)?;

    // Also persist the admin key as an access_token row for the CLI's
    // deploy-key flow, so `deploy_key_for_deployment` lookups return
    // something sensible.
    let kind = match dt {
        DeploymentType::Prod => AccessTokenKind::DeployProd,
        DeploymentType::Dev => AccessTokenKind::DeployDev,
        DeploymentType::Preview => AccessTokenKind::DeployPreview,
    };
    let public_id = random_id();
    let secret_hash = sha256_hex(&args.admin_key);
    let secret_suffix = suffix_of(&args.admin_key);
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind,
            member_id: Some(member_id),
            team_id: Some(project.team_id),
            project_id: Some(project.id),
            deployment_id: Some(new.id),
            name: "registered",
            secret_hash: &secret_hash,
            secret_suffix: &secret_suffix,
            expiry: None,
        })
        .await
        .map_err(ApiError::Internal)?;

    let _ = state
        .storage
        .append_audit(
            project.team_id,
            Some(member_id),
            "registerExternalDeployment",
            &serde_json::json!({
                "deployment_id": new.id,
                "deployment_name": new.name,
                "project_id": project.id,
            }),
        )
        .await;

    Ok((StatusCode::CREATED, Json(deployment_to_response(&new))))
}
