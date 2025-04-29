#[cfg(any(test, feature = "testing"))]
use std::fmt::Debug;
use std::{
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
        schema::SchemaState,
        tables::{
            TableMetadata,
            TABLES_TABLE,
        },
    },
    components::{
        ComponentId,
        ComponentPath,
    },
    document::{
        CreationTime,
        DocumentUpdateWithPrevTs,
        ResolvedDocument,
    },
    identity::InertIdentity,
    index::{
        IndexKey,
        IndexKeyBytes,
    },
    interval::Interval,
    knobs::{
        TEXT_INDEX_SIZE_HARD_LIMIT,
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
    schemas::DatabaseSchema,
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
    },
    version::Version,
    virtual_system_mapping::VirtualSystemMapping,
};
use errors::ErrorMetadata;
use imbl::OrdMap;
use indexing::backend_in_memory_indexes::RangeRequest;
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
use tokio::task;
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
    schema_registry::SchemaRegistry,
    snapshot_manager::{
        Snapshot,
        SnapshotManager,
    },
    table_summary::table_summary_bootstrapping_error,
    token::Token,
    transaction_id_generator::TransactionIdGenerator,
    transaction_index::TransactionIndex,
    write_limits::BiggestDocumentWrites,
    writes::{
        NestedWriteToken,
        NestedWrites,
        TransactionWriteSize,
        Writes,
    },
    ComponentRegistry,
    IndexModel,
    ReadSet,
    SchemaModel,
    SystemMetadataModel,
    TableModel,
    TableRegistry,
    SCHEMAS_TABLE,
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
    pub(crate) writes: NestedWrites<Writes>,

    pub(crate) index: NestedWrites<TransactionIndex>,
    pub(crate) metadata: NestedWrites<TableRegistry>,
    pub(crate) schema_registry: NestedWrites<SchemaRegistry>,
    pub(crate) component_registry: NestedWrites<ComponentRegistry>,
    pub(crate) count_snapshot: Arc<dyn TableCountSnapshot>,
    /// The change in the number of documents in table that have had writes in
    /// this transaction. If there is no entry for a table, assume deltas
    /// are zero.
    pub(crate) table_count_deltas: BTreeMap<TabletId, i64>,

    pub(crate) stats: BTreeMap<TabletId, TableStats>,

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
    async fn count(&self, table: TabletId) -> anyhow::Result<Option<u64>>;
}

pub struct SubtransactionToken {
    writes: NestedWriteToken,
    index: NestedWriteToken,
    tables: NestedWriteToken,
    schema_registry: NestedWriteToken,
    component_registry: NestedWriteToken,
}

