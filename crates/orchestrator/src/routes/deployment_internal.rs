//! Deployment-internal endpoints under `/api/...` (after the `/api` prefix
//! is stripped by the router).
//!
//! These are the load-bearing endpoints used by `crates/big_brain_client`
//! and the CLI's `bigBrainAPI` calls.

use axum::{
    extract::{
        Path,
        State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{
        get,
        post,
    },
    Json,
    Router,
};
use orchestrator_api_types::{
    dashboard::{
        DeviceAuthorizeArgs,
        DeviceAuthorizeResponse,
        OptIn,
    },
    deployment::{
        ClaimPreviewDeploymentArgs,
        ClaimPreviewDeploymentResponse,
        CreateProjectArgs,
        CreateProjectResponse,
        DeploymentAuthResponse,
        DeploymentAuthWithinCurrentProjectArgs,
        HasProjectsResponse,
        ProjectSelectionArgs,
        ProvisionAndAuthorizeArgs,
        TeamAndProjectForDeploymentResponse,
        TeamSummary,
    },
};

use crate::{
    auth::{
        identity::{
            AuthIdentity,
            OptionalAuth,
        },
        tokens::{
            encode_pat,
            mint_token_secret,
            parse_token,
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
        AccessTokenKind,
        DeploymentClass,
        DeploymentType,
    },
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/version", get(orchestrator_version))
        .route("/authorize", post(authorize).head(authorize_head))
        .route("/check_opt_ins", post(check_opt_ins))
        .route("/accept_opt_ins", post(accept_opt_ins))
        .route("/teams", get(list_teams))
        .route("/has_projects", get(has_projects))
        .route("/create_project", post(create_project))
        .route("/dashboard/delete_project/{project_id}", post(delete_project))
        .route(
            "/deployment/{deployment_name}/team_and_project",
            get(team_and_project),
        )
        .route(
            "/deployment/authorize_within_current_project",
            post(authorize_within_current_project),
        )
        .route(
            "/deployment/provision_and_authorize",
            post(provision_and_authorize),
        )
        .route("/claim_preview_deployment", post(claim_preview_deployment))
}

#[utoipa::path(
    get,
    path = "/api/version",
    responses((status = 200, description = "service identity + version JSON")),
    tag = "deployment_internal",
)]
pub(crate) async fn orchestrator_version() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "service": "convex-orchestrator",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

/// CLI auth endpoint. Accepts only a bootstrap token in v1. Human users
/// authenticate through the dashboard (BetterAuth → /api/internal/exchange_session).
#[utoipa::path(
    post,
    path = "/api/authorize",
    request_body = orchestrator_api_types::dashboard::DeviceAuthorizeArgs,
    responses(
        (status = 200, body = orchestrator_api_types::dashboard::DeviceAuthorizeResponse),
        (status = 400, description = "missing bootstrapToken"),
        (status = 401, description = "wrong bootstrap token"),
    ),
    tag = "deployment_internal",
)]
pub(crate) async fn authorize(
    State(state): State<OrchestratorState>,
    Json(args): Json<DeviceAuthorizeArgs>,
) -> ApiResult<Json<DeviceAuthorizeResponse>> {
    let token = args.bootstrap_token.as_deref().ok_or_else(|| {
        ApiError::BadRequest(
            "bootstrap token required; sign in through the dashboard for human accounts".into(),
        )
    })?;
    let configured = state
        .config
        .bootstrap_token
        .as_deref()
        .ok_or(ApiError::Unauthorized)?;
    if !constant_time_eq(token, configured) {
        return Err(ApiError::Unauthorized);
    }
    let m = state
        .storage
        .get_member_by_auth_user_id(crate::state::SYSTEM_AUTH_USER_ID)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "bootstrap token configured but system member missing"
            ))
        })?;
    let access_token = mint_pat_for_member(&state, m.id, &args.device_name).await?;
    Ok(Json(DeviceAuthorizeResponse {
        access_token,
        member_id: m.id as u64,
    }))
}

