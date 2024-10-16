#![feature(iterator_try_collect)]
#![feature(lazy_cell)]
#![feature(let_chains)]

use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use common::{
    components::ComponentPath,
    execution_context::ExecutionId,
    types::{
        ModuleEnvironment,
        StorageUuid,
        UdfIdentifier,
    },
};
use events::usage::{
    UsageEvent,
    UsageEventLogger,
};
use headers::ContentType;
use parking_lot::Mutex;
use pb::usage::{
    CounterWithComponent as CounterWithComponentProto,
    CounterWithTag as CounterWithTagProto,
    FunctionUsageStats as FunctionUsageStatsProto,
};
use value::{
    heap_size::WithHeapSize,
    sha256::Sha256Digest,
};

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
    UncachedQuery,
    Mutation,
    Import,
}

impl CallType {
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
            CallType::Action { memory_in_mb, .. } | CallType::HttpAction { memory_in_mb, .. } => {
                *memory_in_mb
            },
            _ => 0,
        }
    }

    fn duration_millis(&self) -> u64 {
        match self {
            CallType::Action { duration, .. } | CallType::HttpAction { duration, .. } => {
                u64::try_from(duration.as_millis())
                    .expect("Action was running for over 584 billion years??")
            },
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
            UdfIdentifier::Function(path) => (!path.udf_path.is_system(), "function"),
            UdfIdentifier::Http(_) => (true, "http"),
            UdfIdentifier::Cli(_) => (false, "cli"),
        };
        let (component_path, udf_id) = udf_path.clone().into_component_and_udf_path();
        usage_metrics.push(UsageEvent::FunctionCall {
            id: execution_id.to_string(),
            component_path,
            udf_id,
            udf_id_type: udf_id_type.to_string(),
            tag: call_type.tag().to_string(),
            memory_megabytes: call_type.memory_megabytes(),
            duration_millis: call_type.duration_millis(),
            environment: call_type.environment(),
            is_tracked: should_track_calls,
            response_sha256: call_type.response_sha256(),
        });

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
        // Merge the storage stats.
        let (_, udf_id) = udf_path.clone().into_component_and_udf_path();
        for ((component_path, storage_api), function_count) in stats.storage_calls {
            usage_metrics.push(UsageEvent::FunctionStorageCalls {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                call: storage_api,
                count: function_count,
            });
        }

        for (component_path, ingress_size) in stats.storage_ingress_size {
            usage_metrics.push(UsageEvent::FunctionStorageBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                ingress: ingress_size,
                egress: 0,
            });
        }
        for (component_path, egress_size) in stats.storage_egress_size {
            usage_metrics.push(UsageEvent::FunctionStorageBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                ingress: 0,
                egress: egress_size,
            });
        }
        // Merge "by table" bandwidth stats.
        for ((component_path, table_name), ingress_size) in stats.database_ingress_size {
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: ingress_size,
                egress: 0,
            });
        }
        for ((component_path, table_name), egress_size) in stats.database_egress_size {
            usage_metrics.push(UsageEvent::DatabaseBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                egress: egress_size,
            });
        }
        for ((component_path, table_name), ingress_size) in stats.vector_ingress_size {
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: ingress_size,
                egress: 0,
            });
        }
        for ((component_path, table_name), egress_size) in stats.vector_egress_size {
            usage_metrics.push(UsageEvent::VectorBandwidth {
                id: execution_id.to_string(),
                component_path: component_path.serialize(),
                udf_id: udf_id.clone(),
                table_name,
                ingress: 0,
                egress: egress_size,
            });
        }
    }
}

// We can track storage attributed by UDF or not. This is why unlike database
// and vector search egress/ingress those methods are both on
// FunctionUsageTracker and UsageCounters directly.
pub trait StorageUsageTracker: Send + Sync {
    fn track_storage_call(
        &self,
        component_path: ComponentPath,
        storage_api: &'static str,
        storage_id: StorageUuid,
        content_type: Option<ContentType>,
        sha256: Sha256Digest,
    ) -> Box<dyn StorageCallTracker>;
}

pub trait StorageCallTracker: Send + Sync {
    fn track_storage_ingress_size(
        &self,
        component_path: ComponentPath,
        tag: String,
        ingress_size: u64,
    );
    fn track_storage_egress_size(
        &self,
        component_path: ComponentPath,
        tag: String,
        egress_size: u64,
    );
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

impl StorageCallTracker for IndependentStorageCallTracker {
    fn track_storage_ingress_size(
        &self,
        component_path: ComponentPath,
        tag: String,
        ingress_size: u64,
    ) {
        metrics::storage::log_storage_ingress_size(ingress_size);
        self.usage_logger.record(vec![UsageEvent::StorageBandwidth {
            id: self.execution_id.to_string(),
            component_path: component_path.serialize(),
            tag,
            ingress: ingress_size,
            egress: 0,
        }]);
    }

