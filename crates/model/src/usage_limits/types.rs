use errors::ErrorMetadata;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

/// Maximum number of usage limit configs allowed per deployment.
///
/// Limit evaluation is intended to happen on the function invocation path, so
/// keep the configured set small and bounded.
pub const USAGE_LIMITS_LIMIT: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageLimitConfig {
    pub name: Option<String>,
    pub metric: UsageLimitMetric,
    pub window: UsageLimitWindow,
    pub limit_type: UsageLimitType,
    pub limit: u64,
    pub enabled: bool,
}

impl UsageLimitConfig {
    pub fn new(
        metric: UsageLimitMetric,
        window: UsageLimitWindow,
        limit_type: UsageLimitType,
        limit: u64,
        enabled: bool,
    ) -> anyhow::Result<Self> {
        let config = Self {
            name: None,
            metric,
            window,
            limit_type,
            limit,
            enabled,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn key(&self) -> UsageLimitKey {
        UsageLimitKey {
            metric: self.metric,
            window: self.window,
            limit_type: self.limit_type,
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.limit == 0 {
            return Err(ErrorMetadata::bad_request(
                "InvalidUsageLimit",
                "Usage limits must have a positive limit.",
            )
            .into());
        }
        if matches!(self.name.as_deref(), Some("")) {
            return Err(ErrorMetadata::bad_request(
                "InvalidUsageLimitName",
                "Usage limit names cannot be empty.",
            )
            .into());
        }
        Ok(())
    }

}

pub struct UsageLimitKey {
    pub metric: UsageLimitMetric,
    pub window: UsageLimitWindow,
    pub limit_type: UsageLimitType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "camelCase")]
pub enum UsageLimitMetric {
    FunctionCalls,
    DatabaseIoGB,
    DataEgressGB,
    SearchQueryGB,
    QueryMutationComputeGBHours,
    ActionComputeConvexGBHours,
    ActionComputeNodeJsGBHours,
    ActionComputeCpuGBHours,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "camelCase")]
pub enum UsageLimitWindow {
    Hour,
    Day,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "camelCase")]
pub enum UsageLimitType {
    Warning,
    Disable,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedUsageLimitConfig {
    pub name: Option<String>,
    pub metric: String,
    pub window: String,
    pub limit_type: String,
    pub limit: u64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl TryFrom<UsageLimitConfig> for SerializedUsageLimitConfig {
    type Error = anyhow::Error;

    fn try_from(value: UsageLimitConfig) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self {
            name: value.name,
            metric: value.metric.to_string(),
            window: value.window.to_string(),
            limit_type: value.limit_type.to_string(),
            limit: value.limit,
            enabled: value.enabled,
        })
    }
}

impl TryFrom<SerializedUsageLimitConfig> for UsageLimitConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedUsageLimitConfig) -> Result<Self, Self::Error> {
        let config = Self {
            name: value.name,
            metric: value.metric.parse()?,
            window: value.window.parse()?,
            limit_type: value.limit_type.parse()?,
            limit: value.limit,
            enabled: value.enabled,
        };
        config.validate()?;
        Ok(config)
    }
}

codegen_convex_serialization!(UsageLimitConfig, SerializedUsageLimitConfig);
