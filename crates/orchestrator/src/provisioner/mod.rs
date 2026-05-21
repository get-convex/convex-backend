//! Backend provisioning. v1 ships an `External` mode (operator pre-provisions
//! and registers backends via API) and a `Process` mode skeleton.

mod docker;
mod external;
mod process;
pub mod env;
pub mod sidecar;
pub mod tiers;

pub use docker::DockerProvisioner;
pub use external::ExternalProvisioner;
pub use process::ProcessProvisioner;

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::storage::DeploymentType;

/// Strategy `DockerProvisioner` uses when bringing up a new deployment.
/// Decided at orchestrator startup; snapshotted onto each deployment's
/// `storage_mode` column so the rest of its lifecycle (teardown, restart)
/// stays in the original mode.
#[derive(Debug, Clone)]
pub enum ProvisioningStrategy {
    /// v2 behavior — single backend container with a `/convex/data` named
    /// volume holding SQLite + file storage.
    VolumeSqlite,
    /// v3 behavior — backend container plus two sidecars (Postgres + MinIO)
    /// on the existing docker network. Credentials minted per deployment.
    Sidecar {
        postgres_image: String,
        minio_image: String,
    },
}

/// Credentials minted (or reused) for a deployment's sidecar containers.
/// Only populated when the provisioner runs in `Sidecar` mode; the caller
/// snapshots them onto the deployment row so restart can reuse them.
#[derive(Clone)]
pub struct SidecarCredentials {
    pub pg_password: String,
    pub minio_root_user: String,
    pub minio_root_password: String,
}

impl std::fmt::Debug for SidecarCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SidecarCredentials")
            .field("pg_password", &"<redacted>")
            .field("minio_root_user", &"<redacted>")
            .field("minio_root_password", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ProvisionRequest {
    pub deployment_name: String,
    pub deployment_type: DeploymentType,
    pub project_id: i64,
    /// Tier name e.g. "S4" | "S8" | "S16" | "S32". Caller resolves the
    /// project's tier before calling.
    pub tier: String,
    /// Per-project knob overrides layered on top of the tier defaults.
    pub knob_overrides: BTreeMap<String, String>,
    /// When `Some(...)`, use this instance_secret instead of generating a new
    /// one. Used by the restart flow so the backend's admin keys stay valid
    /// across restarts (they are derived from this secret).
    pub existing_instance_secret: Option<String>,
    /// When `Some(...)`, the provisioner reuses these sidecar credentials
    /// instead of minting new ones. Set by the restart flow so the new
    /// backend container connects to the same already-running pg/minio
    /// sidecars with the same credentials.
    pub sidecar_credentials: Option<SidecarCredentials>,
}

#[derive(Debug, Clone)]
pub struct ProvisionResult {
    pub url: String,
    pub site_url: String,
    pub admin_key: String,
    pub admin_key_hash: String,
    pub admin_key_suffix: String,
    pub instance_secret: String,
    pub backend_pid: Option<i64>,
    pub backend_port: i64,
    /// The fully-resolved env that was passed to the backend container
    /// (base + tier defaults + project overrides), snapshotted for audit.
    pub resolved_env: BTreeMap<String, String>,
    /// Populated when the strategy was Sidecar. Carries the credentials
    /// (either freshly minted or reused) so the caller can snapshot them
    /// onto the deployment row.
    pub sidecar_credentials: Option<SidecarCredentials>,
}

impl ProvisionResult {
    /// Destructure for `NewDeployment` insertion: returns the snapshot
    /// columns (storage_mode + optional sidecar creds) borrowed from
    /// `self`. `Some(creds)` → sidecar mode; `None` → volume-sqlite.
    pub fn storage_columns(
        &self,
    ) -> (
        &'static str,
        Option<&str>,
        Option<&str>,
        Option<&str>,
    ) {
        match &self.sidecar_credentials {
            Some(c) => (
                "sidecar",
                Some(c.pg_password.as_str()),
                Some(c.minio_root_user.as_str()),
                Some(c.minio_root_password.as_str()),
            ),
            None => ("volume-sqlite", None, None, None),
        }
    }
}

#[async_trait]
pub trait Provisioner: Send + Sync {
    /// Provision a backend for the given deployment, returning the URL,
    /// minted admin key, and instance secret.
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult>;

    /// Tear down a deployment's backend (if any). External mode is a no-op.
    ///
    /// `storage_mode` is the deployment row's snapshot of which provisioning
    /// strategy was used (`"volume-sqlite"` or `"sidecar"`). The Docker
    /// provisioner branches on it so sidecar-mode deployments get their
    /// pg/minio sidecars + volumes torn down while volume-sqlite mode keeps
    /// the v2 single-volume cleanup. Other provisioner impls (process,
    /// external, stub) ignore the arg since they don't own storage.
    async fn teardown(&self, deployment_name: &str, storage_mode: &str) -> anyhow::Result<()>;

    /// Re-provision a deployment. The default implementation tears down the
    /// existing backend and re-provisions from scratch (losing volume data for
    /// provisioners that store data inside the container). `DockerProvisioner`
    /// overrides this with a volume-preserving version.
    async fn respawn(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        // Default respawn assumes volume-sqlite semantics; the docker
        // provisioner overrides `respawn` entirely so this default is only
        // exercised by impls that ignore `storage_mode` anyway.
        self.teardown(&req.deployment_name, "volume-sqlite").await?;
        self.provision(req).await
    }
}
