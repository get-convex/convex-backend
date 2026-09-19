//! Configuration for mirroring a deployment's data into object storage so it
//! can be queried by analytics engines. These rows live in the log sinks table
//! alongside log streams, but they carry no log sink client.

use std::fmt;

use common::{
    pii::PII,
    types::streaming_export::selection::Selection,
};
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;

/// How often the mirror is refreshed from the deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncPeriod {
    Continuous,
    Hourly,
    Daily,
}

/// A mirror in a Convex-owned bucket, queried through Convex.
#[derive(Debug, Clone, PartialEq)]
pub struct ManagedAnalyticsConfig {
    /// The components, tables, and columns to mirror.
    pub selection: Selection,
    pub period: SyncPeriod,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedManagedAnalyticsConfig {
    pub selection: Selection,
    pub period: SyncPeriod,
}

impl From<ManagedAnalyticsConfig> for SerializedManagedAnalyticsConfig {
    fn from(value: ManagedAnalyticsConfig) -> Self {
        Self {
            selection: value.selection,
            period: value.period,
        }
    }
}

impl From<SerializedManagedAnalyticsConfig> for ManagedAnalyticsConfig {
    fn from(value: SerializedManagedAnalyticsConfig) -> Self {
        Self {
            selection: value.selection,
            period: value.period,
        }
    }
}

impl fmt::Display for ManagedAnalyticsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ManagedAnalyticsConfig {{ period: {:?} }}", self.period)
    }
}

/// A mirror in a customer-owned S3 bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct S3ExportConfig {
    pub bucket: String,
    /// AWS region the bucket lives in, e.g. `us-east-1`.
    pub region: String,
    /// Key prefix within the bucket. `None` writes at the bucket root.
    pub prefix: Option<String>,
    pub access_key_id: PII<String>,
    pub secret_access_key: PII<String>,
    /// The components, tables, and columns to mirror.
    pub selection: Selection,
    pub period: SyncPeriod,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedS3ExportConfig {
    pub bucket: String,
    pub region: String,
    pub prefix: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub selection: Selection,
    pub period: SyncPeriod,
}

impl From<S3ExportConfig> for SerializedS3ExportConfig {
    fn from(value: S3ExportConfig) -> Self {
        Self {
            bucket: value.bucket,
            region: value.region,
            prefix: value.prefix,
            access_key_id: value.access_key_id.0,
            secret_access_key: value.secret_access_key.0,
            selection: value.selection,
            period: value.period,
        }
    }
}

impl From<SerializedS3ExportConfig> for S3ExportConfig {
    fn from(value: SerializedS3ExportConfig) -> Self {
        Self {
            bucket: value.bucket,
            region: value.region,
            prefix: value.prefix,
            access_key_id: PII(value.access_key_id),
            secret_access_key: PII(value.secret_access_key),
            selection: value.selection,
            period: value.period,
        }
    }
}

impl fmt::Display for S3ExportConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "S3ExportConfig {{ bucket: {}, region: {}, period: {:?} }}",
            self.bucket, self.region, self.period
        )
    }
}
