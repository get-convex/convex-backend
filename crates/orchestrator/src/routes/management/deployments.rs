use axum::{
    extract::{
        Path,
        Query,
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
    DeploymentClass,
    DeploymentRegion,
    ListDeploymentClassesResponse,
    ListDeploymentRegionsResponse,
    ListLocalDeploymentsResponse,
    PaginatedDeploymentsResponse,
    PlatformCreateDeploymentArgs,
    PlatformDeploymentResponse,
    PlatformTransferDeploymentArgs,
    PlatformUpdateDeploymentArgs,
};
use serde::Deserialize;

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    ids::random_deployment_name,
    routes::helpers::deployment_to_platform,
    state::OrchestratorState,
    storage::DeploymentType,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/projects/{project_id}/list_deployments",
            get(list_deployments),
        )
        .route(
            "/projects/{project_id}/create_deployment",
            post(create_deployment),
        )
        .route(
            "/projects/{project_id}/deployment",
            get(get_default_deployment_for_project),
        )
        .route(
            "/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
            get(get_default_deployment_by_slug),
        )
        .route("/teams/{team_id}/list_deployments", get(list_team_deployments))
        .route(
            "/teams/{team_id}/list_local_deployments",
            get(list_local_deployments),
        )
        .route(
            "/teams/{team_id}/list_deployment_classes",
            get(list_deployment_classes),
        )
        .route(
            "/teams/{team_id}/list_deployment_regions",
            get(list_deployment_regions),
        )
        .route("/deployments/{deployment_name}", get(get_deployment))
        .route(
            "/deployments/{deployment_name}/delete",
            post(delete_deployment),
        )
        .route(
            "/deployments/{deployment_name}/transfer",
            post(transfer_deployment),
        )
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}/list_deployments",
    params(("project_id" = i64, Path)),
    responses(
        (status = 200, body = Vec<PlatformDeploymentResponse>),
    ),
    tag = "deployments",
)]
pub(crate) async fn list_deployments(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
) -> ApiResult<Json<Vec<PlatformDeploymentResponse>>> {
    let rows = state
        .storage
        .list_deployments(project_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(rows.iter().map(deployment_to_platform).collect()))
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/create_deployment",
    params(("project_id" = i64, Path)),
    request_body = PlatformCreateDeploymentArgs,
    responses(
        (status = 200, body = PlatformDeploymentResponse),
        (status = 400, description = "unknown deployment kind"),
    ),
    tag = "deployments",
)]
pub(crate) async fn create_deployment(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
    Json(args): Json<PlatformCreateDeploymentArgs>,
) -> ApiResult<Json<PlatformDeploymentResponse>> {
    let member_id = auth.require_member()?;
    let dt: DeploymentType = args
        .kind
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("unknown deployment kind {}", args.kind)))?;
    let name = random_deployment_name();
    let project = state
        .storage
        .get_project(project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("project {project_id}")))?;
    let tier = args
        .tier
        .as_deref()
        .unwrap_or(&project.tier)
        .to_string();
    if crate::provisioner::tiers::lookup(&tier).is_none() {
        return Err(ApiError::BadRequest(format!("unknown tier {tier}")));
    }
    // Merge project-level overrides with any deployment-level overrides.
    let project_overrides = project
        .knob_overrides
        .as_object()
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect::<std::collections::BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut overrides = project_overrides;
    if let Some(extra) = &args.knob_overrides {
        for (k, v) in extra {
            if let Err(e) = crate::knob_registry::validate(k, v) {
                return Err(ApiError::BadRequest(e.to_string()));
            }
            overrides.insert(k.clone(), v.clone());
        }
    }
    let result = state
        .provisioner
        .provision(crate::provisioner::ProvisionRequest {
            deployment_name: name.clone(),
            deployment_type: dt,
            project_id,
            tier: tier.clone(),
            knob_overrides: overrides.clone(),
        })
        .await
        .map_err(ApiError::Internal)?;
    let resolved_overrides = serde_json::to_value(&result.resolved_env)
        .map_err(|e| ApiError::Internal(e.into()))?;
    let new = state
        .storage
        .create_deployment(crate::storage::deployments::NewDeployment {
            project_id,
            name: &name,
            deployment_type: dt,
            deployment_class: crate::storage::DeploymentClass::Standard,
            region: args.region.as_deref(),
            url: &result.url,
            site_url: &result.site_url,
            backend_pid: result.backend_pid,
            backend_port: result.backend_port,
            creator_id: Some(member_id),
            preview_identifier: args.preview_identifier.as_deref(),
            instance_secret: &result.instance_secret,
            tier: &tier,
            knob_overrides: &resolved_overrides,
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(deployment_to_platform(&new)))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub(crate) struct DeploymentQuery {
    pub default_dev: Option<bool>,
    pub reference: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}/deployment",
    params(
        ("project_id" = i64, Path),
        DeploymentQuery,
    ),
    responses(
        (status = 200, body = PlatformDeploymentResponse),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deployments",
)]
pub(crate) async fn get_default_deployment_for_project(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(project_id): Path<i64>,
    Query(q): Query<DeploymentQuery>,
) -> ApiResult<Json<PlatformDeploymentResponse>> {
    if let Some(reference) = q.reference {
        let d = state
            .storage
            .get_deployment_by_name(&reference)
            .await
            .map_err(ApiError::Internal)?
            .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
        return Ok(Json(deployment_to_platform(&d)));
    }
    let dt = if q.default_dev.unwrap_or(false) {
        DeploymentType::Dev
    } else {
        DeploymentType::Prod
    };
    let d = state
        .storage
        .find_default_deployment(project_id, dt)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    Ok(Json(deployment_to_platform(&d)))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id_or_slug}/projects/{project_slug}/deployment",
    params(
        ("team_id_or_slug" = String, Path),
        ("project_slug" = String, Path),
        DeploymentQuery,
    ),
    responses(
        (status = 200, body = PlatformDeploymentResponse),
        (status = 404, description = "team, project, or deployment not found"),
    ),
    tag = "deployments",
)]
pub(crate) async fn get_default_deployment_by_slug(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path((team_id_or_slug, project_slug)): Path<(String, String)>,
    Query(q): Query<DeploymentQuery>,
) -> ApiResult<Json<PlatformDeploymentResponse>> {
    let team_id = if let Ok(id) = team_id_or_slug.parse::<i64>() {
        id
    } else {
        state
            .storage
            .get_team_by_slug(&team_id_or_slug)
            .await
            .map_err(ApiError::Internal)?
            .ok_or_else(|| ApiError::NotFound("team".into()))?
            .id
    };
    let project = state
        .storage
        .get_project_by_slug(team_id, &project_slug)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("project".into()))?;
    if let Some(reference) = q.reference {
        let d = state
            .storage
            .get_deployment_by_name(&reference)
            .await
            .map_err(ApiError::Internal)?
            .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
        return Ok(Json(deployment_to_platform(&d)));
    }
    let dt = if q.default_dev.unwrap_or(false) {
        DeploymentType::Dev
    } else {
        DeploymentType::Prod
    };
    let d = state
        .storage
        .find_default_deployment(project.id, dt)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    Ok(Json(deployment_to_platform(&d)))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id}/list_deployments",
    params(("team_id" = i64, Path)),
    responses(
        (status = 200, body = PaginatedDeploymentsResponse),
    ),
    tag = "deployments",
)]
pub(crate) async fn list_team_deployments(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<PaginatedDeploymentsResponse>> {
    let rows = state
        .storage
        .list_deployments_for_team(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(PaginatedDeploymentsResponse {
        deployments: rows.iter().map(deployment_to_platform).collect(),
        cursor: None,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id}/list_local_deployments",
    params(("team_id" = i64, Path)),
    responses(
        (status = 200, body = ListLocalDeploymentsResponse),
    ),
    tag = "deployments",
)]
pub(crate) async fn list_local_deployments(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<ListLocalDeploymentsResponse>> {
    Ok(Json(ListLocalDeploymentsResponse {
        deployments: Vec::new(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id}/list_deployment_classes",
    params(("team_id" = i64, Path)),
    responses(
        (status = 200, body = ListDeploymentClassesResponse),
    ),
    tag = "deployments",
)]
pub(crate) async fn list_deployment_classes(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<ListDeploymentClassesResponse>> {
    Ok(Json(ListDeploymentClassesResponse {
        classes: vec![DeploymentClass {
            name: "standard".into(),
            description: "Standard self-hosted backend".into(),
        }],
    }))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id}/list_deployment_regions",
    params(("team_id" = i64, Path)),
    responses(
        (status = 200, body = ListDeploymentRegionsResponse),
    ),
    tag = "deployments",
)]
pub(crate) async fn list_deployment_regions(
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<ListDeploymentRegionsResponse>> {
    Ok(Json(ListDeploymentRegionsResponse {
        regions: vec![DeploymentRegion {
            name: "self-hosted".into(),
            description: "Local self-hosted region".into(),
        }],
    }))
}

#[utoipa::path(
    get,
    path = "/v1/deployments/{deployment_name}",
    params(("deployment_name" = String, Path)),
    responses(
        (status = 200, body = PlatformDeploymentResponse),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deployments",
)]
pub(crate) async fn get_deployment(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
) -> ApiResult<Json<PlatformDeploymentResponse>> {
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    Ok(Json(deployment_to_platform(&d)))
}

#[utoipa::path(
    post,
    path = "/v1/deployments/{deployment_name}/delete",
    params(("deployment_name" = String, Path)),
    responses(
        (status = 200, description = "deployment torn down and removed"),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deployments",
)]
pub(crate) async fn delete_deployment(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
) -> ApiResult<StatusCode> {
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    // Teardown is best-effort: if `docker rm` can't reach the daemon or
    // the container is already gone we still want to release the DB row
    // (and free the unique deployment-name slot). Mirrors what
    // `cascade_delete_project` does — the orphan container, if any, is
    // invisible to the dashboard once the row is gone, and a stale name
    // would otherwise block re-creation forever.
    if let Err(e) = state.provisioner.teardown(&deployment_name).await {
        tracing::warn!(
            deployment = %deployment_name,
            error = %e,
            "teardown failed during delete; continuing with row deletion",
        );
    }
    state
        .storage
        .delete_deployment(d.id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/v1/deployments/{deployment_name}/transfer",
    params(("deployment_name" = String, Path)),
    request_body = PlatformTransferDeploymentArgs,
    responses(
        (status = 204, description = "deployment moved to the target project"),
        (status = 404, description = "deployment not found"),
    ),
    tag = "deployments",
)]
pub(crate) async fn transfer_deployment(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
    Json(args): Json<PlatformTransferDeploymentArgs>,
) -> ApiResult<StatusCode> {
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    state
        .storage
        .transfer_deployment(d.id, args.project_id as i64)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[allow(dead_code)]
async fn _update_deployment(
    State(state): State<OrchestratorState>,
    Path(deployment_name): Path<String>,
    Json(args): Json<PlatformUpdateDeploymentArgs>,
) -> ApiResult<StatusCode> {
    let d = state
        .storage
        .get_deployment_by_name(&deployment_name)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("deployment".into()))?;
    if let Some(class_str) = args.deployment_class {
        let class: crate::storage::DeploymentClass = class_str.parse().map_err(|_| {
            ApiError::BadRequest(format!("unknown deployment class {class_str}"))
        })?;
        state
            .storage
            .update_deployment_class(d.id, class)
            .await
            .map_err(ApiError::Internal)?;
    }
    Ok(StatusCode::OK)
}
