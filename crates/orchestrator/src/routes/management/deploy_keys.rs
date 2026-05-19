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
    PlatformCreateDeployKeyArgs,
    PlatformCreateDeployKeyResponse,
    PlatformCreatePreviewDeployKeyArgs,
    PlatformCreatePreviewDeployKeyResponse,
    PlatformDeleteDeployKeyArgs,
    PlatformDeletePreviewDeployKeyArgs,
    PlatformDeployKeyResponse,
    PlatformListPreviewDeployKeysResponse,
};

use crate::{
    auth::{
        deploy_keys::encode_deploy_key,
        identity::AuthIdentity,
        tokens::{
            mint_token_secret,
            sha256_hex,
            suffix_of,
        },
    },
    errors::{
        ApiError,
        ApiResult,
    },
    ids::random_id,
    state::OrchestratorState,
    storage::{
        access_tokens::NewAccessToken,
        AccessTokenKind,
        DeploymentType,
    },
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/deployments/{deployment_name}/create_deploy_key",
            post(create_deploy_key),
        )
        .route(
            "/deployments/{deployment_name}/list_deploy_keys",
            get(list_deploy_keys),
        )
        .route(
            "/deployments/{deployment_name}/delete_deploy_key",
            post(delete_deploy_key),
        )
        .route(
            "/projects/{project_id}/create_preview_deploy_key",
            post(create_preview_deploy_key),
        )
        .route(
            "/projects/{project_id}/list_preview_deploy_keys",
            get(list_preview_deploy_keys),
        )
        .route(
            "/projects/{project_id}/delete_preview_deploy_key",
            post(delete_preview_deploy_key),
        )
}

#[utoipa::path(
    post,
    path = "/v1/deployments/{deployment_name}/create_deploy_key",
    params(("deployment_name" = String, Path)),
    request_body = PlatformCreateDeployKeyArgs,
    responses(
        (status = 200, body = PlatformCreateDeployKeyResponse),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deploy_keys",
)]
pub(crate) async fn create_deploy_key(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
    Json(args): Json<PlatformCreateDeployKeyArgs>,
) -> ApiResult<Json<PlatformCreateDeployKeyResponse>> {
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    let kind_prefix = match d.deployment_type {
        DeploymentType::Prod => "prod",
        DeploymentType::Dev => "dev",
        DeploymentType::Preview => "preview",
    };
    let kind_token = match d.deployment_type {
        DeploymentType::Prod => AccessTokenKind::DeployProd,
        DeploymentType::Dev => AccessTokenKind::DeployDev,
        DeploymentType::Preview => AccessTokenKind::DeployPreview,
    };
    let public_id = random_id();
    let secret = mint_token_secret(&public_id);
    let key = encode_deploy_key(kind_prefix, &deployment_name, &secret.secret);
    let hash = sha256_hex(&secret.secret);
    let suffix = suffix_of(&secret.secret);
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind: kind_token,
            member_id: None,
            team_id: None,
            project_id: Some(d.project_id),
            deployment_id: Some(d.id),
            name: &args.name,
            secret_hash: &hash,
            secret_suffix: &suffix,
            expiry: args.expires_at.map(|f| f as i64),
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(PlatformCreateDeployKeyResponse {
        key,
        id: public_id,
        name: args.name,
        creation_time: crate::time::now_unix_ms() as f64,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/deployments/{deployment_name}/list_deploy_keys",
    params(("deployment_name" = String, Path)),
    responses(
        (status = 200, body = Vec<PlatformDeployKeyResponse>),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deploy_keys",
)]
pub(crate) async fn list_deploy_keys(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
) -> ApiResult<Json<Vec<PlatformDeployKeyResponse>>> {
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
            .map(|t| PlatformDeployKeyResponse {
                id: t.public_id,
                name: t.name,
                creation_time: t.creation_time as f64,
                key_suffix: t.secret_suffix,
                expires_at: t.expiry.map(|v| v as f64),
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/deployments/{deployment_name}/delete_deploy_key",
    params(("deployment_name" = String, Path)),
    request_body = PlatformDeleteDeployKeyArgs,
    responses(
        (status = 200, description = "deploy key revoked"),
    ),
    tag = "deploy_keys",
)]
pub(crate) async fn delete_deploy_key(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(_deployment_name): Path<String>,
    Json(args): Json<PlatformDeleteDeployKeyArgs>,
) -> ApiResult<StatusCode> {
    state
        .storage
        .revoke_access_token(&args.id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/create_preview_deploy_key",
    params(("project_id" = i64, Path)),
    request_body = PlatformCreatePreviewDeployKeyArgs,
    responses(
        (status = 200, body = PlatformCreatePreviewDeployKeyResponse),
    ),
    tag = "deploy_keys",
)]
pub(crate) async fn create_preview_deploy_key(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
    Json(args): Json<PlatformCreatePreviewDeployKeyArgs>,
) -> ApiResult<Json<PlatformCreatePreviewDeployKeyResponse>> {
    // The Convex CLI's `isPreviewDeployKey` requires the prefix to split
    // into exactly three colon-separated parts: `preview:<team>:<project>`.
    // It then routes by extracting team/project slugs from those parts and
    // calling `authorize_preview` on the orchestrator. Mint the key in
    // that exact shape so the CLI accepts it and we get the team/project
    // identifiers back on auth (validated against `token.project_id`).
    let project = state
        .storage
        .get_project(project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("project {project_id}")))?;
    let team = state
        .storage
        .get_team(project.team_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("team {}", project.team_id)))?;
    let public_id = random_id();
    let secret = mint_token_secret(&public_id);
    let key = format!(
        "preview:{}:{}|{}",
        team.slug, project.slug, secret.secret
    );
    let hash = sha256_hex(&secret.secret);
    let suffix = suffix_of(&secret.secret);
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind: AccessTokenKind::ProjectDeploy,
            member_id: None,
            team_id: None,
            project_id: Some(project_id),
            deployment_id: None,
            name: &args.name,
            secret_hash: &hash,
            secret_suffix: &suffix,
            expiry: None,
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(PlatformCreatePreviewDeployKeyResponse {
        key,
        id: public_id,
        name: args.name,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}/list_preview_deploy_keys",
    params(("project_id" = i64, Path)),
    responses(
        (status = 200, body = PlatformListPreviewDeployKeysResponse),
    ),
    tag = "deploy_keys",
)]
pub(crate) async fn list_preview_deploy_keys(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<PlatformListPreviewDeployKeysResponse>> {
    let tokens = state
        .storage
        .list_access_tokens_by_project(project_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(PlatformListPreviewDeployKeysResponse {
        keys: tokens
            .into_iter()
            .filter(|t| t.kind == AccessTokenKind::ProjectDeploy)
            .map(|t| PlatformDeployKeyResponse {
                id: t.public_id,
                name: t.name,
                creation_time: t.creation_time as f64,
                key_suffix: t.secret_suffix,
                expires_at: t.expiry.map(|v| v as f64),
            })
            .collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/delete_preview_deploy_key",
    params(("project_id" = i64, Path)),
    request_body = PlatformDeletePreviewDeployKeyArgs,
    responses(
        (status = 200, description = "preview deploy key revoked"),
    ),
    tag = "deploy_keys",
)]
pub(crate) async fn delete_preview_deploy_key(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(_project_id): Path<i64>,
    Json(args): Json<PlatformDeletePreviewDeployKeyArgs>,
) -> ApiResult<StatusCode> {
    state
        .storage
        .revoke_access_token(&args.id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}
