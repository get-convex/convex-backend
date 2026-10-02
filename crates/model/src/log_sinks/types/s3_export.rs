//! Configuration for mirroring a deployment's data into object storage so it
//! can be queried by analytics engines. These documents live in the log sinks
//! table alongside log streams, but they carry no log sink client.

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
use value::codegen_convex_serialization;

/// How often the mirror is refreshed from the deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncPeriod {
    Continuous,
    Hourly,
    Daily,
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
    pub cursor: Option<String>,
    pub progress: Option<S3ExportProgress>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum S3ExportProgress {
    #[serde(rename_all = "camelCase")]
    Snapshotting {
        num_tables_synced: i64,
        total_tables: i64,
        current_component: String,
        current_table: String,
        num_documents_in_current_table: i64,
        total_documents_in_current_table: Option<i64>,
        num_documents_synced: i64,
        total_documents: Option<i64>,
    },
    Stale {
        ts: i64,
    },
    UpToDate {
        ts: i64,
    },
}

codegen_convex_serialization!(S3ExportProgress, S3ExportProgress);

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<S3ExportProgress>,
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
            cursor: value.cursor,
            progress: value.progress,
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
            cursor: value.cursor,
            progress: value.progress,
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
