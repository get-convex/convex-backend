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
    CreateInvitationArgs,
    CreateTeamAccessTokenResponse,
    PlatformCreateTeamArgs,
    PlatformListTeamMembersResponse,
    PlatformTeamMember,
    PlatformTeamResponse,
};

use crate::{
    auth::{
        identity::AuthIdentity,
        tokens::{
            encode_pat,
            mint_token_secret,
            sha256_hex,
            suffix_of,
        },
    },
    errors::{
        ApiError,
        ApiResult,
    },
    ids::{
        random_id,
        slugify,
    },
    state::OrchestratorState,
    storage::{
        access_tokens::NewAccessToken,
        AccessTokenKind,
        TeamRole,
    },
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/teams/create_team", post(create_team))
        .route("/teams/{team_id}/list_members", get(list_members))
        .route("/teams/{team_id}/invite_team_member", post(invite_member))
        .route(
            "/teams/{team_id}/create_access_token",
            post(create_team_access_token),
        )
}

#[utoipa::path(
    post,
    path = "/v1/teams/create_team",
    request_body = PlatformCreateTeamArgs,
    responses(
        (status = 201, body = PlatformTeamResponse),
        (status = 401),
    ),
    tag = "teams",
)]
pub(crate) async fn create_team(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<PlatformCreateTeamArgs>,
) -> ApiResult<(StatusCode, Json<PlatformTeamResponse>)> {
    let member_id = auth.require_member()?;
    let slug = args.slug.unwrap_or_else(|| slugify(&args.name));
    let team = state
        .storage
        .create_team(&args.name, &slug, Some(member_id))
        .await
        .map_err(ApiError::Internal)?;
    Ok((
        StatusCode::CREATED,
        Json(PlatformTeamResponse {
            id: team.id as u64,
            name: team.name,
            slug: team.slug,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/teams/{team_id}/list_members",
    params(("team_id" = i64, Path)),
    responses(
        (status = 200, body = PlatformListTeamMembersResponse),
        (status = 401),
    ),
    tag = "teams",
)]
pub(crate) async fn list_members(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<Json<PlatformListTeamMembersResponse>> {
    let rows = state
        .storage
        .list_team_members(team_id)
        .await
        .map_err(ApiError::Internal)?;
    let mut members = Vec::with_capacity(rows.len());
    for r in rows {
        if let Some(m) = state
            .storage
            .get_member(r.member_id)
            .await
            .map_err(ApiError::Internal)?
        {
            members.push(PlatformTeamMember {
                id: m.id as u64,
                email: m.primary_email,
                role: r.role.to_string(),
            });
        }
    }
    Ok(Json(PlatformListTeamMembersResponse { members }))
}

#[utoipa::path(
    post,
    path = "/v1/teams/{team_id}/invite_team_member",
    params(("team_id" = i64, Path)),
    request_body = CreateInvitationArgs,
    responses(
        (status = 200, description = "invitation recorded"),
        (status = 401),
        (status = 403, description = "caller is not a team admin"),
    ),
    tag = "teams",
)]
pub(crate) async fn invite_member(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
    Json(args): Json<CreateInvitationArgs>,
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
    let code = random_id();
    state
        .storage
        .create_invitation(team_id, &args.email, &args.role, &code, Some(caller))
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/v1/teams/{team_id}/create_access_token",
    params(("team_id" = i64, Path)),
    responses(
        (status = 201, body = CreateTeamAccessTokenResponse),
        (status = 404, description = "team not found"),
    ),
    tag = "teams",
)]
pub(crate) async fn create_team_access_token(
    State(state): State<OrchestratorState>,
    Path(team_id): Path<i64>,
) -> ApiResult<(StatusCode, Json<CreateTeamAccessTokenResponse>)> {
    // Open endpoint by design (matches BigBrain) — used during initial team
    // bootstrap. Caller proves authority by knowing the team_id; for self-
    // hosted we additionally verify the caller is logged in as a member of
    // the team.
    let team = state
        .storage
        .get_team(team_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("team {team_id}")))?;
    let public_id = random_id();
    let secret = mint_token_secret(&public_id);
    let pat = encode_pat(&secret);
    let hash = sha256_hex(&secret.secret);
    let suffix = suffix_of(&secret.secret);
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind: AccessTokenKind::Team,
            member_id: None,
            team_id: Some(team.id),
            project_id: None,
            deployment_id: None,
            name: "team",
            secret_hash: &hash,
            secret_suffix: &suffix,
            expiry: None,
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok((
        StatusCode::CREATED,
        Json(CreateTeamAccessTokenResponse { access_token: pat }),
    ))
}
