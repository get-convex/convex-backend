use common::types::DeploymentId;

/// Mints short-lived JWTs accepted by the LLM gateway.
pub trait LlmGatewayJwtMinter: Send + Sync {
    /// `backend_deployment_id` must come from trusted backend metadata rather
    /// than function input.
    fn mint(&self, backend_deployment_id: Option<DeploymentId>) -> anyhow::Result<String>;
}
