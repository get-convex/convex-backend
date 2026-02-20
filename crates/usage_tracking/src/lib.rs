#![feature(iterator_try_collect)]

use std::{
    collections::BTreeMap,
    fmt::Debug,
    ops::Add,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    components::ComponentPath,
    execution_context::ExecutionId,
    knobs::{
        FUNCTION_LIMIT_WARNING_RATIO,
        TRANSACTION_MAX_READ_SIZE_BYTES,
        TRANSACTION_MAX_READ_SIZE_ROWS,
    },
    types::{
        IndexName,
        ModuleEnvironment,
        StorageUuid,
        UdfIdentifier,
    },
    RequestId,
};
use derive_more::{
    Add,
    AddAssign,
};
use events::usage::{
    FunctionCallUsageFields,
    InsightReadLimitCall,
    UsageEvent,
    UsageEventLogger,
};
use headers::ContentType;
use parking_lot::Mutex;
use pb::usage::{
    CounterWithComponent as CounterWithComponentProto,
    CounterWithTag as CounterWithTagProto,
    CounterWithUrl as CounterWithUrlProto,
    FunctionUsageStats as FunctionUsageStatsProto,
};
use value::sha256::Sha256Digest;

mod metrics;

/// The core usage stats aggregator that is cheaply cloneable
#[derive(Clone, Debug)]
pub struct UsageCounter {
    usage_logger: Arc<dyn UsageEventLogger>,
}

#[derive(Debug, Clone)]
pub struct OccInfo {
    pub table_name: Option<String>,
    pub document_id: Option<String>,
    pub write_source: Option<String>,
    pub retry_count: u64,
}

pub enum CallType {
    Action {
        env: ModuleEnvironment,
        duration: Duration,
        user_execution_time: Option<Duration>,
        memory_in_mb: u64,
    },
    HttpAction {
        duration: Duration,
        user_execution_time: Option<Duration>,
        memory_in_mb: u64,

        /// Sha256 of the response body
        response_sha256: Sha256Digest,
    },
    Export,
    CachedQuery,
    UncachedQuery {
        duration: Duration,
        user_execution_time: Option<Duration>,
        memory_in_mb: u64,
    },
    Mutation {
        duration: Duration,
        user_execution_time: Option<Duration>,
        memory_in_mb: u64,
        occ_info: Option<OccInfo>,
    },
    Import,
    CloudBackup,
    CloudRestore,
    LogStreamPayload,
    IndexBackfill,
}

impl CallType {
    fn tag(&self) -> &'static str {
        match self {
            Self::Action { .. } => "action",
            Self::Export => "export",
            Self::CachedQuery => "cached_query",
            Self::UncachedQuery { .. } => "uncached_query",
            Self::Mutation { .. } => "mutation",
            Self::HttpAction { .. } => "http_action",
            Self::Import => "import",
            Self::CloudBackup => "cloud_backup",
            Self::CloudRestore => "cloud_restore",
            Self::LogStreamPayload => "log_stream_payload",
            Self::IndexBackfill => "index_backfill",
        }
    }

    fn is_occ(&self) -> bool {
        match self {
            Self::Mutation { occ_info, .. } => occ_info.is_some(),
            _ => false,
        }
    }

    fn occ_document_id(&self) -> Option<String> {
        match self {
            Self::Mutation { occ_info, .. } => {
                occ_info.as_ref().and_then(|info| info.document_id.clone())
            },
            _ => None,
        }
    }

    fn occ_table_name(&self) -> Option<String> {
        match self {
            Self::Mutation { occ_info, .. } => {
                occ_info.as_ref().and_then(|info| info.table_name.clone())
            },
            _ => None,
        }
    }

    fn occ_write_source(&self) -> Option<String> {
        match self {
            Self::Mutation { occ_info, .. } => {
                occ_info.as_ref().and_then(|info| info.write_source.clone())
            },
            _ => None,
        }
    }

    fn occ_retry_count(&self) -> Option<u64> {
        match self {
            Self::Mutation { occ_info, .. } => occ_info.as_ref().map(|info| info.retry_count),
            _ => None,
        }
    }

    fn memory_megabytes(&self) -> u64 {
        match self {
            CallType::UncachedQuery { memory_in_mb, .. }
            | CallType::Mutation { memory_in_mb, .. }
            | CallType::Action { memory_in_mb, .. }
            | CallType::HttpAction { memory_in_mb, .. } => *memory_in_mb,
            _ => 0,
        }
    }

    /// Total wall-clock time from the start of executing a request to its
    /// completion.
    fn duration_millis(&self) -> u64 {
        match self {
            CallType::UncachedQuery { duration, .. }
            | CallType::Mutation { duration, .. }
            | CallType::Action { duration, .. }
            | CallType::HttpAction { duration, .. } => u64::try_from(duration.as_millis())
                .expect("Function was running for over 584 billion years??"),
            _ => 0,
        }
    }

    /// Time spent executing user code in the isolate. Does not include
    /// syscalls, waiting for fetch, etc.
    fn user_execution_millis(&self) -> Option<u64> {
        match self {
            CallType::UncachedQuery {
                user_execution_time,
                ..
            }
            | CallType::Mutation {
                user_execution_time,
                ..
            }
            | CallType::Action {
                user_execution_time,
                ..
            }
            | CallType::HttpAction {
                user_execution_time,
                ..
            } => user_execution_time.map(|t| t.as_millis() as u64),
            CallType::CachedQuery
            | CallType::CloudBackup
            | CallType::CloudRestore
            | CallType::Export
            | CallType::Import
            | CallType::LogStreamPayload
            | CallType::IndexBackfill => None,
        }
    }

    fn environment(&self) -> String {
        match self {
            CallType::Action { env, .. } => env,
            // All other UDF types, including HTTP actions run on the isolate
            // only.
            _ => &ModuleEnvironment::Isolate,
        }
        .to_string()
    }

    fn response_sha256(&self) -> Option<String> {
        match self {
            CallType::HttpAction {
                response_sha256, ..
            } => Some(response_sha256.as_hex()),
            _ => None,
        }
    }
}

