#![feature(iterator_try_collect)]
#![feature(lazy_cell)]

use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use common::{
    execution_context::ExecutionId,
    types::{
        ModuleEnvironment,
        UdfIdentifier,
    },
};
use events::usage::{
    UsageEvent,
    UsageEventLogger,
};
use metrics::storage::log_action_compute;
use parking_lot::Mutex;
use pb::usage::{
    CounterWithTag as CounterWithTagProto,
    FunctionUsageStats as FunctionUsageStatsProto,
};
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

mod metrics;

type TableName = String;
type FunctionName = String;
type StorageAPI = String;
type FunctionTag = String;

/// The state maintained by backend usage counters
#[derive(Default, Debug)]
pub struct UsageCounterState {
    pub recent_calls: WithHeapSize<BTreeMap<FunctionName, u64>>,
    pub recent_calls_by_tag: WithHeapSize<BTreeMap<FunctionTag, u64>>,
    pub recent_node_action_compute_time: WithHeapSize<BTreeMap<FunctionName, u64>>,
    pub recent_v8_action_compute_time: WithHeapSize<BTreeMap<FunctionName, u64>>,

    // Storage - note that we don't break storage by function since it can also
    // be called outside of function.
    pub recent_storage_calls: WithHeapSize<BTreeMap<StorageAPI, u64>>,
    pub recent_storage_ingress_size: u64,
    pub recent_storage_egress_size: u64,

    // Bandwidth by table
    pub recent_database_ingress_size: WithHeapSize<BTreeMap<TableName, u64>>,
    pub recent_database_egress_size: WithHeapSize<BTreeMap<TableName, u64>>,
    pub recent_vector_ingress_size: WithHeapSize<BTreeMap<TableName, u64>>,
    pub recent_vector_egress_size: WithHeapSize<BTreeMap<TableName, u64>>,

    // Bandwidth by function
    pub recent_database_ingress_size_by_function: WithHeapSize<BTreeMap<FunctionName, u64>>,
    pub recent_database_egress_size_by_function: WithHeapSize<BTreeMap<FunctionName, u64>>,
    pub recent_vector_ingress_size_by_function: WithHeapSize<BTreeMap<FunctionName, u64>>,
    pub recent_vector_egress_size_by_function: WithHeapSize<BTreeMap<FunctionName, u64>>,
}

impl HeapSize for UsageCounterState {
    fn heap_size(&self) -> usize {
        self.recent_calls.heap_size()
            + self.recent_calls_by_tag.heap_size()
            + self.recent_storage_calls.heap_size()
            + self.recent_node_action_compute_time.heap_size()
            + self.recent_v8_action_compute_time.heap_size()
            + self.recent_database_ingress_size.heap_size()
            + self.recent_database_egress_size.heap_size()
            + self.recent_vector_ingress_size.heap_size()
            + self.recent_vector_egress_size.heap_size()
            + self.recent_database_ingress_size_by_function.heap_size()
            + self.recent_database_egress_size_by_function.heap_size()
            + self.recent_vector_ingress_size_by_function.heap_size()
            + self.recent_vector_egress_size_by_function.heap_size()
    }
}

/// Present if a document is in a table with one or more vector indexes and has
/// an actual vector in at least one of those indexes.
///
/// Should be Absent if the table has no vector indexes or if this particular
/// document does not have a vector in any of the vector indexes.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Debug, proptest_derive::Arbitrary)
)]
pub enum DocInVectorIndex {
    Present,
    Absent,
}

/// The core usage stats aggregator that is cheaply cloneable
#[derive(Clone, Debug)]
pub struct UsageCounter {
    state: Arc<Mutex<UsageCounterState>>,
    usage_logger: Arc<dyn UsageEventLogger>,
}

impl HeapSize for UsageCounter {
    fn heap_size(&self) -> usize {
        self.state.lock().heap_size()
    }
}

pub const KB: u64 = 1 << 10;

pub fn round_up(n: u64, k: u64) -> u64 {
    (n + k - 1) / k * k
}

impl UsageCounter {
    pub fn new(usage_logger: Arc<dyn UsageEventLogger>) -> Self {
        let state = Arc::new(Mutex::new(UsageCounterState::default()));
        Self {
            state,
            usage_logger,
        }
    }

