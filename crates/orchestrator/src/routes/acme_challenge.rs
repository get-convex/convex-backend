//! Serves ACME HTTP-01 challenge responses.
//!
//! Traefik routes `/.well-known/acme-challenge/` for every custom domain here
//! on the plain HTTP entrypoint (see `custom_domains::render_config`), which
//! is what lets a domain be validated *before* it has a certificate.
//!
//! Deliberately unauthenticated: the ACME server is an anonymous client, and
//! the tokens are single-use, high-entropy values that only exist in memory
//! while an order is in flight.

use axum::{
    extract::{
        Path,
        State,
    },
    http::StatusCode,
    routing::get,
    Router,
};

use crate::state::OrchestratorState;

pub fn router() -> Router<OrchestratorState> {
    Router::new().route(
        "/.well-known/acme-challenge/{token}",
        get(serve_challenge),
    )
}

#[utoipa::path(
    get,
    path = "/.well-known/acme-challenge/{token}",
    params(("token" = String, Path)),
    responses(
        (status = 200, description = "Key authorization for an in-flight challenge"),
        (status = 404),
    ),
    tag = "acme",
)]
pub(crate) async fn serve_challenge(
    State(state): State<OrchestratorState>,
    Path(token): Path<String>,
) -> Result<String, StatusCode> {
    // Unknown tokens are indistinguishable from expired ones; 404 either way.
    state.challenges.get(&token).ok_or(StatusCode::NOT_FOUND)
}