impl UsageCounter {
    pub fn new(usage_logger: Arc<dyn UsageEventLogger>) -> Self {
        Self { usage_logger }
    }

    pub async fn track_call(
        &self,
        udf_path: UdfIdentifier,
        execution_id: ExecutionId,
        request_id: RequestId,
        call_type: CallType,
        success: bool,
        stats: FunctionUsageStats,
    ) {
        let mut usage_metrics = Vec::new();

        // Because system udfs might cause usage before any data is added by the user,
        // we do not count their calls. We do count their bandwidth.
        let (should_track_calls, udf_id_type) = match &udf_path {
            UdfIdentifier::Function(path) => (!path.udf_path.is_system(), "function"),
            UdfIdentifier::Http(_) => (true, "http"),
            UdfIdentifier::SystemJob(_) => (false, "_system_job"),
        };

        let (component_path, udf_id) = udf_path.clone().into_component_and_udf_path();
        usage_metrics.push(UsageEvent::FunctionCall {
            fields: FunctionCallUsageFields {
                id: execution_id.to_string(),
                request_id: request_id.to_string(),
                status: if success { "success" } else { "failure" }.to_string(),
                component_path,
                udf_id,
                udf_id_type: udf_id_type.to_string(),
                tag: call_type.tag().to_string(),
                memory_megabytes: call_type.memory_megabytes(),
                duration_millis: call_type.duration_millis(),
                user_execution_millis: call_type.user_execution_millis(),
                environment: call_type.environment(),
                is_tracked: should_track_calls,
                response_sha256: call_type.response_sha256(),
                is_occ: call_type.is_occ(),
                occ_table_name: call_type.occ_table_name(),
                occ_document_id: call_type.occ_document_id(),
                occ_write_source: call_type.occ_write_source(),
                occ_retry_count: call_type.occ_retry_count(),
            },
        });

        // We always track bandwidth, even for system udfs.
        self._track_function_usage(
            udf_path,
            stats,
            execution_id,
            request_id,
            success,
            &mut usage_metrics,
        );
        self.usage_logger.record_async(usage_metrics).await;
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn track_call_test(&self, stats: FunctionUsageStats) {
        use common::components::ComponentFunctionPath;

        let component = ComponentPath::root();
        let path = ComponentFunctionPath {
            component,
            udf_path: "test.js:default".parse().unwrap(),
        };
        let udf = UdfIdentifier::Function(path.canonicalize());
        self.track_call(
            udf,
            ExecutionId::new(),
            RequestId::new(),
            CallType::Action {
                env: ModuleEnvironment::Isolate,
                duration: Duration::from_secs(10),
                user_execution_time: Some(Duration::from_secs(5)),
                memory_in_mb: 10,
            },
            true,
            stats,
        )
        .await;
    }

    // TODO: The existence of this function is a hack due to shortcuts we have
    // done in Node.js usage tracking. It should only be used by Node.js action
    // callbacks. We should only be using track_call() and never calling this
    // this directly. Otherwise, we will have the usage reflected in the usage
    // stats for billing but not in the UDF execution log counters.
    pub async fn track_function_usage(
        &self,
        udf_path: UdfIdentifier,
        execution_id: ExecutionId,
        request_id: RequestId,
        stats: FunctionUsageStats,
    ) {
        let mut usage_metrics = Vec::new();
        self._track_function_usage(
            udf_path,
            stats,
            execution_id,
            request_id,
            true,
            &mut usage_metrics,
        );
        self.usage_logger.record_async(usage_metrics).await;
    }

    pub fn _track_function_usage(
        &self,
        udf_path: UdfIdentifier,
        stats: FunctionUsageStats,
        execution_id: ExecutionId,
        request_id: RequestId,
        success: bool,
        usage_metrics: &mut Vec<UsageEvent>,
    ) {
        // Merge the storage stats.
        let (global_component_path, udf_id) = udf_path.into_component_and_udf_path();
        for ((component_path, storage_api), function_count) in stats.storage_calls {
            usage_metrics.push(UsageEvent::FunctionStorageCalls {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                call: storage_api,
                count: function_count,
            });
        }

        for (component_path, ingress) in stats.storage_ingress {
            usage_metrics.push(UsageEvent::FunctionStorageBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                ingress,
                egress: 0,
            });
        }
        for (component_path, egress) in stats.storage_egress {
            usage_metrics.push(UsageEvent::FunctionStorageBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                ingress: 0,
                egress,
            });
        }
        // Merge "by table" bandwidth stats.
        for ((component_path, table_name), ingress) in stats.database_ingress {
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                request_id: request_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress,
                ingress_v2: 0,
                egress: 0,
                egress_rows: 0,
                egress_v2: 0,
            });
        }
        for ((component_path, table_name), ingress) in stats.database_ingress_v2 {
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                request_id: request_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                ingress_v2: ingress,
                egress: 0,
                egress_rows: 0,
                egress_v2: 0,
            });
        }
        for ((component_path, table_name), egress) in stats.database_egress.clone() {
            let rows = stats
                .database_egress_rows
                .get(&(component_path.clone(), table_name.clone()))
                .unwrap_or(&0);
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                request_id: request_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                ingress_v2: 0,
                egress,
                egress_rows: *rows,
                egress_v2: 0,
            });
        }
        for ((component_path, table_name), egress) in stats.database_egress_v2.clone() {
            let rows = stats
                .database_egress_rows
                .get(&(component_path.clone(), table_name.clone()))
                .unwrap_or(&0);
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                request_id: request_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                ingress_v2: 0,
                egress: 0,
                egress_rows: *rows,
                egress_v2: egress,
            });
        }

        // Check read limits and add InsightReadLimit event if thresholds are exceeded
        let total_rows: u64 = stats.database_egress_rows.values().sum();
        let total_bytes: u64 = stats.database_egress.values().sum();

        let row_threshold =
            (*TRANSACTION_MAX_READ_SIZE_ROWS as f64 * *FUNCTION_LIMIT_WARNING_RATIO) as u64;
        let byte_threshold =
            (*TRANSACTION_MAX_READ_SIZE_BYTES as f64 * *FUNCTION_LIMIT_WARNING_RATIO) as u64;

        let did_exceed_document_threshold = total_rows >= row_threshold;
        let did_exceed_byte_threshold = total_bytes >= byte_threshold;

        if did_exceed_document_threshold || did_exceed_byte_threshold {
            let mut calls = Vec::new();
            let component_path: Option<ComponentPath> =
                match stats.database_egress_rows.first_key_value() {
                    Some(((component_path, _), _)) => Some(component_path.clone()),
                    None => {
                        tracing::error!(
                            "Failed to find component path despite thresholds being exceeded"
                        );
                        None
                    },
                };

            if let Some(component_path) = component_path {
                for ((cp, table_name), egress_rows) in stats.database_egress_rows.into_iter() {
                    let egress = stats
                        .database_egress
                        .get(&(cp, table_name.clone()))
                        .copied()
                        .unwrap_or(0);

                    calls.push(InsightReadLimitCall {
                        table_name,
                        bytes_read: egress,
                        documents_read: egress_rows,
                    });
                }

                usage_metrics.push(UsageEvent::InsightReadLimit {
                    id: execution_id.to_string(),
                    request_id: request_id.to_string(),
                    udf_id: udf_id.clone(),
                    component_path: component_path.serialize(),
                    calls,
                    success,
                });
            }
        }

        for ((component_path, table_name), ingress) in stats.vector_ingress {
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress,
                egress: 0,
                ingress_v2: 0,
            });
        }
        for ((component_path, table_name), egress) in stats.vector_egress {
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                egress,
                ingress_v2: 0,
            });
        }
        for ((component_path, table_name), ingress) in stats.vector_ingress_v2 {
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                egress: 0,
                ingress_v2: ingress,
            });
        }
        for ((component_path, table_name), ingress) in stats.text_ingress {
            usage_metrics.push(UsageEvent::TextWrites {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                size: ingress,
            });
        }
        for (
            (component_path, table_name, index_name),
            TextIndexQueryUsage {
                num_searches,
                bytes_searched,
            },
        ) in stats.text_query_usage
        {
            usage_metrics.push(UsageEvent::TextQuery {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                index_name: index_name.to_string(),
                num_searches,
                bytes_searched,
            })
        }
        for (
            (component_path, table_name, index_name),
            VectorIndexQueryUsage {
                num_searches,
                bytes_searched,
                dimensions,
            },
        ) in stats.vector_query_usage
        {
            usage_metrics.push(UsageEvent::VectorQuery {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                index_name: index_name.to_string(),
                num_searches,
                bytes_searched,
                dimensions,
            })
        }

        for (url, egress) in stats.fetch_egress {
            usage_metrics.push(UsageEvent::NetworkBandwidth {
                id: execution_id.to_string(),
                request_id: request_id.to_string(),
                component_path: global_component_path.clone(),
                udf_id: udf_id.clone(),
                url,
                egress,
            });
        }
    }
}