    pub fn collect(&self) -> UsageCounterState {
        let mut state = self.state.lock();
        UsageCounterState {
            recent_calls: std::mem::take(&mut state.recent_calls),
            recent_calls_by_tag: std::mem::take(&mut state.recent_calls_by_tag),
            recent_storage_ingress_size: std::mem::take(&mut state.recent_storage_ingress_size),
            recent_storage_egress_size: std::mem::take(&mut state.recent_storage_egress_size),
            recent_storage_calls: std::mem::take(&mut state.recent_storage_calls),
            recent_v8_action_compute_time: std::mem::take(&mut state.recent_v8_action_compute_time),
            recent_node_action_compute_time: std::mem::take(
                &mut state.recent_node_action_compute_time,
            ),
            recent_database_ingress_size: std::mem::take(&mut state.recent_database_ingress_size),
            recent_database_egress_size: std::mem::take(&mut state.recent_database_egress_size),
            recent_vector_ingress_size: std::mem::take(&mut state.recent_vector_ingress_size),
            recent_vector_egress_size: std::mem::take(&mut state.recent_vector_egress_size),
            recent_database_ingress_size_by_function: std::mem::take(
                &mut state.recent_database_ingress_size_by_function,
            ),
            recent_database_egress_size_by_function: std::mem::take(
                &mut state.recent_database_egress_size_by_function,
            ),
            recent_vector_ingress_size_by_function: std::mem::take(
                &mut state.recent_vector_ingress_size_by_function,
            ),
            recent_vector_egress_size_by_function: std::mem::take(
                &mut state.recent_vector_egress_size_by_function,
            ),
        }
    }

    // Convert into MB-milliseconds of compute time
    fn calculate_action_compute_time(&self, duration: Duration, memory_in_mb: u64) -> u64 {
        u64::try_from(duration.as_millis())
            .expect("Action was running for over 584 billion years??")
            * memory_in_mb
    }
}

struct ActionStats {
    env: ModuleEnvironment,
    duration: Duration,
    memory_in_mb: u64,
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
    },
    Export,
    CachedQuery,
    UncachedQuery,
    Mutation,
    Import,
}

impl CallType {
    fn action_stats(self) -> Option<ActionStats> {
        match self {
            Self::Action {
                env,
                duration,
                memory_in_mb,
            } => Some(ActionStats {
                env,
                duration,
                memory_in_mb,
            }),
            Self::HttpAction {
                duration,
                memory_in_mb,
            } => Some(ActionStats {
                // Http Actions cannot be run in Node, so they must be in isolate.
                env: ModuleEnvironment::Isolate,
                duration,
                memory_in_mb,
            }),
            _ => None,
        }
    }

