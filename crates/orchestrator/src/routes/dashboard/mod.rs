//! Dashboard private API at `/api/dashboard/...`.

pub(crate) mod access_tokens;
pub(crate) mod audit_log;
pub(crate) mod billing_stub;
pub(crate) mod cloud_backups_stub;
pub(crate) mod custom_domains;
pub(crate) mod deployments;
pub(crate) mod env_vars;
pub(crate) mod host_capacity;
pub mod knob_registry;
pub(crate) mod integrations_stub;
pub(crate) mod profile;
pub(crate) mod projects;
pub(crate) mod teams;
pub(crate) mod usage_stub;

use axum::{
    routing::get,
    Router,
};

use crate::state::OrchestratorState;

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .route("/host_capacity", get(host_capacity::host_capacity))
        .route("/knob_registry", get(knob_registry::knob_registry))
        .merge(profile::router())
        .merge(teams::router())
        .merge(projects::router())
        .merge(deployments::router())
        .merge(custom_domains::router())
        .merge(access_tokens::router())
        .merge(env_vars::router())
        .merge(audit_log::router())
        .merge(billing_stub::router())
        .merge(cloud_backups_stub::router())
        .merge(integrations_stub::router())
        .merge(usage_stub::router())
}