    fn track_storage_egress_size(
        &self,
        component_path: ComponentPath,
        tag: String,
        egress_size: u64,
    ) {
        metrics::storage::log_storage_egress_size(egress_size);
        self.usage_logger.record(vec![UsageEvent::StorageBandwidth {
            id: self.execution_id.to_string(),
            component_path: component_path.serialize(),
            tag,
            ingress: 0,
            egress: egress_size,
        }]);
    }
}

impl StorageUsageTracker for UsageCounter {
    fn track_storage_call(
        &self,
        component_path: ComponentPath,
        storage_api: &'static str,
        storage_id: StorageUuid,
        content_type: Option<ContentType>,
        sha256: Sha256Digest,
    ) -> Box<dyn StorageCallTracker> {
        let execution_id = ExecutionId::new();
        metrics::storage::log_storage_call();
        self.usage_logger.record(vec![UsageEvent::StorageCall {
            id: execution_id.to_string(),
            component_path: component_path.serialize(),
            // Ideally we would track the Id<_storage> instead of the StorageUuid
            // but it's a bit annoying for now, so just going with this.
            storage_id: storage_id.to_string(),
            call: storage_api.to_string(),
            content_type: content_type.map(|c| c.to_string()),
            sha256: sha256.as_hex(),
        }]);

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
    // vectors in a vector index, call `track_vector_ingress_size` instead of
    // this method.
    //
    // You must always check to see if a document is a vector index before
    // calling this method.
    pub fn track_database_ingress_size(
        &self,
        component_path: ComponentPath,
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
            .mutate_entry_or_default((component_path, table_name), |count| *count += ingress_size);
    }

    pub fn track_database_egress_size(
        &self,
        component_path: ComponentPath,
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
            .mutate_entry_or_default((component_path, table_name), |count| *count += egress_size);
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
        component_path: ComponentPath,
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
        let key = (component_path, table_name);
        state
            .database_ingress_size
            .mutate_entry_or_default(key.clone(), |count| {
                *count += ingress_size;
            });
        state
            .vector_ingress_size
            .mutate_entry_or_default(key, |count| {
                *count += ingress_size;
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
        component_path: ComponentPath,
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
        let key = (component_path, table_name);
        state
            .database_egress_size
            .mutate_entry_or_default(key.clone(), |count| *count += egress_size);
        state
            .vector_egress_size
            .mutate_entry_or_default(key, |count| *count += egress_size);
    }
}

// For UDFs, we track storage at the per UDF level, no finer. So we can just
// aggregate over the entire UDF and not worry about sending usage events or
// creating unique execution ids.
// Note: If we want finer-grained breakdown of file bandwidth, we can thread the
// tag through FunctionUsageStats. For now we're just interested in the
// breakdown of file bandwidth from functions vs external sources like snapshot
// export/cloud backups.
impl StorageCallTracker for FunctionUsageTracker {
    fn track_storage_ingress_size(
        &self,
        component_path: ComponentPath,
        _tag: String,
        ingress_size: u64,
    ) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_ingress_size(ingress_size);
        state
            .storage_ingress_size
            .mutate_entry_or_default(component_path, |count| *count += ingress_size);
    }

    fn track_storage_egress_size(
        &self,
        component_path: ComponentPath,
        _tag: String,
        egress_size: u64,
    ) {
        let mut state = self.state.lock();
        metrics::storage::log_storage_egress_size(egress_size);
        state
            .storage_egress_size
            .mutate_entry_or_default(component_path, |count| *count += egress_size);
    }
}

impl StorageUsageTracker for FunctionUsageTracker {
    fn track_storage_call(
        &self,
        component_path: ComponentPath,
        storage_api: &'static str,
        _storage_id: StorageUuid,
        _content_type: Option<ContentType>,
        _sha256: Sha256Digest,
    ) -> Box<dyn StorageCallTracker> {
        let mut state = self.state.lock();
        metrics::storage::log_storage_call();
        state
            .storage_calls
            .mutate_entry_or_default((component_path, storage_api.to_string()), |count| {
                *count += 1
            });
        Box::new(self.clone())
    }
}

type TableName = String;
type StorageAPI = String;

/// User-facing UDF stats, built
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionUsageStats {
    pub storage_calls: WithHeapSize<BTreeMap<(ComponentPath, StorageAPI), u64>>,
    pub storage_ingress_size: WithHeapSize<BTreeMap<ComponentPath, u64>>,
    pub storage_egress_size: WithHeapSize<BTreeMap<ComponentPath, u64>>,
    pub database_ingress_size: WithHeapSize<BTreeMap<(ComponentPath, TableName), u64>>,
    pub database_egress_size: WithHeapSize<BTreeMap<(ComponentPath, TableName), u64>>,
    pub vector_ingress_size: WithHeapSize<BTreeMap<(ComponentPath, TableName), u64>>,
    pub vector_egress_size: WithHeapSize<BTreeMap<(ComponentPath, TableName), u64>>,
}

impl FunctionUsageStats {
    pub fn aggregate(&self) -> AggregatedFunctionUsageStats {
        AggregatedFunctionUsageStats {
            database_read_bytes: self.database_egress_size.values().sum(),
            database_write_bytes: self.database_ingress_size.values().sum(),
            storage_read_bytes: self.storage_egress_size.values().sum(),
            storage_write_bytes: self.storage_ingress_size.values().sum(),
            vector_index_read_bytes: self.vector_egress_size.values().sum(),
            vector_index_write_bytes: self.vector_ingress_size.values().sum(),
        }
    }

    fn merge(&mut self, other: Self) {
        // Merge the storage stats.
        for (key, function_count) in other.storage_calls {
            self.storage_calls
                .mutate_entry_or_default(key, |count| *count += function_count);
        }
        for (key, ingress_size) in other.storage_ingress_size {
            self.storage_ingress_size
                .mutate_entry_or_default(key, |count| *count += ingress_size);
        }
        for (key, egress_size) in other.storage_egress_size {
            self.storage_egress_size
                .mutate_entry_or_default(key, |count| *count += egress_size);
        }

        // Merge "by table" bandwidth other.
        for (key, ingress_size) in other.database_ingress_size {
            self.database_ingress_size
                .mutate_entry_or_default(key.clone(), |count| *count += ingress_size);
        }
        for (key, egress_size) in other.database_egress_size {
            self.database_egress_size
                .mutate_entry_or_default(key.clone(), |count| *count += egress_size);
        }
        for (key, ingress_size) in other.vector_ingress_size {
            self.vector_ingress_size
                .mutate_entry_or_default(key.clone(), |count| *count += ingress_size);
        }
        for (key, egress_size) in other.vector_egress_size {
            self.vector_egress_size
                .mutate_entry_or_default(key.clone(), |count| *count += egress_size);
        }
    }
}

#[cfg(any(test, feature = "testing"))]
mod usage_arbitrary {
    use proptest::prelude::*;

    use crate::{
        ComponentPath,
        FunctionUsageStats,
        StorageAPI,
        TableName,
        WithHeapSize,
    };

    impl Arbitrary for FunctionUsageStats {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            let strategies = (
                proptest::collection::btree_map(
                    any::<(ComponentPath, StorageAPI)>(),
                    0..=1024u64,
                    0..=4,
                )
                .prop_map(WithHeapSize::from),
                proptest::collection::btree_map(any::<ComponentPath>(), 0..=1024u64, 0..=4)
                    .prop_map(WithHeapSize::from),
                proptest::collection::btree_map(any::<ComponentPath>(), 0..=1024u64, 0..=4)
                    .prop_map(WithHeapSize::from),
                proptest::collection::btree_map(
                    any::<(ComponentPath, TableName)>(),
                    0..=1024u64,
                    0..=4,
                )
                .prop_map(WithHeapSize::from),
                proptest::collection::btree_map(
                    any::<(ComponentPath, TableName)>(),
                    0..=1024u64,
                    0..=4,
                )
                .prop_map(WithHeapSize::from),
                proptest::collection::btree_map(
                    any::<(ComponentPath, TableName)>(),
                    0..=1024u64,
                    0..=4,
                )
                .prop_map(WithHeapSize::from),
                proptest::collection::btree_map(
                    any::<(ComponentPath, TableName)>(),
                    0..=1024u64,
                    0..=4,
                )
                .prop_map(WithHeapSize::from),
            );
            strategies
                .prop_map(
                    |(
                        storage_calls,
                        storage_ingress_size,
                        storage_egress_size,
                        database_ingress_size,
                        database_egress_size,
                        vector_ingress_size,
                        vector_egress_size,
                    )| FunctionUsageStats {
                        storage_calls,
                        storage_ingress_size,
                        storage_egress_size,
                        database_ingress_size,
                        database_egress_size,
                        vector_ingress_size,
                        vector_egress_size,
                    },
                )
                .boxed()
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

fn from_by_tag_count(
    counts: Vec<CounterWithTagProto>,
) -> anyhow::Result<impl Iterator<Item = ((ComponentPath, String), u64)>> {
    let counts: Vec<_> = counts
        .into_iter()
        .map(|c| -> anyhow::Result<_> {
            let component_path = ComponentPath::deserialize(c.component_path.as_deref())?;
            let name = c.table_name.context("Missing `tag` field")?;
            let count = c.count.context("Missing `count` field")?;
            Ok(((component_path, name), count))
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
            storage_ingress_size_by_component: to_by_component_count(
                stats.storage_ingress_size.into_iter(),
            ),
            storage_egress_size_by_component: to_by_component_count(
                stats.storage_egress_size.into_iter(),
            ),
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
        let storage_ingress_size =
            from_by_component_tag_count(stats.storage_ingress_size_by_component)?.collect();
        let storage_egress_size =
            from_by_component_tag_count(stats.storage_egress_size_by_component)?.collect();
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
