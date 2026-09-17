use application::ai_gateway_jwt::AiGatewayJwtMinter;
use async_trait::async_trait;
use big_brain_client::BigBrainClient;
use common::types::{
    AttributionClaims,
    DeploymentMetadata,
};
use errors::ErrorMetadata;

pub struct LocalAiGatewayTokenMinter {
    client: Option<BigBrainClient>,
}

impl LocalAiGatewayTokenMinter {
    pub fn new(control_plane_url: String, access_token: Option<String>) -> Self {
        Self {
            client: access_token
                .map(|access_token| BigBrainClient::new(control_plane_url, access_token)),
        }
    }
}

#[async_trait]
impl AiGatewayJwtMinter for LocalAiGatewayTokenMinter {
    async fn mint(
        &self,
        deployment: &DeploymentMetadata,
        attribution: AttributionClaims,
    ) -> anyhow::Result<String> {
        let client = self.client.as_ref().ok_or_else(|| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "AiGatewayUnavailable",
                "`getServiceToken(\"ai-gateway\")` requires an authenticated local deployment. \
                 Run `npx convex login`, then restart `npx convex dev`.",
            ))
        })?;
        let response = client
            .mint_local_ai_gateway_jwt(deployment.name.clone(), attribution)
            .await?;
        Ok(response.token)
    }
}
