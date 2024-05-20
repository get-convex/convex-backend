#[cfg(any(test, feature = "testing"))]
use std::fmt::Debug;
use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    mem,
    ops::Deref,
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::{
        index::{
            database_index::IndexedFields,
            IndexMetadata,
            INDEX_TABLE,
        },
        tables::{
            TableMetadata,
            TABLES_TABLE,
        },
    },
    document::{
        CreationTime,
        DocumentUpdate,
        ResolvedDocument,
    },
    identity::InertIdentity,
    index::{
        IndexKey,
        IndexKeyBytes,
    },
    interval::Interval,
    knobs::{
        SEARCH_INDEX_SIZE_HARD_LIMIT,
        VECTOR_INDEX_SIZE_HARD_LIMIT,
    },
    persistence::RetentionValidator,
    query::{
        CursorPosition,
        Order,
        Search,
        SearchVersion,
    },
    runtime::Runtime,
    sync::split_rw_lock::Reader,
    types::{
        GenericIndexName,
        IndexId,
        IndexName,
        PersistenceVersion,
        RepeatableTimestamp,
        StableIndexName,
        TableName,
        TableStats,
        TabletIndexName,
        WriteTimestamp,
    },
    value::{
        id_v6::DeveloperDocumentId,
        ConvexObject,
        ResolvedDocumentId,
        Size,
        TableMapping,
        VirtualTableMapping,
    },
    version::Version,
};
use errors::ErrorMetadata;
use indexing::backend_in_memory_indexes::{
    BatchKey,
    RangeRequest,
};
use keybroker::{
    Identity,
    UserIdentityAttributes,
};
use maplit::btreemap;
use search::CandidateRevision;
use sync_types::{
    AuthenticationToken,
    Timestamp,
};
use usage_tracking::FunctionUsageTracker;
use value::{
    TableNamespace,
    TableNumber,
    TabletId,
};

use crate::{
    bootstrap_model::{
        defaults::BootstrapTableIds,
        table::{
            NUM_RESERVED_LEGACY_TABLE_NUMBERS,
            NUM_RESERVED_SYSTEM_TABLE_NUMBERS,
        },
    },
    committer::table_dependency_sort_key,
    execution_size::FunctionExecutionSize,
    metrics,
    patch::PatchValue,
    preloaded::PreloadedIndexRange,
    query::{
        IndexRangeResponse,
        TableFilter,
    },
    reads::TransactionReadSet,
    snapshot_manager::{
        Snapshot,
        SnapshotManager,
    },
    token::Token,
    transaction_id_generator::TransactionIdGenerator,
    transaction_index::TransactionIndex,
    virtual_tables::VirtualSystemMapping,
    write_limits::BiggestDocumentWrites,
    writes::{
        TransactionWriteSize,
        Writes,
    },
    IndexModel,
    ReadSet,
    SchemaModel,
    SystemMetadataModel,
    TableModel,
    TableRegistry,
    VirtualTableMetadata,
    VIRTUAL_TABLES_TABLE,
};

/// Safe default number of items to return for each list or filter operation
/// when we're writing internal code and don't know what other value to choose.
pub const DEFAULT_PAGE_SIZE: usize = 512;

pub const MAX_PAGE_SIZE: usize = 1024;
pub struct Transaction<RT: Runtime> {
    pub(crate) identity: Identity,
    pub(crate) id_generator: TransactionIdGenerator,

    pub(crate) next_creation_time: CreationTime,

    // Size of any functions scheduled from this transaction.
    pub scheduled_size: TransactionWriteSize,

    pub(crate) reads: TransactionReadSet,
    pub(crate) writes: Writes,

    pub(crate) index: TransactionIndex,
    pub(crate) metadata: TableRegistry,
    pub(crate) count_snapshot: Arc<dyn TableCountSnapshot>,
    /// The change in the number of documents in table that have had writes in
    /// this transaction. If there is no entry for a table, assume deltas
    /// are zero.
    pub(crate) table_count_deltas: BTreeMap<TabletId, i64>,

    pub(crate) stats: BTreeMap<TableNumber, TableStats>,

    pub(crate) retention_validator: Arc<dyn RetentionValidator>,

    pub(crate) runtime: RT,

    pub usage_tracker: FunctionUsageTracker,
    pub(crate) virtual_system_mapping: VirtualSystemMapping,

    #[cfg(any(test, feature = "testing"))]
    index_size_override: Option<usize>,
}

#[cfg(any(test, feature = "testing"))]
impl<RT: Runtime> Debug for Transaction<RT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction").finish()
    }
}

