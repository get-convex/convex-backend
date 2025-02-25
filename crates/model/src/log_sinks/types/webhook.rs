use std::fmt;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebhookConfig {
    pub url: reqwest::Url,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedWebhookConfig {
    pub url: String,
}

impl From<WebhookConfig> for SerializedWebhookConfig {
    fn from(value: WebhookConfig) -> Self {
        Self {
            url: value.url.to_string(),
        }
    }
}

impl TryFrom<SerializedWebhookConfig> for WebhookConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedWebhookConfig) -> Result<Self, Self::Error> {
        Ok(WebhookConfig {
            url: value.url.parse()?,
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

    use super::WebhookConfig;

    impl Arbitrary for WebhookConfig {
        type Parameters = ();

        type Strategy = impl Strategy<Value = WebhookConfig>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<proptest_http::ArbitraryUri>().prop_filter_map(
                "Invalid URL for WebhookConfig",
                |url| {
                    reqwest::Url::parse(url.0.to_string().as_str())
                        .ok()
                        .map(|url| WebhookConfig { url })
                },
            )
        }
    }
}
