//! Top-level HTTP router.

use std::time::Duration;

use axum::{
    http::{
        header,
        HeaderName,
        Method,
        StatusCode,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::cors::{
    AllowHeaders,
    AllowMethods,
    AllowOrigin,
    CorsLayer,
};

use crate::{
    routes,
    state::OrchestratorState,
};

pub fn build_router(state: OrchestratorState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
        ]))
        .allow_headers(AllowHeaders::list([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("convex-client"),
            HeaderName::from_static("google-analytics-client-id"),
        ]))
        .max_age(Duration::from_secs(3600));

    let api_router = routes::deployment_internal::router();
    let dashboard_router = routes::dashboard::router();
    let internal_router = routes::internal::router();
    let management_router = routes::management::router();

    Router::new()
        .nest("/api/dashboard", dashboard_router)
        .nest("/api/internal", internal_router)
        .nest("/api", api_router)
        .nest("/v1", management_router)
        // Unauthenticated by design: the ACME server is an anonymous client.
        // Traefik forwards this path here for every custom domain.
        .merge(routes::acme_challenge::router())
        .route("/version", get(version))
        .route("/health", get(health))
        .fallback(not_found)
        .with_state(state)
        .layer(ServiceBuilder::new().layer(cors))
}

async fn version() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "service": "convex-orchestrator",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "code": "NotFound",
            "message": "endpoint not implemented in convex-orchestrator",
        })),
    )
}

