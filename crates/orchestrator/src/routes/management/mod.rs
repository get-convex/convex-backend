//! Public Management API at `/v1/...`.

pub(crate) mod deployments;
pub(crate) mod env_vars;
pub(crate) mod members;
pub(crate) mod projects;
pub(crate) mod teams;
pub(crate) mod tokens;

use axum::Router;

use crate::state::OrchestratorState;

pub fn router() -> Router<OrchestratorState> {
    Router::new()
        .merge(teams::router())
        .merge(projects::router())
        .merge(deployments::router())
        .merge(env_vars::router())
        .merge(tokens::router())
        .merge(members::router())
}