#[async_trait]
pub trait TableCountSnapshot: Send + Sync + 'static {
    /// Returns the number of documents in the table at the timestamp of the
    /// snapshot.
    async fn count(&self, table: TabletId) -> anyhow::Result<u64>;
}

impl<RT: Runtime> Transaction<RT> {
    pub fn new(
        identity: Identity,
        id_generator: TransactionIdGenerator,
        creation_time: CreationTime,
        index: TransactionIndex,
        metadata: TableRegistry,
        count: Arc<dyn TableCountSnapshot>,
        runtime: RT,
        usage_tracker: FunctionUsageTracker,
        retention_validator: Arc<dyn RetentionValidator>,
        virtual_system_mapping: VirtualSystemMapping,
    ) -> Self {
        Self {
            identity,
            reads: TransactionReadSet::new(),
            writes: Writes::new(),
            id_generator,
            next_creation_time: creation_time,
            scheduled_size: TransactionWriteSize::default(),
            index,
            metadata,
            count_snapshot: count,
            table_count_deltas: BTreeMap::new(),
            stats: BTreeMap::new(),
            runtime,
            retention_validator,
            usage_tracker,
            virtual_system_mapping,
            #[cfg(any(test, feature = "testing"))]
            index_size_override: None,
        }
    }

    pub fn persistence_version(&self) -> PersistenceVersion {
        self.index.index_registry().persistence_version()
    }

    pub fn table_mapping(&mut self) -> &TableMapping {
        self.take_table_mapping_dep();
        self.metadata.table_mapping()
    }

    pub fn virtual_table_mapping(&self) -> &VirtualTableMapping {
        self.metadata.virtual_table_mapping()
    }

    pub fn virtual_system_mapping(&self) -> &VirtualSystemMapping {
        &self.virtual_system_mapping
    }

