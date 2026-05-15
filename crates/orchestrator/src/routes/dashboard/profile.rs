use axum::{
    extract::{
        Path,
        State,
    },
    http::StatusCode,
    routing::{
        get,
        post,
        put,
    },
    Json,
    Router,
};
use orchestrator_api_types::dashboard::{
    GetOptInsResponse,
    MemberDataResponse,
    MemberResponse,
    OptIn,
    ProfileEmailArgs,
    ProfileEmailResponse,
    UpdateProfileNameArgs,
};

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    routes::helpers::team_to_response,
    state::OrchestratorState,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/member_data", get(get_member_data))
        .route("/profile_emails/list", get(list_profile_emails))
        .route("/profile_emails/create", post(create_profile_email))
        .route("/profile_emails/delete", post(delete_profile_email))
        .route("/profile_emails/update_primary", post(update_primary_email))
        .route(
            "/profile_emails/resend_verification",
            post(resend_verification),
        )
        .route("/profile_emails/verify/{code}", post(verify_email))
        .route("/update_profile_name", put(update_profile_name))
        .route("/delete_account", post(delete_account))
        .route("/optins", get(get_opt_ins).put(accept_opt_ins))
        .route("/identities", get(get_identities))
        .route("/list_identities", get(get_identities))
        .route("/unlink_identity", post(unlink_identity))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/profile",
    responses((status = 200, body = MemberResponse), (status = 401)),
    tag = "dashboard",
)]
pub(crate) async fn get_profile(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<MemberResponse>> {
    let id = auth.require_member()?;
    let m = state
        .storage
        .get_member(id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(MemberResponse {
        id: m.id as u64,
        email: m.primary_email,
        name: m.name,
    }))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/member_data",
    responses((status = 200, body = MemberDataResponse)),
    tag = "dashboard",
)]
pub(crate) async fn get_member_data(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<MemberDataResponse>> {
    let id = auth.require_member()?;
    let m = state
        .storage
        .get_member(id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Unauthorized)?;
    let teams = state
        .storage
        .list_teams_for_member(id)
        .await
        .map_err(ApiError::Internal)?
        .iter()
        .map(team_to_response)
        .collect();
    Ok(Json(MemberDataResponse {
        member: MemberResponse {
            id: m.id as u64,
            email: m.primary_email,
            name: m.name,
        },
        teams,
    }))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/profile_emails/list",
    responses((status = 200, body = Vec<ProfileEmailResponse>)),
    tag = "dashboard",
)]
pub(crate) async fn list_profile_emails(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<Vec<ProfileEmailResponse>>> {
    let id = auth.require_member()?;
    let m = state
        .storage
        .get_member(id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(vec![ProfileEmailResponse {
        id: m.id as u64,
        email: m.primary_email,
        is_verified: true,
        is_primary: true,
    }]))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/profile_emails/create",
    request_body = ProfileEmailArgs,
    responses((status = 501, description = "stubbed in v1")),
    tag = "dashboard",
)]
pub(crate) async fn create_profile_email(
    _auth: AuthIdentity,
    Json(_args): Json<ProfileEmailArgs>,
) -> ApiResult<StatusCode> {
    Err(ApiError::NotImplemented(
        "secondary profile emails are not implemented in v1".into(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/profile_emails/delete",
    request_body = ProfileEmailArgs,
    responses((status = 501, description = "stubbed in v1")),
    tag = "dashboard",
)]
pub(crate) async fn delete_profile_email(
    _auth: AuthIdentity,
    Json(_args): Json<ProfileEmailArgs>,
) -> ApiResult<StatusCode> {
    Err(ApiError::NotImplemented(
        "secondary profile emails are not implemented in v1".into(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/profile_emails/update_primary",
    request_body = ProfileEmailArgs,
    responses((status = 501, description = "stubbed in v1")),
    tag = "dashboard",
)]
pub(crate) async fn update_primary_email(
    _auth: AuthIdentity,
    Json(_args): Json<ProfileEmailArgs>,
) -> ApiResult<StatusCode> {
    Err(ApiError::NotImplemented(
        "secondary profile emails are not implemented in v1".into(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/profile_emails/resend_verification",
    request_body = ProfileEmailArgs,
    responses((status = 200, description = "stub no-op")),
    tag = "dashboard",
)]
pub(crate) async fn resend_verification(
    _auth: AuthIdentity,
    Json(_args): Json<ProfileEmailArgs>,
) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/profile_emails/verify/{code}",
    params(("code" = String, Path)),
    responses((status = 200, description = "stub no-op")),
    tag = "dashboard",
)]
pub(crate) async fn verify_email(
    _auth: AuthIdentity,
    Path(_code): Path<String>,
) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[utoipa::path(
    put,
    path = "/api/dashboard/update_profile_name",
    request_body = UpdateProfileNameArgs,
    responses((status = 204, description = "name updated")),
    tag = "dashboard",
)]
pub(crate) async fn update_profile_name(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(args): Json<UpdateProfileNameArgs>,
) -> ApiResult<StatusCode> {
    let id = auth.require_member()?;
    state
        .storage
        .update_member_name(id, &args.name)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/delete_account",
    responses((status = 200, description = "member deleted")),
    tag = "dashboard",
)]
pub(crate) async fn delete_account(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<StatusCode> {
    let id = auth.require_member()?;
    state
        .storage
        .delete_member(id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/dashboard/optins",
    responses((status = 200, body = GetOptInsResponse)),
    tag = "dashboard",
)]
pub(crate) async fn get_opt_ins(_auth: AuthIdentity) -> ApiResult<Json<GetOptInsResponse>> {
    Ok(Json(GetOptInsResponse {
        opt_ins_to_accept: Vec::new(),
    }))
}

#[utoipa::path(
    put,
    path = "/api/dashboard/optins",
    request_body = Vec<OptIn>,
    responses((status = 204, description = "opt-ins accepted")),
    tag = "dashboard",
)]
pub(crate) async fn accept_opt_ins(
    auth: AuthIdentity,
    State(state): State<OrchestratorState>,
    Json(opt_ins): Json<Vec<OptIn>>,
) -> ApiResult<StatusCode> {
    let id = auth.require_member()?;
    for o in opt_ins {
        state
            .storage
            .accept_opt_in(id, &o.name)
            .await
            .map_err(ApiError::Internal)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/dashboard/identities",
    responses((status = 200, description = "list of linked identities (empty in self-hosted)")),
    tag = "dashboard",
)]
pub(crate) async fn get_identities(_auth: AuthIdentity) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/unlink_identity",
    responses((status = 200, description = "stub no-op")),
    tag = "dashboard",
)]
pub(crate) async fn unlink_identity(_auth: AuthIdentity) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}