impl<RT: Runtime> Transaction<RT> {
    pub fn new(
        identity: Identity,
        id_generator: TransactionIdGenerator,
        creation_time: CreationTime,
        index: TransactionIndex,
        metadata: TableRegistry,
        schema_registry: SchemaRegistry,
        component_registry: ComponentRegistry,
        count: Arc<dyn TableCountSnapshot>,
        runtime: RT,
        usage_tracker: FunctionUsageTracker,
        retention_validator: Arc<dyn RetentionValidator>,
        virtual_system_mapping: VirtualSystemMapping,
    ) -> Self {
        Self {
            identity,
            reads: TransactionReadSet::new(),
            writes: NestedWrites::new(Writes::new()),
            id_generator,
            next_creation_time: creation_time,
            scheduled_size: TransactionWriteSize::default(),
            index: NestedWrites::new(index),
            metadata: NestedWrites::new(metadata),
            schema_registry: NestedWrites::new(schema_registry),
            component_registry: NestedWrites::new(component_registry),
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

    pub fn virtual_system_mapping(&self) -> &VirtualSystemMapping {
        &self.virtual_system_mapping
    }

    /// Checks both virtual tables and tables to get the table number to name
    /// mapping. If table is excluded by `table_filter`, returns error as if
    /// the table doesn't exist.
    pub fn all_tables_number_to_name(
        &mut self,
        namespace: TableNamespace,
        table_filter: TableFilter,
    ) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        let table_mapping = self.table_mapping().namespace(namespace);
        let virtual_system_mapping = self.virtual_system_mapping().clone();
        move |number| {
            let name = table_mapping.number_to_name()(number)?;
            if let Some(virtual_name) = virtual_system_mapping.system_to_virtual_table(&name) {
                Ok(virtual_name.clone())
            } else {
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
        namespace: TableNamespace,
        table_filter: TableFilter,
    ) -> anyhow::Result<TableName> {
        match self.all_tables_number_to_name(namespace, table_filter)(id.table()) {
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

    pub fn begin_subtransaction(&mut self) -> SubtransactionToken {
        SubtransactionToken {
            writes: self.writes.begin_nested(),
            index: self.index.begin_nested(),
            tables: self.metadata.begin_nested(),
            schema_registry: self.schema_registry.begin_nested(),
            component_registry: self.component_registry.begin_nested(),
        }
    }

    pub fn commit_subtransaction(&mut self, tokens: SubtransactionToken) -> anyhow::Result<()> {
        self.writes.commit_nested(tokens.writes)?;
        self.index.commit_nested(tokens.index)?;
        self.metadata.commit_nested(tokens.tables)?;
        self.schema_registry.commit_nested(tokens.schema_registry)?;
        self.component_registry
            .commit_nested(tokens.component_registry)?;
        Ok(())
    }

    pub fn rollback_subtransaction(&mut self, tokens: SubtransactionToken) -> anyhow::Result<()> {
        self.writes.rollback_nested(tokens.writes)?;
        self.index.rollback_nested(tokens.index)?;
        self.metadata.rollback_nested(tokens.tables)?;
        self.schema_registry
            .rollback_nested(tokens.schema_registry)?;
        self.component_registry
            .rollback_nested(tokens.component_registry)?;
        Ok(())
    }

    pub fn require_not_nested(&self) -> anyhow::Result<()> {
        self.writes.require_not_nested()?;
        self.index.require_not_nested()?;
        self.metadata.require_not_nested()?;
        self.schema_registry.require_not_nested()?;
        self.component_registry.require_not_nested()?;
        Ok(())
    }

    pub fn writes(&self) -> &NestedWrites<Writes> {
        &self.writes
    }

    pub fn into_reads_and_writes(self) -> (TransactionReadSet, NestedWrites<Writes>) {
        (self.reads, self.writes)
    }

    pub fn biggest_document_writes(&self) -> Option<BiggestDocumentWrites> {
        let mut max_size = 0;
        let mut biggest_document_id = None;
        let mut max_nesting = 0;
        let mut most_nested_document_id = None;
        for (document_id, DocumentUpdateWithPrevTs { new_document, .. }) in
            self.writes.coalesced_writes()
        {
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
    #[fastrace::trace]
    pub fn apply_function_runner_tx(
        &mut self,
        begin_timestamp: Timestamp,
        reads: ReadSet,
        num_intervals: usize,
        user_tx_size: crate::reads::TransactionReadSize,
        system_tx_size: crate::reads::TransactionReadSize,
        updates: OrdMap<ResolvedDocumentId, DocumentUpdateWithPrevTs>,
        rows_read_by_tablet: BTreeMap<TabletId, u64>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            *self.begin_timestamp() == begin_timestamp,
            "Timestamp mismatch"
        );

        self.reads
            .merge(reads, num_intervals, user_tx_size, system_tx_size);

        self.merge_writes(updates)?;

        for (tablet_id, rows_read) in rows_read_by_tablet {
            self.stats.entry(tablet_id).or_default().rows_read += rows_read;
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
        updates: OrdMap<ResolvedDocumentId, DocumentUpdateWithPrevTs>,
    ) -> anyhow::Result<()> {
        let existing_updates = self.writes().as_flat()?.clone().into_updates();

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
                    *existing_update == update,
                    "Conflicting updates for document {id}"
                );
                preserved_update_count += 1;
                continue;
            }

            if let Some(ref document) = update.new_document {
                let doc_creation_time = document.creation_time();
                if doc_creation_time >= self.next_creation_time {
                    self.next_creation_time = doc_creation_time;
                    self.next_creation_time.increment()?;
                }
            }
            self.apply_validated_write(
                id,
                update
                    .old_document
                    .map(|(d, ts)| (d, WriteTimestamp::Committed(ts))),
                update.new_document,
            )?;
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

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    pub async fn get_with_ts(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<(ResolvedDocument, WriteTimestamp)>> {
        task::consume_budget().await;
        let table_name = match self.table_mapping().tablet_name(id.tablet_id) {
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
        task::consume_budget().await;

        let table_name = self.table_mapping().tablet_name(id.tablet_id)?;
        let namespace = self.table_mapping().tablet_namespace(id.tablet_id)?;

        let (old_document, old_ts) =
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
        SchemaModel::new(self, namespace)
            .enforce(&new_document)
            .await?;

        self.apply_validated_write(id, Some((old_document, old_ts)), Some(new_document.clone()))?;
        Ok(new_document)
    }

    pub fn is_system(&mut self, namespace: TableNamespace, table_number: TableNumber) -> bool {
        let tablet_id =
            match self.table_mapping().namespace(namespace).number_to_tablet()(table_number) {
                Err(_) => None,
                Ok(id) => Some(id),
            };
        tablet_id.is_some_and(|id| self.table_mapping().is_system_tablet(id))
    }

    #[convex_macro::instrument_future]
    pub(crate) async fn replace_inner(
        &mut self,
        id: ResolvedDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        task::consume_budget().await;

        let table_name = self.table_mapping().tablet_name(id.tablet_id)?;
        let namespace = self.table_mapping().tablet_namespace(id.tablet_id)?;
        let (old_document, old_ts) =
            self.get_inner(id, table_name)
                .await?
                .context(ErrorMetadata::bad_request(
                    "NonexistentDocument",
                    format!("Replace on nonexistent document ID {id}"),
                ))?;

        // Replace document.
        let new_document = old_document.replace_value(value)?;

        SchemaModel::new(self, namespace)
            .enforce(&new_document)
            .await?;

        self.apply_validated_write(
            new_document.id(),
            Some((old_document, old_ts)),
            Some(new_document.clone()),
        )?;
        Ok(new_document)
    }

    #[convex_macro::instrument_future]
    pub async fn delete_inner(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocument> {
        task::consume_budget().await;

        let table_name = self.table_mapping().tablet_name(id.tablet_id)?;
        let (document, ts) =
            self.get_inner(id, table_name)
                .await?
                .context(ErrorMetadata::bad_request(
                    "NonexistentDocument",
                    format!("Delete on nonexistent document ID {id}"),
                ))?;

        self.apply_validated_write(document.id(), Some((document.clone(), ts)), None)?;
        Ok(document)
    }

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    pub async fn count(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<Option<u64>> {
        let virtual_system_mapping = self.virtual_system_mapping().clone();
        let system_table = if virtual_system_mapping.is_virtual_table(table) {
            virtual_system_mapping.virtual_to_system_table(table)?
        } else {
            table
        };
        TableModel::new(self).count(namespace, system_table).await
    }

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    pub async fn must_count(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<u64> {
        TableModel::new(self)
            .count(namespace, table)
            .await?
            .ok_or_else(|| {
                table_summary_bootstrapping_error(Some(
                    "Table count unavailable while bootstrapping",
                ))
            })
    }

    pub fn into_token(self) -> anyhow::Result<Token> {
        if !self.is_readonly() {
            anyhow::bail!("Transaction isn't readonly");
        }
        metrics::log_read_tx(&self);
        let ts = *self.begin_timestamp();
        Ok(Token::new(Arc::new(self.reads.into_read_set()), ts))
    }

    pub fn take_stats(&mut self) -> BTreeMap<TableName, TableStats> {
        let stats = mem::take(&mut self.stats);
        stats
            .into_iter()
            .map(|(tablet, stats)| {
                (
                    match self.table_mapping().tablet_name(tablet) {
                        Ok(name) => name,
                        Err(_) => {
                            // This is unusual, but possible if the tablet was created in a
                            // subtransaction that was rolled back. Such a tablet never gets
                            // created, but might still have usage stats.
                            tracing::warn!("Tablet {tablet} does not exist");
                            // It's fine to return "_unknown" here because nothing requires
                            // these to correspond to an actual table.
                            //
                            // We use "_unknown" to avoid colliding with valid user table names.
                            "_unknown"
                                .parse()
                                .expect("'_unknown' should be a valid table name")
                        },
                    },
                    stats,
                )
            })
            .collect()
    }

    pub fn stats_by_tablet(&self) -> &BTreeMap<TabletId, TableStats> {
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

    /// Reads the schema from the cache, and records a read dependency.
    /// Used by SchemaModel.
    pub(crate) fn get_schema_by_state(
        &mut self,
        namespace: TableNamespace,
        state: SchemaState,
    ) -> anyhow::Result<Option<(ResolvedDocumentId, DatabaseSchema)>> {
        if !self
            .table_mapping()
            .namespace(namespace)
            .name_exists(&SCHEMAS_TABLE)
        {
            return Ok(None);
        }
        let schema_tablet = self
            .table_mapping()
            .namespace(namespace)
            .id(&SCHEMAS_TABLE)?
            .tablet_id;
        self.schema_registry
            .get_by_state(namespace, state, schema_tablet, &mut self.reads)
    }

    pub fn get_component_path(&mut self, component_id: ComponentId) -> Option<ComponentPath> {
        self.component_registry
            .get_component_path(component_id, &mut self.reads)
    }

    pub fn must_component_path(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<ComponentPath> {
        self.component_registry
            .must_component_path(component_id, &mut self.reads)
    }

    /// Get the component path for a document ID. This might be None when table
    /// namespaces for new components are created in `start_push`,  but
    /// components have not yet been created.
    pub fn component_path_for_document_id(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<ComponentPath>> {
        self.component_registry.component_path_from_document_id(
            self.metadata.table_mapping(),
            id,
            &mut self.reads,
        )
    }

    // XXX move to table model?
    #[cfg(any(test, feature = "testing"))]
    pub async fn create_system_table_testing(
        &mut self,
        namespace: TableNamespace,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<bool> {
        self.create_system_table(namespace, table_name, default_table_number)
            .await
    }

    async fn table_number_for_system_table(
        &mut self,
        namespace: TableNamespace,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<TableNumber> {
        Ok(if let Some(default_table_number) = default_table_number {
            let existing_name =
                self.table_mapping().namespace(namespace).number_to_name()(default_table_number)
                    .ok();
            let table_number = if let Some(existing_name) = existing_name {
                if self.virtual_system_mapping.is_virtual_table(table_name)
                    && *self
                        .virtual_system_mapping
                        .virtual_to_system_table(table_name)?
                        == existing_name
                {
                    // Table number is occupied by the system table for the virtual table we're
                    // creating. Allow it.
                    default_table_number
                } else if cfg!(any(test, feature = "testing")) {
                    // In tests, have a hard failure on conflicting default table numbers. In
                    // real system, have a looser fallback where we pick
                    // another table number.
                    anyhow::bail!(
                        "{default_table_number} is used by both {table_name} and {existing_name}"
                    );
                } else {
                    // If the table_number requested is taken, just pick a higher table number.
                    // This might be true for older backends that have lower-numbered system
                    // tables.
                    TableModel::new(self)
                        .next_system_table_number(namespace)
                        .await?
                }
            } else {
                default_table_number
            };
            // TODO(CX-6699) handle system table number exhaustion.
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
            TableModel::new(self)
                .next_system_table_number(namespace)
                .await?
        })
    }

    /// Creates a new system table, with _id and _creationTime indexes, returns
    /// false if table already existed
    pub async fn create_system_table(
        &mut self,
        namespace: TableNamespace,
        table_name: &TableName,
        default_table_number: Option<TableNumber>,
    ) -> anyhow::Result<bool> {
        anyhow::ensure!(self.identity().is_system());
        anyhow::ensure!(
            table_name.is_system(),
            "{table_name:?} is not a valid system table name!"
        );

        let is_new = !TableModel::new(self).table_exists(namespace, table_name);
        if is_new {
            let table_number = self
                .table_number_for_system_table(namespace, table_name, default_table_number)
                .await?;
            let metadata = TableMetadata::new(namespace, table_name.clone(), table_number);
            let table_doc_id = SystemMetadataModel::new_global(self)
                .insert(&TABLES_TABLE, metadata.try_into()?)
                .await?;
            let tablet_id = TabletId(table_doc_id.internal_id());

            let by_id_index = IndexMetadata::new_enabled(
                GenericIndexName::by_id(tablet_id),
                IndexedFields::by_id(),
            );
            SystemMetadataModel::new_global(self)
                .insert(&INDEX_TABLE, by_id_index.try_into()?)
                .await?;
            let metadata = IndexMetadata::new_enabled(
                GenericIndexName::by_creation_time(tablet_id),
                IndexedFields::creation_time(),
            );
            SystemMetadataModel::new_global(self)
                .insert(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            tracing::info!("Created system table: {table_name}");
        } else {
            tracing::debug!("Skipped creating system table {table_name} since it already exists");
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
        let index_name = TabletIndexName::by_id(id.tablet_id);
        let printable_index_name = IndexName::by_id(table_name.clone());
        let index_key = IndexKey::new(vec![], id.into());
        let interval = Interval::prefix(index_key.to_bytes().into());
        let range_request = RangeRequest {
            index_name: index_name.clone(),
            printable_index_name,
            interval: interval.clone(),
            order: Order::Asc,
            // Request 2 to best-effort verify uniqueness of by_id index.
            max_size: 2,
        };

        let mut results = self
            .index
            .range_batch(&mut self.reads, btreemap! { 0 => range_request })
            .await;
        self.reads
            .record_indexed_directly(index_name, IndexedFields::by_id(), interval)?;
        let IndexRangeResponse {
            page: range_results,
            cursor,
        } = results.remove(&0).context("expected result")??;
        if range_results.len() > 1 {
            Err(anyhow::anyhow!("Got multiple values for id {id:?}"))?;
        }
        if !matches!(cursor, CursorPosition::End) {
            Err(anyhow::anyhow!(
                "Querying 2 items for a single id didn't exhaust interval for {id:?}"
            ))?;
        }
        let result = match range_results.into_iter().next() {
            Some((_, doc, timestamp)) => {
                let is_virtual_table = self.virtual_system_mapping().is_virtual_table(&table_name);
                let component_path = self
                    .component_path_for_document_id(doc.id())?
                    .unwrap_or_default();
                self.reads.record_read_document(
                    component_path,
                    table_name,
                    doc.size(),
                    &self.usage_tracker,
                    is_virtual_table,
                )?;

                Some((doc, timestamp))
            },
            None => None,
        };
        self.stats.entry(id.tablet_id).or_default().rows_read += 1;
        Ok(result)
    }

    /// Apply a validated write to the [Transaction], updating the
    /// [IndexRegistry] and [TableRegistry]. Validated means the write
    /// has already been checked for schema enforcement.
    pub(crate) fn apply_validated_write(
        &mut self,
        id: ResolvedDocumentId,
        old_document_and_ts: Option<(ResolvedDocument, WriteTimestamp)>,
        new_document: Option<ResolvedDocument>,
    ) -> anyhow::Result<()> {
        // Implement something like two-phase commit between the index and the document
        // store. We first guarantee that the changes are valid for the index and
        // metadata and then let inserting into writes the commit
        // point so that the Transaction is never in an inconsistent state.
        let is_system_document = self.table_mapping().is_system_tablet(id.tablet_id);
        let bootstrap_tables = self.bootstrap_tables();
        let old_document = old_document_and_ts.as_ref().map(|(doc, _)| doc);
        let index_update = self
            .index
            .begin_update(old_document.cloned(), new_document.clone())?;
        let schema_update = self.schema_registry.begin_update(
            self.metadata.table_mapping(),
            id,
            old_document,
            new_document.as_ref(),
        )?;
        let component_update = self.component_registry.begin_update(
            self.metadata.table_mapping(),
            id,
            old_document,
            new_document.as_ref(),
        )?;
        let metadata_update = self.metadata.begin_update(
            index_update.registry(),
            id,
            old_document.map(|d| d.value().deref()),
            new_document.as_ref().map(|d| d.value().deref()),
        )?;
        let stats = self.stats.entry(id.tablet_id).or_default();
        let mut delta = 0;
        match (old_document, new_document.as_ref()) {
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
            old_document_and_ts,
            new_document,
        )?;
        stats.rows_written += 1;

        index_update.apply();
        metadata_update.apply();
        schema_update.apply();
        component_update.apply();

        *self.table_count_deltas.entry(id.tablet_id).or_default() += delta;
        Ok(())
    }

    pub(crate) async fn insert_document(
        &mut self,
        document: ResolvedDocument,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let document_id = document.id();
        let namespace = self
            .table_mapping()
            .tablet_namespace(document_id.tablet_id)?;
        SchemaModel::new(self, namespace).enforce(&document).await?;
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
        let component_path = self
            .component_path_for_document_id(document.id())?
            .unwrap_or_default();
        self.reads.record_read_document(
            component_path,
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
        namespace: TableNamespace,
        index_name: &IndexName,
        interval: &Interval,
    ) -> anyhow::Result<PreloadedIndexRange> {
        let stable_index_name = IndexModel::new(self).stable_index_name(
            namespace,
            index_name,
            TableFilter::IncludePrivateSystemTables,
        )?;
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

#[derive(Debug)]
pub struct IndexRangeRequest {
    pub stable_index_name: StableIndexName,
    pub interval: Interval,
    pub order: Order,
    pub max_rows: usize,
    pub version: Option<Version>,
}

/// FinalTransaction is a finalized Transaction.
/// After all persistence reads have been performed and validated, and all
/// writes have been staged, a FinalTransaction stores the transaction until it
/// has been fully committed.
pub struct FinalTransaction {
    pub(crate) begin_timestamp: RepeatableTimestamp,
    pub(crate) table_mapping: TableMapping,
    pub(crate) component_registry: ComponentRegistry,

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
        // All subtransactions must have committed or rolled back.
        transaction.require_not_nested()?;

        let begin_timestamp = transaction.begin_timestamp();
        let table_mapping = transaction.table_mapping().clone();
        let component_registry = transaction.component_registry.deref().clone();
        // Note that we do a best effort validation for memory index sizes. We
        // use the latest snapshot instead of the transaction base snapshot. This
        // is both more accurate and also avoids pedant hitting transient errors.
        let latest_snapshot = snapshot_reader.lock().latest_snapshot();
        Self::validate_memory_index_sizes(&table_mapping, &transaction, &latest_snapshot)?;
        Ok(Self {
            begin_timestamp,
            table_mapping,
            component_registry,
            reads: transaction.reads,
            writes: transaction.writes.into_flat()?,
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
        let mut search_size_limit = *TEXT_INDEX_SIZE_HARD_LIMIT;
        #[cfg(any(test, feature = "testing"))]
        if let Some(size) = transaction.index_size_override {
            search_size_limit = size;
        }

        let modified_tables: BTreeSet<_> = transaction
            .writes
            .as_flat()?
            .coalesced_writes()
            .map(|(id, _)| id.tablet_id)
            .collect();
        Self::validate_memory_index_size(
            table_mapping,
            base_snapshot,
            &modified_tables,
            base_snapshot.text_indexes.in_memory_sizes().into_iter(),
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
