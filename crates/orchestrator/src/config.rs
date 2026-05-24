use std::{
    path::PathBuf,
    str::FromStr,
};

use crate::provisioner::ProvisioningStrategy;

pub const DEFAULT_TEAM_NAME: &str = "Self-Hosted";

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
    /// Human-facing name for the auto-created default team. The slug remains
    /// `self-hosted` for stable CLI/dashboard lookups.
    pub default_team_name: String,
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
    /// When true, spawned backend containers get exact-host Traefik labels
    /// so browser traffic goes straight Traefik -> backend. The
    /// in-orchestrator proxy remains as a wildcard fallback.
    pub direct_backend_routing: bool,
    /// When true, new deployments spawn Postgres + MinIO sidecar containers
    /// alongside the backend. When false, new deployments use the v2
    /// volume+sqlite path. Existing deployments keep whatever `storage_mode`
    /// they were created with.
    pub enable_sidecars: bool,
    /// Docker image for the Postgres sidecar (used when `enable_sidecars`).
    pub postgres_image: String,
    /// Docker image for the MinIO sidecar (used when `enable_sidecars`).
    pub minio_image: String,
}

impl OrchestratorConfig {
    pub fn default_team_display_name(&self) -> &str {
        let name = self.default_team_name.trim();
        if name.is_empty() {
            DEFAULT_TEAM_NAME
        } else {
            name
        }
    }

    /// Resolve the v3 docker provisioning strategy from the flag bundle.
    /// `enable_sidecars=true` → Sidecar with the two image refs;
    /// `enable_sidecars=false` → VolumeSqlite (v2 escape hatch).
    pub fn provisioning_strategy(&self) -> ProvisioningStrategy {
        if self.enable_sidecars {
            ProvisioningStrategy::Sidecar {
                postgres_image: self.postgres_image.clone(),
                minio_image: self.minio_image.clone(),
            }
        } else {
            ProvisioningStrategy::VolumeSqlite
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(enable_sidecars: bool) -> OrchestratorConfig {
        OrchestratorConfig {
            database_url: "postgres://test".into(),
            data_root: PathBuf::from("/tmp/orch-test"),
            public_origin: "http://localhost".into(),
            bootstrap_token: None,
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
            enable_sidecars,
            postgres_image: "postgres:16-alpine".into(),
            minio_image: "quay.io/minio/minio:latest".into(),
        }
    }

    #[test]
    fn enable_sidecars_true_yields_sidecar_strategy() {
        let cfg = test_config(true);
        assert!(matches!(
            cfg.provisioning_strategy(),
            ProvisioningStrategy::Sidecar { .. }
        ));
    }

    #[test]
    fn enable_sidecars_false_yields_volume_sqlite() {
        let cfg = test_config(false);
        assert!(matches!(
            cfg.provisioning_strategy(),
            ProvisioningStrategy::VolumeSqlite
        ));
    }

    #[test]
    fn sidecar_strategy_carries_configured_images() {
        let mut cfg = test_config(true);
        cfg.postgres_image = "custom-pg:17".into();
        cfg.minio_image = "custom-mc:rel1".into();
        match cfg.provisioning_strategy() {
            ProvisioningStrategy::Sidecar {
                postgres_image,
                minio_image,
            } => {
                assert_eq!(postgres_image, "custom-pg:17");
                assert_eq!(minio_image, "custom-mc:rel1");
            },
            other => panic!("expected Sidecar, got {other:?}"),
        }
    }

    #[test]
    fn default_team_name_is_configurable() {
        let mut cfg = test_config(true);
        cfg.default_team_name = "  Defy Works  ".into();

        assert_eq!(cfg.default_team_display_name(), "Defy Works");

        cfg.default_team_name = "".into();

        assert_eq!(cfg.default_team_display_name(), "Self-Hosted");
    }

    #[test]
    fn direct_backend_routing_defaults_on_in_test_config() {
        let cfg = test_config(true);

        assert!(cfg.direct_backend_routing);
    }
}
