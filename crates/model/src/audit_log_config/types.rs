use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

/// Data model for an entry in the AUDIT_LOG_CONFIG_TABLE.
/// There should be at most one row in this table per deployment.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditLogConfig {
    /// The name of the AWS Firehose delivery stream to send audit logs to.
    /// None if no firehose stream has been configured yet.
    pub firehose_stream_name: Option<String>,
    /// Whether to forward audit logs to configured log streams (e.g. Datadog,
    /// Axiom).
    pub include_in_log_streams: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedAuditLogConfig {
    pub firehose_stream_name: Option<String>,
    #[serde(default)]
    pub include_in_log_streams: bool,
}

impl From<AuditLogConfig> for SerializedAuditLogConfig {
    fn from(value: AuditLogConfig) -> Self {
        Self {
            firehose_stream_name: value.firehose_stream_name,
            include_in_log_streams: value.include_in_log_streams,
        }
    }
}

impl From<SerializedAuditLogConfig> for AuditLogConfig {
    fn from(value: SerializedAuditLogConfig) -> Self {
        Self {
            firehose_stream_name: value.firehose_stream_name,
            include_in_log_streams: value.include_in_log_streams,
        }
    }
}

codegen_convex_serialization!(AuditLogConfig, SerializedAuditLogConfig);