pub fn constant_time_eq_str(a: &str, b: &str) -> bool {
    constant_time_eq(a, b)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[utoipa::path(
    head,
    path = "/api/authorize",
    responses((status = 200, description = "ok if bearer token is recognized")),
    tag = "deployment_internal",
)]
pub(crate) async fn authorize_head(_auth: OptionalAuth) -> impl IntoResponse {
    StatusCode::OK
}

#[utoipa::path(
    post,
    path = "/api/check_opt_ins",
    responses((status = 200, body = orchestrator_api_types::dashboard::GetOptInsResponse)),
    tag = "deployment_internal",
)]
pub(crate) async fn check_opt_ins(_auth: AuthIdentity) -> ApiResult<Json<orchestrator_api_types::dashboard::GetOptInsResponse>> {
    Ok(Json(orchestrator_api_types::dashboard::GetOptInsResponse {
        opt_ins_to_accept: Vec::new(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/accept_opt_ins",
    request_body = Vec<OptIn>,
    responses((status = 204, description = "opt-ins recorded")),
    tag = "deployment_internal",
)]
pub(crate) async fn accept_opt_ins(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(opt_ins): Json<Vec<OptIn>>,
) -> ApiResult<StatusCode> {
    let member_id = auth.require_member()?;
    for o in opt_ins {
        state
            .storage
            .accept_opt_in(member_id, &o.name)
            .await
            .map_err(ApiError::Internal)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/teams",
    responses((status = 200, body = Vec<TeamSummary>)),
    tag = "deployment_internal",
)]
pub(crate) async fn list_teams(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<Vec<TeamSummary>>> {
    let member_id = auth.require_member()?;
    let teams = state
        .storage
        .list_teams_for_member(member_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        teams
            .into_iter()
            .map(|t| TeamSummary {
                id: t.id as u64,
                name: t.name,
                slug: t.slug,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/has_projects",
    responses((status = 200, body = HasProjectsResponse)),
    tag = "deployment_internal",
)]
pub(crate) async fn has_projects(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<HasProjectsResponse>> {
    let n = state.storage.count_projects().await.map_err(ApiError::Internal)?;
    Ok(Json(HasProjectsResponse {
        has_projects: n > 0,
    }))
}

#[utoipa::path(
    post,
    path = "/api/create_project",
    request_body = CreateProjectArgs,
    responses(
        (status = 200, body = CreateProjectResponse),
        (status = 400, description = "team not found / unknown deployment type"),
        (status = 403, description = "caller is not a team member"),
    ),
    tag = "deployment_internal",
)]
pub(crate) async fn create_project(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<CreateProjectArgs>,
) -> ApiResult<Json<CreateProjectResponse>> {
    let member_id = auth.require_member()?;
    let team = state
        .storage
        .get_team_by_slug(&args.team)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::BadRequest(format!("team {} not found", args.team)))?;
    let role = state
        .storage
        .get_team_role(team.id, member_id)
        .await
        .map_err(ApiError::Internal)?;
    if role.is_none() {
        return Err(ApiError::Forbidden);
    }

    let slug = crate::ids::slugify(&args.project_name);
    let project = state
        .storage
        .create_project(team.id, &args.project_name, &slug, false)
        .await
        .map_err(ApiError::Internal)?;

    // Resolve and validate tier + knob overrides from the request.
    let tier = args
        .tier
        .as_deref()
        .unwrap_or(crate::provisioner::tiers::DEFAULT_TIER);
    if crate::provisioner::tiers::lookup(tier).is_none() {
        return Err(ApiError::BadRequest(format!("unknown tier {tier}")));
    }
    let overrides = args.knob_overrides.clone().unwrap_or_default();
    for (k, v) in &overrides {
        if let Err(e) = crate::knob_registry::validate(k, v) {
            return Err(ApiError::BadRequest(e.to_string()));
        }
    }
    let overrides_json =
        serde_json::to_value(&overrides).map_err(|e| ApiError::Internal(e.into()))?;
    state
        .storage
        .update_project_settings(project.id, Some(tier), Some(&overrides_json))
        .await
        .map_err(ApiError::Internal)?;

    // Optionally provision a deployment.
    let mut deployment_name = None;
    let mut deployment_url = None;
    let mut admin_key_full = None;
    if let Some(dt_str) = args.deployment_type.as_deref() {
        let dt: DeploymentType = dt_str
            .parse()
            .map_err(|_| ApiError::BadRequest(format!("unknown deployment type {dt_str}")))?;
        crate::routes::management::deployments::ensure_host_capacity(&state, tier).await?;
        let name = crate::ids::random_deployment_name();
        let result = state
            .provisioner
            .provision(crate::provisioner::ProvisionRequest {
                deployment_name: name.clone(),
                deployment_type: dt,
                project_id: project.id,
                tier: tier.to_string(),
                knob_overrides: overrides.clone(),
            })
            .await
            .map_err(ApiError::Internal)?;
        let resolved_overrides = serde_json::to_value(&result.resolved_env)
            .map_err(|e| ApiError::Internal(e.into()))?;
        let new = state
            .storage
            .create_deployment(crate::storage::deployments::NewDeployment {
                project_id: project.id,
                name: &name,
                deployment_type: dt,
                deployment_class: DeploymentClass::Standard,
                region: args.region.as_deref(),
                url: &result.url,
                site_url: &result.site_url,
                backend_pid: result.backend_pid,
                backend_port: result.backend_port,
                creator_id: Some(member_id),
                preview_identifier: None,
                instance_secret: &result.instance_secret,
                tier,
                knob_overrides: &resolved_overrides,
            })
            .await
            .map_err(ApiError::Internal)?;
        // Store the deploy key as an access_token row.
        let kind = match dt {
            DeploymentType::Prod => AccessTokenKind::DeployProd,
            DeploymentType::Dev => AccessTokenKind::DeployDev,
            DeploymentType::Preview => AccessTokenKind::DeployPreview,
        };
        let public_id = random_id();
        state
            .storage
            .create_access_token(NewAccessToken {
                public_id: &public_id,
                kind,
                member_id: Some(member_id),
                team_id: Some(team.id),
                project_id: Some(project.id),
                deployment_id: Some(new.id),
                name: "system",
                secret_hash: &result.admin_key_hash,
                secret_suffix: &result.admin_key_suffix,
                expiry: None,
            })
            .await
            .map_err(ApiError::Internal)?;
        deployment_name = Some(new.name);
        deployment_url = Some(new.url);
        admin_key_full = Some(result.admin_key);
    }

    state
        .storage
        .append_audit(
            team.id,
            Some(member_id),
            "createProject",
            &serde_json::json!({"project_id": project.id, "name": project.name}),
        )
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(CreateProjectResponse {
        project_id: project.id as u64,
        project_slug: project.slug,
        team_slug: team.slug,
        deployment_name,
        url: deployment_url,
        admin_key: admin_key_full,
    }))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/delete_project/{project_id}",
    params(("project_id" = i64, Path)),
    responses(
        (status = 200, description = "project and its dependents deleted"),
        (status = 403, description = "caller is not a team member"),
        (status = 404, description = "project not found"),
    ),
    tag = "deployment_internal",
)]
pub(crate) async fn delete_project(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<StatusCode> {
    let member_id = auth.require_member()?;
    let project = state
        .storage
        .get_project(project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("project {project_id}")))?;
    let role = state
        .storage
        .get_team_role(project.team_id, member_id)
        .await
        .map_err(ApiError::Internal)?;
    if role.is_none() {
        return Err(ApiError::Forbidden);
    }
    crate::routes::helpers::cascade_delete_project(&state, project_id)
        .await
        .map_err(ApiError::Internal)?;
    let _ = state
        .storage
        .append_audit(
            project.team_id,
            Some(member_id),
            "deleteProject",
            &serde_json::json!({"project_id": project_id}),
        )
        .await;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/deployment/{deployment_name}/team_and_project",
    params(("deployment_name" = String, Path)),
    responses(
        (status = 200, body = TeamAndProjectForDeploymentResponse),
        (status = 404, description = "deployment, project, or team not found"),
    ),
    tag = "deployment_internal",
)]
pub(crate) async fn team_and_project(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
) -> ApiResult<Json<TeamAndProjectForDeploymentResponse>> {
    let _ = auth;
    resolve_team_and_project(&state, &deployment_name).await
}

#[utoipa::path(
    post,
    path = "/api/deployment/authorize_within_current_project",
    request_body = DeploymentAuthWithinCurrentProjectArgs,
    responses(
        (status = 200, body = DeploymentAuthResponse),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deployment_internal",
)]
pub(crate) async fn authorize_within_current_project(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<DeploymentAuthWithinCurrentProjectArgs>,
) -> ApiResult<Json<DeploymentAuthResponse>> {
    let _ = auth;
    let deployment = state
        .storage
        .get_deployment_by_name(&args.selected_deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| {
            ApiError::NotFound(format!("deployment {}", args.selected_deployment_name))
        })?;
    let dt = to_common_deployment_type(deployment.deployment_type);
    let admin_key = admin_key_for_deployment(&state, &deployment)
        .await
        .ok_or(ApiError::Forbidden)?;
    Ok(Json(DeploymentAuthResponse {
        deployment_name: deployment.name.clone(),
        admin_key: admin_key.into(),
        url: deployment.url,
        deployment_type: dt,
    }))
}

