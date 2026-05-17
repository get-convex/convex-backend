use axum::{
    http::StatusCode,
    routing::{
        get,
        post,
    },
    Json,
    Router,
};
use orchestrator_api_types::management::{
    CreatePersonalAccessTokenArgs,
    CreatePersonalAccessTokenResponse,
    DeletePersonalAccessTokenArgs,
    PaginatedPersonalAccessTokensResponse,
    PlatformDeployKeyResponse,
    PlatformTokenDetailsResponse,
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
    ids::random_id,
    state::OrchestratorState,
    storage::{
        access_tokens::NewAccessToken,
        AccessTokenKind,
    },
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/list_personal_access_tokens",
            get(list_personal_access_tokens),
        )
        .route(
            "/create_personal_access_token",
            post(create_personal_access_token),
        )
        .route(
            "/delete_personal_access_token",
            post(delete_personal_access_token),
        )
        .route("/token_details", get(token_details))
}

#[utoipa::path(
    get,
    path = "/v1/list_personal_access_tokens",
    responses(
        (status = 200, body = PaginatedPersonalAccessTokensResponse),
        (status = 401, description = "missing or invalid bearer token"),
    ),
    tag = "tokens",
)]
pub(crate) async fn list_personal_access_tokens(
    auth: AuthIdentity,
    axum::extract::State(state): axum::extract::State<OrchestratorState>,
) -> ApiResult<Json<PaginatedPersonalAccessTokensResponse>> {
    let id = auth.require_member()?;
    let tokens = state
        .storage
        .list_access_tokens_by_member(id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(PaginatedPersonalAccessTokensResponse {
        tokens: tokens
            .into_iter()
            .filter(|t| t.kind == AccessTokenKind::Pat)
            .map(|t| PlatformDeployKeyResponse {
                id: t.public_id,
                name: t.name,
                creation_time: t.creation_time as f64,
                key_suffix: t.secret_suffix,
                expires_at: t.expiry.map(|v| v as f64),
            })
            .collect(),
        cursor: None,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/create_personal_access_token",
    request_body = CreatePersonalAccessTokenArgs,
    responses(
        (status = 200, body = CreatePersonalAccessTokenResponse),
        (status = 401),
    ),
    tag = "tokens",
)]
pub(crate) async fn create_personal_access_token(
    auth: AuthIdentity,
    axum::extract::State(state): axum::extract::State<OrchestratorState>,
    Json(args): Json<CreatePersonalAccessTokenArgs>,
) -> ApiResult<Json<CreatePersonalAccessTokenResponse>> {
    let member_id = auth.require_member()?;
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
            name: &args.name,
            secret_hash: &hash,
            secret_suffix: &suffix,
            expiry: args.expires_at.map(|f| f as i64),
        })
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(CreatePersonalAccessTokenResponse {
        access_token: pat,
        id: public_id,
        name: args.name,
        creation_time: crate::time::now_unix_ms() as f64,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/delete_personal_access_token",
    request_body = DeletePersonalAccessTokenArgs,
    responses(
        (status = 200, description = "token revoked"),
        (status = 401),
    ),
    tag = "tokens",
)]
pub(crate) async fn delete_personal_access_token(
    _auth: AuthIdentity,
    axum::extract::State(state): axum::extract::State<OrchestratorState>,
    Json(args): Json<DeletePersonalAccessTokenArgs>,
) -> ApiResult<StatusCode> {
    state
        .storage
        .revoke_access_token(&args.id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/v1/token_details",
    responses(
        (status = 200, body = PlatformTokenDetailsResponse),
        (status = 401),
    ),
    tag = "tokens",
)]
pub(crate) async fn token_details(
    auth: AuthIdentity,
) -> ApiResult<Json<PlatformTokenDetailsResponse>> {
    Ok(Json(PlatformTokenDetailsResponse {
        kind: auth.token.kind.to_string(),
        team_id: auth.team_id.map(|t| t as u64),
        project_id: auth.project_id.map(|p| p as u64),
        deployment_name: None,
        member_id: auth.member_id.map(|m| m as u64),
    }))
}