/// Aggregate OpenAPI definition for every HTTP surface the orchestrator
/// exposes.
///
/// Three groups, by tag:
///
/// - **`/v1/...`** — public Management API. The wire contract for typed
///   external clients (CLI codegen, third-party integrations).
/// - **`/api/...`** (deployment-internal) — load-bearing endpoints used by
///   `crates/big_brain_client` and the CLI's `bigBrainAPI` calls.
/// - **`/api/dashboard/...`** — private dashboard API. The `_stubs` tag
///   marks endpoints that intentionally return canned data because the
///   underlying feature is Cloud-only (Orb billing, WorkOS, Vercel, etc.).
/// - **`/api/internal/...`** — service-key-gated endpoints used only by
///   `dashboard-orchestrator`'s server side.
///
/// Where a single handler backs multiple routes (common in the stub files
/// like `billing_stub.rs` and `integrations_stub.rs`), only the primary
/// route is documented here; the handler's `#[utoipa::path]` annotation
/// names the alias routes in its `description`.
#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "convex-orchestrator",
        description = "Self-hosted replacement for Convex Cloud's BigBrain orchestrator.",
    ),
    paths(
        // Public Management API (/v1/...)
        crate::routes::management::tokens::list_personal_access_tokens,
        crate::routes::management::tokens::create_personal_access_token,
        crate::routes::management::tokens::delete_personal_access_token,
        crate::routes::management::tokens::token_details,
        crate::routes::management::teams::create_team,
        crate::routes::management::teams::list_members,
        crate::routes::management::teams::invite_member,
        crate::routes::management::teams::create_team_access_token,
        crate::routes::management::projects::create_project,
        crate::routes::management::projects::list_projects,
        crate::routes::management::projects::get_project,
        crate::routes::management::projects::get_project_by_slug,
        crate::routes::management::projects::delete_project,
        crate::routes::management::projects::get_project_settings,
        crate::routes::management::projects::patch_project_settings,
        crate::routes::management::deployments::list_deployments,
        crate::routes::management::deployments::create_deployment,
        crate::routes::management::deployments::get_default_deployment_for_project,
        crate::routes::management::deployments::get_default_deployment_by_slug,
        crate::routes::management::deployments::list_team_deployments,
        crate::routes::management::deployments::list_local_deployments,
        crate::routes::management::deployments::list_deployment_classes,
        crate::routes::management::deployments::list_deployment_regions,
        crate::routes::management::deployments::get_deployment,
        crate::routes::management::deployments::delete_deployment,
        crate::routes::management::deployments::transfer_deployment,
        crate::routes::management::deployments::get_deployment_settings,
        crate::routes::management::deployments::patch_deployment_settings,
        crate::routes::management::deployments::restart_deployment,
        crate::routes::management::env_vars::list,
        crate::routes::management::env_vars::update,
        // Deployment-internal (/api/...)
        crate::routes::deployment_internal::orchestrator_version,
        crate::routes::deployment_internal::authorize,
        crate::routes::deployment_internal::authorize_head,
        crate::routes::deployment_internal::check_opt_ins,
        crate::routes::deployment_internal::accept_opt_ins,
        crate::routes::deployment_internal::list_teams,
        crate::routes::deployment_internal::has_projects,
        crate::routes::deployment_internal::create_project,
        crate::routes::deployment_internal::delete_project,
        crate::routes::deployment_internal::team_and_project,
        crate::routes::deployment_internal::authorize_within_current_project,
        crate::routes::deployment_internal::provision_and_authorize,
        crate::routes::deployment_internal::claim_preview_deployment,
        // Dashboard private API (/api/dashboard/...)
        crate::routes::dashboard::profile::get_profile,
        crate::routes::dashboard::profile::get_member_data,
        crate::routes::dashboard::profile::list_profile_emails,
        crate::routes::dashboard::profile::create_profile_email,
        crate::routes::dashboard::profile::delete_profile_email,
        crate::routes::dashboard::profile::update_primary_email,
        crate::routes::dashboard::profile::resend_verification,
        crate::routes::dashboard::profile::verify_email,
        crate::routes::dashboard::profile::update_profile_name,
        crate::routes::dashboard::profile::delete_account,
        crate::routes::dashboard::profile::get_opt_ins,
        crate::routes::dashboard::profile::accept_opt_ins,
        crate::routes::dashboard::profile::get_identities,
        crate::routes::dashboard::profile::unlink_identity,
        crate::routes::dashboard::teams::list_teams,
        crate::routes::dashboard::teams::create_team,
        crate::routes::dashboard::teams::update_team,
        crate::routes::dashboard::teams::delete_team,
        crate::routes::dashboard::teams::list_members,
        crate::routes::dashboard::teams::remove_member,
        crate::routes::dashboard::teams::update_member_role,
        crate::routes::dashboard::teams::get_entitlements_stub,
        crate::routes::dashboard::teams::unpause_deployments,
        crate::routes::dashboard::teams::list_invites,
        crate::routes::dashboard::teams::create_invite,
        crate::routes::dashboard::teams::cancel_invite,
        crate::routes::dashboard::teams::accept_invite,
        crate::routes::dashboard::teams::get_project_roles,
        crate::routes::dashboard::teams::update_project_roles,
        crate::routes::dashboard::teams::apply_referral_code,
        crate::routes::dashboard::teams::get_referral_state_stub,
        crate::routes::dashboard::teams::validate_referral_code_stub,
        crate::routes::dashboard::projects::list_projects,
        crate::routes::dashboard::projects::get_project_by_slug,
        crate::routes::dashboard::projects::get_project,
        crate::routes::dashboard::projects::update_project,
        crate::routes::dashboard::projects::delete_projects,
        crate::routes::dashboard::projects::transfer_project,
        crate::routes::dashboard::deployments::get_deployment_by_id,
        crate::routes::dashboard::deployments::get_deployment_auth_dashboard,
        crate::routes::dashboard::deployments::register_deployment,
        crate::routes::dashboard::access_tokens::list_team_tokens,
        crate::routes::dashboard::access_tokens::list_project_tokens,
        crate::routes::dashboard::access_tokens::list_deployment_tokens,
        crate::routes::dashboard::access_tokens::delete_access_token,
        crate::routes::dashboard::env_vars::list_env_vars,
        crate::routes::dashboard::env_vars::update_env_vars,
        crate::routes::dashboard::custom_domains::list_custom_domains,
        crate::routes::dashboard::custom_domains::create_custom_domain,
        crate::routes::dashboard::custom_domains::delete_custom_domain,
        crate::routes::dashboard::custom_domains::verify_custom_domain,
        crate::routes::dashboard::custom_domains::retry_custom_domain,
        crate::routes::dashboard::custom_domains::list_dns_creds,
        crate::routes::dashboard::custom_domains::create_dns_cred,
        crate::routes::dashboard::custom_domains::delete_dns_cred,
        crate::routes::acme_challenge::serve_challenge,
        crate::routes::dashboard::audit_log::get_audit_log_events,
        crate::routes::dashboard::billing_stub::orb_subscription,
        crate::routes::dashboard::billing_stub::empty_list,
        crate::routes::dashboard::billing_stub::no_failure,
        crate::routes::dashboard::billing_stub::ok,
        crate::routes::dashboard::billing_stub::empty_object,
        crate::routes::dashboard::billing_stub::zero_spend,
        crate::routes::dashboard::billing_stub::spending_limits_unbounded,
        crate::routes::dashboard::cloud_backups_stub::not_implemented,
        crate::routes::dashboard::cloud_backups_stub::not_implemented_obj,
        crate::routes::dashboard::cloud_backups_stub::empty_list,
        crate::routes::dashboard::cloud_backups_stub::empty_object,
        crate::routes::dashboard::integrations_stub::empty_list,
        crate::routes::dashboard::integrations_stub::empty_object,
        crate::routes::dashboard::integrations_stub::empty_url,
        crate::routes::dashboard::integrations_stub::false_object,
        crate::routes::dashboard::integrations_stub::ok,
        crate::routes::dashboard::integrations_stub::ok_obj,
        crate::routes::dashboard::integrations_stub::not_configured,
        crate::routes::dashboard::host_capacity::host_capacity,
        crate::routes::dashboard::knob_registry::knob_registry,
        crate::routes::dashboard::usage_stub::team_usage_state,
        crate::routes::dashboard::usage_stub::billing_period,
        crate::routes::dashboard::usage_stub::token_info,
        crate::routes::dashboard::usage_stub::usage_query,
        // Service-key-gated internal endpoints (/api/internal/...)
        crate::routes::internal::exchange_session,
        crate::routes::internal::health,
    ),
    tags(
        (name = "tokens", description = "/v1: personal access tokens"),
        (name = "teams", description = "/v1: teams and team membership"),
        (name = "projects", description = "/v1: projects within a team"),
        (name = "deployments", description = "/v1: deployments within a project"),
        (name = "env_vars", description = "/v1: project-level default environment variables"),
        (name = "deployment_internal", description = "/api: load-bearing CLI / big_brain_client endpoints"),
        (name = "dashboard", description = "/api/dashboard: private dashboard API"),
        (name = "dashboard_stubs", description = "/api/dashboard: stubbed Cloud-only endpoints (Orb, WorkOS, Vercel, Discord, cloud backups, usage analytics)"),
        (name = "internal", description = "/api/internal: service-key-gated endpoints used only by dashboard-orchestrator"),
    ),
)]
pub struct OrchestratorOpenApi;