#[utoipa::path(
    post,
    path = "/api/deployment/provision_and_authorize",
    request_body = ProvisionAndAuthorizeArgs,
    responses(
        (status = 200, body = DeploymentAuthResponse),
        (status = 400, description = "missing slug or unknown deployment type"),
        (status = 404, description = "team or project not found"),
    ),
    tag = "deployment_internal",
)]
pub(crate) async fn provision_and_authorize(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<ProvisionAndAuthorizeArgs>,
) -> ApiResult<Json<DeploymentAuthResponse>> {
    let member_id = auth.require_member()?;
    let team_slug = args
        .team_slug
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("teamSlug required".into()))?;
    let project_slug = args
        .project_slug
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("projectSlug required".into()))?;
    let team = state
        .storage
        .get_team_by_slug(team_slug)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("team {team_slug}")))?;
    let project = state
        .storage
        .get_project_by_slug(team.id, project_slug)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("project {project_slug}")))?;
    let dt: DeploymentType = args.deployment_type.parse().map_err(|_| {
        ApiError::BadRequest(format!("unknown deployment type {}", args.deployment_type))
    })?;
    if let Some(existing) = state
        .storage
        .find_default_deployment(project.id, dt)
        .await
        .map_err(ApiError::Internal)?
    {
        let admin_key = admin_key_for_deployment(&state, &existing)
            .await
            .ok_or(ApiError::Forbidden)?;
        return Ok(Json(DeploymentAuthResponse {
            deployment_name: existing.name.clone(),
            admin_key: admin_key.into(),
            url: existing.url,
            deployment_type: to_common_deployment_type(dt),
        }));
    }

    let name = crate::ids::random_deployment_name();
    let result = state
        .provisioner
        .provision(crate::provisioner::ProvisionRequest {
            deployment_name: name.clone(),
            deployment_type: dt,
            project_id: project.id,
            tier: crate::provisioner::tiers::DEFAULT_TIER.to_string(),
            knob_overrides: std::collections::BTreeMap::new(),
        })
        .await
        .map_err(ApiError::Internal)?;
    let new = state
        .storage
        .create_deployment(crate::storage::deployments::NewDeployment {
            project_id: project.id,
            name: &name,
            deployment_type: dt,
            deployment_class: DeploymentClass::Standard,
            region: None,
            url: &result.url,
            site_url: &result.site_url,
            backend_pid: result.backend_pid,
            backend_port: result.backend_port,
            creator_id: Some(member_id),
            preview_identifier: None,
            instance_secret: &result.instance_secret,
            tier: crate::provisioner::tiers::DEFAULT_TIER,
            knob_overrides: &serde_json::json!({}),
        })
        .await
        .map_err(ApiError::Internal)?;
    let kind = match dt {
        DeploymentType::Prod => AccessTokenKind::DeployProd,
        DeploymentType::Dev => AccessTokenKind::DeployDev,
        DeploymentType::Preview => AccessTokenKind::DeployPreview,
    };
    let public_id = random_id();
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind,
            member_id: Some(member_id),
            team_id: Some(team.id),
            project_id: Some(project.id),
            deployment_id: Some(new.id),
            name: "system",
            secret_hash: &result.admin_key_hash,
            secret_suffix: &result.admin_key_suffix,
            expiry: None,
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(DeploymentAuthResponse {
        deployment_name: new.name,
        admin_key: result.admin_key.into(),
        url: new.url,
        deployment_type: to_common_deployment_type(dt),
    }))
}

