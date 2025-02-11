use async_trait::async_trait;
use errors::ErrorMetadata;
use keybroker::Identity;

/// Logic to check authorization based on Access Token
#[async_trait]
pub trait AccessTokenAuth: Send + Sync {
    async fn is_authorized(
        &self,
        instance_name: &str,
        access_token: &str,
    ) -> anyhow::Result<Identity>;
}
pub struct NullAccessTokenAuth;

#[async_trait]
impl AccessTokenAuth for NullAccessTokenAuth {
    async fn is_authorized(
        &self,
        _instance_name: &str,
        _access_token: &str,
    ) -> anyhow::Result<Identity> {
        anyhow::bail!(ErrorMetadata::unauthenticated(
            "BadAdminKey",
            "The provided admin key was invalid for this instance",
        ))
    }
}