    fn tag(&self) -> &'static str {
        match self {
            Self::Action { .. } => "action",
            Self::Export => "export",
            Self::CachedQuery => "cached_query",
            Self::UncachedQuery => "uncached_query",
            Self::Mutation => "mutation",
            Self::HttpAction { .. } => "http_action",
            Self::Import => "import",
        }
    }

    fn memory_megabytes(&self) -> u64 {
        match self {
            CallType::Action { memory_in_mb, .. } => *memory_in_mb,
            _ => 0,
        }
    }

    fn duration_millis(&self) -> u64 {
        match self {
            CallType::Action { duration, .. } => u64::try_from(duration.as_millis())
                .expect("Action was running for over 584 billion years??"),
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
}

impl UsageCounter {
    pub fn track_call(
        &self,
        udf_path: UdfIdentifier,
        execution_id: ExecutionId,
        call_type: CallType,
        stats: FunctionUsageStats,
    ) {
        let mut usage_metrics = Vec::new();

        // Because system udfs might cause usage before any data is added by the user,
        // we do not count their calls. We do count their bandwidth.
        let (should_track_calls, udf_id_type) = match &udf_path {
            UdfIdentifier::Function(path) => (!path.is_system(), "function"),
            UdfIdentifier::Http(_) => (true, "http"),
            UdfIdentifier::Cli(_) => (false, "cli"),
        };
        usage_metrics.push(UsageEvent::FunctionCall {
            id: execution_id.to_string(),
            udf_id: udf_path.to_string(),
            udf_id_type: udf_id_type.to_string(),
            tag: call_type.tag().to_string(),
            memory_megabytes: call_type.memory_megabytes(),
            duration_millis: call_type.duration_millis(),
            environment: call_type.environment(),
            is_tracked: should_track_calls,
        });

        if should_track_calls {
            let mut state = self.state.lock();
            state
                .recent_calls
                .mutate_entry_or_default(udf_path.to_string(), |count| *count += 1);
            state
                .recent_calls_by_tag
                .mutate_entry_or_default(call_type.tag().to_string(), |count| *count += 1);

            if let Some(ActionStats {
                env,
                memory_in_mb,
                duration,
            }) = call_type.action_stats()
            {
                let value = self.calculate_action_compute_time(duration, memory_in_mb);
                log_action_compute(&env);
                match env {
                    ModuleEnvironment::Isolate => state
                        .recent_v8_action_compute_time
                        .mutate_entry_or_default(udf_path.to_string(), |count| *count += value),
                    ModuleEnvironment::Node => state
                        .recent_node_action_compute_time
                        .mutate_entry_or_default(udf_path.to_string(), |count| *count += value),
                    // If the UDF can't be called because it was either deleted or not visible to
                    // the caller, it errors before we know what environment it would have executed
                    // in. We can bill a call for these, but there was no actual execution to bill
                    // here.
                    ModuleEnvironment::Invalid => {},
                }
            }
        }
        // We always track bandwidth, even for system udfs.
        self._track_function_usage(udf_path, stats, execution_id, &mut usage_metrics);
        self.usage_logger.record(usage_metrics);
    }

    // TODO: The existence of this function is a hack due to shortcuts we have
    // done in Node.js usage tracking. It should only be used by Node.js action
    // callbacks. We should only be using track_call() and never calling this
    // this directly. Otherwise, we will have the usage reflected in the usage
    // stats for billing but not in the UDF execution log counters.
    pub fn track_function_usage(
        &self,
        udf_path: UdfIdentifier,
        execution_id: ExecutionId,
        stats: FunctionUsageStats,
    ) {
        let mut usage_metrics = Vec::new();
        self._track_function_usage(udf_path, stats, execution_id, &mut usage_metrics);
        self.usage_logger.record(usage_metrics);
    }

    pub fn _track_function_usage(
        &self,
        udf_path: UdfIdentifier,
        stats: FunctionUsageStats,
        execution_id: ExecutionId,
        usage_metrics: &mut Vec<UsageEvent>,
    ) {
        let mut state = self.state.lock();

        let aggregated_stats = stats.aggregate();

        // Merge the storage stats.
        for (storage_api, function_count) in stats.storage_calls {
            state
                .recent_storage_calls
                .mutate_entry_or_default(storage_api.clone(), |count| *count += function_count);
            usage_metrics.push(UsageEvent::FunctionStorageCalls {
                id: execution_id.to_string(),
                udf_id: udf_path.to_string(),
                call: storage_api,
                count: function_count,
            });
        }
        state.recent_storage_ingress_size += stats.storage_ingress_size;
        state.recent_storage_egress_size += stats.storage_egress_size;
        usage_metrics.push(UsageEvent::FunctionStorageBandwidth {
            id: execution_id.to_string(),
            udf_id: udf_path.to_string(),
            ingress: stats.storage_ingress_size,
            egress: stats.storage_egress_size,
        });
        // Merge "by table" bandwidth stats.
        for (table_name, ingress_size) in stats.database_ingress_size {
            state
                .recent_database_ingress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += ingress_size);
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                udf_id: udf_path.to_string(),
                table_name,
                ingress: ingress_size,
                egress: 0,
            });
        }
        for (table_name, egress_size) in stats.database_egress_size {
            state
                .recent_database_egress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += egress_size);
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                udf_id: udf_path.to_string(),
                table_name,
                ingress: 0,
                egress: egress_size,
            });
        }
        for (table_name, ingress_size) in stats.vector_ingress_size {
            state
                .recent_vector_ingress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += ingress_size);
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                udf_id: udf_path.to_string(),
                table_name,
                ingress: ingress_size,
                egress: 0,
            });
        }
        for (table_name, egress_size) in stats.vector_egress_size {
            state
                .recent_vector_egress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += egress_size);
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                udf_id: udf_path.to_string(),
                table_name,
                ingress: 0,
                egress: egress_size,
            });
        }

        // Update the "by function" stats using the aggregated stats.
        state
            .recent_database_ingress_size_by_function
            .mutate_entry_or_default(udf_path.to_string(), |size| {
                *size += aggregated_stats.database_write_bytes
            });
        state
            .recent_database_egress_size_by_function
            .mutate_entry_or_default(udf_path.to_string(), |size| {
                *size += aggregated_stats.database_read_bytes
            });
        state
            .recent_vector_ingress_size_by_function
            .mutate_entry_or_default(udf_path.to_string(), |size| {
                *size += aggregated_stats.vector_index_write_bytes
            });
        state
            .recent_vector_egress_size_by_function
            .mutate_entry_or_default(udf_path.to_string(), |size| {
                *size += aggregated_stats.vector_index_read_bytes
            });
    }
}

