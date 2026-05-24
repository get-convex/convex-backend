//! Integration tests for `convex-orchestrator`.
//!
//! Two layers, per the plan in
//! `docs/superpowers/specs/2026-05-02-convex-orchestrator-plan.md`:
//!
//! 1. **Default-run, no DB.** Asserts that the public Management API surface
//!    (`/v1/...`) advertised by `--print-openapi` matches the wire contract
//!    the dashboard's typed clients and `crates/big_brain_client` already
//!    expect, and round-trips the load-bearing deployment-internal DTOs
//!    through `serde_json` against their upstream definitions in
//!    `big_brain_private_api_types`.
//!
//! 2. **`#[ignore]`-gated, requires `TEST_ORCHESTRATOR_DATABASE_URL`.** Spins
//!    up a real `OrchestratorState` against a test Postgres, swaps in a stub
//!    provisioner, builds the axum router, and exercises every load-bearing
//!    deployment-internal endpoint end-to-end with `tower::oneshot`. Any
//!    response that fails to deserialize back into the upstream
//!    `big_brain_private_api_types` shape fails the test.
//!
//! Run only the default suite: `cargo test -p orchestrator --test integration`.
//! Run the DB-backed tests too:
//!   `TEST_ORCHESTRATOR_DATABASE_URL=postgres://... cargo test -p orchestrator
//!     --test integration -- --include-ignored`.

use orchestrator::{
    provisioner::{
        ProvisionRequest,
        ProvisionResult,
        Provisioner,
    },
    router::OrchestratorOpenApi,
};

// ---------------------------------------------------------------------------
// Layer 1: contract checks (no DB)
// ---------------------------------------------------------------------------

/// Documented `(method, path)` pairs that must appear in the spec.
///
/// This list is the wire contract. Adding a new public endpoint means adding
/// the route + a line here; renaming or dropping one breaks this test.
const EXPECTED_MANAGEMENT_OPERATIONS: &[(&str, &str)] = &[
    // tokens
    ("get", "/v1/list_personal_access_tokens"),
    ("post", "/v1/create_personal_access_token"),
    ("post", "/v1/delete_personal_access_token"),
    ("get", "/v1/token_details"),
    // teams
    ("post", "/v1/teams/create_team"),
    ("get", "/v1/teams/{team_id}/list_members"),
    ("post", "/v1/teams/{team_id}/invite_team_member"),
    ("post", "/v1/teams/{team_id}/create_access_token"),
    // projects
    ("post", "/v1/teams/{team_id}/create_project"),
    ("get", "/v1/teams/{team_id}/list_projects"),
    ("get", "/v1/projects/{project_id}"),
    ("get", "/v1/teams/{team_id_or_slug}/projects/{project_slug}"),
    ("post", "/v1/projects/{project_id}/delete"),
    ("get", "/v1/projects/{project_id}/settings"),
    ("patch", "/v1/projects/{project_id}/settings"),
    // deployments
    ("get", "/v1/projects/{project_id}/list_deployments"),
    ("post", "/v1/projects/{project_id}/create_deployment"),
    ("get", "/v1/projects/{project_id}/deployment"),
    ("get", "/v1/teams/{team_id_or_slug}/projects/{project_slug}/deployment"),
    ("get", "/v1/teams/{team_id}/list_deployments"),
    ("get", "/v1/teams/{team_id}/list_local_deployments"),
    ("get", "/v1/teams/{team_id}/list_deployment_classes"),
    ("get", "/v1/teams/{team_id}/list_deployment_regions"),
    ("get", "/v1/deployments/{deployment_name}"),
    ("post", "/v1/deployments/{deployment_name}/delete"),
    ("post", "/v1/deployments/{deployment_name}/transfer"),
    ("get", "/v1/deployments/{deployment_name}/settings"),
    ("patch", "/v1/deployments/{deployment_name}/settings"),
    ("post", "/v1/deployments/{deployment_name}/restart"),
    // env vars
    ("get", "/v1/projects/{project_id}/list_default_environment_variables"),
    ("post", "/v1/projects/{project_id}/update_default_environment_variables"),
];

