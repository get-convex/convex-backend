#![feature(lazy_cell)]
#![feature(arc_unwrap_or_clone)]
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
    document::DocumentUpdate,
    execution_context::ExecutionContext,
    log_lines::LogLine,
    query_journal::QueryJournal,
    runtime::Runtime,
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
use futures::channel::mpsc;
use isolate::{
    ActionCallbacks,
    FunctionOutcome,
    ValidatedUdfPathAndArgs,
};
use keybroker::Identity;
use model::environment_variables::types::{
    EnvVarName,
    EnvVarValue,
};
#[cfg(any(test, feature = "testing"))]
use proptest::strategy::Strategy;
use sync_types::Timestamp;
use usage_tracking::FunctionUsageStats;
use value::{
    ResolvedDocumentId,
    TableNumber,
};

mod in_memory_indexes;
mod isolate_worker;
mod metrics;
mod module_cache;
mod proto;
pub mod server;

#[async_trait]
pub trait FunctionRunner<RT: Runtime>: Send + Sync + 'static {
    async fn run_function(
        &self,
        path_and_args: ValidatedUdfPathAndArgs,
        udf_type: UdfType,
        identity: Identity,
        ts: RepeatableTimestamp,
        existing_writes: FunctionWrites,
        journal: QueryJournal,
        log_line_sender: Option<mpsc::UnboundedSender<LogLine>>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        in_memory_index_last_modified: BTreeMap<IndexId, Timestamp>,
        context: ExecutionContext,
    ) -> anyhow::Result<(
        Option<FunctionFinalTransaction>,
        FunctionOutcome,
        FunctionUsageStats,
    )>;

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
    pub rows_read: BTreeMap<TableNumber, u64>,
}

impl<RT: Runtime> From<Transaction<RT>> for FunctionFinalTransaction {
    fn from(tx: Transaction<RT>) -> Self {
        let begin_timestamp = *tx.begin_timestamp();
        let rows_read = tx
            .stats()
            .iter()
            .map(|(table, stats)| (*table, stats.rows_read))
            .collect();
        let (reads, writes) = tx.into_reads_and_writes();
        Self {
            begin_timestamp,
            reads: reads.into(),
            writes: writes.into(),
            rows_read,
        }
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
    pub updates: BTreeMap<ResolvedDocumentId, DocumentUpdate>,

    // All of the new DocumentIds that were generated in this transaction.
    pub generated_ids: BTreeSet<ResolvedDocumentId>,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for FunctionWrites {
    type Parameters = ();

    type Strategy = impl Strategy<Value = FunctionWrites>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (
            proptest::collection::vec(proptest::prelude::any::<DocumentUpdate>(), 0..4),
            proptest::collection::btree_set(proptest::prelude::any::<ResolvedDocumentId>(), 0..4),
        )
            .prop_map(|(updates, generated_ids)| Self {
                updates: updates.into_iter().map(|u| (u.id, u)).collect(),
                generated_ids,
            })
            .boxed()
    }
}

impl From<Writes> for FunctionWrites {
    fn from(writes: Writes) -> Self {
        let (updates, generated_ids) = writes.into_updates_and_generated_ids();
        Self {
            updates,
            generated_ids,
        }
    }
}
