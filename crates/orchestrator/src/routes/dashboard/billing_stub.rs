//! Billing/Orb endpoints stubbed for self-hosted use.

use axum::{
    extract::Path,
    http::StatusCode,
    routing::{
        get,
        post,
        put,
    },
    Json,
    Router,
};
use orchestrator_api_types::stubs::{
    StubOrbSubscriptionResponse,
    StubSpendingLimitsResponse,
};

use crate::{
    auth::identity::AuthIdentity,
    errors::ApiResult,
    state::OrchestratorState,
    stub_data,
};

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route(
            "/teams/{team_id}/get_orb_subscription",
            get(orb_subscription),
        )
        .route("/teams/{team_id}/list_active_plans", get(empty_list))
        .route("/teams/{team_id}/list_invoices", get(empty_list))
        .route("/teams/{team_id}/has_failed_payment", get(no_failure))
        .route(
            "/teams/{team_id}/get_discounted_plan/{plan_id}/{promo_code}",
            get(orb_subscription),
        )
        .route("/teams/{team_id}/create_subscription", post(ok))
        .route("/teams/{team_id}/change_subscription_plan", post(ok))
        .route("/teams/{team_id}/cancel_orb_subscription", post(ok))
        .route(
            "/teams/{team_id}/unschedule_cancel_orb_subscription",
            post(ok),
        )
        .route("/teams/{team_id}/update_billing_address", put(ok))
        .route("/teams/{team_id}/update_billing_contact", put(ok))
        .route("/teams/{team_id}/update_payment_method", put(ok))
        .route(
            "/teams/{team_id}/create_setup_intent",
            post(empty_object),
        )
        .route("/teams/{team_id}/set_spending_limit", post(ok))
        .route("/teams/{team_id}/get_current_spend", get(zero_spend))
        .route(
            "/teams/{team_id}/get_spending_limits",
            get(spending_limits_unbounded),
        )
}

// In `billing_stub.rs`, several routes share a single handler implementation
// (most just return `200 OK` or an empty list). Each route annotated below
// is a distinct path; the underlying handler simply duplicates work that
// the dashboard's billing UI would otherwise expect from Cloud-only
// endpoints. See `pub fn router()` above for the full set of bindings.

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/get_orb_subscription",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = StubOrbSubscriptionResponse, description = "stub: returns a self-hosted plan record")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn orb_subscription(
    _auth: AuthIdentity,
) -> ApiResult<Json<StubOrbSubscriptionResponse>> {
    Ok(Json(stub_data::orb_subscription_stub()))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/list_active_plans",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: empty list. Also bound to /list_invoices.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_list(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(Vec::new()))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/has_failed_payment",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: { hasFailedPayment: false }")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn no_failure(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"hasFailedPayment": false})))
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/create_subscription",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub no-op. Also backs change_subscription_plan, cancel_orb_subscription, unschedule_cancel_orb_subscription, update_billing_*, set_spending_limit.")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn ok(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<StatusCode> {
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/dashboard/teams/{team_id}/create_setup_intent",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: empty JSON object")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn empty_object(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/get_current_spend",
    params(("team_id" = i64, Path)),
    responses((status = 200, description = "stub: { currentSpendCents: 0 }")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn zero_spend(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"currentSpendCents": 0})))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/teams/{team_id}/get_spending_limits",
    params(("team_id" = i64, Path)),
    responses((status = 200, body = StubSpendingLimitsResponse, description = "stub: no thresholds, zero spend")),
    tag = "dashboard_stubs",
)]
pub(crate) async fn spending_limits_unbounded(
    _auth: AuthIdentity,
    Path(_team_id): Path<i64>,
) -> ApiResult<Json<StubSpendingLimitsResponse>> {
    Ok(Json(stub_data::spending_limits_unbounded()))
}