// We can track storage attributed by UDF or not. This is why unlike database
// and vector search egress/ingress those methods are both on
// FunctionUsageTracker and UsageCounters directly.
#[async_trait]
pub trait StorageUsageTracker: Send + Sync {
    async fn track_storage_call(
        &self,
        component_path: ComponentPath,
        storage_api: &'static str,
        storage_id: StorageUuid,
        content_type: Option<ContentType>,
        sha256: Sha256Digest,
    ) -> Box<dyn StorageCallTracker>;
}

#[async_trait]
pub trait StorageCallTracker: Send + Sync {
    async fn track_storage_ingress(&self, component_path: ComponentPath, tag: String, ingress: u64);
    async fn track_storage_egress(&self, component_path: ComponentPath, tag: String, egress: u64);
}

struct IndependentStorageCallTracker {
    execution_id: ExecutionId,
    usage_logger: Arc<dyn UsageEventLogger>,
}

impl IndependentStorageCallTracker {
    fn new(execution_id: ExecutionId, usage_logger: Arc<dyn UsageEventLogger>) -> Self {
        Self {
            execution_id,
            usage_logger,
        }
    }
}

#[async_trait]
impl StorageCallTracker for IndependentStorageCallTracker {
    async fn track_storage_ingress(
        &self,
        component_path: ComponentPath,
        tag: String,
        ingress: u64,
    ) {
        metrics::storage::log_storage_ingress(ingress);
        self.usage_logger
            .record_async(vec![UsageEvent::StorageBandwidth {
                id: self.execution_id.to_string(),
                component_path: component_path.serialize(),
                tag,
                ingress,
                egress: 0,
            }])
            .await;
    }