fn to_common_deployment_type(dt: DeploymentType) -> common::types::DeploymentType {
    match dt {
        DeploymentType::Prod => common::types::DeploymentType::Prod,
        DeploymentType::Dev => common::types::DeploymentType::Dev,
        DeploymentType::Preview => common::types::DeploymentType::Preview,
    }
}

#[utoipa::path(
    post,
    path = "/api/claim_preview_deployment",
    request_body = ClaimPreviewDeploymentArgs,
    responses((status = 200, body = ClaimPreviewDeploymentResponse)),
    tag = "deployment_internal",
)]
pub(crate) async fn claim_preview_deployment(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<ClaimPreviewDeploymentArgs>,
) -> ApiResult<Json<ClaimPreviewDeploymentResponse>> {
    let member_id = auth.require_member()?;
    let project = resolve_project_for_selection(&state, &args.project_selection).await?;
    // Reuse existing preview if identifier matches; otherwise create.
    let existing = state
        .storage
        .list_deployments(project.id)
        .await
        .map_err(ApiError::Internal)?
        .into_iter()
        .find(|d| {
            d.deployment_type == DeploymentType::Preview
                && d.preview_identifier.as_deref() == Some(args.identifier.as_str())
        });
    let deployment = match existing {
        Some(d) => d,
        None => {
            let name = crate::ids::random_deployment_name();
            let result = state
                .provisioner
                .provision(crate::provisioner::ProvisionRequest {
                    deployment_name: name.clone(),
                    deployment_type: DeploymentType::Preview,
                    project_id: project.id,
                    tier: crate::provisioner::tiers::DEFAULT_TIER.to_string(),
                    knob_overrides: std::collections::BTreeMap::new(),
                })
                .await
                .map_err(ApiError::Internal)?;
            let new = state
                .storage
                .create_deployment(crate::storage::deployments::NewDeployment {
                    project_id: project.id,
                    name: &name,
                    deployment_type: DeploymentType::Preview,
                    deployment_class: DeploymentClass::Standard,
                    region: None,
                    url: &result.url,
                    site_url: &result.site_url,
                    backend_pid: result.backend_pid,
                    backend_port: result.backend_port,
                    creator_id: Some(member_id),
                    preview_identifier: Some(&args.identifier),
                    instance_secret: &result.instance_secret,
                    tier: crate::provisioner::tiers::DEFAULT_TIER,
                    knob_overrides: &serde_json::json!({}),
                })
                .await
                .map_err(ApiError::Internal)?;
            let public_id = random_id();
            state
                .storage
                .create_access_token(NewAccessToken {
                    public_id: &public_id,
                    kind: AccessTokenKind::DeployPreview,
                    member_id: Some(member_id),
                    team_id: None,
                    project_id: Some(project.id),
                    deployment_id: Some(new.id),
                    name: "system",
                    secret_hash: &result.admin_key_hash,
                    secret_suffix: &result.admin_key_suffix,
                    expiry: None,
                })
                .await
                .map_err(ApiError::Internal)?;
            new
        },
    };
    let admin_key = admin_key_for_deployment(&state, &deployment)
        .await
        .ok_or(ApiError::Forbidden)?;
    let _ = deployment_to_response(&deployment);
    Ok(Json(ClaimPreviewDeploymentResponse {
        deployment_name: deployment.name,
        admin_key,
        url: deployment.url,
        deployment_type: "preview".into(),
    }))
}

