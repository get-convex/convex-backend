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

// TODO(CX-5845): Use proper serializable types for constants rather than
// Strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum UsageEvent {
    FunctionCall {
        /// The ExecutionId of a particular UDF
        id: String,
        /// The path of a component. Uniquely identifies a component in a
        /// project.
        component_path: Option<String>,
        /// The path / name of the UDF
        udf_id: String,
        /// The type of the udf identifier (http, function, cli)
        udf_id_type: String,
        /// "storage", "mutation", "cached_query" etc.
        tag: String,
        /// The memory used in megabytes by the UDF, or 0 if we don't track
        /// memory for this tag type.
        memory_megabytes: u64,
        /// The duration in milliseconds of the UDF, or 0 if we don't track
        /// execution time for this tag type.
        duration_millis: u64,
        /// Whether this was run in V8 or Node, or "unknown".
        environment: String,
        /// True if we think it's a call we should track in usage. Right now
        /// this is basically any UDF that's neither system nor
        /// triggered by the CLI . This could be derived from path and
        /// udf type, but it seems better to be explicit)
        is_tracked: bool,
        /// The sha256 of the response body. Only set for HTTP actions.
        response_sha256: Option<String>,
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
        storage_id: String,
        call: String,
        content_type: Option<String>,
        sha256: String,
    },
    /// Bandwidth from a storage call outside of a user function (e.g. snapshot
    /// import/export).
    StorageBandwidth {
        id: String,
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
        total_size: u64,
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
