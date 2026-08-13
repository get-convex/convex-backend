use common::types::{
    AttributionClaims,
    DeploymentMetadata,
};

/// Mints short-lived JWTs accepted by the AI gateway.
pub trait AiGatewayJwtMinter: Send + Sync {
    /// `deployment` and `attribution` must both come from trusted backend
    /// metadata or execution state rather than function input.
    fn mint(
        &self,
        deployment: &DeploymentMetadata,
        attribution: AttributionClaims,
    ) -> anyhow::Result<String>;
}