async fn resolve_team_and_project(
    state: &OrchestratorState,
    deployment_name: &str,
) -> ApiResult<Json<TeamAndProjectForDeploymentResponse>> {
    let d = state
        .storage
        .get_deployment_by_name(deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("deployment {deployment_name}")))?;
    let p = state
        .storage
        .get_project(d.project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("project {}", d.project_id)))?;
    let t = state
        .storage
        .get_team(p.team_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("team {}", p.team_id)))?;
    Ok(Json(TeamAndProjectForDeploymentResponse {
        team: big_brain_private_api_types::TeamSlug::from(t.slug),
        project: big_brain_private_api_types::ProjectSlug::from(p.slug),
        team_id: common::types::TeamId(t.id as u64),
        project_id: common::types::ProjectId(p.id as u64),
        deployment_id: Some(common::types::DeploymentId(d.id as u64)),
    }))
}

async fn resolve_project_for_selection(
    state: &OrchestratorState,
    sel: &ProjectSelectionArgs,
) -> ApiResult<crate::storage::ProjectRecord> {
    match sel {
        ProjectSelectionArgs::DeploymentName {
            deployment_name, ..
        } => {
            let d = state
                .storage
                .get_deployment_by_name(deployment_name)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| {
                    ApiError::NotFound(format!("deployment {}", deployment_name))
                })?;
            let p = state
                .storage
                .get_project(d.project_id)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| ApiError::NotFound(format!("project {}", d.project_id)))?;
            Ok(p)
        },
        ProjectSelectionArgs::TeamAndProjectSlugs {
            team_slug,
            project_slug,
        } => {
            let t = state
                .storage
                .get_team_by_slug(team_slug)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| ApiError::NotFound(format!("team {team_slug}")))?;
            let p = state
                .storage
                .get_project_by_slug(t.id, project_slug)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| ApiError::NotFound(format!("project {project_slug}")))?;
            Ok(p)
        },
    }
}

