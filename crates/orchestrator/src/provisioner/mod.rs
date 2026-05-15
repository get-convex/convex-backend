//! Backend provisioning. v1 ships an `External` mode (operator pre-provisions
//! and registers backends via API) and a `Process` mode skeleton.

mod docker;
mod external;
mod process;

pub use docker::DockerProvisioner;
pub use external::ExternalProvisioner;
pub use process::ProcessProvisioner;

use async_trait::async_trait;

use crate::storage::DeploymentType;

#[derive(Debug, Clone)]
pub struct ProvisionRequest {
    pub deployment_name: String,
    pub deployment_type: DeploymentType,
    pub project_id: i64,
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
}

#[async_trait]
pub trait Provisioner: Send + Sync {
    /// Provision a backend for the given deployment, returning the URL,
    /// minted admin key, and instance secret.
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult>;

    /// Tear down a deployment's backend (if any). External mode is a no-op.
    async fn teardown(&self, deployment_name: &str) -> anyhow::Result<()>;
}