/// Generate the OpenAPI spec for the orchestrator.
///
/// Drift-tested against `crates/orchestrator/openapi.json`; regenerate the
/// fixture by running
/// `cargo run -p orchestrator --bin convex-orchestrator -- --print-openapi
///   > crates/orchestrator/openapi.json`.
pub fn openapi_spec() -> anyhow::Result<serde_json::Value> {
    use utoipa::OpenApi;
    let mut spec = serde_json::to_value(OrchestratorOpenApi::openapi())?;
    // Pin the `info.version` to the orchestrator crate version so the spec
    // is stable across utoipa upgrades that may emit a default version.
    if let Some(info) = spec.get_mut("info").and_then(|v| v.as_object_mut()) {
        info.insert(
            "version".into(),
            serde_json::Value::String(env!("CARGO_PKG_VERSION").into()),
        );
    }
    Ok(spec)
}

#[cfg(test)]
mod openapi_tests {
    use pretty_assertions::assert_eq;

    /// Drift guard for `crates/orchestrator/openapi.json`.
    ///
    /// Compares the live generated spec to the checked-in fixture as parsed
    /// `serde_json::Value`s, so whitespace-only changes (e.g. dprint
    /// reformatting the fixture) don't trip the test. To regenerate after
    /// intentional changes:
    /// `cargo run -p orchestrator --bin convex-orchestrator -- --print-openapi
    ///   > crates/orchestrator/openapi.json && dprint fmt crates/orchestrator/openapi.json`
    #[test]
    fn openapi_fixture_matches() {
        let generated = super::openapi_spec().expect("openapi_spec()");

        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../openapi.json"))
                .expect("openapi.json parses as JSON");

        assert_eq!(
            generated, fixture,
            "openapi.json drift — regenerate the fixture (see test docstring)"
        );
    }
}