    async fn track_storage_egress(&self, component_path: ComponentPath, tag: String, egress: u64) {
        metrics::storage::log_storage_egress(egress);
        self.usage_logger
            .record_async(vec![UsageEvent::StorageBandwidth {
                id: self.execution_id.to_string(),
                component_path: component_path.serialize(),
                tag,
                ingress: 0,
                egress,
            }])
            .await;
    }
}

#[async_trait]
impl StorageUsageTracker for UsageCounter {
    async fn track_storage_call(
        &self,
        component_path: ComponentPath,
        storage_api: &'static str,
        storage_id: StorageUuid,
        content_type: Option<ContentType>,
        sha256: Sha256Digest,
    ) -> Box<dyn StorageCallTracker> {
        let execution_id = ExecutionId::new();
        metrics::storage::log_storage_call();
        self.usage_logger
            .record_async(vec![UsageEvent::StorageCall {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                // Ideally we would track the Id<_storage> instead of the StorageUuid
                // but it's a bit annoying for now, so just going with this.
                storage_id: storage_id.to_string(),
                call: storage_api.to_string(),
                content_type: content_type.map(|c| c.to_string()),
                sha256: sha256.as_hex(),
            }])
            .await;

        Box::new(IndependentStorageCallTracker::new(
            execution_id,
            self.usage_logger.clone(),
        ))
    }
}

/// Usage tracker used within a Transaction. Note that this structure does not
/// directly report to the backend global counters and instead only buffers the
/// counters locally. The counters get rolled into the global ones via
/// UsageCounters::track_call() at the end of each UDF. This provides a
/// consistent way to account for usage, where we only bill people for usage
/// that makes it to the UdfExecution log.
#[derive(Debug, Clone)]
pub struct FunctionUsageTracker {
    // TODO: We should ideally not use an Arc<Mutex> here. The best way to achieve
    // this is to move the logic for accounting ingress out of the Committer into
    // the Transaction. Then Transaction can solely own the counters and we can
    // remove clone(). The alternative is for the Committer to take ownership of
    // the usage tracker and then return it, but this will make it complicated if
    // we later decide to charge people for OCC bandwidth.
    state: Arc<Mutex<FunctionUsageStats>>,
}

impl FunctionUsageTracker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FunctionUsageStats::default())),
        }
    }

    /// Calculate FunctionUsageStats here
    pub fn gather_user_stats(self) -> FunctionUsageStats {
        self.state.lock().clone()
    }

    /// Adds the given usage stats to the current tracker.
    pub fn add(&self, stats: FunctionUsageStats) {
        self.state.lock().merge(stats);
    }

    // Tracks database usage from write operations (insert/update/delete) for
    // documents that are not in vector indexes. If the document has one or more
    // vectors in a vector index, call `track_vector_ingress` instead of
    // this method.
    //
    // You must always check to see if a document is a vector index before
    // calling this method.
    pub fn track_database_ingress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        ingress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        // Skip v1 ingress tracking if configured (e.g., for streaming imports)
        if state.skip_v1_database_ingress {
            return;
        }
        *state
            .database_ingress
            .entry((component_path, table_name))
            .or_default() += ingress;
    }

    pub fn track_database_ingress_v2(
        &self,
        component_path: ComponentPath,
        table_name: String,
        ingress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        *state
            .database_ingress_v2
            .entry((component_path, table_name))
            .or_default() += ingress;
    }

    pub fn track_database_egress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        egress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        *state
            .database_egress
            .entry((component_path, table_name))
            .or_default() += egress;
    }

    pub fn track_database_egress_v2(
        &self,
        component_path: ComponentPath,
        table_name: String,
        egress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        *state
            .database_egress_v2
            .entry((component_path, table_name))
            .or_default() += egress;
    }

    pub fn track_database_egress_rows(
        &self,
        component_path: ComponentPath,
        table_name: String,
        egress_rows: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        *state
            .database_egress_rows
            .entry((component_path, table_name))
            .or_default() += egress_rows;
    }

    // Tracks the vector ingress surcharge for documents
    // that have one or more vectors in a vector index.
    //
    // Vector bandwidth is a surcharge on vector related bandwidth usage.
    // Ingress is a bit trickier than egress because vector ingress needs to be
    // updated whenever the mutated document is in a vector index. To be in a
    // vector index the document must both be in a table with a vector index and
    // have at least one vector that's actually used in the index.
    pub fn track_vector_ingress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        ingress: u64,
        ingress_v2: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        let key = (component_path, table_name);
        *state.vector_ingress.entry(key.clone()).or_default() += ingress;
        *state.vector_ingress_v2.entry(key).or_default() += ingress_v2;
    }

    // Tracks bandwidth usage from vector searches
    //
    // Vector bandwidth is a surcharge on vector related bandwidth usage. As a
    // result it counts against both bandwidth egress and vector egress. It's an
    // error to increment vector egress without also incrementing database
    // egress. The reverse is not true however, it's totally fine to increment
    // general database egress without incrementing vector egress if the operation
    // is not a vector search.
    //
    // Unlike track_database_ingress, this method is explicitly vector related
    // because we should always know that the relevant operation is a vector
    // search. In contrast, for ingress any insert/update/delete could happen to
    // impact a vector index.
    // TODO: This will be deprecated after the business plan change, but
    // we still need to track database egress.
    pub fn track_vector_egress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        egress: u64,
    ) {
        // Note that vector search counts as both database and vector bandwidth
        // per the comment above.
        let mut state = self.state.lock();
        let key = (component_path, table_name);
        *state.database_egress.entry(key.clone()).or_default() += egress;
        *state.vector_egress.entry(key).or_default() += egress;
    }

    pub fn track_vector_query(
        &self,
        component_path: ComponentPath,
        table_name: String,
        index_name: IndexName,
        index_size: u64,
        dimensions: u64,
    ) {
        let mut state = self.state.lock();
        let key = (component_path, table_name, index_name);
        *state.vector_query_usage.entry(key).or_default() += VectorIndexQueryUsage {
            num_searches: 1,
            bytes_searched: index_size,
            dimensions,
        };
    }

    pub fn track_text_ingress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        ingress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        *state
            .text_ingress
            .entry((component_path, table_name))
            .or_default() += ingress;
    }

    pub fn track_text_query(
        &self,
        component_path: ComponentPath,
        table_name: TableName,
        index_name: IndexName,
        index_size: u64,
    ) {
        let mut state = self.state.lock();
        let key = (component_path, table_name, index_name);
        *state.text_query_usage.entry(key).or_default() += TextIndexQueryUsage {
            num_searches: 1,
            bytes_searched: index_size,
        };
    }

    /// Only track egress - because AWS only charges egress
    pub fn track_fetch_egress(&self, url: String, egress: u64) {
        let mut state = self.state.lock();
        *state.fetch_egress.entry(url).or_default() += egress;
    }

    /// Configure this tracker to skip v1 database ingress tracking.
    /// Used for streaming imports which should only track v2 ingress.
    pub fn without_v1_database_ingress(self) -> Self {
        {
            let mut state = self.state.lock();
            state.skip_v1_database_ingress = true;
        }
        self
    }
}

