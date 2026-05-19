use std::{
    path::PathBuf,
    str::FromStr,
};

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub database_url: String,
    pub data_root: PathBuf,
    pub public_origin: String,
    pub bootstrap_token: Option<String>,
    pub provisioner_mode: ProvisionerMode,
    /// Shared secret the dashboard uses to call `/api/internal/*`. Required
    /// for BetterAuth-mediated logins to mint PATs. If unset, the internal
    /// endpoints return 503.
    pub service_key: Option<String>,
    /// Comma-separated list of email addresses that are auto-promoted to
    /// admin on first registration.
    pub admin_emails: Vec<String>,
    /// First-run registration policy.
    pub registration_mode: RegistrationMode,
    /// Image tag the docker provisioner pulls/runs for each deployment.
    pub backend_image: String,
    /// Optional docker network name to attach spawned containers to.
    pub backend_network: Option<String>,
    /// Container name prefix for spawned backends (e.g. `orchestrator-`).
    pub backend_container_prefix: String,
    /// Hostname suffix used by the reverse proxy to route requests.
    /// Default `localhost`; full host header is `<deployment>.<router_host>`.
    pub router_host: String,
    /// Public port the proxy is reachable on from the browser. Used to
    /// build deployment URLs of the form `<scheme>://<name>.<host>[:<port>]`.
    pub router_public_port: u16,
    /// Public scheme (`http` or `https`) the proxy is reachable on. When
    /// terminating TLS in front of the orchestrator (Traefik etc.), set to
    /// `https`. Default `http` for raw-port local dev.
    pub router_public_scheme: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisionerMode {
    /// Operator pre-provisions backends and registers their URLs and admin
    /// keys. The orchestrator only stores metadata.
    External,
    /// Orchestrator mints credentials and reserves a port but doesn't spawn
    /// a backend — useful when the operator is running backends out-of-band
    /// but wants the dashboard's project/key flows to work end-to-end.
    Process,
    /// Orchestrator runs `docker run` to spawn one `convex-local-backend`
    /// container per deployment. Requires `/var/run/docker.sock` mounted.
    Docker,
}

impl FromStr for ProvisionerMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "external" => Ok(Self::External),
            "process" => Ok(Self::Process),
            "docker" => Ok(Self::Docker),
            other => Err(anyhow::anyhow!(
                "unknown provisioner mode {other:?} (expected `external`, `process`, or \
                 `docker`)"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    /// Only emails in `admin_emails` get admin; everyone else gets the
    /// `developer` role on the default team.
    Allowlist,
    /// First registration becomes admin; subsequent registrations land as
    /// developer. Documented as "evaluation mode".
    Open,
    /// Only emails with a valid invite code may register (not yet implemented
    /// — falls through to Allowlist for now).
    InviteOnly,
}

impl FromStr for RegistrationMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "allowlist" => Ok(Self::Allowlist),
            "open" => Ok(Self::Open),
            "invite-only" => Ok(Self::InviteOnly),
            other => Err(anyhow::anyhow!(
                "unknown registration mode {other:?} (expected allowlist, open, or invite-only)"
            )),
        }
    }
}
