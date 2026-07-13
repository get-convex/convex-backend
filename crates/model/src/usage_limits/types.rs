use errors::ErrorMetadata;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::Display)]
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

const BYTES_PER_GB: f64 = (1u64 << 30) as f64;
const SECS_PER_HOUR: f64 = 3600.0;

impl UsageLimitMetric {
    /// Canonical name for this metric in the in-memory usage metric stores.
    /// The seed pipeline's finer-grained metrics are combined into these
    /// buckets at hydration; see [`Self::from_seed_metric`].
    pub fn metric_name(&self) -> &'static str {
        match self {
            Self::FunctionCalls => "function_calls",
            Self::DatabaseIoGB => "database_io_bytes",
            Self::DataEgressGB => "data_egress_bytes",
            Self::SearchQueryGB => "search_query_gb",
            Self::QueryMutationComputeGBHours => "query_mutation_compute_gbs",
            Self::ActionComputeConvexGBHours => "action_compute_convex_gbs",
            Self::ActionComputeNodeJsGBHours => "action_compute_nodejs_gbs",
            Self::ActionComputeCpuGBHours => "action_compute_cpu_gbs",
        }
    }

    /// Map one of the usage pipeline's rollup metric names (the
    /// `metric_name` values in `deployment_usage_*_rollup`) to the
    /// enforcement bucket it feeds. Several source metrics can feed one
    /// bucket; hydration sums them.
    ///
    /// Every source metric's unit matches its bucket's raw unit, so seeded
    /// values are used as-is.
    pub fn from_seed_metric(name: &str) -> Option<Self> {
        Some(match name {
            "udf_calls" | "storage_calls" | "udf_storage_calls" => Self::FunctionCalls,
            "reactor_gbs" => Self::QueryMutationComputeGBHours,
            "action_gbs" => Self::ActionComputeConvexGBHours,
            "action_node_gbs" => Self::ActionComputeNodeJsGBHours,
            "action_user_gbs" => Self::ActionComputeCpuGBHours,
            "db_ingress" | "db_egress" => Self::DatabaseIoGB,
            "text_query_search_gb" | "vector_query_search_gb_dims" => Self::SearchQueryGB,
            "network_egress" | "storage_bandwidth_egress" | "udf_storage_bandwidth_egress" => {
                Self::DataEgressGB
            },
            _ => return None,
        })
    }

    /// Convert a configured limit from this metric's user-facing unit
    /// (calls, GB, or GB-hours) into the raw unit its store counts in
    /// (calls, bytes, GB, or GB·s).
    pub fn limit_in_raw_units(&self, limit: u64) -> f64 {
        match self {
            Self::FunctionCalls | Self::SearchQueryGB => limit as f64,
            Self::DatabaseIoGB | Self::DataEgressGB => limit as f64 * BYTES_PER_GB,
            Self::QueryMutationComputeGBHours
            | Self::ActionComputeConvexGBHours
            | Self::ActionComputeNodeJsGBHours
            | Self::ActionComputeCpuGBHours => limit as f64 * SECS_PER_HOUR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "camelCase")]
pub enum UsageLimitWindow {
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