// For UDFs, we track storage at the per UDF level, no finer. So we can just
// aggregate over the entire UDF and not worry about sending usage events or
// creating unique execution ids.
// Note: If we want finer-grained breakdown of file bandwidth, we can thread the
// tag through FunctionUsageStats. For now we're just interested in the
// breakdown of file bandwidth from functions vs external sources like snapshot
// export/cloud backups.
#[async_trait]
impl StorageCallTracker for FunctionUsageTracker {
    async fn track_storage_ingress(
        &self,
        component_path: ComponentPath,
        _tag: String,
        ingress: u64,
    ) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_ingress(ingress);
        *state.storage_ingress.entry(component_path).or_default() += ingress;
    }

    async fn track_storage_egress(&self, component_path: ComponentPath, _tag: String, egress: u64) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_egress(egress);
        *state.storage_egress.entry(component_path).or_default() += egress;
    }
}

#[async_trait]
impl StorageUsageTracker for FunctionUsageTracker {
    async fn track_storage_call(
        &self,
        component_path: ComponentPath,
        storage_api: &'static str,
        _storage_id: StorageUuid,
        _content_type: Option<ContentType>,
        _sha256: Sha256Digest,
    ) -> Box<dyn StorageCallTracker> {
        let mut state = self.state.lock();
        metrics::storage::log_storage_call();
        *state
            .storage_calls
            .entry((component_path, storage_api.to_string()))
            .or_default() += 1;
        Box::new(self.clone())
    }
}

type TableName = String;
type StorageAPI = String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Add, Default, AddAssign)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TextIndexQueryUsage {
    #[cfg_attr(any(test, feature = "testing"), proptest(strategy = "0..=1024u64"))]
    pub num_searches: u64,
    #[cfg_attr(any(test, feature = "testing"), proptest(strategy = "0..=1024u64"))]
    pub bytes_searched: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, AddAssign)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorIndexQueryUsage {
    #[cfg_attr(any(test, feature = "testing"), proptest(strategy = "0..=1024u64"))]
    pub num_searches: u64,
    #[cfg_attr(any(test, feature = "testing"), proptest(strategy = "0..=1024u64"))]
    pub bytes_searched: u64,
    #[cfg_attr(any(test, feature = "testing"), proptest(strategy = "0..=1024u64"))]
    pub dimensions: u64,
}

impl Add for VectorIndexQueryUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            num_searches: self.num_searches + other.num_searches,
            // Take the max of the index_size
            bytes_searched: self.bytes_searched + other.bytes_searched,
            // Take the max of the num_dimensions. num_dimensions shouldn't usually change, but if
            // there is some change to the index that commits in the middle of an action
            // between two vector searches, just take the max dimensions.
            dimensions: self.dimensions.max(other.dimensions),
        }
    }
}

/// User-facing UDF stats, built
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct FunctionUsageStats {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, StorageAPI)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub storage_calls: BTreeMap<(ComponentPath, StorageAPI), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<ComponentPath>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub storage_ingress: BTreeMap<ComponentPath, u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<ComponentPath>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub storage_egress: BTreeMap<ComponentPath, u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub database_ingress: BTreeMap<(ComponentPath, TableName), u64>,
    /// Includes ingress for tables that have virtual tables
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub database_ingress_v2: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub database_egress: BTreeMap<(ComponentPath, TableName), u64>,
    /// Includes egress for tables that have virtual tables
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub database_egress_v2: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub database_egress_rows: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub vector_ingress: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub vector_ingress_v2: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub vector_egress: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub text_ingress: BTreeMap<(ComponentPath, TableName), u64>,

    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName, IndexName)>(), \
                             proptest::arbitrary::any::<TextIndexQueryUsage>(), 0..=4,
            )")
    )]
    pub text_query_usage: BTreeMap<(ComponentPath, TableName, IndexName), TextIndexQueryUsage>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<(ComponentPath, TableName, IndexName)>(), \
                             proptest::arbitrary::any::<VectorIndexQueryUsage>(), 0..=4,
            )")
    )]
    pub vector_query_usage: BTreeMap<(ComponentPath, TableName, IndexName), VectorIndexQueryUsage>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<String>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub fetch_egress: BTreeMap<String, u64>,

    /// If true, skip tracking v1 database ingress.
    /// Used for streaming imports which should only track v2 ingress.
    #[cfg_attr(any(test, feature = "testing"), proptest(value = "false"))]
    pub skip_v1_database_ingress: bool,
}