/// Resolve the admin key the dashboard should use to authenticate against
/// a deployment.
///
/// If the deployment row has an `instance_secret` that already looks like an
/// admin key (`<deployment_name>|<secret>`), return it verbatim — that's the
/// real key the running backend accepts (set up either by the operator or
/// by a future process-mode supervisor that captures the backend's own
/// generated key). Otherwise, mint a fresh ephemeral key and return that;
/// useful in the v1 demo path where no real backend is running yet.
pub async fn ephemeral_admin_key(
    state: &OrchestratorState,
    deployment: &crate::storage::DeploymentRecord,
) -> Option<String> {
    if !deployment.instance_secret.is_empty()
        && deployment
            .instance_secret
            .starts_with(&format!("{}|", deployment.name))
    {
        return Some(deployment.instance_secret.clone());
    }
    let secret = mint_token_secret(&deployment.name);
    let pat = encode_pat(&secret);
    let hash = sha256_hex(&secret.secret);
    let suffix = suffix_of(&secret.secret);
    let public_id = random_id();
    let kind = match deployment.deployment_type {
        DeploymentType::Prod => AccessTokenKind::DeployProd,
        DeploymentType::Dev => AccessTokenKind::DeployDev,
        DeploymentType::Preview => AccessTokenKind::DeployPreview,
    };
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind,
            member_id: None,
            team_id: None,
            project_id: Some(deployment.project_id),
            deployment_id: Some(deployment.id),
            name: "ephemeral",
            secret_hash: &hash,
            secret_suffix: &suffix,
            expiry: Some(crate::time::now_unix_ms() + 3_600_000),
        })
        .await
        .ok()?;
    Some(pat)
}

async fn admin_key_for_deployment(
    state: &OrchestratorState,
    deployment: &crate::storage::DeploymentRecord,
) -> Option<String> {
    ephemeral_admin_key(state, deployment).await
}

async fn mint_pat_for_member(
    state: &OrchestratorState,
    member_id: i64,
    name: &str,
) -> ApiResult<String> {
    let public_id = random_id();
    let secret = mint_token_secret(&public_id);
    let pat = encode_pat(&secret);
    let hash = sha256_hex(&secret.secret);
    let suffix = suffix_of(&secret.secret);
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind: AccessTokenKind::Pat,
            member_id: Some(member_id),
            team_id: None,
            project_id: None,
            deployment_id: None,
            name,
            secret_hash: &hash,
            secret_suffix: &suffix,
            expiry: None,
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok(pat)
}

// `parse_token` import retained for future revocation work
#[allow(dead_code)]
fn _ensure_parse_token_used(_: &str) {
    let _ = parse_token;
}
