use std::collections::BTreeMap;

use url::Url;

use crate::fivetran_sdk::{
    form_field::Type,
    FormField,
    TextField,
};

const CONFIG_KEY_DEPLOYMENT_URL: &str = "url";
const CONFIG_KEY_DEPLOYMENT_KEY: &str = "key";

#[derive(Debug, Clone, Copy)]
pub struct AllowAllHosts(pub bool);

/// The configuration parameters used by the destination, requested to users by
/// the Fivetran UI. Users can obtain these values from the Convex dashboard in
/// the deployment’s settings page.
pub struct Config {
    /// The domain where the deployment is hosted (e.g. "https://aware-llama-900.convex.cloud").
    pub deploy_url: Url,

    /// The key giving admin permissions to the deployment
    /// (e.g. "prod:aware-llama-900|016b26d3900d5e482f1780969c2fa608a773140fb221db21785a9b2775b50263da6a258301b6374ef72b4c120e237c20ac50")
    pub deploy_key: String,
}

impl Config {
    /// Layout of the fields visible in the Fivetran UI
    #[allow(dead_code)]
    pub fn fivetran_fields() -> Vec<FormField> {
        vec![
            FormField {
                name: CONFIG_KEY_DEPLOYMENT_URL.to_string(),
                label: "Deployment URL".to_string(),
                required: true,
                description: Some(
                    "The domain where the deployment is hosted (\"https://….convex.cloud\"). You \
                     can find it in the deployment settings page of the Convex dashboard."
                        .to_string(),
                ),
                r#type: Some(Type::TextField(TextField::PlainText as i32)),
            },
            FormField {
                name: CONFIG_KEY_DEPLOYMENT_KEY.to_string(),
                label: "Deploy Key".to_string(),
                required: true,
                description: Some(
                    "The key giving access to your deployment. You can find it in the deployment \
                     settings page of the Convex dashboard."
                        .to_string(),
                ),
                r#type: Some(Type::TextField(TextField::Password as i32)),
            },
        ]
    }

    /// Validates user-supplied configuration parameters
    /// and creates a [`Config`] instance if they are valid.
    #[allow(dead_code)]
    pub fn from_parameters(
        configuration: BTreeMap<String, String>,
        allow_all_hosts: AllowAllHosts,
    ) -> anyhow::Result<Self> {
        let Some(deploy_url) = configuration.get(CONFIG_KEY_DEPLOYMENT_URL) else {
            anyhow::bail!("Missing {CONFIG_KEY_DEPLOYMENT_URL}");
        };

        let Ok(deploy_url) = Url::parse(deploy_url) else {
            anyhow::bail!("Invalid {CONFIG_KEY_DEPLOYMENT_URL} (must be an URL)");
        };

        let Some(host) = deploy_url.host_str() else {
            anyhow::bail!("Invalid deploy URL: must contain a host.");
        };

        if deploy_url.path() != "/"
            || deploy_url.query().is_some()
            || deploy_url.username() != ""
            || deploy_url.password().is_some()
            || deploy_url.fragment().is_some()
            || (deploy_url.scheme() != "http" && deploy_url.scheme() != "https")
        {
            anyhow::bail!("Invalid deploy URL: must be a root URL.");
        }

        if !allow_all_hosts.0
            && (deploy_url.port().is_some()
                || deploy_url.scheme() != "https"
                || !host.ends_with(".convex.cloud"))
        {
            anyhow::bail!("Invalid deploy URL: must be a Convex deployment URL.");
        }

        let Some(deploy_key) = configuration.get(CONFIG_KEY_DEPLOYMENT_KEY) else {
            anyhow::bail!("Missing {CONFIG_KEY_DEPLOYMENT_KEY}");
        };

        Ok(Config {
            deploy_url,
            deploy_key: deploy_key.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use maplit::btreemap;

    use super::*;

    const VALID_DEPLOY_KEY: &str = "prod:aware-llama-900|016b26d3900d5e482f1780969c2fa608a773140fb221db21785a9b2775b50263da6a258301b6374ef72b4c120e237c20ac50";

    #[test]
    fn accepts_valid_parameters() {
        let api = Config::from_parameters(
            btreemap! {
                "url".to_string() => "https://aware-llama-900.convex.cloud".to_string(),
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(false),
        )
        .unwrap();

        assert_eq!(
            api.deploy_url.to_string(),
            "https://aware-llama-900.convex.cloud/"
        );
        assert_eq!(api.deploy_key, "prod:aware-llama-900|016b26d3900d5e482f1780969c2fa608a773140fb221db21785a9b2775b50263da6a258301b6374ef72b4c120e237c20ac50");
    }

    #[test]
    fn accepts_valid_parameters_with_trailing_slash() {
        let api = Config::from_parameters(
            btreemap! {
                "url".to_string() => "https://aware-llama-900.convex.cloud/".to_string(),
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(false),
        )
        .unwrap();

        assert_eq!(
            api.deploy_url.to_string(),
            "https://aware-llama-900.convex.cloud/"
        );
        assert_eq!(api.deploy_key, "prod:aware-llama-900|016b26d3900d5e482f1780969c2fa608a773140fb221db21785a9b2775b50263da6a258301b6374ef72b4c120e237c20ac50");
    }

    #[test]
    fn refuses_missing_deploy_url() {
        assert!(Config::from_parameters(
            btreemap! {
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(true)
        )
        .is_err());
    }

    #[test]
    fn refuses_missing_deploy_key() {
        assert!(Config::from_parameters(
            btreemap! {
                "url".to_string() => "https://aware-llama-900.convex.cloud".to_string(),
            },
            AllowAllHosts(true)
        )
        .is_err());
    }

    #[test]
    fn refuses_invalid_urls() {
        for url in [
            "aware lalama convex",
            "https://aware-llama-900.convex.cloud/api/",
            "https://aware-llama-900.convex.cloud?abc",
            "https://aware-llama-900.convex.cloud?abc=def",
            "https://root:hunter2@aware-llama-900.convex.cloud",
            "https://aware-llama-900.convex.cloud/#abc",
            "ftp://aware-llama-900.convex.cloud/",
            "/",
        ] {
            assert!(
                Config::from_parameters(
                    btreemap! {
                        "url".to_string() => url.to_string(),
                        "key".to_string() => VALID_DEPLOY_KEY.to_string(),
                    },
                    AllowAllHosts(true)
                )
                .is_err(),
                "{url} is not a valid deploy URL"
            );
        }
    }

    #[test]
    fn refuses_non_convex_hosts_when_allow_all_hosts_is_disabled() {
        assert!(Config::from_parameters(
            btreemap! {
                "url".to_string() => "https://localhost".to_string(),
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(false)
        )
        .is_err());
    }

    #[test]
    fn refuses_http_hosts_when_allow_all_hosts_is_disabled() {
        assert!(Config::from_parameters(
            btreemap! {
                "url".to_string() => "http://aware-llama-900.convex.cloud".to_string(),
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(false)
        )
        .is_err());
    }

    #[test]
    fn refuses_non_default_ports_when_allow_all_hosts_is_disabled() {
        assert!(Config::from_parameters(
            btreemap! {
                "url".to_string() => "https://aware-llama-900.convex.cloud:1337".to_string(),
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(false)
        )
        .is_err());
    }

    #[test]
    fn accepts_non_convex_hosts_when_allow_all_hosts_is_enabled() {
        assert!(Config::from_parameters(
            btreemap! {
                "url".to_string() => "http://localhost".to_string(),
                "key".to_string() => VALID_DEPLOY_KEY.to_string(),
            },
            AllowAllHosts(true)
        )
        .is_ok());
    }
}
