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
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Default, strum::EnumString, strum::Display,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum WebhookFormat {
    #[default]
    Json,
    Jsonl,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedWebhookConfig {
    pub url: String,
    pub format: String,
}

impl From<WebhookConfig> for SerializedWebhookConfig {
    fn from(value: WebhookConfig) -> Self {
        Self {
            url: value.url.to_string(),
            format: value.format.to_string(),
        }
    }
}

impl TryFrom<SerializedWebhookConfig> for WebhookConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedWebhookConfig) -> Result<Self, Self::Error> {
        Ok(WebhookConfig {
            url: value.url.parse()?,
            format: value.format.parse().unwrap_or(WebhookFormat::Json),
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
