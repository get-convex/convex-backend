use std::collections::{
    BTreeMap,
    VecDeque,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::tables::TableState,
    components::{
        ComponentId,
        ComponentPath,
    },
    document::{
        DocumentUpdate,
        ResolvedDocument,
    },
    knobs::MAX_TRANSACTION_WINDOW,
    runtime::block_in_place,
    types::{
        DatabaseIndexUpdate,
        RepeatableReason,
        RepeatableTimestamp,
        Timestamp,
        WriteTimestamp,
    },
};
use errors::ErrorMetadata;
use imbl::OrdMap;
use indexing::{
    backend_in_memory_indexes::BackendInMemoryIndexes,
    index_registry::IndexRegistry,
};
use search::TextIndexManager;
use value::{
    ResolvedDocumentId,
    TableMapping,
    TableName,
    TableNamespace,
    TabletId,
};
use vector::{
    DocInVectorIndex,
    VectorIndexManager,
};

use crate::{
    schema_registry::SchemaRegistry,
    table_registry::{
        TableUpdate,
        TableUpdateMode,
    },
    table_summary::{
        table_summary_bootstrapping_error,
        TableSummarySnapshot,
    },
    transaction::TableCountSnapshot,
    ComponentRegistry,
    TableRegistry,
    TableSummary,
    TransactionReadSet,
};

/// The [SnapshotManager] maintains multiple versions of the [Snapshot]s at
/// different timestamps. The snapshots internally use immutable data structures
/// with copy-on-write semantics to provide memory-efficient multiversioning.
///
/// We maintain a bounded time range of versions,
/// determined by `MAX_TRANSACTION_WINDOW`, allowing the `Database` layer to
/// begin a transaction in any timestamp within that range.
pub struct SnapshotManager {
    persisted_max_repeatable_ts: Timestamp,
    versions: VecDeque<(Timestamp, Snapshot)>,
}

#[derive(Clone)]
/// This is a wrapper on [TableSummarySnapshot] that is filtered to tables that
/// exist and tracks the user document and size counts.
pub struct TableSummaries {
    pub tables: OrdMap<TabletId, TableSummary>,
    pub num_user_documents: usize,
    pub user_size: usize,
}

#[async_trait]
impl TableCountSnapshot for Option<TableSummaries> {
    async fn count(&self, table: TabletId) -> anyhow::Result<Option<u64>> {
        let result = match self {
            Some(table_summaries) => {
                let count = table_summaries
                    .tables
                    .get(&table)
                    .map_or(0, |summary| summary.num_values() as u64);
                Some(count)
            },
            None => None,
        };
        Ok(result)
    }
}

impl TableSummaries {
    pub fn new(
        TableSummarySnapshot { tables, ts: _ }: TableSummarySnapshot,
        table_mapping: &TableMapping,
    ) -> Self {
        // Filter out non-existent tables before counting. Otherwise is_system_table
        // will return false and count non-existent tables toward user document counts.
        let tables: OrdMap<TabletId, TableSummary> = tables
            .into_iter()
            .filter(|(table_id, _table_summary)| table_mapping.tablet_id_exists(*table_id))
            .collect::<OrdMap<_, _>>();
        let (num_user_documents, user_size) = tables
            .iter()
            .filter(|(table_id, _summary)| !table_mapping.is_system_tablet(**table_id))
            .fold((0, 0), |(acc_docs, acc_size), (_, summary)| {
                (
                    acc_docs + summary.num_values(),
                    acc_size + summary.total_size(),
                )
            });
        Self {
            tables,
            num_user_documents,
            user_size,
        }
    }

    pub fn tablet_summary(&self, table: &TabletId) -> TableSummary {
        self.tables
            .get(table)
            .cloned()
            .unwrap_or(TableSummary::empty())
    }

