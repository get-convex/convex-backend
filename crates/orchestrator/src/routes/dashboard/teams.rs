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
use orchestrator_api_types::{
    dashboard::{
        CreateTeamArgs,
        InvitationArgs,
        InvitationResponse,
        RemoveMemberArgs,
        TeamMember,
        TeamResponse,
        UpdateMemberRoleArgs,
        UpdateTeamArgs,
    },
    stubs::{
        StubReferralState,
        StubTeamEntitlementsResponse,
        StubValidateReferralCode,
    },
};

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    ids::{
        random_id,
        slugify,
    },
    routes::helpers::team_to_response,
    state::OrchestratorState,
    storage::TeamRole,
    stub_data,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/teams", get(list_teams).post(create_team))
        .route("/teams/{team_id}", post(update_team))
        .route("/teams/{team_id}/delete", post(delete_team))
        .route("/teams/{team_id}/members", get(list_members))
        .route("/teams/{team_id}/remove_member", post(remove_member))
        .route("/teams/{team_id}/update_member_role", post(update_member_role))
        .route(
            "/teams/{team_id}/get_entitlements",
            get(get_entitlements_stub),
        )
        .route("/teams/{team_id}/unpause", post(unpause_deployments))
        .route(
            "/teams/{team_id}/invites",
            get(list_invites).post(create_invite),
        )
        .route("/teams/{team_id}/invites/cancel", post(cancel_invite))
        .route("/invites/{code}/accept", post(accept_invite))
        .route(
            "/teams/{team_id}/get_project_roles",
            get(get_project_roles),
        )
        .route(
            "/teams/{team_id}/update_project_roles",
            post(update_project_roles),
        )
        .route(
            "/teams/{team_id}/apply_referral_code",
            post(apply_referral_code),
        )
        .route(
            "/teams/{team_id}/referral_state",
            get(get_referral_state_stub),
        )
        .route("/validate_referral_code", get(validate_referral_code_stub))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams",
    responses((status = 200, body = Vec<TeamResponse>)),
    tag = "dashboard",
)]
pub(crate) async fn list_teams(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<Vec<TeamResponse>>> {
    let id = auth.require_member()?;
    let teams = state
        .storage
        .list_teams_for_member(id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(teams.iter().map(team_to_response).collect()))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams",
    request_body = CreateTeamArgs,
    responses((status = 200, body = TeamResponse)),
    tag = "dashboard",
)]
pub(crate) async fn create_team(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<CreateTeamArgs>,
) -> ApiResult<Json<TeamResponse>> {
    let member_id = auth.require_member()?;
    let slug = args.slug.unwrap_or_else(|| slugify(&args.name));
    let team = state
        .storage
        .create_team(&args.name, &slug, Some(member_id))
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(team_to_response(&team)))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}",
    params(("team_id" = i64, Path)),
    request_body = UpdateTeamArgs,
    responses((status = 200), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn update_team(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<UpdateTeamArgs>,
) -> ApiResult<StatusCode> {
    let member_id = auth.require_member()?;
    let role = state
        .storage
        .get_team_role(team_id, member_id)
        .await
        .map_err(ApiError::Internal)?;
    if !matches!(role, Some(TeamRole::Admin)) {
        return Err(ApiError::Forbidden);
    }
    state
        .storage
        .update_team(team_id, args.name.as_deref(), args.slug.as_deref())
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/delete",
    params(("team_id" = i64, Path)),
    responses((status = 200), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn delete_team(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<StatusCode> {
    let member_id = auth.require_member()?;
    let role = state
        .storage
        .get_team_role(team_id, member_id)
        .await
        .map_err(ApiError::Internal)?;
    if !matches!(role, Some(TeamRole::Admin)) {
        return Err(ApiError::Forbidden);
    }
    state
        .storage
        .delete_team(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/members",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = Vec<TeamMember>), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn list_members(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<Vec<TeamMember>>> {
    let member_id = auth.require_member()?;
    if state
        .storage
        .get_team_role(team_id, member_id)
        .await
        .map_err(ApiError::Internal)?
        .is_none()
    {
        return Err(ApiError::Forbidden);
    }
    let rows = state
        .storage
        .list_team_members(team_id)
        .await
        .map_err(ApiError::Internal)?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        if let Some(m) = state
            .storage
            .get_member(r.member_id)
            .await
            .map_err(ApiError::Internal)?
        {
            // Hide the synthetic "System (bootstrap)" member from the team
            // members list — it exists only so the CLI bootstrap-token flow
            // has a member to attach PATs to, not as an actual user. Real
            // operators don't need to see or manage it.
            if m.auth_user_id == crate::state::SYSTEM_AUTH_USER_ID {
                continue;
            }
            out.push(TeamMember {
                id: m.id as u64,
                email: m.primary_email,
                name: m.name,
                role: r.role.to_string(),
            });
        }
    }
    Ok(Json(out))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/remove_member",
    params(("team_id" = i64, Path)),
    request_body = RemoveMemberArgs,
    responses((status = 200), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn remove_member(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<RemoveMemberArgs>,
) -> ApiResult<StatusCode> {
    let caller = auth.require_member()?;
    if !matches!(
        state
            .storage
            .get_team_role(team_id, caller)
            .await
            .map_err(ApiError::Internal)?,
        Some(TeamRole::Admin)
    ) {
        return Err(ApiError::Forbidden);
    }
    state
        .storage
        .remove_team_member(team_id, args.member_id as i64)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/update_member_role",
    params(("team_id" = i64, Path)),
    request_body = UpdateMemberRoleArgs,
    responses((status = 200), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn update_member_role(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<UpdateMemberRoleArgs>,
) -> ApiResult<StatusCode> {
    let caller = auth.require_member()?;
    if !matches!(
        state
            .storage
            .get_team_role(team_id, caller)
            .await
            .map_err(ApiError::Internal)?,
        Some(TeamRole::Admin)
    ) {
        return Err(ApiError::Forbidden);
    }
    let role: TeamRole = args
        .role
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("unknown role {}", args.role)))?;
    state
        .storage
        .add_team_member(team_id, args.member_id as i64, role)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/get_entitlements",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = StubTeamEntitlementsResponse, description = "stub: returns unlimited self-hosted entitlements")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn get_entitlements_stub(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<StubTeamEntitlementsResponse>> {
    Ok(Json(stub_data::entitlements_unlimited()))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/unpause",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub no-op (no pause/unpause flow in self-hosted)")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn unpause_deployments(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/invites",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = Vec<InvitationResponse>)),
    tag = "dashboard",
)]
pub(crate) async fn list_invites(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<Vec<InvitationResponse>>> {
    let member_id = auth.require_member()?;
    if state
        .storage
        .get_team_role(team_id, member_id)
        .await
        .map_err(ApiError::Internal)?
        .is_none()
    {
        return Err(ApiError::Forbidden);
    }
    let invites = state
        .storage
        .list_invitations(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        invites
            .into_iter()
            .map(|i| InvitationResponse {
                id: i.id as u64,
                email: i.email,
                role: i.role,
                code: i.code,
                created_at: i.created_at as f64,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/invites",
    params(("team_id" = i64, Path)),
    request_body = InvitationArgs,
    responses((status = 200, body = InvitationResponse), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn create_invite(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<InvitationArgs>,
) -> ApiResult<Json<InvitationResponse>> {
    let caller = auth.require_member()?;
    if !matches!(
        state
            .storage
            .get_team_role(team_id, caller)
            .await
            .map_err(ApiError::Internal)?,
        Some(TeamRole::Admin)
    ) {
        return Err(ApiError::Forbidden);
    }
    let code = random_id();
    let inv = state
        .storage
        .create_invitation(team_id, &args.email, &args.role, &code, Some(caller))
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(InvitationResponse {
        id: inv.id as u64,
        email: inv.email,
        role: inv.role,
        code: inv.code,
        created_at: inv.created_at as f64,
    }))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/invites/cancel",
    params(("team_id" = i64, Path)),
    request_body = RemoveMemberArgs,
    responses((status = 200), (status = 403)),
    tag = "dashboard",
)]
pub(crate) async fn cancel_invite(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<RemoveMemberArgs>,
) -> ApiResult<StatusCode> {
    let caller = auth.require_member()?;
    if !matches!(
        state
            .storage
            .get_team_role(team_id, caller)
            .await
            .map_err(ApiError::Internal)?,
        Some(TeamRole::Admin)
    ) {
        return Err(ApiError::Forbidden);
    }
    state
        .storage
        .cancel_invitation(args.member_id as i64)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/invites/{code}/accept",
    params(("code" = String, Path)),
    responses(
        (status = 200, description = "invitation accepted, caller added to team"),
        (status = 404, description = "invitation not found"),
        (status = 409, description = "invitation already accepted"),
    ),
    tag = "dashboard",
)]
pub(crate) async fn accept_invite(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(code): Path<String>,
) -> ApiResult<StatusCode> {
    let caller = auth.require_member()?;
    let inv = state
        .storage
        .get_invitation_by_code(&code)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("invitation".into()))?;
    if inv.accepted_at.is_some() {
        return Err(ApiError::Conflict("invitation already accepted".into()));
    }
    let member = state
        .storage
        .get_member(caller)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Forbidden)?;
    if !inv
        .email
        .trim()
        .eq_ignore_ascii_case(member.primary_email.trim())
    {
        return Err(ApiError::Forbidden);
    }
    let role: TeamRole = inv
        .role
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("unknown role {}", inv.role)))?;
    state
        .storage
        .add_team_member(inv.team_id, caller, role)
        .await
        .map_err(ApiError::Internal)?;
    state
        .storage
        .accept_invitation(&code)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

// Cloud's `useProjectRoles` hook expects an array of `{ projectId, memberId }`
// pairs covering every (project, member) where the member has admin rights
// on that project. We return the team's full set in one shot — the caller
// filters client-side.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectRoleEntry {
    project_id: i64,
    member_id: i64,
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/get_project_roles",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = Vec<ProjectRoleEntry>)),
    tag = "dashboard",
)]
pub(crate) async fn get_project_roles(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<Vec<ProjectRoleEntry>>> {
    let pairs = state
        .storage
        .list_project_admins_for_team(team_id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(
        pairs
            .into_iter()
            .map(|(project_id, member_id)| ProjectRoleEntry {
                project_id,
                member_id,
            })
            .collect(),
    ))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateProjectRolesArgs {
    project_id: i64,
    /// Replaces the full admin set for the project.
    admin_member_ids: Vec<i64>,
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/update_project_roles",
    params(("team_id" = i64, Path)),
    request_body = UpdateProjectRolesArgs,
    responses((status = 200), (status = 404)),
    tag = "dashboard",
)]
pub(crate) async fn update_project_roles(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<UpdateProjectRolesArgs>,
) -> ApiResult<StatusCode> {
    // Verify the project actually belongs to this team — without this a
    // team admin could promote members on another team's project.
    let project = state
        .storage
        .get_project(args.project_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("project".into()))?;
    if project.team_id != team_id {
        return Err(ApiError::NotFound("project".into()));
    }
    state
        .storage
        .set_project_admins(
            args.project_id,
            &args.admin_member_ids,
            auth.member_id,
        )
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/apply_referral_code",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub no-op (no referral program in self-hosted)")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn apply_referral_code(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/referral_state",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = StubReferralState, description = "stub: empty referral state")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn get_referral_state_stub(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<StubReferralState>> {
    let team = state
        .storage
        .get_team(team_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("team {team_id}")))?;
    Ok(Json(stub_data::empty_referral(&team.slug)))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/validate_referral_code",
    responses((status = 200, body = StubValidateReferralCode, description = "stub: always invalid")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn validate_referral_code_stub(
    _auth: AuthIdentity,
) -> ApiResult<Json<StubValidateReferralCode>> {
    Ok(Json(stub_data::invalid_referral_code()))
}