// We can track storage attributed by UDF or not. This is why unlike database
// and vector search egress/ingress those methods are both on
// FunctionUsageTracker and UsageCounters directly.
pub trait StorageUsageTracker: Send + Sync {
    fn track_storage_call(&self, storage_api: &'static str) -> Box<dyn StorageCallTracker>;
}

pub trait StorageCallTracker: Send + Sync {
    fn track_storage_ingress_size(&self, ingress_size: u64);
    fn track_storage_egress_size(&self, egress_size: u64);
}

struct IndependentStorageCallTracker {
    execution_id: ExecutionId,
    state: Arc<Mutex<UsageCounterState>>,
    usage_logger: Arc<dyn UsageEventLogger>,
}

impl IndependentStorageCallTracker {
    fn new(
        execution_id: ExecutionId,
        state: Arc<Mutex<UsageCounterState>>,
        usage_logger: Arc<dyn UsageEventLogger>,
    ) -> Self {
        Self {
            execution_id,
            state,
            usage_logger,
        }
    }
}

impl StorageCallTracker for IndependentStorageCallTracker {
    fn track_storage_ingress_size(&self, ingress_size: u64) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_ingress_size(ingress_size);
        state.recent_storage_ingress_size += ingress_size;

        self.usage_logger.record(vec![UsageEvent::StorageBandwidth {
            id: self.execution_id.to_string(),
            ingress: ingress_size,
            egress: 0,
        }]);
    }

    fn track_storage_egress_size(&self, egress_size: u64) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_egress_size(egress_size);
        state.recent_storage_egress_size += egress_size;
        self.usage_logger.record(vec![UsageEvent::StorageBandwidth {
            id: self.execution_id.to_string(),
            ingress: 0,
            egress: egress_size,
        }]);
    }
}

