#![feature(iterator_try_collect)]
#![feature(impl_trait_in_assoc_type)]
#![feature(try_blocks)]
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    auth::AuthConfig,
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    components::{
        ComponentDefinitionPath,
        ComponentName,
        Resource,
    },
    document::DocumentUpdateWithPrevTs,
    errors::JsError,
    execution_context::ExecutionContext,
    log_lines::LogLine,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    types::{
        IndexId,
        RepeatableTimestamp,
        UdfType,
    },
};
use database::{
    ReadSet,
    Transaction,
    TransactionReadSet,
    TransactionReadSize,
    Writes,
};
use imbl::OrdMap;
use isolate::ActionCallbacks;
use keybroker::Identity;
use model::{
    config::types::ModuleConfig,
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    modules::module_versions::{
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    udf_config::types::UdfConfig,
};
#[cfg(any(test, feature = "testing"))]
use proptest::strategy::Strategy;
use server::{
    FunctionMetadata,
    HttpActionMetadata,
};
use sync_types::{
    CanonicalizedModulePath,
    Timestamp,
};
use tokio::sync::mpsc;
use udf::{
    EvaluateAppDefinitionsResult,
    FunctionOutcome,
};
use usage_tracking::FunctionUsageStats;
use value::{
    identifier::Identifier,
    ResolvedDocumentId,
    TabletId,
};

mod in_memory_indexes;
pub mod in_process_function_runner;
mod metrics;
mod module_cache;
pub mod server;

#[async_trait]
pub trait FunctionRunner<RT: Runtime>: Send + Sync + 'static {
    async fn run_function(
        &self,
        udf_type: UdfType,
        identity: Identity,
        ts: RepeatableTimestamp,
        existing_writes: FunctionWrites,
        log_line_sender: Option<mpsc::UnboundedSender<LogLine>>,
        function_metadata: Option<FunctionMetadata>,
        http_action_metadata: Option<HttpActionMetadata>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        in_memory_index_last_modified: BTreeMap<IndexId, Timestamp>,
        context: ExecutionContext,
    ) -> anyhow::Result<(
        Option<FunctionFinalTransaction>,
        FunctionOutcome,
        FunctionUsageStats,
    )>;

    async fn analyze(
        &self,
        udf_config: UdfConfig,
        modules: BTreeMap<CanonicalizedModulePath, ModuleConfig>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>>;

    async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult>;

    async fn evaluate_component_initializer(
        &self,
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        path: ComponentDefinitionPath,
        definition: ModuleConfig,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>>;

    async fn evaluate_schema(
        &self,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<DatabaseSchema>;

    async fn evaluate_auth_config(
        &self,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        explanation: &str,
    ) -> anyhow::Result<AuthConfig>;

    /// Set the action callbacks. Only used for InProcessFunctionRunner to break
    /// a reference cycle between ApplicationFunctionRunner and dyn
    /// FunctionRunner.
    fn set_action_callbacks(&self, action_callbacks: Arc<dyn ActionCallbacks>);
}

/// Reads and writes from a UDF that executed in Funrun
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Clone, Debug, PartialEq, proptest_derive::Arbitrary)
)]
pub struct FunctionFinalTransaction {
    pub begin_timestamp: Timestamp,
    pub reads: FunctionReads,
    pub writes: FunctionWrites,
    pub rows_read_by_tablet: BTreeMap<TabletId, u64>,
}

impl<RT: Runtime> TryFrom<Transaction<RT>> for FunctionFinalTransaction {
    type Error = anyhow::Error;

    fn try_from(tx: Transaction<RT>) -> anyhow::Result<Self> {
        tx.require_not_nested()?;
        let begin_timestamp = *tx.begin_timestamp();
        let rows_read_by_tablet = tx
            .stats_by_tablet()
            .iter()
            .map(|(table, stats)| (*table, stats.rows_read))
            .collect();
        let (reads, writes) = tx.into_reads_and_writes();
        Ok(Self {
            begin_timestamp,
            reads: reads.into(),
            writes: writes.into_flat()?.into(),
            rows_read_by_tablet,
        })
    }
}

#[cfg_attr(
    any(test, feature = "testing"),
    derive(Clone, Debug, PartialEq, proptest_derive::Arbitrary)
)]
pub struct FunctionReads {
    pub reads: ReadSet,
    pub num_intervals: usize,
    pub user_tx_size: TransactionReadSize,
    pub system_tx_size: TransactionReadSize,
}

impl From<TransactionReadSet> for FunctionReads {
    fn from(read_set: TransactionReadSet) -> Self {
        let num_intervals = read_set.num_intervals();
        let user_tx_size = read_set.user_tx_size().clone();
        let system_tx_size = read_set.system_tx_size().clone();
        let reads = read_set.into_read_set();
        Self {
            reads,
            num_intervals,
            user_tx_size,
            system_tx_size,
        }
    }
}

/// Subset of [`Writes`] that is returned by [FunctionRunner] after a function
/// has executed.
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
#[derive(Clone, Default)]
pub struct FunctionWrites {
    pub updates: OrdMap<ResolvedDocumentId, DocumentUpdateWithPrevTs>,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for FunctionWrites {
    type Parameters = ();

    type Strategy = impl Strategy<Value = FunctionWrites>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        proptest::collection::vec(proptest::prelude::any::<DocumentUpdateWithPrevTs>(), 0..4)
            .prop_map(|updates| Self {
                updates: updates.into_iter().map(|u| (u.id, u)).collect(),
            })
            .boxed()
    }
}

impl From<Writes> for FunctionWrites {
    fn from(writes: Writes) -> Self {
        Self {
            updates: writes.into_updates(),
        }
    }
}