    pub(crate) fn update(
        &mut self,
        document_id: ResolvedDocumentId,
        old: Option<&ResolvedDocument>,
        new: Option<&ResolvedDocument>,
        table_update: Option<&TableUpdate>,
        table_mapping: &TableMapping,
    ) -> anyhow::Result<()> {
        let mut table_summary = self
            .tables
            .get(&document_id.tablet_id)
            .ok_or_else(|| {
                anyhow::anyhow!("Updating non-existent table {}", document_id.tablet_id)
            })?
            .clone();
        if let Some(old_value) = old {
            table_summary = table_summary.remove(&old_value.value().0)?;
        }
        if let Some(new_value) = new {
            table_summary = table_summary.insert(&new_value.value().0);
        }
        if let Some(TableUpdate {
            namespace: _,
            table_id_and_number,
            table_name: _,
            state: _,
            mode,
        }) = table_update
        {
            match mode {
                TableUpdateMode::Create => {
                    assert!(self
                        .tables
                        .insert(table_id_and_number.tablet_id, TableSummary::empty())
                        .is_none());
                },
                TableUpdateMode::Activate => {},
                TableUpdateMode::Drop => {
                    self.tables.remove(&table_id_and_number.tablet_id);
                },
            }
        }
        let new_info_num_values = table_summary.num_values();
        let new_info_total_size = table_summary.total_size();
        match self.tables.insert(document_id.tablet_id, table_summary) {
            Some(old_summary) => {
                if !table_mapping.is_system_tablet(document_id.tablet_id) {
                    self.num_user_documents =
                        self.num_user_documents + new_info_num_values - old_summary.num_values();
                    self.user_size =
                        self.user_size + new_info_total_size - old_summary.total_size();
                }
            },
            None => panic!("Applying update for non-existent table!"),
        }
        Ok(())
    }
}
/// A snapshot of the database indexes and metadata at a certain timestamp.
#[derive(Clone)]
pub struct Snapshot {
    pub table_registry: TableRegistry,
    pub schema_registry: SchemaRegistry,
    pub component_registry: ComponentRegistry,
    pub table_summaries: Option<TableSummaries>,
    pub index_registry: IndexRegistry,
    pub in_memory_indexes: BackendInMemoryIndexes,
    pub text_indexes: TextIndexManager,
    pub vector_indexes: VectorIndexManager,
}

impl Snapshot {
    pub(crate) fn update(
        &mut self,
        document_update: &DocumentUpdate,
        commit_ts: Timestamp,
    ) -> anyhow::Result<(Vec<DatabaseIndexUpdate>, DocInVectorIndex)> {
        block_in_place(|| {
            let removal = document_update.old_document.as_ref();
            let insertion = document_update.new_document.as_ref();
            let document_id = document_update.id;
            let table_update = self
                .table_registry
                .update(
                    &self.index_registry,
                    document_id,
                    removal.map(|d| &d.value().0),
                    insertion.map(|d| &d.value().0),
                )
                .context("Table registry update failed")?;
            self.schema_registry.update(
                self.table_registry.table_mapping(),
                document_id,
                removal,
                insertion,
            )?;
            self.component_registry.update(
                self.table_registry.table_mapping(),
                document_id,
                removal,
                insertion,
            )?;
            if let Some(table_summaries) = self.table_summaries.as_mut() {
                table_summaries
                    .update(
                        document_id,
                        removal,
                        insertion,
                        table_update.as_ref(),
                        self.table_registry.table_mapping(),
                    )
                    .context("Table summaries update failed")?;
            };

            self.index_registry
                .update(removal, insertion)
                .context("Index update failed")?;
            let in_memory_index_updates = self.in_memory_indexes.update(
                &self.index_registry,
                commit_ts,
                removal.cloned(),
                insertion.cloned(),
            );

            self.text_indexes
                .update(
                    &self.index_registry,
                    removal,
                    insertion,
                    WriteTimestamp::Committed(commit_ts),
                )
                .context("Search index update failed")?;

            let doc_in_vector_index = self
                .vector_indexes
                .update(
                    &self.index_registry,
                    removal,
                    insertion,
                    WriteTimestamp::Committed(commit_ts),
                )
                .context("Vector index update failed")?;
            Ok((in_memory_index_updates, doc_in_vector_index))
        })
    }

    pub fn must_table_summaries(&self) -> anyhow::Result<&TableSummaries> {
        self.table_summaries
            .as_ref()
            .context(table_summary_bootstrapping_error(None))
    }

