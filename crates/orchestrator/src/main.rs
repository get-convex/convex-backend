use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use orchestrator::{
    config::OrchestratorConfig,
    state::OrchestratorState,
};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

#[derive(Parser, Debug)]
#[command(
    name = "convex-orchestrator",
    about = "Self-hosted replacement for Convex Cloud's BigBrain orchestrator."
)]
struct Args {
    /// Address for the provisioning API (mirrors prod BigBrain port 8050).
    #[arg(long, env = "CONVEX_ORCHESTRATOR_PROVISION_ADDR", default_value = "0.0.0.0:8050")]
    provision_addr: String,

    /// Postgres connection URL. PlanetScale Postgres requires `sslmode=require`.
    /// Format: `postgres://user:pass@host:5432/dbname?sslmode=require`.
    /// Required to start the server; not required when only `--print-openapi`
    /// is set.
    #[arg(long, env = "CONVEX_ORCHESTRATOR_DATABASE_URL")]
    database_url: Option<String>,

    /// Directory where per-deployment data is stored when running the
    /// process-mode provisioner.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_DATA_ROOT",
        default_value = "convex_orchestrator_data"
    )]
    data_root: PathBuf,

    /// Public origin used to construct deployment URLs in responses.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_PUBLIC_ORIGIN",
        default_value = "http://127.0.0.1"
    )]
    public_origin: String,

    /// Optional bootstrap token: if set, the first POST /api/authorize
    /// using this token mints a new owner member + PAT.
    #[arg(long, env = "CONVEX_ORCHESTRATOR_BOOTSTRAP_TOKEN")]
    bootstrap_token: Option<String>,

    /// Print the OpenAPI spec to stdout and exit.
    #[arg(long, default_value_t = false)]
    print_openapi: bool,

    /// Provisioner mode: `external`, `process`, or `docker`.
    #[arg(long, env = "CONVEX_ORCHESTRATOR_PROVISIONER", default_value = "external")]
    provisioner: String,

    /// Image tag the docker provisioner uses for spawned backends.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_BACKEND_IMAGE",
        default_value = "ghcr.io/get-convex/convex-backend:latest"
    )]
    backend_image: String,

    /// Container name prefix for backends spawned by the docker provisioner.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_BACKEND_CONTAINER_PREFIX",
        default_value = "orchestrator-deployment-"
    )]
    backend_container_prefix: String,

    /// Optional docker network name to attach spawned backends to. Empty =
    /// default bridge.
    #[arg(long, env = "CONVEX_ORCHESTRATOR_BACKEND_NETWORK")]
    backend_network: Option<String>,

    /// Address the reverse proxy listens on. Routes browser requests of
    /// the form `<deployment>.<router_host>` to the matching container.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_ROUTER_ADDR",
        default_value = "0.0.0.0:9000"
    )]
    router_addr: String,

    /// Hostname suffix used to construct deployment URLs. Browsers must
    /// resolve `*.<router_host>` to the proxy. The default `localhost`
    /// works because `*.localhost` resolves to the loopback per RFC 6761.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_ROUTER_HOST",
        default_value = "localhost"
    )]
    router_host: String,

    /// Public port for the proxy as seen by browsers. Almost always equals
    /// the port portion of `--router-addr`, but split out so docker port
    /// mapping (e.g. host 9000 → container 9000) can override it.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_ROUTER_PUBLIC_PORT",
        default_value_t = 9000
    )]
    router_public_port: u16,

    /// Public scheme (`http` or `https`) for browser-facing deployment URLs.
    /// Set to `https` when terminating TLS in front of the orchestrator
    /// (e.g. via Traefik), `http` for raw-port local dev.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_ROUTER_PUBLIC_SCHEME",
        default_value = "http"
    )]
    router_public_scheme: String,

    /// When true, spawned backend containers get direct Traefik routers for
    /// their API and site hosts. The in-orchestrator proxy remains a
    /// wildcard fallback for unlabeled deployments.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_DIRECT_BACKEND_ROUTING",
        default_value_t = true
    )]
    direct_backend_routing: bool,

    /// Shared secret the dashboard-orchestrator uses to call internal
    /// PAT-minting endpoints after BetterAuth authenticates a user.
    #[arg(long, env = "CONVEX_ORCHESTRATOR_SERVICE_KEY")]
    service_key: Option<String>,

    /// Comma-separated list of email addresses that auto-receive the admin
    /// role on first registration. If unset and `--registration-mode` is
    /// `allowlist`, only the first registrant ever becomes admin.
    #[arg(long, env = "CONVEX_ORCHESTRATOR_ADMIN_EMAILS", value_delimiter = ',')]
    admin_emails: Vec<String>,

    /// Human-facing name for the auto-created default team. The slug remains
    /// `self-hosted`.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_DEFAULT_TEAM_NAME",
        default_value = "Self-Hosted"
    )]
    default_team_name: String,

    /// First-run registration policy: `allowlist` (default) | `open` |
    /// `invite-only`.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_REGISTRATION",
        default_value = "allowlist"
    )]
    registration: String,

    /// When true (default), new deployments spawn Postgres + MinIO sidecar
    /// containers alongside the backend. When false, new deployments use
    /// the v2 volume+sqlite path. Existing deployments keep their original
    /// storage_mode regardless.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_ENABLE_SIDECARS",
        default_value_t = true
    )]
    enable_sidecars: bool,

    /// Docker image for the Postgres sidecar.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_POSTGRES_IMAGE",
        default_value = "postgres:16-alpine"
    )]
    postgres_image: String,

    /// Docker image for the MinIO sidecar.
    #[arg(
        long,
        env = "CONVEX_ORCHESTRATOR_MINIO_IMAGE",
        default_value = "quay.io/minio/minio:latest"
    )]
    minio_image: String,
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("orchestrator=info,tower_http=info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let args = Args::parse();

    if args.print_openapi {
        let spec = orchestrator::router::openapi_spec()?;
        println!("{}", serde_json::to_string_pretty(&spec)?);
        return Ok(());
    }

    let database_url = args
        .database_url
        .context("--database-url (or CONVEX_ORCHESTRATOR_DATABASE_URL) is required")?;

    let config = OrchestratorConfig {
        database_url,
        data_root: args.data_root,
        public_origin: args.public_origin,
        bootstrap_token: args.bootstrap_token,
        provisioner_mode: args.provisioner.parse()?,
        service_key: args.service_key,
        admin_emails: args.admin_emails,
        default_team_name: args.default_team_name,
        registration_mode: args.registration.parse()?,
        backend_image: args.backend_image,
        backend_network: args.backend_network,
        backend_container_prefix: args.backend_container_prefix.clone(),
        router_host: args.router_host.clone(),
        router_public_port: args.router_public_port,
        router_public_scheme: args.router_public_scheme.clone(),
        direct_backend_routing: args.direct_backend_routing,
        enable_sidecars: args.enable_sidecars,
        postgres_image: args.postgres_image,
        minio_image: args.minio_image,
    };

    tracing::info!(?config.data_root, "starting convex-orchestrator");

    let state = OrchestratorState::new(config).await?;

    let app = orchestrator::router::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(&args.provision_addr)
        .await
        .with_context(|| format!("binding {}", args.provision_addr))?;

    tracing::info!(addr = %args.provision_addr, "convex-orchestrator listening");

    // Spawn the reverse proxy on a separate task. It listens on a different
    // port from the management API and routes by Host-header subdomain.
    let proxy_state = state.clone();
    let proxy_addr: std::net::SocketAddr = args
        .router_addr
        .parse()
        .with_context(|| format!("parsing --router-addr {}", args.router_addr))?;
    let proxy_cfg = orchestrator::proxy::ProxyConfig::new(
        args.router_host.clone(),
        args.backend_container_prefix.clone(),
    );
    tokio::spawn(async move {
        if let Err(e) = orchestrator::proxy::serve_proxy(proxy_state, proxy_cfg, proxy_addr).await {
            tracing::error!(error = %e, "convex-orchestrator proxy exited");
        }
    });

    axum::serve(listener, app.into_make_service())
        .await
        .context("serving HTTP")?;

    Ok(())
}