impl StorageUsageTracker for UsageCounter {
    fn track_storage_call(&self, storage_api: &'static str) -> Box<dyn StorageCallTracker> {
        let mut state = self.state.lock();
        let execution_id = ExecutionId::new();
        metrics::storage::log_storage_call();
        state
            .recent_storage_calls
            .mutate_entry_or_default(storage_api.to_string(), |count| *count += 1);

        self.usage_logger.record(vec![UsageEvent::StorageCall {
            id: execution_id.to_string(),
            call: storage_api.to_string(),
        }]);

        Box::new(IndependentStorageCallTracker::new(
            execution_id,
            self.state.clone(),
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
    // this is to move the logic for accounting ingress out of the Commiter into
    // the Transaction. Then Transaction can solely own the counters and we can
    // remove clone(). The alternative is for the Commiter to take ownership of
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
    // vectors in a vector index, call `track_vector_ingress_size` instead of
    // this method.
    //
    // You must always check to see if a document is a vector index before
    // calling this method.
    pub fn track_database_ingress_size(
        &self,
        table_name: String,
        ingress_size: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        state
            .database_ingress_size
            .mutate_entry_or_default(table_name.clone(), |count| {
                *count += round_up(ingress_size, KB)
            });
    }

    pub fn track_database_egress_size(
        &self,
        table_name: String,
        egress_size: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        let mut state = self.state.lock();
        state
            .database_egress_size
            .mutate_entry_or_default(table_name.clone(), |count| {
                *count += round_up(egress_size, KB)
            });
    }

    // Tracks the vector ingress surcharge and database usage for documents
    // that have one or more vectors in a vector index.
    //
    // If the document does not have a vector in a vector index, call
    // `track_database_ingress_size` instead of this method.
    //
    // Vector bandwidth is a surcharge on vector related bandwidth usage. As a
    // result it counts against both bandwidth ingress and vector ingress.
    // Ingress is a bit trickier than egress because vector ingress needs to be
    // updated whenever the mutated document is in a vector index. To be in a
    // vector index the document must both be in a table with a vector index and
    // have at least one vector that's actually used in the index.
    pub fn track_vector_ingress_size(
        &self,
        table_name: String,
        ingress_size: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        // Note that vector search counts as both database and vector bandwidth
        // per the comment above.
        let mut state = self.state.lock();
        let rounded_size = round_up(ingress_size, KB);
        state
            .database_ingress_size
            .mutate_entry_or_default(table_name.clone(), |count| {
                *count += rounded_size;
            });
        state
            .vector_ingress_size
            .mutate_entry_or_default(table_name.clone(), |count| {
                *count += rounded_size;
            });
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
    // Unlike track_database_ingress_size, this method is explicitly vector related
    // because we should always know that the relevant operation is a vector
    // search. In contrast, for ingress any insert/update/delete could happen to
    // impact a vector index.
    pub fn track_vector_egress_size(
        &self,
        table_name: String,
        egress_size: u64,
        skip_logging: bool,
    ) {
        if skip_logging {
            return;
        }

        // Note that vector search counts as both database and vector bandwidth
        // per the comment above.
        let mut state = self.state.lock();
        let rounded_size = round_up(egress_size, KB);
        state
            .database_egress_size
            .mutate_entry_or_default(table_name.clone(), |count| *count += rounded_size);
        state
            .vector_egress_size
            .mutate_entry_or_default(table_name.clone(), |count| *count += rounded_size);
    }
}

// For UDFs, we track storage at the per UDF level, no finer. So we can just
// aggregate over the entire UDF and not worry about sending usage events or
// creating unique execution ids.
impl StorageCallTracker for FunctionUsageTracker {
    fn track_storage_ingress_size(&self, ingress_size: u64) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_ingress_size(ingress_size);
        state.storage_ingress_size += ingress_size;
    }

    fn track_storage_egress_size(&self, egress_size: u64) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_egress_size(egress_size);
        state.storage_egress_size += egress_size;
    }
}

impl StorageUsageTracker for FunctionUsageTracker {
    fn track_storage_call(&self, storage_api: &'static str) -> Box<dyn StorageCallTracker> {
        let mut state = self.state.lock();
        metrics::storage::log_storage_call();
        state
            .storage_calls
            .mutate_entry_or_default(storage_api.to_string(), |count| *count += 1);
        Box::new(self.clone())
    }
}

/// User-facing UDF stats, built
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct FunctionUsageStats {
    pub storage_calls: WithHeapSize<BTreeMap<StorageAPI, u64>>,
    pub storage_ingress_size: u64,
    pub storage_egress_size: u64,
    pub database_ingress_size: WithHeapSize<BTreeMap<TableName, u64>>,
    pub database_egress_size: WithHeapSize<BTreeMap<TableName, u64>>,
    pub vector_ingress_size: WithHeapSize<BTreeMap<TableName, u64>>,
    pub vector_egress_size: WithHeapSize<BTreeMap<TableName, u64>>,
}

impl FunctionUsageStats {
    pub fn aggregate(&self) -> AggregatedFunctionUsageStats {
        AggregatedFunctionUsageStats {
            database_read_bytes: self.database_egress_size.values().sum(),
            database_write_bytes: self.database_ingress_size.values().sum(),
            storage_read_bytes: self.storage_egress_size,
            storage_write_bytes: self.storage_ingress_size,
            vector_index_read_bytes: self.vector_egress_size.values().sum(),
            vector_index_write_bytes: self.vector_ingress_size.values().sum(),
        }
    }

    fn merge(&mut self, other: Self) {
        // Merge the storage stats.
        for (storage_api, function_count) in other.storage_calls {
            self.storage_calls
                .mutate_entry_or_default(storage_api, |count| *count += function_count);
        }
        self.storage_ingress_size += other.storage_ingress_size;
        self.storage_egress_size += other.storage_egress_size;

        // Merge "by table" bandwidth other.
        for (table_name, ingress_size) in other.database_ingress_size {
            self.database_ingress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += ingress_size);
        }
        for (table_name, egress_size) in other.database_egress_size {
            self.database_egress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += egress_size);
        }
        for (table_name, ingress_size) in other.vector_ingress_size {
            self.vector_ingress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += ingress_size);
        }
        for (table_name, egress_size) in other.vector_egress_size {
            self.vector_egress_size
                .mutate_entry_or_default(table_name.clone(), |count| *count += egress_size);
        }
    }
}