    pub fn iter_user_table_summaries(
        &self,
    ) -> anyhow::Result<impl Iterator<Item = ((TableNamespace, TableName), &'_ TableSummary)> + '_>
    {
        let result = self
            .must_table_summaries()?
            .tables
            .iter()
            .filter(|(table_id, _)| {
                matches!(
                    self.table_registry.tablet_states().get(table_id),
                    Some(TableState::Active)
                )
            })
            .map(|(table_id, summary)| {
                (
                    (
                        self.table_mapping()
                            .tablet_namespace(*table_id)
                            .expect("active table should have namespace"),
                        self.table_mapping()
                            .tablet_name(*table_id)
                            .expect("active table should have name"),
                    ),
                    summary,
                )
            })
            .filter(|((_, table_name), _)| !table_name.is_system());
        Ok(result)
    }

    pub fn table_mapping(&self) -> &TableMapping {
        self.table_registry.table_mapping()
    }

    pub fn table_summary(
        &self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> Option<TableSummary> {
        let table_id = match self.table_mapping().namespace(namespace).id(table) {
            Ok(table_id) => table_id,
            Err(_) => return Some(TableSummary::empty()),
        };
        let table_summaries = self.table_summaries.as_ref()?;
        Some(table_summaries.tablet_summary(&table_id.tablet_id))
    }

    pub fn must_table_summary(
        &self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<TableSummary> {
        self.table_summary(namespace, table)
            .context(table_summary_bootstrapping_error(None))
    }

    pub fn get_user_document_and_index_storage(
        &self,
    ) -> anyhow::Result<BTreeMap<(TableNamespace, TableName), (usize, usize)>> {
        let table_mapping = self.table_mapping().clone();

        let mut document_storage_by_table = BTreeMap::new();
        for (table_name, summary) in self.iter_user_table_summaries()? {
            let table_size = summary.total_size();
            document_storage_by_table.insert(table_name, (table_size, 0));
        }

        // TODO: We are currently using document size * index count as a rough
        // approximation for how much storage indexes use, but we should fix this to
        // only charge for the fields that are indexed.
        for index in self.index_registry.all_enabled_indexes() {
            // Only count storage for active tables, and avoid an error below
            // where `document_storage_by_table` is missing hidden tables.
            if !table_mapping.is_active(*index.name.table()) {
                continue;
            }
            let table_namespace = table_mapping.tablet_namespace(*index.name.table())?;
            let index_name = index
                .name
                .clone()
                .map_table(&table_mapping.tablet_to_name())
                .unwrap();
            let table_name = index_name.table().clone();

            if !index_name.is_system_owned() {
                let (document_size, total_index_size) = *document_storage_by_table
                    .get(&(table_namespace, table_name.clone()))
                    .with_context(|| {
                        format!(
                            "Index {index_name} on a nonexistent table {table_name} in namespace \
                             {:?}",
                            table_namespace
                        )
                    })?;
                document_storage_by_table.insert(
                    (table_namespace, table_name),
                    (document_size, total_index_size + document_size),
                );
            }
        }

        Ok(document_storage_by_table)
    }

    pub fn component_ids_to_paths(&self) -> BTreeMap<ComponentId, ComponentPath> {
        self.component_registry
            .all_component_paths(&mut TransactionReadSet::new())
    }
}

impl SnapshotManager {
    pub fn new(initial_ts: Timestamp, initial_snapshot: Snapshot) -> Self {
        let mut versions = VecDeque::new();
        versions.push_back((initial_ts, initial_snapshot));
        Self {
            versions,
            persisted_max_repeatable_ts: initial_ts,
        }
    }

    pub fn latest(&self) -> (RepeatableTimestamp, Snapshot) {
        let (ts, snapshot) = self
            .versions
            .back()
            .cloned()
            .expect("snapshot versions empty");
        (
            RepeatableTimestamp::new_validated(ts, RepeatableReason::SnapshotManagerLatest),
            snapshot,
        )
    }

    pub fn latest_snapshot(&self) -> Snapshot {
        let (_, snapshot) = self.latest();
        snapshot
    }

    fn earliest_ts(&self) -> Timestamp {
        self.versions
            .front()
            .map(|(ts, ..)| *ts)
            .expect("snapshot versions empty")
    }

    /// latest_ts has been committed to persistence and no earlier
    /// timestamps may be used for future commits, so it is safe to read from
    /// this snapshot.
    pub fn latest_ts(&self) -> RepeatableTimestamp {
        let ts = self
            .versions
            .back()
            .map(|(ts, ..)| *ts)
            .expect("snapshot versions empty");
        RepeatableTimestamp::new_validated(ts, RepeatableReason::SnapshotManagerLatest)
    }

    /// While latest_ts has been part of some commit and the backend process is
    /// aware that it is repeatable, other processes like db-verifier
    /// may not know yet that it is safe to read from that snapshot.
    ///
    /// In contrast, persisted_max_repeatable_ts has been written as
    /// max_repeatable_ts to persistence, so every process knows it is safe
    /// to read from.
    ///
    /// Retention guarantees that min_snapshot_ts <=
    /// persisted_max_repeatable_ts.
    ///
    /// If Retention only ensured that min_snapshot_ts <= latest_ts, then
    /// we might persist some min_snapshot_ts > persisted_max_repeatable_ts.
    /// Then other processes (that don't have access to latest_ts) would not
    /// be able to read from any snapshot safely.
    pub fn persisted_max_repeatable_ts(&self) -> RepeatableTimestamp {
        self.latest_ts()
            .prior_ts(self.persisted_max_repeatable_ts)
            .expect(
                "persisted_max_repeatable_ts is always bumped after latest_ts and both are \
                 monotonic",
            )
    }

    // Note that timestamp must be within MAX_TRANSACTION_WINDOW from the last
    // transaction
    pub fn snapshot(&self, ts: Timestamp) -> anyhow::Result<Snapshot> {
        if ts < self.earliest_ts() {
            return Err(
                anyhow::anyhow!(ErrorMetadata::operational_internal_server_error()).context(
                    format!("Timestamp {ts} is too early, retry with a higher timestamp"),
                ),
            );
        }
        anyhow::ensure!(
            ts <= *self.latest_ts(),
            "Timestamp {ts} is more recent than latest_ts {}",
            self.latest_ts(),
        );
        let i = match self.versions.binary_search_by_key(&ts, |&(ts, ..)| ts) {
            Ok(i) => i,
            // This is the index where insertion would preserve sorted order. That is, it's the
            // first index that has a timestamp greater than `ts`. So, we'd like the immediately
            // preceding element.
            Err(i) => i.checked_sub(1).unwrap(),
        };
        let (_, snapshot) = &self.versions[i];
        Ok(snapshot.clone())
    }

    /// Overwrites the state of the latest snapshot to include the given set of
    /// backfilled in memory vector changes.
    ///
    /// This is a bit sketchy. We want asynchronously load data that lives in
    /// the snapshot. These asynchronous loads are not data mutations, so we
    /// don't want to advance the timestamp. That leaves us with mutating
    /// the existing snapshot, which means a transaction at a given
    /// timestamp may fail initially, then succeed later when our async load
    /// completes.
    ///
    /// This pattern works as long as transactions that depend on the data that
    /// we're amending treat the failures while that data is loading as
    /// transient system errors (timeouts, database issues etc) and retry.
    pub fn overwrite_last_snapshot_text_and_vector_indexes(
        &mut self,
        text_indexes: TextIndexManager,
        vector_indexes: VectorIndexManager,
    ) {
        let (_ts, ref mut snapshot) = self.versions.back_mut().expect("snapshot versions empty");
        snapshot.text_indexes = text_indexes;
        snapshot.vector_indexes = vector_indexes;
    }

    pub fn overwrite_last_snapshot_table_summary(&mut self, table_summary: TableSummarySnapshot) {
        let (_ts, ref mut snapshot) = self.versions.back_mut().expect("snapshot versions empty");
        let table_mapping = snapshot.table_mapping();
        let table_summaries = TableSummaries::new(table_summary, table_mapping);
        snapshot.table_summaries = Some(table_summaries);
    }

    /// Overwrites the in-memory indexes for the latest snapshot.
    ///
    /// This is a bit sketchy but it allows us to asynchronously load indexes
    /// into memory instead of blocking startup. This model also helps us
    /// correctly layer application specific system tables that the database
    /// depend on.
    ///
    /// This pattern works since the transaction fallbacks to reading from the
    /// database if the in-memory indexes are not present. Thus they have no
    /// effect other than performance.
    pub fn overwrite_last_snapshot_in_memory_indexes(
        &mut self,
        in_memory_indexes: BackendInMemoryIndexes,
    ) {
        let (_ts, ref mut snapshot) = self.versions.back_mut().expect("snapshot versions empty");
        snapshot.in_memory_indexes = in_memory_indexes;
    }

    pub fn push(&mut self, ts: Timestamp, snapshot: Snapshot) {
        assert!(*self.latest_ts() < ts);
        while self.versions.len() > 1 && (ts - self.earliest_ts()) > *MAX_TRANSACTION_WINDOW {
            self.versions.pop_front();
        }
        self.versions.push_back((ts, snapshot));
    }

    pub fn bump_persisted_max_repeatable_ts(&mut self, ts: Timestamp) -> bool {
        let (latest_ts, snapshot) = self.latest();
        if ts > *latest_ts {
            self.push(ts, snapshot);
            self.persisted_max_repeatable_ts = ts;
            true
        } else {
            false
        }
    }
}