    /// Checks both virtual tables and tables to get the table number to name
    /// mapping. If table is excluded by `table_filter`, returns error as if
    /// the table doesn't exist.
    pub fn all_tables_number_to_name(
        &mut self,
        table_filter: TableFilter,
    ) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        let table_mapping = self.table_mapping().clone();
        let virtual_table_mapping = self.virtual_table_mapping().clone();
        move |number| {
            if let Some(name) = virtual_table_mapping.name_if_exists(number) {
                Ok(name)
            } else {
                let name = table_mapping
                    .namespace(TableNamespace::Global)
                    .number_to_name()(number)?;
                match table_filter {
                    TableFilter::IncludePrivateSystemTables => {},
                    TableFilter::ExcludePrivateSystemTables => {
                        anyhow::ensure!(!name.is_system(), "cannot find table {number:?}");
                    },
                }
                Ok(name)
            }
        }
    }

    pub fn bootstrap_tables(&mut self) -> BootstrapTableIds {
        BootstrapTableIds::new(self.table_mapping())
    }

    pub fn resolve_idv6(
        &mut self,
        id: DeveloperDocumentId,
        table_filter: TableFilter,
    ) -> anyhow::Result<TableName> {
        match self.all_tables_number_to_name(table_filter)(*id.table()) {
            Ok(table_name) => Ok(table_name),
            Err(_) => anyhow::bail!("Table for ID \"{}\" not found", id.encode()),
        }
    }

    pub fn runtime(&self) -> &RT {
        &self.runtime
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn inert_identity(&self) -> InertIdentity {
        self.identity.clone().into()
    }

    pub fn user_identity(&self) -> Option<UserIdentityAttributes> {
        match self.identity.clone() {
            Identity::User(identity) => Some(identity.attributes),
            Identity::ActingUser(_, identity) => Some(identity),
            _ => None,
        }
    }

    pub fn authentication_token(&self) -> AuthenticationToken {
        self.identity.clone().into()
    }

    pub fn begin_timestamp(&self) -> RepeatableTimestamp {
        self.index.base_snapshot().timestamp()
    }

    pub fn is_readonly(&self) -> bool {
        self.writes.is_empty()
    }

    pub fn writes(&self) -> &Writes {
        &self.writes
    }

    pub fn into_reads_and_writes(self) -> (TransactionReadSet, Writes) {
        (self.reads, self.writes)
    }

    pub fn biggest_document_writes(&self) -> Option<BiggestDocumentWrites> {
        let mut max_size = 0;
        let mut biggest_document_id = None;
        let mut max_nesting = 0;
        let mut most_nested_document_id = None;
        for (document_id, DocumentUpdate { new_document, .. }) in self.writes.coalesced_writes() {
            let (size, nesting) = new_document
                .as_ref()
                .map(|document| (document.value().size(), document.value().nesting()))
                .unwrap_or((0, 0));
            if size > max_size {
                max_size = size;
                biggest_document_id = Some(document_id);
            }
            if nesting > max_nesting {
                max_nesting = nesting;
                most_nested_document_id = Some(document_id);
            }
        }
        if let Some(biggest_document_id) = biggest_document_id
            && let Some(most_nested_document_id) = most_nested_document_id
        {
            Some(BiggestDocumentWrites {
                max_size: ((*biggest_document_id).into(), max_size),
                max_nesting: ((*most_nested_document_id).into(), max_nesting),
            })
        } else {
            None
        }
    }

    pub fn execution_size(&self) -> FunctionExecutionSize {
        FunctionExecutionSize {
            num_intervals: self.reads.num_intervals(),
            read_size: self.reads.user_tx_size().to_owned(),
            write_size: self.writes.user_size().to_owned(),
            scheduled_size: self.scheduled_size.clone(),
        }
    }

    /// Applies the reads and writes from FunctionRunner to the Transaction.
    #[minitrace::trace]
    pub fn apply_function_runner_tx(
        &mut self,
        begin_timestamp: Timestamp,
        reads: ReadSet,
        num_intervals: usize,
        user_tx_size: crate::reads::TransactionReadSize,
        system_tx_size: crate::reads::TransactionReadSize,
        updates: BTreeMap<ResolvedDocumentId, DocumentUpdate>,
        generated_ids: BTreeSet<ResolvedDocumentId>,
        rows_read: BTreeMap<TableNumber, u64>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            *self.begin_timestamp() == begin_timestamp,
            "Timestamp mismatch"
        );

        self.reads
            .merge(reads, num_intervals, user_tx_size, system_tx_size);

        self.merge_writes(updates, generated_ids)?;

        for (table, rows_read) in rows_read {
            self.stats.entry(table).or_default().rows_read += rows_read;
        }

        Ok(())
    }

    // Checks that if this transaction already has some writes, they are included
    // in the given `updates`. This means the passed in `updates` are a superset
    // of the existing `updates` on this transaction, and that the merged-in
    // writes cannot modify documents already written to in this transaction.
    // In most scenarios this transaction will have no writes.
    pub fn merge_writes(
        &mut self,
        updates: BTreeMap<ResolvedDocumentId, DocumentUpdate>,
        // TODO: Delete generated_ids, they are included as (None, None)
        // update in updates.
        generated_ids: BTreeSet<ResolvedDocumentId>,
    ) -> anyhow::Result<()> {
        let (existing_updates, existing_generated_ids) =
            self.writes().clone().into_updates_and_generated_ids();

        // TODO: Delete generated_ids, they are included as (None, None)
        // update in updates. This check is redundant.
        for id in generated_ids.iter() {
            anyhow::ensure!(
                existing_updates.contains_key(id) || !existing_generated_ids.contains(id),
                "Conflicting generated ID {id}"
            );
        }

        let mut updates = updates.into_iter().collect::<Vec<_>>();
        updates.sort_by_key(|(id, update)| {
            table_dependency_sort_key(
                self.bootstrap_tables(),
                (*id).into(),
                update.new_document.as_ref(),
            )
        });

        let mut preserved_update_count = 0;
        for (id, update) in updates {
            // Ensure that the existing update matches, and that
            // that the merged-in writes didn't otherwise modify documents
            // already written to in this transaction.
            if let Some(existing_update) = existing_updates.get(&id) {
                anyhow::ensure!(
                    existing_update == &update,
                    "Conflicting updates for document {id}"
                );
                preserved_update_count += 1;
                continue;
            }

            if let Some(ref document) = update.new_document {
                let doc_creation_time = document
                    .creation_time()
                    .context("Insert must have a creation time")?;
                if doc_creation_time >= self.next_creation_time {
                    self.next_creation_time = doc_creation_time;
                    self.next_creation_time.increment()?;
                }
            }
            self.apply_validated_write(id, update.old_document, update.new_document)?;
        }
        assert_eq!(
            preserved_update_count,
            existing_updates.len(),
            "Existing write was not preserved"
        );

        Ok(())
    }

    /// Return the document with the given `id` or None if document doesn't
    /// exist.
    pub async fn get(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<ResolvedDocument>> {
        Ok(self.get_with_ts(id).await?.map(|(doc, _)| doc))
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn get_with_ts(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<(ResolvedDocument, WriteTimestamp)>> {
        let table_name = match self.table_mapping().tablet_name(id.table().tablet_id) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        if self.virtual_system_mapping().is_virtual_table(&table_name) {
            anyhow::bail!("Virtual tables should use UserFacingModel::get_with_ts");
        }
        self.get_inner(id, table_name).await
    }

    #[convex_macro::instrument_future]
    pub(crate) async fn patch_inner(
        &mut self,
        id: ResolvedDocumentId,
        value: PatchValue,
    ) -> anyhow::Result<ResolvedDocument> {
        let table_name = self.table_mapping().tablet_name(id.table().tablet_id)?;

        let (old_document, _) =
            self.get_inner(id, table_name.clone())
                .await?
                .context(ErrorMetadata::bad_request(
                    "NonexistentDocument",
                    format!("Update on nonexistent document ID {id}"),
                ))?;

        let new_document = {
            let patched_value = value
                .clone()
                .apply(old_document.value().clone().into_value())?;
            old_document.replace_value(patched_value)?
        };
        SchemaModel::new(self).enforce(&new_document).await?;

        self.apply_validated_write(id, Some(old_document), Some(new_document.clone()))?;
        Ok(new_document)
    }

    pub fn is_system(&mut self, table_number: TableNumber) -> bool {
        self.table_mapping()
            .namespace(TableNamespace::Global)
            .is_system(table_number)
            || self.virtual_table_mapping().number_exists(&table_number)
    }

    #[convex_macro::instrument_future]
    pub(crate) async fn replace_inner(
        &mut self,
        id: ResolvedDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        let table_name = self.table_mapping().tablet_name(id.table().tablet_id)?;
        let (old_document, _) =
            self.get_inner(id, table_name)
                .await?
                .context(ErrorMetadata::bad_request(
                    "NonexistentDocument",
                    format!("Replace on nonexistent document ID {id}"),
                ))?;

        // Replace document.
        let new_document = old_document.replace_value(value)?;

        SchemaModel::new(self).enforce(&new_document).await?;

        self.apply_validated_write(
            new_document.id(),
            Some(old_document),
            Some(new_document.clone()),
        )?;
        Ok(new_document)
    }

    #[convex_macro::instrument_future]
    pub async fn delete_inner(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocument> {
        let table_name = self.table_mapping().tablet_name(id.table().tablet_id)?;
        let (document, _) =
            self.get_inner(id, table_name)
                .await?
                .context(ErrorMetadata::bad_request(
                    "NonexistentDocument",
                    format!("Delete on nonexistent document ID {id}"),
                ))?;

        self.apply_validated_write(document.id(), Some(document.clone()), None)?;
        Ok(document)
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn count(&mut self, table: &TableName) -> anyhow::Result<u64> {
        let virtual_system_mapping = self.virtual_system_mapping().clone();
        let system_table = if virtual_system_mapping.is_virtual_table(table) {
            virtual_system_mapping.virtual_to_system_table(table)?
        } else {
            table
        };
        TableModel::new(self).count(system_table).await
    }

    pub fn into_token(self) -> anyhow::Result<Token> {
        if !self.is_readonly() {
            anyhow::bail!("Transaction isn't readonly");
        }
        metrics::log_read_tx(&self);
        let ts = *self.begin_timestamp();
        Ok(Token::new(self.reads.into_read_set(), ts))
    }

    pub fn take_stats(&mut self) -> BTreeMap<TableName, TableStats> {
        let stats = mem::take(&mut self.stats);
        stats
            .into_iter()
            .map(|(table, stats)| {
                (
                    self.table_mapping()
                        .namespace(TableNamespace::Global)
                        .number_to_name()(table)
                    .expect("table should exist"),
                    stats,
                )
            })
            .collect()
    }

    pub fn stats(&self) -> &BTreeMap<TableNumber, TableStats> {
        &self.stats
    }

    fn take_table_mapping_dep(&mut self) {
        let tables_by_id = TabletIndexName::by_id(
            self.metadata
                .table_mapping()
                .namespace(TableNamespace::Global)
                .id(&TABLES_TABLE)
                .expect("_tables should exist")
                .tablet_id,
        );
        self.reads
            .record_indexed_derived(tables_by_id, IndexedFields::by_id(), Interval::all());
    }

    // XXX move to table model?
    #[cfg(any(test, feature = "testing"))]
    pub async fn create_system_table_testing(
        &mut self,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<bool> {
        self.create_system_table(table_name, default_table_number)
            .await
    }

    async fn table_number_for_system_table(
        &mut self,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<TableNumber> {
        Ok(if let Some(default_table_number) = default_table_number {
            let table_number = if self
                .table_mapping()
                .namespace(TableNamespace::Global)
                .table_number_exists()(default_table_number)
            {
                // In tests, have a hard failure on conflicting default table numbers. In
                // real system, have a looser fallback where we pick
                // another table number.
                if cfg!(any(test, feature = "testing")) {
                    let table_mapping = self.table_mapping().namespace(TableNamespace::Global);
                    let existing_tn = table_mapping.name_by_number_if_exists(default_table_number);
                    anyhow::bail!(
                        "{default_table_number} is used by both {table_name} and {existing_tn:?}"
                    );
                }

                // If the table_number requested is taken, just pick a higher table number.
                // This might be true for older backends that have lower-numbered system
                // tables.
                TableModel::new(self).next_system_table_number().await?
            } else {
                default_table_number
            };
            anyhow::ensure!(
                table_number < TableNumber::try_from(NUM_RESERVED_SYSTEM_TABLE_NUMBERS)?,
                "{table_number} picked for system table {table_name} is reserved for user tables"
            );
            anyhow::ensure!(
                table_number >= TableNumber::try_from(NUM_RESERVED_LEGACY_TABLE_NUMBERS)?,
                "{table_number} picked for system table {table_name} is reserved for legacy tables"
            );
            table_number
        } else {
            TableModel::new(self).next_system_table_number().await?
        })
    }

    /// Creates a new system table, with _id and _creationTime indexes, returns
    /// false if table already existed
    pub async fn create_system_table(
        &mut self,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<bool> {
        anyhow::ensure!(self.identity().is_system());
        anyhow::ensure!(
            table_name.is_system(),
            "{table_name:?} is not a valid system table name!"
        );

        let is_new = !TableModel::new(self).table_exists(table_name);
        if is_new {
            let table_number = self
                .table_number_for_system_table(table_name, default_table_number)
                .await?;
            let metadata = TableMetadata::new(table_name.clone(), table_number);
            let table_doc_id = SystemMetadataModel::new(self)
                .insert(&TABLES_TABLE, metadata.try_into()?)
                .await?;
            let tablet_id = TabletId(table_doc_id.internal_id());

            let by_id_index = IndexMetadata::new_enabled(
                GenericIndexName::by_id(tablet_id),
                IndexedFields::by_id(),
            );
            SystemMetadataModel::new(self)
                .insert(&INDEX_TABLE, by_id_index.try_into()?)
                .await?;
            let metadata = IndexMetadata::new_enabled(
                GenericIndexName::by_creation_time(tablet_id),
                IndexedFields::creation_time(),
            );
            SystemMetadataModel::new(self)
                .insert(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            tracing::info!("Created system table: {table_name}");
        } else {
            tracing::debug!("Skipped creating system table {table_name} since it already exists");
        };
        Ok(is_new)
    }

    /// Creates a new virtual table, returns false if table already existed
    pub async fn create_virtual_table(
        &mut self,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<bool> {
        anyhow::ensure!(self.identity().is_system());

        anyhow::ensure!(
            table_name.is_valid_virtual(),
            "{table_name:?} is not a valid virtual table name!"
        );

        let is_new = !self.virtual_table_mapping().name_exists(table_name);
        if is_new {
            let table_number = self
                .table_number_for_system_table(table_name, default_table_number)
                .await?;
            let metadata = VirtualTableMetadata::new(table_name.clone(), table_number);
            let table_doc_id = SystemMetadataModel::new(self)
                .insert(&VIRTUAL_TABLES_TABLE, metadata.try_into()?)
                .await?;
            tracing::info!("Created virtual table: {table_name} with doc_id {table_doc_id}");
        } else {
            tracing::debug!("Skipped creating virtual table {table_name} since it already exists");
        };
        Ok(is_new)
    }
}

// Private methods for `Transaction`: Place all authorization checks closer to
// the public interface.
impl<RT: Runtime> Transaction<RT> {
    pub(crate) async fn get_inner(
        &mut self,
        id: ResolvedDocumentId,
        table_name: TableName,
    ) -> anyhow::Result<Option<(ResolvedDocument, WriteTimestamp)>> {
        let mut batch_result = self
            .get_inner_batch(btreemap! {
                0 => (id, table_name),
            })
            .await;
        batch_result
            .remove(&0)
            .context("get_inner_batch missing batch key")?
    }

    pub(crate) async fn get_inner_batch(
        &mut self,
        ids: BTreeMap<BatchKey, (ResolvedDocumentId, TableName)>,
    ) -> BTreeMap<BatchKey, anyhow::Result<Option<(ResolvedDocument, WriteTimestamp)>>> {
        let mut ranges = BTreeMap::new();
        let batch_size = ids.len();
        for (batch_key, (id, table_name)) in ids.iter() {
            let index_name = TabletIndexName::by_id(id.table().tablet_id);
            let printable_index_name = IndexName::by_id(table_name.clone());
            let index_key = IndexKey::new(vec![], (*id).into());
            let interval = Interval::prefix(index_key.into_bytes().into());
            ranges.insert(
                *batch_key,
                RangeRequest {
                    index_name,
                    printable_index_name,
                    interval,
                    order: Order::Asc,
                    // Request 2 to best-effort verify uniqueness of by_id index.
                    max_size: 2,
                },
            );
        }

        let mut results = self.index.range_batch(&mut self.reads, ranges).await;
        let mut batch_result = BTreeMap::new();
        for (batch_key, (id, table_name)) in ids {
            let result: anyhow::Result<_> = try {
                let IndexRangeResponse {
                    page: range_results,
                    cursor,
                } = results.remove(&batch_key).context("expected result")??;
                if range_results.len() > 1 {
                    Err(anyhow::anyhow!("Got multiple values for id {id:?}"))?;
                }
                if !matches!(cursor, CursorPosition::End) {
                    Err(anyhow::anyhow!(
                        "Querying 2 items for a single id didn't exhaust interval for {id:?}"
                    ))?;
                }
                match range_results.into_iter().next() {
                    Some((_, doc, timestamp)) => {
                        let is_virtual_table =
                            self.virtual_table_mapping().name_exists(&table_name);
                        self.reads.record_read_document(
                            table_name,
                            doc.size(),
                            &self.usage_tracker,
                            is_virtual_table,
                        )?;
                        assert!(batch_result
                            .insert(batch_key, Ok(Some((doc, timestamp))))
                            .is_none());
                    },
                    None => {
                        assert!(batch_result.insert(batch_key, Ok(None)).is_none());
                    },
                }
                self.stats
                    .entry(id.table().table_number)
                    .or_default()
                    .rows_read += 1;
            };
            if let Err(e) = result {
                assert!(batch_result.insert(batch_key, Err(e)).is_none());
            }
        }
        assert_eq!(batch_result.len(), batch_size);
        batch_result
    }

    /// Apply a validated write to the [Transaction], updating the
    /// [IndexRegistry] and [TableRegistry]. Validated means the write
    /// has already been checked for schema enforcement.
    pub(crate) fn apply_validated_write(
        &mut self,
        id: ResolvedDocumentId,
        old_document: Option<ResolvedDocument>,
        new_document: Option<ResolvedDocument>,
    ) -> anyhow::Result<()> {
        // Implement something like two-phase commit between the index and the document
        // store. We first guarantee that the changes are valid for the index and
        // metadata and then let inserting into writes the commit
        // point so that the Transaction is never in an inconsistent state.
        let is_system_document = self
            .table_mapping()
            .namespace(TableNamespace::Global)
            .is_system(id.table().table_number);
        let bootstrap_tables = self.bootstrap_tables();
        let index_update = self
            .index
            .begin_update(old_document.clone(), new_document.clone())?;
        let metadata_update = self.metadata.begin_update(
            index_update.registry(),
            id,
            old_document.as_ref().map(|d| d.value().deref()),
            new_document.as_ref().map(|d| d.value().deref()),
        )?;
        let stats = self.stats.entry(id.table().table_number).or_default();
        let mut delta = 0;
        match (old_document.as_ref(), new_document.as_ref()) {
            (None, None) => {
                stats.rows_deleted += 1;
                stats.rows_created += 1;
            },
            (Some(_previous), None) => {
                // Delete
                stats.rows_deleted += 1;
                delta = -1;
            },
            (old, Some(_new)) => {
                // Insert or replace
                if old.is_none() {
                    // Insert
                    stats.rows_created += 1;
                    delta = 1;
                }
            },
        };
        // NB: Writes::update is fallible, so be sure to call it before applying the
        // index and metadata updates.
        self.writes.update(
            bootstrap_tables,
            is_system_document,
            &mut self.reads,
            id,
            DocumentUpdate {
                id,
                old_document,
                new_document,
            },
        )?;
        stats.rows_written += 1;

        index_update.apply();
        metadata_update.apply();

        *self
            .table_count_deltas
            .entry(id.table().tablet_id)
            .or_default() += delta;
        Ok(())
    }

    pub(crate) async fn insert_document(
        &mut self,
        document: ResolvedDocument,
    ) -> anyhow::Result<ResolvedDocumentId> {
        SchemaModel::new(self).enforce(&document).await?;
        let document_id = document.id();
        self.apply_validated_write(document_id, None, Some(document))?;
        Ok(document_id)
    }

    pub async fn search(
        &mut self,
        stable_index_name: &StableIndexName,
        search: &Search,
        version: SearchVersion,
    ) -> anyhow::Result<Vec<(CandidateRevision, IndexKeyBytes)>> {
        let Some(tablet_index_name) = stable_index_name.tablet_index_name() else {
            return Ok(vec![]);
        };
        let search = search.clone().to_internal(tablet_index_name.clone())?;
        self.index
            .search(&mut self.reads, &search, tablet_index_name.clone(), version)
            .await
    }

    // TODO(lee) Make this private.
    // We ideally want the transaction to call this internally so caller doesn't
    // have to call this. However, this is currently hard since the query layer
    // doesn't persist a stream.
    pub fn record_read_document(
        &mut self,
        document: &ResolvedDocument,
        table_name: &TableName,
    ) -> anyhow::Result<()> {
        let is_virtual_table = self.virtual_system_mapping().is_virtual_table(table_name);
        self.reads.record_read_document(
            table_name.clone(),
            document.size(),
            &self.usage_tracker,
            is_virtual_table,
        )
    }

    // Preload an index range against the transaction, building a snapshot of
    // all its current values for future use. Preloading has a few limitations:
    //
    //   - It doesn't reflect subsequent updates to the index.
    //   - It currently only supports unique database indexes with a single indexed
    //     field.
    //   - It loads the entirety of the index into memory.
    //   - The preloaded snapshot only permits point queries on the index key.
    //
    pub async fn preload_index_range(
        &mut self,
        index_name: &IndexName,
        interval: &Interval,
    ) -> anyhow::Result<PreloadedIndexRange> {
        let stable_index_name = IndexModel::new(self)
            .stable_index_name(index_name, TableFilter::IncludePrivateSystemTables)?;
        let StableIndexName::Physical(tablet_index_name) = stable_index_name else {
            anyhow::bail!(
                "Can only preload ranges on physical tables. Failed for index: {index_name} with \
                 {stable_index_name:?}"
            );
        };
        self.index
            .preload_index_range(&mut self.reads, &tablet_index_name, index_name, interval)
            .await
    }

    fn start_index_range(&mut self, request: IndexRangeRequest) -> anyhow::Result<RangeResponse> {
        if request.interval.is_empty() {
            return Ok(RangeResponse::Ready(IndexRangeResponse {
                page: vec![],
                cursor: CursorPosition::End,
            }));
        }
        let tablet_index_name = match request.stable_index_name {
            StableIndexName::Physical(tablet_index_name) => tablet_index_name,
            StableIndexName::Virtual(..) => {
                anyhow::bail!(
                    "Can't query virtual tables from index_range (use \
                     UserFacingModel::index_range instead)"
                );
            },
            StableIndexName::Missing => {
                return Ok(RangeResponse::Ready(IndexRangeResponse {
                    page: vec![],
                    cursor: CursorPosition::End,
                }));
            },
        };
        let index_name = tablet_index_name
            .clone()
            .map_table(&self.table_mapping().tablet_to_name())?;
        let max_rows = cmp::min(request.max_rows, MAX_PAGE_SIZE);
        Ok(RangeResponse::WaitingOn(RangeRequest {
            index_name: tablet_index_name,
            printable_index_name: index_name,
            interval: request.interval,
            order: request.order,
            max_size: max_rows,
        }))
    }

    /// NOTE: returns a page of results. Callers must call record_read_document
    /// for all documents returned from the index stream.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn index_range_batch(
        &mut self,
        requests: BTreeMap<BatchKey, IndexRangeRequest>,
    ) -> BTreeMap<BatchKey, anyhow::Result<IndexRangeResponse>> {
        let batch_size = requests.len();
        let mut results = BTreeMap::new();
        let mut fetch_requests = BTreeMap::new();
        for (batch_key, request) in requests {
            match self.start_index_range(request) {
                Err(e) => {
                    results.insert(batch_key, Err(e));
                },
                Ok(RangeResponse::Ready(result)) => {
                    results.insert(batch_key, Ok(result));
                },
                Ok(RangeResponse::WaitingOn(fetch_request)) => {
                    fetch_requests.insert(batch_key, fetch_request);
                },
            }
        }
        let fetch_results = self
            .index
            .range_batch(&mut self.reads, fetch_requests)
            .await;
        for (batch_key, fetch_result) in fetch_results {
            results.insert(batch_key, fetch_result);
        }
        assert_eq!(results.len(), batch_size);
        results
    }

    /// Used when a system table is served from cache - to manually add a read
    /// dependency on that system table.
    pub fn record_system_table_cache_hit(
        &mut self,
        index_name: TabletIndexName,
        fields: IndexedFields,
        interval: Interval,
    ) {
        self.reads
            .record_indexed_derived(index_name, fields, interval)
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn set_index_size_hard_limit(&mut self, size: usize) {
        self.index_size_override = Some(size);
    }

    pub async fn finalize(
        self,
        snapshot_reader: Reader<SnapshotManager>,
    ) -> anyhow::Result<FinalTransaction> {
        FinalTransaction::new(self, snapshot_reader).await
    }
}

pub struct IndexRangeRequest {
    pub stable_index_name: StableIndexName,
    pub interval: Interval,
    pub order: Order,
    pub max_rows: usize,
    pub version: Option<Version>,
}

pub enum RangeResponse {
    Ready(IndexRangeResponse),
    WaitingOn(RangeRequest),
}

/// FinalTransaction is a finalized Transaction.
/// After all persistence reads have been performed and validated, and all
/// writes have been staged, a FinalTransaction stores the transaction until it
/// has been fully committed.
pub struct FinalTransaction {
    pub(crate) begin_timestamp: RepeatableTimestamp,
    pub(crate) table_mapping: TableMapping,

    pub(crate) reads: TransactionReadSet,
    pub(crate) writes: Writes,

    pub(crate) usage_tracker: FunctionUsageTracker,
}

impl FinalTransaction {
    pub fn is_readonly(&self) -> bool {
        self.writes.is_empty()
    }

    pub async fn new<RT: Runtime>(
        mut transaction: Transaction<RT>,
        snapshot_reader: Reader<SnapshotManager>,
    ) -> anyhow::Result<Self> {
        let begin_timestamp = transaction.begin_timestamp();
        let table_mapping = transaction.table_mapping().clone();
        // Note that we do a best effort validation for memory index sizes. We
        // use the latest snapshot instead of the transaction base snapshot. This
        // is both more accurate and also avoids pedant hitting transient errors.
        let latest_snapshot = snapshot_reader.lock().latest_snapshot();
        Self::validate_memory_index_sizes(&table_mapping, &transaction, &latest_snapshot)?;
        Ok(Self {
            begin_timestamp,
            table_mapping,
            reads: transaction.reads,
            writes: transaction.writes,
            usage_tracker: transaction.usage_tracker.clone(),
        })
    }

    fn validate_memory_index_sizes<RT: Runtime>(
        table_mapping: &TableMapping,
        transaction: &Transaction<RT>,
        base_snapshot: &Snapshot,
    ) -> anyhow::Result<()> {
        #[allow(unused_mut)]
        let mut vector_size_limit = *VECTOR_INDEX_SIZE_HARD_LIMIT;
        #[cfg(any(test, feature = "testing"))]
        if let Some(size) = transaction.index_size_override {
            vector_size_limit = size;
        }

        #[allow(unused_mut)]
        let mut search_size_limit = *SEARCH_INDEX_SIZE_HARD_LIMIT;
        #[cfg(any(test, feature = "testing"))]
        if let Some(size) = transaction.index_size_override {
            search_size_limit = size;
        }

        let modified_tables: BTreeSet<_> = transaction
            .writes
            .coalesced_writes()
            .map(|(id, _)| id.table().tablet_id)
            .collect();
        Self::validate_memory_index_size(
            table_mapping,
            base_snapshot,
            &modified_tables,
            base_snapshot.search_indexes.in_memory_sizes().into_iter(),
            search_size_limit,
            "Search",
        )?;
        Self::validate_memory_index_size(
            table_mapping,
            base_snapshot,
            &modified_tables,
            base_snapshot.vector_indexes.in_memory_sizes().into_iter(),
            vector_size_limit,
            "Vector",
        )?;
        Ok(())
    }

    fn validate_memory_index_size(
        table_mapping: &TableMapping,
        base_snapshot: &Snapshot,
        modified_tables: &BTreeSet<TabletId>,
        iterator: impl Iterator<Item = (IndexId, usize)>,
        hard_limit: usize,
        index_type: &'static str,
    ) -> anyhow::Result<()> {
        for (index_id, size) in iterator {
            if size < hard_limit {
                continue;
            }

            // Note that we are getting an index by name without adding any read dependency.
            // This is fine since we are only decided whether to let the transaction
            // through or not. If we do not, we will throw a non-JS error.
            let index = base_snapshot
                .index_registry
                .enabled_index_by_index_id(&index_id)
                .cloned()
                .with_context(|| anyhow::anyhow!("failed to find index id {index_id}"))?
                .name();

            if !modified_tables.contains(index.table()) {
                // NOTE: All operation make the in memory size larger, including
                // deletes and updates with smaller values. So we reject any
                // modification if we are over the limit.
                continue;
            }

            anyhow::bail!(ErrorMetadata::overloaded(
                format!("{}IndexTooLarge", index_type),
                format!(
                    "Too many writes to {}, backoff and try again",
                    index.map_table(&table_mapping.tablet_to_name())?
                )
            ))
        }
        Ok(())
    }
}