#[test]
fn openapi_exposes_all_management_endpoints() {
    use utoipa::OpenApi;
    let spec = serde_json::to_value(OrchestratorOpenApi::openapi())
        .expect("serialize openapi spec");
    let paths = spec
        .get("paths")
        .and_then(|p| p.as_object())
        .expect("openapi spec has a `paths` object");

    let mut missing = Vec::new();
    for (method, path) in EXPECTED_MANAGEMENT_OPERATIONS {
        match paths.get(*path).and_then(|item| item.get(*method)) {
            Some(_) => {},
            None => missing.push(format!("{} {}", method.to_uppercase(), path)),
        }
    }
    assert!(
        missing.is_empty(),
        "OpenAPI spec is missing these documented operations:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn openapi_does_not_expose_undocumented_management_endpoints() {
    use utoipa::OpenApi;
    let spec = serde_json::to_value(OrchestratorOpenApi::openapi())
        .expect("serialize openapi spec");
    let paths = spec
        .get("paths")
        .and_then(|p| p.as_object())
        .expect("openapi spec has a `paths` object");

    let expected: std::collections::HashSet<(&str, &str)> =
        EXPECTED_MANAGEMENT_OPERATIONS.iter().copied().collect();

    // Only enforce the contract on `/v1/...` paths. Dashboard/internal/
    // deployment-internal endpoints are also annotated but are not part of
    // the public Management API surface; their wire shape is governed by the
    // dashboard / CLI / `big_brain_client` deserializers, not this test.
    let mut extra = Vec::new();
    for (path, item) in paths {
        if !path.starts_with("/v1/") {
            continue;
        }
        let item = item.as_object().expect("path item must be an object");
        for method in item.keys() {
            // Filter to HTTP methods only (ignore parameters/summary/etc.).
            if !matches!(
                method.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "trace"
            ) {
                continue;
            }
            if !expected.contains(&(method.as_str(), path.as_str())) {
                extra.push(format!("{} {}", method.to_uppercase(), path));
            }
        }
    }
    assert!(
        extra.is_empty(),
        "OpenAPI spec advertises /v1 operations not in the documented contract \
         (add them to EXPECTED_MANAGEMENT_OPERATIONS):\n  {}",
        extra.join("\n  ")
    );
}

#[test]
fn deployment_internal_dto_wire_format() {
    // Pin the wire format of the orchestrator's own deployment-internal DTOs
    // to camelCase, since the CLI and `big_brain_client` decode by field name.
    use orchestrator_api_types::deployment::{
        CreateProjectArgs,
        CreateProjectResponse,
        HasProjectsResponse,
        TeamSummary,
        UrlForKeyArgs,
        UrlForKeyResponse,
    };

    let v = serde_json::to_value(HasProjectsResponse {
        has_projects: true,
    })
    .unwrap();
    assert_eq!(v, serde_json::json!({"hasProjects": true}));

    let v = serde_json::to_value(TeamSummary {
        id: 7,
        name: "Self-Hosted".into(),
        slug: "self-hosted".into(),
    })
    .unwrap();
    assert_eq!(
        v,
        serde_json::json!({"id": 7, "name": "Self-Hosted", "slug": "self-hosted"})
    );

    // Args with optional fields must omit them (`#[serde(default)]` is
    // pointless on the wire if `serde(skip_serializing_if)` isn't set, but
    // the CLI sends only what the user supplied — verify both directions).
    let parsed: CreateProjectArgs = serde_json::from_value(serde_json::json!({
        "team": "self-hosted",
        "projectName": "Demo",
    }))
    .unwrap();
    assert_eq!(parsed.team, "self-hosted");
    assert_eq!(parsed.project_name, "Demo");
    assert!(parsed.deployment_type.is_none());

    let v = serde_json::to_value(CreateProjectResponse {
        project_id: 1,
        project_slug: "demo".into(),
        team_slug: "self-hosted".into(),
        deployment_name: Some("happy-otter-1".into()),
        url: Some("http://happy-otter-1.localhost:9000".into()),
        admin_key: None,
    })
    .unwrap();
    assert_eq!(v["projectId"], 1);
    assert_eq!(v["projectSlug"], "demo");
    assert_eq!(v["teamSlug"], "self-hosted");
    assert_eq!(v["deploymentName"], "happy-otter-1");

    let v = serde_json::to_value(UrlForKeyArgs {
        deploy_key: "prod:happy-otter-1|secret".into(),
    })
    .unwrap();
    assert_eq!(v, serde_json::json!({"deployKey": "prod:happy-otter-1|secret"}));

    let v = serde_json::to_value(UrlForKeyResponse {
        url: "http://happy-otter-1.localhost:9000".into(),
        deployment_name: "happy-otter-1".into(),
    })
    .unwrap();
    assert_eq!(v["url"], "http://happy-otter-1.localhost:9000");
    assert_eq!(v["deploymentName"], "happy-otter-1");
}

#[test]
fn deployment_auth_response_is_byte_identical_to_upstream() {
    // The orchestrator re-exports `big_brain_private_api_types` for
    // deployment-internal credential exchange so the CLI and
    // `big_brain_client` see byte-identical wire types. If somebody swaps the
    // re-export for a fork, this round-trip will fail.
    use big_brain_private_api_types as upstream;
    use orchestrator_api_types::deployment as exported;

    let upstream_json = serde_json::json!({
        "deploymentName": "happy-otter-123",
        "adminKey": "prod:happy-otter-123|s_secret",
        "url": "http://happy-otter-123.localhost:9000",
        "deploymentType": "prod",
        "reference": null,
        "isDefault": false,
    });
    let via_upstream: upstream::DeploymentAuthResponse =
        serde_json::from_value(upstream_json.clone()).expect("upstream parse");
    let via_exported: exported::DeploymentAuthResponse =
        serde_json::from_value(upstream_json).expect("re-export parse");

    let a = serde_json::to_value(&via_upstream).unwrap();
    let b = serde_json::to_value(&via_exported).unwrap();
    assert_eq!(a, b, "DeploymentAuthResponse re-export drifted from upstream");
}

// ---------------------------------------------------------------------------
// Layer 2: full DB integration (gated)
// ---------------------------------------------------------------------------

/// Stub provisioner that returns canned `ProvisionResult`s, so the integration
/// test doesn't depend on docker / a real backend.
struct StubProvisioner;

#[async_trait::async_trait]
impl Provisioner for StubProvisioner {
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        Ok(ProvisionResult {
            url: format!("http://{}.localhost:9000", req.deployment_name),
            site_url: format!("http://{}-site.localhost:9000", req.deployment_name),
            admin_key: format!("stub-admin-key-{}", req.deployment_name),
            admin_key_hash: "stub-hash".into(),
            admin_key_suffix: "stub".into(),
            instance_secret: "stub-instance-secret".into(),
            backend_instance_secret: "0".repeat(64),
            backend_pid: None,
            backend_port: 0,
            resolved_env: std::collections::BTreeMap::new(),
            sidecar_credentials: None,
        })
    }

    async fn teardown(&self, _deployment_name: &str, _storage_mode: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Full deployment-internal flow against a real Postgres. Set
/// `TEST_ORCHESTRATOR_DATABASE_URL` to enable.
///
/// **The test owns the database** — it drops and recreates the orchestrator's
/// schema. Point it at a throwaway database, never your dev one.
#[tokio::test]
#[ignore = "needs TEST_ORCHESTRATOR_DATABASE_URL"]
async fn deployment_internal_flow_against_real_db() {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{
            header::AUTHORIZATION,
            Request,
            StatusCode,
        },
    };
    use http_body_util::BodyExt;
    use orchestrator::{
        config::{
            OrchestratorConfig,
            ProvisionerMode,
            RegistrationMode,
        },
        router::build_router,
        state::OrchestratorState,
    };
    use orchestrator_api_types::deployment::{
        CreateProjectArgs,
        CreateProjectResponse,
        HasProjectsResponse,
        TeamSummary,
    };
    use tower::ServiceExt;

    let database_url = std::env::var("TEST_ORCHESTRATOR_DATABASE_URL")
        .expect("TEST_ORCHESTRATOR_DATABASE_URL not set (this test is `#[ignore]` by default)");
    let bootstrap_token = format!("test-bootstrap-{}", uuid_like());

    // Wipe the orchestrator's schema so the test is idempotent across runs.
    // The docstring already warns the caller that this owns the DB.
    reset_public_schema(&database_url).await;

    let data_root = tempfile::tempdir().expect("tempdir for data root");
    let config = OrchestratorConfig {
        database_url,
        data_root: data_root.path().to_path_buf(),
        public_origin: "http://localhost".into(),
        bootstrap_token: Some(bootstrap_token.clone()),
        provisioner_mode: ProvisionerMode::External,
        service_key: None,
        admin_emails: Vec::new(),
        default_team_name: "Self-Hosted".into(),
        registration_mode: RegistrationMode::Allowlist,
        backend_image: "irrelevant".into(),
        backend_network: None,
        backend_container_prefix: "test-".into(),
        router_host: "localhost".into(),
        router_public_port: 9000,
        router_public_scheme: "http".into(),
        direct_backend_routing: true,
        enable_sidecars: false,
        postgres_image: "postgres:16-alpine".into(),
        minio_image: "quay.io/minio/minio:latest".into(),
    };

    // Construct OrchestratorState the public way, then swap in the stub
    // provisioner so we can exercise create_deployment without docker.
    let mut state = OrchestratorState::new(config).await.expect("orchestrator state");
    state.provisioner = Arc::new(StubProvisioner);

    let app = build_router(state.clone());
    let bearer = format!("Bearer {bootstrap_token}");

    // 1. GET /api/teams should return the bootstrap team and deserialize as
    //    Vec<TeamSummary>.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/teams")
                .header(AUTHORIZATION, &bearer)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("send /api/teams");
    assert_eq!(resp.status(), StatusCode::OK, "GET /api/teams");
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let teams: Vec<TeamSummary> =
        serde_json::from_slice(&body).expect("Vec<TeamSummary> shape");
    assert!(!teams.is_empty(), "bootstrap team should be present");
    let team_slug = teams[0].slug.clone();

    // 2. GET /api/has_projects → HasProjectsResponse { hasProjects: false }.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/has_projects")
                .header(AUTHORIZATION, &bearer)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("send /api/has_projects");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let _: HasProjectsResponse =
        serde_json::from_slice(&body).expect("HasProjectsResponse shape");

    // 3. POST /api/create_project → CreateProjectResponse.
    let create_args = CreateProjectArgs {
        team: team_slug.clone(),
        project_name: "Integration Test Project".into(),
        deployment_type: Some("dev".into()),
        region: None,
        tier: None,
        provisioning_mode: None,
        knob_overrides: None,
    };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/create_project")
                .header(AUTHORIZATION, &bearer)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_args).unwrap()))
                .unwrap(),
        )
        .await
        .expect("send /api/create_project");
    assert_eq!(resp.status(), StatusCode::OK, "POST /api/create_project");
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let project: CreateProjectResponse =
        serde_json::from_slice(&body).expect("CreateProjectResponse shape");
    assert_eq!(project.team_slug, team_slug);

    // 4. has_projects flips to true.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/has_projects")
                .header(AUTHORIZATION, &bearer)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("send /api/has_projects (post-create)");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let after: HasProjectsResponse = serde_json::from_slice(&body).unwrap();
    assert!(after.has_projects, "has_projects should be true after create");
}

#[cfg(test)]
fn uuid_like() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:032x}")
}

/// `DROP SCHEMA public CASCADE; CREATE SCHEMA public;` so the next migration
/// run starts from a clean slate. Plain `tokio_postgres::NoTls`; do not point
/// at a TLS-only host without adapting this.
#[cfg(test)]
async fn reset_public_schema(database_url: &str) {
    use tokio_postgres::NoTls;
    let (client, conn) = tokio_postgres::connect(database_url, NoTls)
        .await
        .expect("connect to test postgres for reset");
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("test postgres reset connection ended: {e}");
        }
    });
    client
        .batch_execute("DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public;")
        .await
        .expect("reset public schema");
}
