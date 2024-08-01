use std::sync::Arc;

use keybroker::{
    Identity,
    KeyBroker,
};

use crate::{
    access_token_auth::AccessTokenAuth,
    metrics::{
        log_deploy_key_use,
        DeployKeyType,
    },
};

pub struct ApplicationAuth {
    key_broker: KeyBroker,
    access_token_auth: Arc<dyn AccessTokenAuth>,
}

// Encapsulates auth logic supporting both legacy Deploy Keys and new Convex
// Access tokens
impl ApplicationAuth {
    pub fn new(key_broker: KeyBroker, access_token_auth: Arc<dyn AccessTokenAuth>) -> Self {
        Self {
            key_broker,
            access_token_auth,
        }
    }

    pub async fn check_key(
        &self,
        admin_key_or_access_token: String,
        instance_name: String,
    ) -> anyhow::Result<Identity> {
        if self
            .key_broker
            .is_encrypted_admin_key(&admin_key_or_access_token)
        {
            // assume this is a legacy Deploy Key
            log_deploy_key_use(DeployKeyType::Legacy);
            self.key_broker.check_admin_key(&admin_key_or_access_token)
        } else {
            // assume this is an Access Token
            // Access Tokens are base64 encoded strings
            log_deploy_key_use(DeployKeyType::AccessToken);
            self.access_token_auth
                .is_authorized(&instance_name, &admin_key_or_access_token)
                .await
        }
    }
}
