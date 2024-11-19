use async_trait::async_trait;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum UdfType {
    Action,
    Query,
    Mutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct FunctionCallUsageFields {
    /// The ExecutionId of a particular UDF
    pub id: String,
    /// The path of a component. Uniquely identifies a component in a
    /// project.
    pub component_path: Option<String>,
    /// The path / name of the UDF
    pub udf_id: String,
    /// The type of the udf identifier (http, function, cli)
    pub udf_id_type: String,
    /// "storage", "mutation", "cached_query" etc.
    pub tag: String,
    /// The memory used in megabytes by the UDF, or 0 if we don't track
    /// memory for this tag type.
    pub memory_megabytes: u64,
    /// The duration in milliseconds of the UDF, or 0 if we don't track
    /// execution time for this tag type.
    pub duration_millis: u64,
    /// Whether this was run in V8 or Node, or "unknown".
    pub environment: String,
    /// True if we think it's a call we should track in usage. Right now
    /// this is basically any UDF that's neither system nor
    /// triggered by the CLI.
    /// Function calls events that are OCCs are also not tracked.
    /// This could be derived from path and
    /// udf type, but it seems better to be explicit)
    pub is_tracked: bool,
    /// The sha256 of the response body. Only set for HTTP actions.
    pub response_sha256: Option<String>,
    /// Whether this function call resulted in an OCC.
    pub is_occ: bool,
    /// The name of the table that the OCC occurred on. Only set if is_occ is
    /// true.
    pub occ_table_name: Option<String>,
    /// The document ID of the document that the OCC occurred on. Only set if
    /// is_occ is true.
    pub occ_document_id: Option<String>,
    /// The retry number of the OCC. Only set if is_occ is true.
    pub occ_retry_count: Option<u64>,
}

// TODO(CX-5845): Use proper serializable types for constants rather than
// Strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum UsageEvent {
    FunctionCall {
        #[serde(flatten)]
        fields: FunctionCallUsageFields,
    },
    /// A set of storage calls originating from a single user function
    /// invocation.
    FunctionStorageCalls {
        id: String,
        component_path: Option<String>,
        udf_id: String,
        call: String,
        count: u64,
    },
    /// Bandwidth from one or more storage calls originating from a single user
    /// function invocation.
    FunctionStorageBandwidth {
        id: String,
        component_path: Option<String>,
        udf_id: String,
        ingress: u64,
        egress: u64,
    },
    /// A single storage call originating outside of a user function (e.g.
    /// snapshot import/export)
    StorageCall {
        id: String,
        component_path: Option<String>,
        storage_id: String,
        call: String,
        content_type: Option<String>,
        sha256: String,
    },
    /// Bandwidth from a storage call outside of a user function (e.g. snapshot
    /// import/export).
    StorageBandwidth {
        id: String,
        component_path: Option<String>,
        tag: String,
        ingress: u64,
        egress: u64,
    },
    DatabaseBandwidth {
        id: String,
        component_path: Option<String>,
        udf_id: String,
        table_name: String,
        ingress: u64,
        egress: u64,
    },
    VectorBandwidth {
        id: String,
        component_path: Option<String>,
        udf_id: String,
        table_name: String,
        ingress: u64,
        egress: u64,
    },

    // Current* events record the current storage state as of a time, they're not incremental
    // deltas. So a new Current* value should replace the previous value. If a tables Vec is
    // empty, that means no tables have any usage of the type in question.
    CurrentVectorStorage {
        tables: Vec<TableVectorStorage>,
    },
    CurrentDatabaseStorage {
        tables: Vec<TableDatabaseStorage>,
    },
    CurrentFileStorage {
        // TODO(Rebecca): tag and total_size can be cleaned up after we start using the other
        // fields
        tag: String,
        total_size: u64,
        total_user_file_size: u64,
        total_cloud_backup_size: u64,
        total_snapshot_export_size: u64,
    },
    CurrentDocumentCounts {
        tables: Vec<TableDocumentCount>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TableDocumentCount {
    pub component_path: Option<String>,
    pub table_name: String,
    pub num_documents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TableDatabaseStorage {
    pub component_path: Option<String>,
    pub table_name: String,
    pub total_document_size: u64,
    pub total_index_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TableVectorStorage {
    pub component_path: Option<String>,
    pub table_name: String,
    pub size: u64,
}

/// Fire off usage events into the ether.
#[async_trait]
pub trait UsageEventLogger: Send + Sync + std::fmt::Debug {
    /// A close to zero cost log method that dumps events into a buffer
    ///
    /// Implementations may choose to drop records on the floor if buffers are
    /// unexpectedly full. If you can accept the penalty for waiting for the
    /// buffer to empty out, use record_async instead.
    fn record(&self, events: Vec<UsageEvent>);

    /// Dump events into a buffer, waiting for the buffer to empty if it's full.
    async fn record_async(&self, events: Vec<UsageEvent>);

    /// Cleanly shutdown, flushing events
    async fn shutdown(&self) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct NoOpUsageEventLogger;

#[async_trait]
impl UsageEventLogger for NoOpUsageEventLogger {
    fn record(&self, _events: Vec<UsageEvent>) {}

    async fn record_async(&self, _events: Vec<UsageEvent>) {}

    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_function_call_serialization() {
        let event = UsageEvent::FunctionCall {
            fields: FunctionCallUsageFields {
                id: "123".to_string(),
                component_path: Some("component/path".to_string()),
                udf_id: "udf_id".to_string(),
                udf_id_type: "http".to_string(),
                tag: "tag".to_string(),
                memory_megabytes: 100,
                duration_millis: 200,
                environment: "Node".to_string(),
                is_tracked: true,
                response_sha256: Some("sha256".to_string()),
                is_occ: false,
                occ_table_name: None,
                occ_document_id: None,
                occ_retry_count: None,
            },
        };

        let output = serde_json::to_string(&event).unwrap();
        let expected_output = json!({"FunctionCall": {
            "id": "123",
            "component_path": "component/path",
            "udf_id": "udf_id",
            "udf_id_type": "http",
            "tag": "tag",
            "memory_megabytes": 100,
            "duration_millis": 200,
            "environment": "Node",
            "is_tracked": true,
            "response_sha256": "sha256",
            "is_occ": false,
            "occ_table_name": null,
            "occ_document_id": null,
            "occ_retry_count": null,
        }})
        .to_string();

        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_function_call_serialization_with_occ() {
        let event = UsageEvent::FunctionCall {
            fields: FunctionCallUsageFields {
                id: "123".to_string(),
                component_path: Some("component/path".to_string()),
                udf_id: "udf_id".to_string(),
                udf_id_type: "http".to_string(),
                tag: "tag".to_string(),
                memory_megabytes: 100,
                duration_millis: 200,
                environment: "Node".to_string(),
                is_tracked: true,
                response_sha256: Some("sha256".to_string()),
                is_occ: true,
                occ_table_name: Some("table_name".to_string()),
                occ_document_id: Some("document_id".to_string()),
                occ_retry_count: Some(1),
            },
        };

        let output = serde_json::to_string(&event).unwrap();
        let expected_output = json!({"FunctionCall": {
            "id": "123",
            "component_path": "component/path",
            "udf_id": "udf_id",
            "udf_id_type": "http",
            "tag": "tag",
            "memory_megabytes": 100,
            "duration_millis": 200,
            "environment": "Node",
            "is_tracked": true,
            "response_sha256": "sha256",
            "is_occ": true,
            "occ_table_name": "table_name",
            "occ_document_id": "document_id",
            "occ_retry_count": 1,
        }})
        .to_string();

        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_function_storage_calls_serialization() {
        let event = UsageEvent::FunctionStorageCalls {
            id: "456".to_string(),
            component_path: Some("component/path".to_string()),
            udf_id: "udf_id".to_string(),
            call: "call".to_string(),
            count: 10,
        };

        let output = serde_json::to_string(&event).unwrap();
        let expected_output = json!({"FunctionStorageCalls": {
            "id": "456",
            "component_path": "component/path",
            "udf_id": "udf_id",
            "call": "call",
            "count": 10,
        }})
        .to_string();

        assert_eq!(output, expected_output);
    }
}
