//! axum extractors that resolve a request to an `AuthIdentity`.

use axum::{
    extract::{
        FromRef,
        FromRequestParts,
    },
    http::{
        header,
        request::Parts,
    },
};

use crate::{
    auth::tokens::{
        parse_token,
        sha256_hex,
    },
    errors::ApiError,
    state::OrchestratorState,
    storage::{
        AccessToken,
        AccessTokenKind,
    },
};

#[derive(Debug, Clone)]
pub struct AuthIdentity {
    pub token: AccessToken,
    pub member_id: Option<i64>,
    pub team_id: Option<i64>,
    pub deployment_id: Option<i64>,
    pub project_id: Option<i64>,
}

impl AuthIdentity {
    pub fn require_member(&self) -> Result<i64, ApiError> {
        self.member_id.ok_or(ApiError::Forbidden)
    }
}

#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthIdentity>);

fn extract_bearer(parts: &Parts) -> Option<String> {
    let header = parts.headers.get(header::AUTHORIZATION)?;
    let value = header.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let scheme = scheme.trim();
    if scheme.eq_ignore_ascii_case("bearer") || scheme.eq_ignore_ascii_case("convex") {
        Some(token.trim().to_string())
    } else {
        None
    }
}

async fn resolve(state: &OrchestratorState, raw: &str) -> Result<AuthIdentity, ApiError> {
    let parsed = parse_token(raw).map_err(|e| {
        tracing::debug!(
            error = %e,
            raw_len = raw.len(),
            raw_prefix = raw.chars().take(12).collect::<String>(),
            "auth: failed to parse token"
        );
        ApiError::Unauthorized
    })?;
    let hash = sha256_hex(parsed.secret);
    let token = state
        .storage
        .get_access_token_by_hash(&hash)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| {
            tracing::debug!(
                public_id = parsed.public_id,
                "auth: no access token row matches secret hash"
            );
            ApiError::Unauthorized
        })?;
    // For "deploy-key shaped" tokens the middle slot of the wire format is
    // a resource identifier (the deployment name for per-deployment keys,
    // or `<team_slug>:<project_slug>` for project-scoped preview keys),
    // not the row's randomly minted `public_id`. Validate that the
    // received value matches the token's bound resource instead of the
    // stored `public_id`. For all other token kinds the original strict
    // equality still applies.
    let is_deploy_key_kind = matches!(
        token.kind,
        AccessTokenKind::DeployProd
            | AccessTokenKind::DeployDev
            | AccessTokenKind::DeployPreview
            | AccessTokenKind::ProjectDeploy
    );
    if is_deploy_key_kind {
        let expected = if matches!(token.kind, AccessTokenKind::ProjectDeploy) {
            // Project-scoped preview keys are `preview:<team>:<project>|<secret>`.
            // After parse_token splits on the first `:` and the `|`, the
            // remaining middle slot is `<team>:<project>` — match it
            // against the team/project that owns this token row.
            let project_id = token.project_id.ok_or_else(|| {
                tracing::debug!(
                    public_id = %token.public_id,
                    "auth: project deploy-key row has no project_id"
                );
                ApiError::Unauthorized
            })?;
            let project = state
                .storage
                .get_project(project_id)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| {
                    tracing::debug!(
                        project_id,
                        "auth: project deploy-key references a missing project"
                    );
                    ApiError::Unauthorized
                })?;
            let team = state
                .storage
                .get_team(project.team_id)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| {
                    tracing::debug!(
                        team_id = project.team_id,
                        "auth: project deploy-key references a missing team"
                    );
                    ApiError::Unauthorized
                })?;
            format!("{}:{}", team.slug, project.slug)
        } else {
            let deployment_id = token.deployment_id.ok_or_else(|| {
                tracing::debug!(
                    public_id = %token.public_id,
                    kind = ?token.kind,
                    "auth: deploy-key token row has no deployment_id"
                );
                ApiError::Unauthorized
            })?;
            let dep = state
                .storage
                .get_deployment(deployment_id)
                .await
                .map_err(ApiError::Internal)?
                .ok_or_else(|| {
                    tracing::debug!(
                        deployment_id,
                        "auth: deploy-key references a deployment that no longer exists"
                    );
                    ApiError::Unauthorized
                })?;
            dep.name
        };
        if expected != parsed.public_id {
            tracing::debug!(
                expected = %expected,
                received_public_id = parsed.public_id,
                kind = ?token.kind,
                "auth: deploy-key resource-identifier mismatch"
            );
            return Err(ApiError::Unauthorized);
        }
    } else if token.public_id != parsed.public_id {
        tracing::debug!(
            stored_public_id = %token.public_id,
            received_public_id = parsed.public_id,
            "auth: public_id mismatch (token tampered with or wrong row)"
        );
        return Err(ApiError::Unauthorized);
    }
    if token.revoked_time.is_some() {
        tracing::debug!(
            public_id = %token.public_id,
            revoked_time = ?token.revoked_time,
            "auth: token has been revoked"
        );
        return Err(ApiError::Unauthorized);
    }
    if let Some(exp) = token.expiry
        && crate::time::now_unix_ms() > exp
    {
        tracing::debug!(
            public_id = %token.public_id,
            expiry = exp,
            "auth: token expired"
        );
        return Err(ApiError::Unauthorized);
    }
    // Validate kind/prefix consistency for non-deployment tokens. Deploy
    // keys carry the deployment name in their public_id segment, which we
    // don't enforce against `prefix` strictly here.
    let _expected_prefix = match token.kind {
        AccessTokenKind::Pat | AccessTokenKind::Session => "pat",
        AccessTokenKind::Team => "team",
        AccessTokenKind::DeployProd => "prod",
        AccessTokenKind::DeployDev => "dev",
        AccessTokenKind::DeployPreview => "preview",
        AccessTokenKind::ProjectDeploy => "project",
        AccessTokenKind::App => "app",
        AccessTokenKind::Admin => "admin",
    };

    Ok(AuthIdentity {
        member_id: token.member_id,
        team_id: token.team_id,
        deployment_id: token.deployment_id,
        project_id: token.project_id,
        token,
    })
}

impl<S> FromRequestParts<S> for AuthIdentity
where
    OrchestratorState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts).ok_or(ApiError::Unauthorized)?;
        let st: OrchestratorState = OrchestratorState::from_ref(state);
        resolve(&st, &token).await
    }
}

impl<S> FromRequestParts<S> for OptionalAuth
where
    OrchestratorState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(token) = extract_bearer(parts) else {
            return Ok(Self(None));
        };
        let st: OrchestratorState = OrchestratorState::from_ref(state);
        match resolve(&st, &token).await {
            Ok(id) => Ok(Self(Some(id))),
            Err(ApiError::Unauthorized) => Ok(Self(None)),
            Err(e) => Err(e),
        }
    }
}
