//! `/api/internal/*` — service-key-authenticated endpoints used only by the
//! dashboard-orchestrator. Never expose these to end users.

use axum::{
    extract::State,
    http::{
        HeaderMap,
        StatusCode,
    },
    response::IntoResponse,
    routing::post,
    Json,
    Router,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    auth::tokens::{
        encode_pat,
        mint_token_secret,
        sha256_hex,
        suffix_of,
    },
    config::RegistrationMode,
    errors::{
        ApiError,
        ApiResult,
    },
    ids::random_id,
    state::OrchestratorState,
    storage::{
        access_tokens::NewAccessToken,
        AccessTokenKind,
        InvitationRecord,
        TeamRole,
    },
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/exchange_session", post(exchange_session))
        .route("/health", post(health))
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSessionArgs {
    /// BetterAuth user ID — opaque, treated as a stable foreign key.
    pub auth_user_id: String,
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Optional team invitation code. Required for a non-allowlisted user to
    /// mint the temporary session token needed to accept an invite.
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSessionResponse {
    pub access_token: String,
    pub member_id: u64,
    pub team_slug: String,
    pub role: String,
}

#[utoipa::path(
    post,
    path = "/api/internal/exchange_session",
    request_body = ExchangeSessionArgs,
    responses(
        (status = 200, body = ExchangeSessionResponse),
        (status = 400, description = "invalid email / authUserId"),
        (status = 401, description = "x-service-key missing or wrong"),
        (status = 501, description = "service key not configured on this orchestrator"),
    ),
    tag = "internal",
    security(("service_key" = [])),
)]
pub(crate) async fn exchange_session(
    State(state): State<OrchestratorState>,
    headers: HeaderMap,
    Json(args): Json<ExchangeSessionArgs>,
) -> ApiResult<Json<ExchangeSessionResponse>> {
    require_service_key(&state, &headers)?;

    let email = args.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BadRequest("invalid email".into()));
    }
    if args.auth_user_id.is_empty() {
        return Err(ApiError::BadRequest("authUserId required".into()));
    }
    if args.auth_user_id.starts_with("system:") {
        return Err(ApiError::BadRequest(
            "authUserId may not collide with the system namespace".into(),
        ));
    }

    let invite = matching_invitation(&state, &email, args.invite_code.as_deref()).await?;
    let is_admin_email = email_is_admin(&state, &email);

    let member = state
        .storage
        .upsert_member(&args.auth_user_id, &email, args.name.as_deref())
        .await
        .map_err(ApiError::Internal)?;

    let existing_teams = state
        .storage
        .list_teams_for_member(member.id)
        .await
        .map_err(ApiError::Internal)?;

    let (team_id, team_slug, role, token_team_id) = if let Some(team) = existing_teams.first() {
        let role = if is_admin_email {
            state
                .storage
                .add_team_member(team.id, member.id, TeamRole::Admin)
                .await
                .map_err(ApiError::Internal)?;
            TeamRole::Admin
        } else {
            state
                .storage
                .get_team_role(team.id, member.id)
                .await
                .map_err(ApiError::Internal)?
                .unwrap_or(TeamRole::Developer)
        };
        (team.id, team.slug.clone(), role, Some(team.id))
    } else if is_admin_email {
        let team = default_team(&state, Some(member.id)).await?;
        state
            .storage
            .add_team_member(team.id, member.id, TeamRole::Admin)
            .await
            .map_err(ApiError::Internal)?;
        (team.id, team.slug, TeamRole::Admin, Some(team.id))
    } else {
        match state.config.registration_mode {
            RegistrationMode::Open => {
                let team = default_team(&state, Some(member.id)).await?;
                // First non-system member becomes admin in `open` mode.
                let total = state
                    .storage
                    .count_members()
                    .await
                    .map_err(ApiError::Internal)?;
                let role = if total <= 1 {
                    TeamRole::Admin
                } else {
                    TeamRole::Developer
                };
                state
                    .storage
                    .add_team_member(team.id, member.id, role)
                    .await
                    .map_err(ApiError::Internal)?;
                (team.id, team.slug, role, Some(team.id))
            },
            RegistrationMode::Allowlist | RegistrationMode::InviteOnly => {
                let inv = invite.ok_or(ApiError::Forbidden)?;
                let team = state
                    .storage
                    .get_team(inv.team_id)
                    .await
                    .map_err(ApiError::Internal)?
                    .ok_or_else(|| ApiError::NotFound("team".into()))?;
                let role: TeamRole = inv
                    .role
                    .parse()
                    .map_err(|_| ApiError::BadRequest(format!("unknown role {}", inv.role)))?;
                (team.id, team.slug, role, None)
            },
        }
    };

    // Mint a fresh PAT for this session.
    let public_id = random_id();
    let secret = mint_token_secret(&public_id);
    let pat = encode_pat(&secret);
    let hash = sha256_hex(&secret.secret);
    let suffix = suffix_of(&secret.secret);
    state
        .storage
        .create_access_token(NewAccessToken {
            public_id: &public_id,
            kind: AccessTokenKind::Session,
            member_id: Some(member.id),
            team_id: token_team_id,
            project_id: None,
            deployment_id: None,
            name: "dashboard-session",
            secret_hash: &hash,
            secret_suffix: &suffix,
            // 24-hour TTL by default. The dashboard re-exchanges the session
            // when its BetterAuth cookie refreshes, so this stays bound to
            // the live session.
            expiry: Some(crate::time::now_unix_ms() + 24 * 3_600_000),
        })
        .await
        .map_err(ApiError::Internal)?;

    state
        .storage
        .append_audit(
            team_id,
            Some(member.id),
            "memberSignIn",
            &serde_json::json!({
                "auth_user_id": args.auth_user_id,
                "email": email,
                "role": role.to_string(),
                "method": "betterauth-session",
            }),
        )
        .await
        .ok();

    Ok(Json(ExchangeSessionResponse {
        access_token: pat,
        member_id: member.id as u64,
        team_slug,
        role: role.to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/internal/health",
    responses(
        (status = 200, description = "service-key validated"),
        (status = 401, description = "x-service-key missing or wrong"),
    ),
    tag = "internal",
    security(("service_key" = [])),
)]
pub(crate) async fn health(
    State(state): State<OrchestratorState>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    require_service_key(&state, &headers)?;
    Ok(StatusCode::OK)
}

fn require_service_key(
    state: &OrchestratorState,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    let configured = state.config.service_key.as_deref().ok_or_else(|| {
        ApiError::NotImplemented(
            "internal endpoints disabled: set CONVEX_ORCHESTRATOR_SERVICE_KEY".into(),
        )
    })?;
    let presented = headers
        .get("x-service-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    if !crate::routes::deployment_internal::constant_time_eq_str(presented, configured) {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

fn email_is_admin(state: &OrchestratorState, email: &str) -> bool {
    state
        .config
        .admin_emails
        .iter()
        .any(|e| e.eq_ignore_ascii_case(email))
}

async fn default_team(
    state: &OrchestratorState,
    creator_id: Option<i64>,
) -> ApiResult<crate::storage::TeamRecord> {
    match state
        .storage
        .get_team_by_slug("self-hosted")
        .await
        .map_err(ApiError::Internal)?
    {
        Some(t) => Ok(t),
        None => state
            .storage
            .create_team(
                state.config.default_team_display_name(),
                "self-hosted",
                creator_id,
            )
            .await
            .map_err(ApiError::Internal),
    }
}

async fn matching_invitation(
    state: &OrchestratorState,
    email: &str,
    invite_code: Option<&str>,
) -> ApiResult<Option<InvitationRecord>> {
    let Some(code) = invite_code.map(str::trim).filter(|c| !c.is_empty()) else {
        return Ok(None);
    };
    let inv = state
        .storage
        .get_invitation_by_code(code)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("invitation".into()))?;
    if inv.accepted_at.is_some() {
        return Err(ApiError::Conflict("invitation already accepted".into()));
    }
    if !inv.email.trim().eq_ignore_ascii_case(email) {
        return Err(ApiError::Forbidden);
    }
    Ok(Some(inv))
}