impl FunctionUsageStats {
    pub fn aggregate(&self) -> AggregatedFunctionUsageStats {
        AggregatedFunctionUsageStats {
            database_read_bytes: self.database_egress.values().sum(),
            database_write_bytes: self.database_ingress.values().sum(),
            database_read_documents: self.database_egress_rows.values().sum(),
            storage_read_bytes: self.storage_egress.values().sum(),
            storage_write_bytes: self.storage_ingress.values().sum(),
            vector_index_read_bytes: self.vector_egress.values().sum(),
            vector_index_write_bytes: self.vector_ingress.values().sum(),
            text_index_write_bytes: self.text_ingress.values().sum(),
            vector_index_write_bytes_v2: self.vector_ingress_v2.values().sum(),
        }
    }

    fn merge(
        &mut self,
        Self {
            storage_calls,
            storage_ingress,
            storage_egress,
            database_ingress,
            database_ingress_v2,
            database_egress,
            database_egress_v2,
            database_egress_rows,
            vector_ingress,
            vector_ingress_v2,
            vector_egress,
            text_ingress,
            text_query_usage,
            vector_query_usage,
            fetch_egress,
            skip_v1_database_ingress: _,
        }: Self,
    ) {
        for (key, function_count) in storage_calls {
            *self.storage_calls.entry(key).or_default() += function_count;
        }
        for (key, ingress) in storage_ingress {
            *self.storage_ingress.entry(key).or_default() += ingress;
        }
        for (key, egress) in storage_egress {
            *self.storage_egress.entry(key).or_default() += egress;
        }
        for (key, ingress) in database_ingress {
            *self.database_ingress.entry(key).or_default() += ingress;
        }
        for (key, ingress) in database_ingress_v2 {
            *self.database_ingress_v2.entry(key).or_default() += ingress;
        }
        for (key, egress) in database_egress {
            *self.database_egress.entry(key).or_default() += egress;
        }
        for (key, egress) in database_egress_v2 {
            *self.database_egress_v2.entry(key).or_default() += egress;
        }
        for (key, egress_rows) in database_egress_rows {
            *self.database_egress_rows.entry(key.clone()).or_default() += egress_rows;
        }
        for (key, ingress) in vector_ingress {
            *self.vector_ingress.entry(key.clone()).or_default() += ingress;
        }
        for (key, egress) in vector_egress {
            *self.vector_egress.entry(key.clone()).or_default() += egress;
        }
        for (key, ingress) in vector_ingress_v2 {
            *self.vector_ingress_v2.entry(key.clone()).or_default() += ingress;
        }
        for (key, ingress) in text_ingress {
            *self.text_ingress.entry(key.clone()).or_default() += ingress;
        }
        for (key, text_index_usage) in text_query_usage {
            *self.text_query_usage.entry(key).or_default() += text_index_usage;
        }
        for (key, vector_index_usage) in vector_query_usage {
            *self.vector_query_usage.entry(key).or_default() += vector_index_usage;
        }
        for (key, egress) in fetch_egress {
            *self.fetch_egress.entry(key.clone()).or_default() += egress;
        }
    }
}

fn to_by_tag_count(
    counts: impl Iterator<Item = ((ComponentPath, String), u64)>,
) -> Vec<CounterWithTagProto> {
    counts
        .map(
            |((component_path, table_name), count)| CounterWithTagProto {
                component_path: component_path.serialize(),
                table_name: Some(table_name),
                count: Some(count),
            },
        )
        .collect()
}

fn to_by_component_count(
    counts: impl Iterator<Item = (ComponentPath, u64)>,
) -> Vec<CounterWithComponentProto> {
    counts
        .map(|(component_path, count)| CounterWithComponentProto {
            component_path: component_path.serialize(),
            count: Some(count),
        })
        .collect()
}

fn to_by_url_count(counts: impl Iterator<Item = (String, u64)>) -> Vec<CounterWithUrlProto> {
    counts
        .map(|(url, count)| CounterWithUrlProto {
            url: Some(url),
            count: Some(count),
        })
        .collect()
}

fn from_by_tag_count(
    counts: Vec<CounterWithTagProto>,
) -> anyhow::Result<impl Iterator<Item = ((ComponentPath, String), u64)>> {
    let counts: Vec<_> = counts
        .into_iter()
        .map(|c| -> anyhow::Result<_> {
            let component_path = ComponentPath::deserialize(c.component_path.as_deref())?;
            let name = c.table_name.context("Missing `table_name` field")?;
            let count = c.count.context("Missing `count` field")?;
            Ok(((component_path, name), count))
        })
        .try_collect()?;
    Ok(counts.into_iter())
}

