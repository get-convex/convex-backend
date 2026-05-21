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
}

#[async_trait]
pub trait Provisioner: Send + Sync {
    /// Provision a backend for the given deployment, returning the URL,
    /// minted admin key, and instance secret.
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult>;

    /// Tear down a deployment's backend (if any). External mode is a no-op.
    async fn teardown(&self, deployment_name: &str) -> anyhow::Result<()>;

    /// Re-provision a deployment. The default implementation tears down the
    /// existing backend and re-provisions from scratch (losing volume data for
    /// provisioners that store data inside the container). `DockerProvisioner`
    /// overrides this with a volume-preserving version.
    async fn respawn(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        self.teardown(&req.deployment_name).await?;
        self.provision(req).await
    }
}
