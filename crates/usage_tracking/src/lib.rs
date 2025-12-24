#![feature(iterator_try_collect)]

use std::{
    collections::BTreeMap,
    fmt::Debug,
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
        ModuleEnvironment,
        StorageUuid,
        UdfIdentifier,
    },
    RequestId,
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

impl UsageCounter {
    pub fn new(usage_logger: Arc<dyn UsageEventLogger>) -> Self {
        Self { usage_logger }
    }

    // Used for tracking storage ingress outside of a user function (e.g. snapshot
    // import/export).
    pub async fn track_independent_storage_ingress(
        &self,
        component_path: ComponentPath,
        tag: String,
        ingress: u64,
    ) {
        let independent_tracker =
            IndependentStorageCallTracker::new(ExecutionId::new(), self.usage_logger.clone());

        independent_tracker
            .track_storage_ingress(component_path, tag, ingress)
            .await;
    }

    // Used for tracking storage egress outside of a user function (e.g. snapshot
    // import/export).
    pub async fn track_independent_storage_egress(
        &self,
        component_path: ComponentPath,
        tag: String,
        egress: u64,
    ) {
        let independent_tracker =
            IndependentStorageCallTracker::new(ExecutionId::new(), self.usage_logger.clone());

        independent_tracker
            .track_storage_egress(component_path, tag, egress)
            .await;
    }
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
        memory_in_mb: u64,
    },
    HttpAction {
        duration: Duration,
        memory_in_mb: u64,

        /// Sha256 of the response body
        response_sha256: Sha256Digest,
    },
    Export,
    CachedQuery,
    UncachedQuery {
        duration: Duration,
        memory_in_mb: u64,
    },
    Mutation {
        duration: Duration,
        memory_in_mb: u64,
        occ_info: Option<OccInfo>,
    },
    Import,
    CloudBackup,
    CloudRestore,
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
        let (_, udf_id) = udf_path.into_component_and_udf_path();
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
            usage_metrics.push(UsageEvent::TextBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress,
                egress: 0,
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
    pub fn track_vector_egress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        egress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        // Note that vector search counts as both database and vector bandwidth
        // per the comment above.
        let mut state = self.state.lock();
        let key = (component_path, table_name);
        *state.database_egress.entry(key.clone()).or_default() += egress;
        *state.vector_egress.entry(key).or_default() += egress;
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

    pub fn track_text_egress(
        &self,
        component_path: ComponentPath,
        table_name: String,
        egress: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        // TODO(jordan): decide if we also need to track database egress
        let mut state = self.state.lock();
        *state
            .text_egress
            .entry((component_path, table_name))
            .or_default() += egress;
    }

    /// Only track egress - because AWS only charges egress
    pub fn track_fetch_egress(&self, url: String, egress: u64) {
        let mut state = self.state.lock();
        *state.fetch_egress.entry(url).or_default() += egress;
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
              proptest::arbitrary::any::<(ComponentPath, TableName)>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub text_egress: BTreeMap<(ComponentPath, TableName), u64>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::btree_map(
              proptest::arbitrary::any::<String>(), 0..=1024u64, 0..=4,
            )")
    )]
    pub fetch_egress: BTreeMap<String, u64>,
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
            text_index_read_bytes: self.text_egress.values().sum(),
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
            text_egress,
            fetch_egress,
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
        for (key, egress) in text_egress {
            *self.text_egress.entry(key.clone()).or_default() += egress;
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
            let name = c.url.context("Missing `url` field")?;
            let count = c.count.context("Missing `count` field")?;
            Ok((name, count))
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
            text_egress: to_by_tag_count(stats.text_egress.into_iter()),
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
        let text_egress = from_by_tag_count(stats.text_egress)?.collect();
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
            text_egress,
            vector_ingress_v2,
            fetch_egress,
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
    pub text_index_read_bytes: u64,
    pub text_index_write_bytes: u64,
    pub vector_index_write_bytes_v2: u64,
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::{
        FunctionUsageStats,
        FunctionUsageStatsProto,
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
}