fn to_by_tag_count(counts: impl Iterator<Item = (String, u64)>) -> Vec<CounterWithTagProto> {
    counts
        .map(|(tag, count)| CounterWithTagProto {
            name: Some(tag),
            count: Some(count),
        })
        .collect()
}

fn from_by_tag_count(
    counts: Vec<CounterWithTagProto>,
) -> anyhow::Result<impl Iterator<Item = (String, u64)>> {
    let counts: Vec<_> = counts
        .into_iter()
        .map(|c| -> anyhow::Result<_> {
            let name = c.name.context("Missing `tag` field")?;
            let count = c.count.context("Missing `count` field")?;
            Ok((name, count))
        })
        .try_collect()?;
    Ok(counts.into_iter())
}

impl From<FunctionUsageStats> for FunctionUsageStatsProto {
    fn from(stats: FunctionUsageStats) -> Self {
        FunctionUsageStatsProto {
            storage_calls: to_by_tag_count(stats.storage_calls.into_iter()),
            storage_ingress_size: Some(stats.storage_ingress_size),
            storage_egress_size: Some(stats.storage_egress_size),
            database_ingress_size: to_by_tag_count(stats.database_ingress_size.into_iter()),
            database_egress_size: to_by_tag_count(stats.database_egress_size.into_iter()),
            vector_ingress_size: to_by_tag_count(stats.vector_ingress_size.into_iter()),
            vector_egress_size: to_by_tag_count(stats.vector_egress_size.into_iter()),
        }
    }
}

impl TryFrom<FunctionUsageStatsProto> for FunctionUsageStats {
    type Error = anyhow::Error;

    fn try_from(stats: FunctionUsageStatsProto) -> anyhow::Result<Self> {
        let storage_calls = from_by_tag_count(stats.storage_calls)?.collect();
        let storage_ingress_size = stats
            .storage_ingress_size
            .context("Missing `storage_ingress_size` field")?;
        let storage_egress_size = stats
            .storage_egress_size
            .context("Missing `storage_egress_size` field")?;
        let database_ingress_size = from_by_tag_count(stats.database_ingress_size)?.collect();
        let database_egress_size = from_by_tag_count(stats.database_egress_size)?.collect();
        let vector_ingress_size = from_by_tag_count(stats.vector_ingress_size)?.collect();
        let vector_egress_size = from_by_tag_count(stats.vector_egress_size)?.collect();

        Ok(FunctionUsageStats {
            storage_calls,
            storage_ingress_size,
            storage_egress_size,
            database_ingress_size,
            database_egress_size,
            vector_ingress_size,
            vector_egress_size,
        })
    }
}

/// User-facing UDF stats, that is logged in the UDF execution log
/// and might be used for debugging purposes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AggregatedFunctionUsageStats {
    pub database_read_bytes: u64,
    pub database_write_bytes: u64,
    pub storage_read_bytes: u64,
    pub storage_write_bytes: u64,
    pub vector_index_read_bytes: u64,
    pub vector_index_write_bytes: u64,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::{
        FunctionUsageStats,
        FunctionUsageStatsProto,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_usage_stats_roundtrips(stats in any::<FunctionUsageStats>()) {
            assert_roundtrips::<FunctionUsageStats, FunctionUsageStatsProto>(stats);
        }
    }
}
