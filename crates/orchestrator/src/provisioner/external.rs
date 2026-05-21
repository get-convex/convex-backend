//! External provisioner: backends are managed by the operator. The
//! orchestrator's `register_external_deployment` API stores URL + admin key.

use async_trait::async_trait;

use super::{
    ProvisionRequest,
    ProvisionResult,
    Provisioner,
};

pub struct ExternalProvisioner;

impl ExternalProvisioner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExternalProvisioner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provisioner for ExternalProvisioner {
    async fn provision(&self, _req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        anyhow::bail!(
            "external provisioner: deployments must be pre-provisioned and registered via \
             POST /api/dashboard/deployments/register"
        )
    }

    async fn teardown(&self, _deployment_name: &str, _storage_mode: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