fn from_by_url_count(
    counts: Vec<CounterWithUrlProto>,
) -> anyhow::Result<impl Iterator<Item = (String, u64)>> {
    let counts: Vec<_> = counts
        .into_iter()
        .map(|c| -> anyhow::Result<_> {
            let url = c.url.context("Missing `url` field")?;
            let count = c.count.context("Missing `count` field")?;
            Ok((url, count))
        })
        .try_collect()?;
    Ok(counts.into_iter())
}

fn from_by_component_tag_count(
    counts: Vec<CounterWithComponentProto>,
) -> anyhow::Result<impl Iterator<Item = (ComponentPath, u64)>> {
    let counts: Vec<_> = counts
        .into_iter()
        .map(|c| -> anyhow::Result<_> {
            let component_path = ComponentPath::deserialize(c.component_path.as_deref())?;
            let count = c.count.context("Missing `count` field")?;
            Ok((component_path, count))
        })
        .try_collect()?;
    Ok(counts.into_iter())
}

fn to_text_query_usage(
    usage: impl Iterator<Item = ((ComponentPath, TableName, IndexName), TextIndexQueryUsage)>,
) -> Vec<pb::usage::TextQueryUsage> {
    usage
        .map(
            |((component_path, table_name, index_name), usage)| pb::usage::TextQueryUsage {
                component_path: component_path.serialize(),
                table_name: Some(table_name),
                index_name: Some(index_name.to_string()),
                num_searches: Some(usage.num_searches),
                bytes_searched: Some(usage.bytes_searched),
            },
        )
        .collect()
}

fn to_vector_query_usage(
    usage: impl Iterator<Item = ((ComponentPath, TableName, IndexName), VectorIndexQueryUsage)>,
) -> Vec<pb::usage::VectorQueryUsage> {
    usage
        .map(
            |((component_path, table_name, index_name), usage)| pb::usage::VectorQueryUsage {
                component_path: component_path.serialize(),
                table_name: Some(table_name),
                index_name: Some(index_name.to_string()),
                num_searches: Some(usage.num_searches),
                bytes_searched: Some(usage.bytes_searched),
                dimensions: Some(usage.dimensions),
            },
        )
        .collect()
}

fn from_text_query_usage(
    usage: Vec<pb::usage::TextQueryUsage>,
) -> anyhow::Result<
    impl Iterator<Item = ((ComponentPath, TableName, IndexName), TextIndexQueryUsage)>,
> {
    let usage: Vec<_> = usage
        .into_iter()
        .map(|u| -> anyhow::Result<_> {
            let component_path = ComponentPath::deserialize(u.component_path.as_deref())?;
            let table_name = u.table_name.context("Missing `table_name` field")?;
            let index_name: IndexName = u
                .index_name
                .context("Missing `index_name` field")?
                .parse()?;
            let num_searches = u.num_searches.context("Missing `num_searches` field")?;
            let bytes_searched = u
                .bytes_searched
                .context("Missing `num_segment_searches` field")?;
            Ok((
                (component_path, table_name, index_name),
                TextIndexQueryUsage {
                    num_searches,
                    bytes_searched,
                },
            ))
        })
        .try_collect()?;
    Ok(usage.into_iter())
}

fn from_vector_query_usage(
    usage: Vec<pb::usage::VectorQueryUsage>,
) -> anyhow::Result<
    impl Iterator<Item = ((ComponentPath, TableName, IndexName), VectorIndexQueryUsage)>,
> {
    let usage: Vec<_> = usage
        .into_iter()
        .map(|u| -> anyhow::Result<_> {
            let component_path = ComponentPath::deserialize(u.component_path.as_deref())?;
            let table_name = u.table_name.context("Missing `table_name` field")?;
            let index_name: IndexName = u
                .index_name
                .context("Missing `index_name` field")?
                .parse()?;
            let num_searches = u.num_searches.context("Missing `num_searches` field")?;
            let bytes_searched = u
                .bytes_searched
                .context("Missing `num_segment_searches` field")?;
            let dimensions = u.dimensions.context("Missing `num_dimensions` field")?;
            Ok((
                (component_path, table_name, index_name),
                VectorIndexQueryUsage {
                    num_searches,
                    bytes_searched,
                    dimensions,
                },
            ))
        })
        .try_collect()?;
    Ok(usage.into_iter())
}

impl From<FunctionUsageStats> for FunctionUsageStatsProto {
    fn from(stats: FunctionUsageStats) -> Self {
        FunctionUsageStatsProto {
            storage_calls: to_by_tag_count(stats.storage_calls.into_iter()),
            storage_ingress_by_component: to_by_component_count(stats.storage_ingress.into_iter()),
            storage_egress_by_component: to_by_component_count(stats.storage_egress.into_iter()),
            database_ingress: to_by_tag_count(stats.database_ingress.into_iter()),
            database_ingress_v2: to_by_tag_count(stats.database_ingress_v2.into_iter()),
            database_egress: to_by_tag_count(stats.database_egress.into_iter()),
            database_egress_v2: to_by_tag_count(stats.database_egress_v2.into_iter()),
            database_egress_rows: to_by_tag_count(stats.database_egress_rows.into_iter()),
            vector_ingress: to_by_tag_count(stats.vector_ingress.into_iter()),
            vector_egress: to_by_tag_count(stats.vector_egress.into_iter()),
            text_ingress: to_by_tag_count(stats.text_ingress.into_iter()),
            text_query_usage: to_text_query_usage(stats.text_query_usage.into_iter()),
            vector_query_usage: to_vector_query_usage(stats.vector_query_usage.into_iter()),
            vector_ingress_v2: to_by_tag_count(stats.vector_ingress_v2.into_iter()),
            fetch_egress: to_by_url_count(stats.fetch_egress.into_iter()),
        }
    }
}

