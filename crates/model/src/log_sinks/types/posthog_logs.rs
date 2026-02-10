use std::fmt;

use common::pii::PII;
use serde::{
    Deserialize,
    Serialize,
};

pub const DEFAULT_POSTHOG_HOST: &str = "https://us.i.posthog.com";

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct PostHogLogsConfig {
    pub api_key: PII<String>,
    pub host: Option<String>,
    pub service_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedPostHogLogsConfig {
    pub api_key: String,
    pub host: Option<String>,
    pub service_name: Option<String>,
}

impl From<PostHogLogsConfig> for SerializedPostHogLogsConfig {
    fn from(value: PostHogLogsConfig) -> Self {
        Self {
            api_key: value.api_key.0,
            host: value.host,
            service_name: value.service_name,
        }
    }
}

impl From<SerializedPostHogLogsConfig> for PostHogLogsConfig {
    fn from(value: SerializedPostHogLogsConfig) -> Self {
        Self {
            api_key: PII(value.api_key),
            host: value.host,
            service_name: value.service_name,
        }
    }
}

impl fmt::Display for PostHogLogsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PostHogLogsConfig {{ host: {:?} }}",
            self.host.as_deref().unwrap_or(DEFAULT_POSTHOG_HOST)
        )
    }
}
