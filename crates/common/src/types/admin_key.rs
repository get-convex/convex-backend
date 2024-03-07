use headers::Authorization;
use serde::Serialize;
use sync_types::headers::ConvexAdminAuthorization;

/// Encrypted admin key
#[derive(Serialize, Clone, derive_more::Display)]
pub struct AdminKey(String);

impl AdminKey {
    pub fn new(key: String) -> Self {
        Self(key)
    }

    pub fn as_header(&self) -> anyhow::Result<Authorization<ConvexAdminAuthorization>> {
        Ok(Authorization(ConvexAdminAuthorization::from_admin_key(
            &self.0,
        )?))
    }
}

impl From<AdminKey> for AdminKeyParts {
    fn from(value: AdminKey) -> Self {
        split_admin_key(&value.0)
            .map(|(instance_name, encrypted_part)| {
                let (deployment_type_prefix, instance_name) = instance_name
                    .split_once(':')
                    .map(|(deployment_type_prefix, name)| (Some(deployment_type_prefix), name))
                    .unwrap_or((None, instance_name));
                AdminKeyParts {
                    deployment_type_prefix: deployment_type_prefix.map(|d| d.to_string()),
                    instance_name: Some(instance_name.to_string()),
                    encrypted_part: encrypted_part.to_string(),
                }
            })
            .unwrap_or(AdminKeyParts {
                deployment_type_prefix: None,
                instance_name: None,
                encrypted_part: value.0.to_string(),
            })
    }
}

impl TryFrom<AdminKeyParts> for AdminKey {
    type Error = anyhow::Error;

    fn try_from(value: AdminKeyParts) -> Result<Self, Self::Error> {
        let encrypted_part = value.encrypted_part;
        let key = match (value.deployment_type_prefix, value.instance_name) {
            (None, None) => encrypted_part,
            (None, Some(instance_identifier)) => format!("{instance_identifier}|{encrypted_part}"),
            (Some(_), None) => anyhow::bail!("Invalid admin key parts"),
            (Some(deployment_type_prefix), Some(instance_identifier)) => {
                format!("{deployment_type_prefix}:{instance_identifier}|{encrypted_part}")
            },
        };
        Ok(AdminKey::new(key))
    }
}

/// The different parts of 'prod:happy-animal-123|restofkey'
pub struct AdminKeyParts {
    pub deployment_type_prefix: Option<String>,
    pub instance_name: Option<String>,
    pub encrypted_part: String,
}

// TODO - encompass these floating methods into the `AdminKey` type

pub fn split_admin_key(admin_key: &str) -> Option<(&str, &str)> {
    admin_key.split_once('|')
}

pub fn format_admin_key(instance_name: &str, encrypted_part: &str) -> String {
    format!("{}|{}", instance_name, encrypted_part)
}

// For a key like
// "prod:some-depl-name123|sa67asd6a5da6d5:sd6f5sdf76dsf4ds6f4s68fd"
// strips the initial "prod:" prefix.
pub fn remove_type_prefix_from_admin_key(admin_key: &str) -> String {
    split_admin_key(admin_key)
        .map(|(instance_name, encrypted_key)| {
            format_admin_key(
                remove_type_prefix_from_instance_name(instance_name),
                encrypted_key,
            )
        })
        .unwrap_or(admin_key.to_string())
}

// Dashboard adds a superficial prod: or dev: prefix
// for user's visibility to the admin key's instance name.
// CLI also adds this prefix to CONVEX_DEPLOYMENT env var.
// This method strips the prefix.
pub fn remove_type_prefix_from_instance_name(instance_name: &str) -> &str {
    instance_name
        .split_once(':')
        .map(|(_, name)| name)
        .unwrap_or(instance_name)
}