impl TryFrom<FunctionUsageStatsProto> for FunctionUsageStats {
    type Error = anyhow::Error;

    fn try_from(stats: FunctionUsageStatsProto) -> anyhow::Result<Self> {
        let storage_calls = from_by_tag_count(stats.storage_calls)?.collect();
        let storage_ingress =
            from_by_component_tag_count(stats.storage_ingress_by_component)?.collect();
        let storage_egress =
            from_by_component_tag_count(stats.storage_egress_by_component)?.collect();
        let database_ingress = from_by_tag_count(stats.database_ingress)?.collect();
        let database_ingress_v2 = from_by_tag_count(stats.database_ingress_v2)?.collect();
        let database_egress = from_by_tag_count(stats.database_egress)?.collect();
        let database_egress_v2 = from_by_tag_count(stats.database_egress_v2)?.collect();
        let database_egress_rows = from_by_tag_count(stats.database_egress_rows)?.collect();
        let vector_ingress = from_by_tag_count(stats.vector_ingress)?.collect();
        let vector_egress = from_by_tag_count(stats.vector_egress)?.collect();
        let text_ingress = from_by_tag_count(stats.text_ingress)?.collect();
        let text_query_usage = from_text_query_usage(stats.text_query_usage)?.collect();
        let vector_query_usage = from_vector_query_usage(stats.vector_query_usage)?.collect();
        let vector_ingress_v2 = from_by_tag_count(stats.vector_ingress_v2)?.collect();
        let fetch_egress = from_by_url_count(stats.fetch_egress)?.collect();

        Ok(FunctionUsageStats {
            storage_calls,
            storage_ingress,
            storage_egress,
            database_ingress,
            database_ingress_v2,
            database_egress_rows,
            database_egress,
            database_egress_v2,
            vector_ingress,
            vector_egress,
            text_ingress,
            text_query_usage,
            vector_query_usage,
            vector_ingress_v2,
            fetch_egress,
            skip_v1_database_ingress: false,
        })
    }
}

/// User-facing UDF stats, that is logged in the UDF execution log
/// and might be used for debugging purposes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AggregatedFunctionUsageStats {
    pub database_read_bytes: u64,
    pub database_write_bytes: u64,
    pub database_read_documents: u64,
    pub storage_read_bytes: u64,
    pub storage_write_bytes: u64,
    pub vector_index_read_bytes: u64,
    pub vector_index_write_bytes: u64,
    pub text_index_write_bytes: u64,
    pub vector_index_write_bytes_v2: u64,
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::components::ComponentPath;
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::{
        FunctionUsageStats,
        FunctionUsageStatsProto,
        FunctionUsageTracker,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_usage_stats_roundtrips(stats in any::<FunctionUsageStats>()) {
            assert_roundtrips::<FunctionUsageStats, FunctionUsageStatsProto>(stats);
        }
    }

    #[test]
    fn test_without_v1_database_ingress() {
        // Create a tracker with skip_v1_database_ingress enabled
        let usage = FunctionUsageTracker::new().without_v1_database_ingress();

        // Track both v1 and v2 ingress/egress
        let component_path = ComponentPath::root();
        let table_name = "test_table".to_string();
        usage.track_database_ingress(component_path.clone(), table_name.clone(), 100, false);
        usage.track_database_ingress_v2(component_path.clone(), table_name.clone(), 200, false);
        usage.track_database_egress(component_path.clone(), table_name.clone(), 150, false);
        usage.track_database_egress_v2(component_path.clone(), table_name.clone(), 250, false);

        // Test vector ingress as well
        usage.track_vector_ingress(
            component_path.clone(),
            "vector_table".to_string(),
            50,
            75,
            false,
        );

        // Gather stats and verify
        let stats = usage.gather_user_stats();

        // ONLY v1 database_ingress should be skipped (NOT egress or vector)
        let v1_ingress = stats
            .database_ingress
            .get(&(component_path.clone(), table_name.clone()))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            v1_ingress, 0,
            "Expected database_ingress (v1) to be 0 when skip_v1_database_ingress is set"
        );

        // v2 ingress should be tracked
        let v2_ingress = stats
            .database_ingress_v2
            .get(&(component_path.clone(), table_name.clone()))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            v2_ingress, 200,
            "Expected database_ingress_v2 to be tracked normally"
        );

        let v1_egress = stats
            .database_egress
            .get(&(component_path.clone(), table_name.clone()))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            v1_egress, 150,
            "Expected database_egress (v1) to still be tracked (only ingress is skipped)"
        );

        // v2 egress should be tracked
        let v2_egress = stats
            .database_egress_v2
            .get(&(component_path.clone(), table_name.clone()))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            v2_egress, 250,
            "Expected database_egress_v2 to be tracked normally"
        );

        let v1_vector_ingress = stats
            .vector_ingress
            .get(&(component_path.clone(), "vector_table".to_string()))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            v1_vector_ingress, 50,
            "Expected vector_ingress (v1) to still be tracked (only database_ingress is skipped)"
        );

        // v2 vector ingress should be tracked
        let v2_vector_ingress = stats
            .vector_ingress_v2
            .get(&(component_path, "vector_table".to_string()))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            v2_vector_ingress, 75,
            "Expected vector_ingress_v2 to be tracked normally"
        );

        // Verify the flag is set
        assert!(
            stats.skip_v1_database_ingress,
            "Expected skip_v1_database_ingress flag to be true"
        );
    }
}
