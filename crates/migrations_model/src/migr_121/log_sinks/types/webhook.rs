use std::fmt;

use serde::{
    Deserialize,
    Serialize,
};

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub struct WebhookConfig {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::reqwest_url_strategy()")
    )]
    pub url: reqwest::Url,
    pub format: WebhookFormat,
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WebhookFormat {
    #[default]
    Json,
    Jsonl,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedWebhookConfig {
    pub url: String,
    #[serde(default)]
    pub format: WebhookFormat,
}

impl From<WebhookConfig> for SerializedWebhookConfig {
    fn from(value: WebhookConfig) -> Self {
        Self {
            url: value.url.to_string(),
            format: value.format,
        }
    }
}

impl TryFrom<SerializedWebhookConfig> for WebhookConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedWebhookConfig) -> Result<Self, Self::Error> {
        Ok(WebhookConfig {
            url: value.url.parse()?,
            format: value.format,
        })
    }
}

impl fmt::Display for WebhookConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WebhookConfig {{ url: ... }}")
    }
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::prelude::*;

    pub fn reqwest_url_strategy() -> impl Strategy<Value = reqwest::Url> {
        any::<proptest_http::ArbitraryUri>()
            .prop_filter_map("Invalid URL for WebhookConfig", |url| {
                reqwest::Url::parse(url.0.to_string().as_str()).ok()
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::migr_121::log_sinks::types::webhook::{
        SerializedWebhookConfig,
        WebhookConfig,
        WebhookFormat,
    };

    #[test]
    fn test_deserialize_missing_format() -> anyhow::Result<()> {
        let serialized = r#"
            {
                "url": "https://example.com"
            }
        "#;
        let config: SerializedWebhookConfig = serde_json::from_str(serialized)?;
        let config = WebhookConfig::try_from(config)?;
        assert_eq!(
            config,
            WebhookConfig {
                url: "https://example.com".parse()?,
                format: WebhookFormat::Json,
            }
        );
        Ok(())
    }
}
