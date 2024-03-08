#[cfg(any(test, feature = "testing"))]
use std::fmt::Debug;
use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    convert::TryInto,
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
        DeveloperDocument,
        DocumentUpdate,
        ResolvedDocument,
        CREATION_TIME_FIELD,
        ID_FIELD,
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
        id_v6::DocumentIdV6,
        ConvexObject,
        ConvexValue,
        DeveloperDocumentId,
        ResolvedDocumentId,
        Size,
        TableIdAndTableNumber,
        TableMapping,
        VirtualTableMapping,
    },
    version::Version,
};
use errors::ErrorMetadata;
use itertools::Itertools;
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
    check_user_size,
    FieldName,
    TableId,
    TableIdentifier,
    TableNumber,
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
    database::unauthorized_error,
    defaults::bootstrap_system_tables,
    execution_size::FunctionExecutionSize,
    metrics::{
        self,
        log_virtual_table_get,
        log_virtual_table_query,
    },
    patch::PatchValue,
    preloaded::PreloadedIndexRange,
    query::TableFilter,
    reads::TransactionReadSet,
    snapshot_manager::{
        Snapshot,
        SnapshotManager,
    },
    token::Token,
    transaction_id_generator::TransactionIdGenerator,
    transaction_index::{
        BatchKey,
        RangeRequest,
        TransactionIndex,
    },
    virtual_tables::{
        VirtualSystemMapping,
        VirtualTable,
    },
    write_limits::BiggestDocumentWrites,
    writes::{
        TransactionWriteSize,
        Writes,
    },
    IndexModel,
    ReadSet,
    SchemaModel,
    TableModel,
    TableRegistry,
    VirtualTableMetadata,
    VIRTUAL_TABLES_TABLE,
};

/// Safe default number of items to return for each list or filter operation
/// when we're writing internal code and don't know what other value to choose.
pub const DEFAULT_PAGE_SIZE: usize = 512;

const MAX_PAGE_SIZE: usize = 1024;
pub struct Transaction<RT: Runtime> {
    identity: Identity,
    id_generator: TransactionIdGenerator,

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
    pub(crate) table_count_deltas: BTreeMap<TableId, i64>,

    stats: BTreeMap<TableNumber, TableStats>,

    retention_validator: Arc<dyn RetentionValidator>,

    runtime: RT,

    pub usage_tracker: FunctionUsageTracker,
    virtual_system_mapping: VirtualSystemMapping,

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
    async fn count(&self, table: TableId) -> anyhow::Result<u64>;
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
                let name = table_mapping.number_to_name()(number)?;
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
        id: DocumentIdV6,
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

        let mut updates = updates
            .into_iter()
            .map(|(id, write)| (id, write))
            .collect::<Vec<_>>();
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

    #[convex_macro::instrument_future]
    pub async fn get_with_ts(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<(ResolvedDocument, WriteTimestamp)>> {
        let table_name = match self.table_mapping().tablet_name(id.table().table_id) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        if self.virtual_system_mapping().is_virtual_table(&table_name) {
            anyhow::bail!("Virtual tables should use get_with_ts_user_facing");
        }
        self.get_inner(id, table_name).await
    }

    #[convex_macro::instrument_future]
    pub async fn get_with_ts_user_facing(
        &mut self,
        id: DeveloperDocumentId,
        version: Option<Version>,
    ) -> anyhow::Result<Option<(DeveloperDocument, WriteTimestamp)>> {
        let mut batch_result = self
            .get_batch(btreemap! {
                0 => (id, version),
            })
            .await;
        batch_result
            .remove(&0)
            .context("get_batch missing batch key")?
    }

    /// Fetches a batch of documents by id.
    /// Stage 1: For each requested ID, set up the fetch, reading table and
    ///     index ids, checking virtual tables, computing index intervals,
    ///     and looking in the cache. In particular, cache hits for the
    ///     entire batch are based on the initial state.
    /// Stage 2: Execute all of the underlying fetches against persistence in
    ///     parallel.
    /// Stage 3: For each requested ID, add it to the cache and
    ///     usage records, and munge the index range's results into
    ///     DeveloperDocuments.
    ///
    /// This leads to completely deterministic results, down to usage counts
    /// and which requests hit the cache.
    /// Throughout the stages, each item in the batch is effectively separate,
    /// so their errors are calculated independently.
    /// Since stage 3 mutates common state in a loop, the items can affect each
    /// other, e.g. if one item overflows the transaction limits, the remainder
    /// of the batch will throw similar errors.
    /// TODO(lee) dedupe duplicate fetches within a batch, which requires
    /// cloning errors.
    pub async fn get_batch(
        &mut self,
        ids: BTreeMap<BatchKey, (DeveloperDocumentId, Option<Version>)>,
    ) -> BTreeMap<BatchKey, anyhow::Result<Option<(DeveloperDocument, WriteTimestamp)>>> {
        let mut results = BTreeMap::new();
        let mut ids_to_fetch = BTreeMap::new();
        let batch_size = ids.len();
        for (batch_key, (id, version)) in ids {
            let resolve_result: anyhow::Result<_> = try {
                if self.virtual_table_mapping().number_exists(id.table()) {
                    // TODO(lee) batch virtual table gets
                    log_virtual_table_get();
                    let table_name = self.virtual_table_mapping().name(*id.table())?;
                    match VirtualTable::new(self).get(&id, version).await? {
                        Some(result) => {
                            self.reads.record_read_document(
                                table_name,
                                result.0.size(),
                                &self.usage_tracker,
                                true,
                            )?;
                            assert!(results.insert(batch_key, Ok(Some(result))).is_none());
                        },
                        None => {
                            assert!(results.insert(batch_key, Ok(None)).is_none());
                        },
                    }
                } else {
                    if !self.table_mapping().table_number_exists()(*id.table()) {
                        assert!(results.insert(batch_key, Ok(None)).is_none());
                        continue;
                    }
                    let id_ = id.map_table(self.table_mapping().inject_table_id())?;
                    let table_name = self.table_mapping().tablet_name(id_.table().table_id)?;
                    ids_to_fetch.insert(batch_key, (id_, table_name));
                }
            };
            if let Err(e) = resolve_result {
                assert!(results.insert(batch_key, Err(e)).is_none());
            }
        }
        let fetched_results = self.get_inner_batch(ids_to_fetch).await;
        for (batch_key, inner_result) in fetched_results {
            let result: anyhow::Result<_> = try {
                let developer_result = inner_result?.map(|(doc, ts)| (doc.to_developer(), ts));
                assert!(results.insert(batch_key, Ok(developer_result)).is_none());
            };
            if let Err(e) = result {
                assert!(results.insert(batch_key, Err(e)).is_none());
            }
        }
        assert_eq!(results.len(), batch_size);
        results
    }

    /// Creates a new document with given value in the specified table.
    #[convex_macro::instrument_future]
    pub async fn insert_user_facing(
        &mut self,
        table: TableName,
        value: ConvexObject,
    ) -> anyhow::Result<DeveloperDocumentId> {
        if self.virtual_system_mapping().is_virtual_table(&table) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "ReadOnlyTable",
                format!("{table} is a read-only table"),
            ));
        }

        check_user_size(value.size())?;
        self.retention_validator.fail_if_falling_behind()?;
        let id = self.id_generator.generate(&table);

        let creation_time = self.next_creation_time.increment()?;

        if table.is_system() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidTableName",
                format!("Invalid table name {table} starts with metadata prefix '_'")
            ));
        }

        // Note that the index and document store updates within `self.insert_document`
        // below are fallible, and since the layers above still have access to
        // the `Transaction` in that case (we only have `&mut self` here, not a
        // consuming `self`), we need to make sure we leave the transaction in a
        // consistent state on error.
        //
        // It's okay for us to insert the table write here and fail below: At worse the
        // transaction will contain an insertion for an empty table's `_tables`
        // record. On the other hand, it's not okay for us to succeed an
        // insertion into the index/document store and then fail to insert the
        // table metadata. If the user then subsequently commits that transaction,
        // they'll have a record that points to a nonexistent table.
        TableModel::new(self).insert_table_metadata(&table).await?;
        let document = ResolvedDocument::new(
            id.clone()
                .map_table(self.table_mapping().name_to_id_user_input())?,
            creation_time,
            value,
        )?;
        let document_id = self.insert_document(document).await?;

        Ok(document_id.into())
    }

    /// Inserts a new document as part of a snapshot import.
    /// This is like `insert_user_facing` with a few differences:
    /// - the table for insertion is chosen by table id, not table name or
    ///   number.
    /// - nonexistent tables won't be created implicitly.
    /// - the _creationTime may be user-specified.
    /// - only admin/system auth is allowed.
    #[convex_macro::instrument_future]
    pub async fn insert_for_import(
        &mut self,
        table_id: TableIdAndTableNumber,
        table_name: &TableName,
        value: ConvexObject,
        table_mapping_for_schema: &TableMapping,
    ) -> anyhow::Result<DeveloperDocumentId> {
        if self.virtual_system_mapping().is_virtual_table(table_name) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "ReadOnlyTable",
                format!("{table_name} is a read-only table"),
            ));
        }
        anyhow::ensure!(
            bootstrap_system_tables()
                .iter()
                .all(|t| t.table_name() != table_name),
            "Cannot import into bootstrap system table {table_name}"
        );
        if !(self.identity.is_admin() || self.identity.is_system()) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnauthorizedImport",
                "Import requires admin auth"
            ));
        }

        check_user_size(value.size())?;
        self.retention_validator.fail_if_falling_behind()?;
        let id_field = FieldName::from(ID_FIELD.clone());
        let internal_id = if let Some(ConvexValue::String(s)) = value.get(&id_field) {
            let id_v6 = DocumentIdV6::decode(s).context(ErrorMetadata::bad_request(
                "InvalidId",
                format!("invalid _id '{s}'"),
            ))?;
            anyhow::ensure!(
                *id_v6.table() == table_id.table_number,
                ErrorMetadata::bad_request(
                    "ImportConflict",
                    format!(
                        "_id {s} cannot be imported into '{table_name}' because its IDs have a \
                         different format"
                    )
                )
            );
            id_v6.internal_id()
        } else {
            self.id_generator.generate_internal()
        };
        let id = table_id.id(internal_id);

        let creation_time_field = FieldName::from(CREATION_TIME_FIELD.clone());
        let creation_time = if let Some(ConvexValue::Float64(f)) = value.get(&creation_time_field) {
            CreationTime::try_from(*f)?
        } else {
            self.next_creation_time.increment()?
        };

        let document = ResolvedDocument::new(id, creation_time, value)?;
        SchemaModel::new(self)
            .enforce_with_table_mapping(&document, table_mapping_for_schema)
            .await?;
        self.apply_validated_write(id, None, Some(document))?;

        Ok(id.into())
    }

    /// Insert a new document and immediately read it. Prefer using `insert`
    /// unless you need to read the creation time.
    #[cfg(any(test, feature = "testing"))]
    #[convex_macro::instrument_future]
    pub async fn insert_for_test(
        &mut self,
        table: &TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        self._insert_metadata(table, value).await
    }

    /// Creates a new document with given value in the specified table.
    #[convex_macro::instrument_future]
    pub async fn insert_system_document(
        &mut self,
        table: &TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        anyhow::ensure!(table.is_system());
        if !(self.identity.is_system() || self.identity.is_admin()) {
            anyhow::bail!(unauthorized_error("insert_metadata"));
        }
        let table_id = self.table_mapping().id(table).with_context(|| {
            if cfg!(any(test, feature = "testing")) {
                format!(
                    "Failed to find system table {table} in a test. Try initializing system \
                     tables with:\nDbFixtures::new(&rt).await?.with_model().await?"
                )
            } else {
                format!("Failed to find system table {table}")
            }
        })?;
        let id = self.id_generator.generate(&table_id);
        let creation_time = self.next_creation_time.increment()?;
        let document = ResolvedDocument::new(id, creation_time, value)?;
        self.insert_document(document).await
    }

    /// Creates a new document with given value in the specified table.
    #[convex_macro::instrument_future]
    pub async fn _insert_metadata(
        &mut self,
        table: &TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        TableModel::new(self).insert_table_metadata(table).await?;
        let table_id = self.table_mapping().id(table)?;
        let id = self.id_generator.generate(&table_id);
        let creation_time = self.next_creation_time.increment()?;
        let document = ResolvedDocument::new(id, creation_time, value)?;
        self.insert_document(document).await
    }

    /// Insert a new document and immediately read it. Prefer using `insert`
    /// unless you need to read the creation time.
    #[cfg(any(test, feature = "testing"))]
    #[convex_macro::instrument_future]
    pub async fn insert_and_get(
        &mut self,
        table: TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        let id = self.insert_for_test(&table, value).await?;
        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document with id {id} must exist"))
    }

    /// Merges the existing document with the given object. Will overwrite any
    /// conflicting fields.
    #[convex_macro::instrument_future]
    pub async fn patch_user_facing(
        &mut self,
        id: DeveloperDocumentId,
        value: PatchValue,
    ) -> anyhow::Result<DeveloperDocument> {
        if self.is_system(*id.table()) && !(self.identity.is_admin() || self.identity.is_system()) {
            anyhow::bail!(unauthorized_error("patch"))
        }
        self.retention_validator.fail_if_falling_behind()?;

        let id_ = id.map_table(self.table_mapping().inject_table_id())?;

        let new_document = self.patch_inner(id_, value).await?;

        // Check the size of the patched document.
        if !self.is_system(*id.table()) {
            check_user_size(new_document.size())?;
        }

        let developer_document = new_document.to_developer();
        Ok(developer_document)
    }

    /// Merges the existing document with the given object. Will overwrite any
    /// conflicting fields.
    #[convex_macro::instrument_future]
    pub async fn patch_system_document(
        &mut self,
        id: ResolvedDocumentId,
        value: PatchValue,
    ) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self.table_mapping().is_system(id.table().table_number));

        self.patch_inner(id, value).await
    }

    #[convex_macro::instrument_future]
    async fn patch_inner(
        &mut self,
        id: ResolvedDocumentId,
        value: PatchValue,
    ) -> anyhow::Result<ResolvedDocument> {
        let table_name = self.table_mapping().tablet_name(id.table().table_id)?;

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

    /// Replace the document with the given value.
    #[convex_macro::instrument_future]
    pub async fn replace_user_facing(
        &mut self,
        id: DeveloperDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<DeveloperDocument> {
        if self.is_system(*id.table()) && !(self.identity.is_admin() || self.identity.is_system()) {
            anyhow::bail!(unauthorized_error("replace"))
        }
        if !self.is_system(*id.table()) {
            check_user_size(value.size())?;
        }
        self.retention_validator.fail_if_falling_behind()?;
        let id_ = id.map_table(self.table_mapping().inject_table_id())?;

        let new_document = self.replace_inner(id_, value).await?;
        let developer_document = new_document.to_developer();
        Ok(developer_document)
    }

    pub fn is_system(&mut self, table_number: TableNumber) -> bool {
        self.table_mapping().is_system(table_number)
            || self.virtual_table_mapping().number_exists(&table_number)
    }

    #[convex_macro::instrument_future]
    pub async fn replace_system_document(
        &mut self,
        id: ResolvedDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self.table_mapping().is_system(id.table().table_number));

        self.replace_inner(id, value).await
    }

    #[convex_macro::instrument_future]
    async fn replace_inner(
        &mut self,
        id: ResolvedDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        let table_name = self.table_mapping().tablet_name(id.table().table_id)?;
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
            *new_document.id(),
            Some(old_document),
            Some(new_document.clone()),
        )?;
        Ok(new_document)
    }

    /// Delete the document at the given path -- called from user facing APIs
    /// (e.g. syscalls)
    #[convex_macro::instrument_future]
    pub async fn delete_user_facing(
        &mut self,
        id: DeveloperDocumentId,
    ) -> anyhow::Result<DeveloperDocument> {
        if self.is_system(*id.table()) && !(self.identity.is_admin() || self.identity.is_system()) {
            anyhow::bail!(unauthorized_error("delete"))
        }
        self.retention_validator.fail_if_falling_behind()?;

        let id_ = id.map_table(&self.table_mapping().inject_table_id())?;
        let document = self.delete_inner(id_).await?;
        Ok(document.to_developer())
    }

    /// Delete the document at the given path.
    #[convex_macro::instrument_future]
    pub async fn delete_system_document(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self.table_mapping().is_system(id.table().table_number));

        self.delete_inner(id).await
    }

    #[convex_macro::instrument_future]
    pub async fn delete_inner(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocument> {
        let table_name = self.table_mapping().tablet_name(id.table().table_id)?;
        let (document, _) =
            self.get_inner(id, table_name)
                .await?
                .context(ErrorMetadata::bad_request(
                    "NonexistentDocument",
                    format!("Delete on nonexistent document ID {id}"),
                ))?;

        self.apply_validated_write(*document.id(), Some(document.clone()), None)?;
        Ok(document)
    }

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
                    self.table_mapping().number_to_name()(table).expect("table should exist"),
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
                .id(&TABLES_TABLE)
                .expect("_tables should exist")
                .table_id,
        );
        self.reads
            .record_indexed_derived(tables_by_id, IndexedFields::by_id(), Interval::all());
    }

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
            let table_number = if self.table_mapping().table_number_exists()(default_table_number) {
                // In tests, have a hard failure on conflicting default table numbers. In
                // real system, have a looser fallback where we pick
                // another table number.
                if cfg!(any(test, feature = "testing")) {
                    let existing_tn = self
                        .table_mapping()
                        .name_by_number_if_exists(default_table_number);
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
            let table_doc_id = self
                .insert_system_document(&TABLES_TABLE, metadata.try_into()?)
                .await?;
            let table_id = TableId(table_doc_id.internal_id());

            let by_id_index = IndexMetadata::new_enabled(
                GenericIndexName::by_id(table_id),
                IndexedFields::by_id(),
            );
            self.insert_system_document(&INDEX_TABLE, by_id_index.try_into()?)
                .await?;
            let metadata = IndexMetadata::new_enabled(
                GenericIndexName::by_creation_time(table_id),
                IndexedFields::creation_time(),
            );
            self.insert_system_document(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            tracing::info!("Created system table: {table_name}");
        } else {
            tracing::info!("Skipped creating system table {table_name} since it already exists");
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
            let table_doc_id = self
                .insert_system_document(&VIRTUAL_TABLES_TABLE, metadata.try_into()?)
                .await?;
            tracing::info!("Created virtual table: {table_name} with doc_id {table_doc_id}");
        } else {
            tracing::info!("Skipped creating virtual table {table_name} since it already exists");
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
            let index_name = TabletIndexName::by_id(id.table().table_id);
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
                let (range_results, remaining) =
                    results.remove(&batch_key).context("expected result")??;
                if range_results.len() > 1 {
                    Err(anyhow::anyhow!("Got multiple values for id {id:?}"))?;
                }
                if !remaining.is_empty() {
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
    fn apply_validated_write(
        &mut self,
        id: ResolvedDocumentId,
        old_document: Option<ResolvedDocument>,
        new_document: Option<ResolvedDocument>,
    ) -> anyhow::Result<()> {
        // Implement something like two-phase commit between the index and the document
        // store. We first guarantee that the changes are valid for the index and
        // metadata and then let inserting into writes the commit
        // point so that the Transaction is never in an inconsistent state.
        let is_system_document = self.table_mapping().is_system(id.table().table_number);
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
            .entry(id.table().table_id)
            .or_default() += delta;
        Ok(())
    }

    async fn insert_document(
        &mut self,
        document: ResolvedDocument,
    ) -> anyhow::Result<ResolvedDocumentId> {
        SchemaModel::new(self).enforce(&document).await?;
        let document_id = *document.id();
        self.apply_validated_write(document_id, None, Some(document))?;
        Ok(document_id)
    }

    pub async fn search(
        &mut self,
        search: &Search,
        version: SearchVersion,
    ) -> anyhow::Result<Vec<(CandidateRevision, IndexKeyBytes)>> {
        // If the table doesn't exist, short circuit to avoid erroring in the
        // table_mapping. Also take a dependency on the table not existing.
        if !TableModel::new(self).table_exists(search.index_name.table()) {
            return Ok(vec![]);
        }
        let search = search
            .clone()
            .to_internal(&self.table_mapping().name_to_id_user_input())?;
        let index_name = search.index_name.clone();
        self.index
            .search(&mut self.reads, &search, index_name, version)
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

    pub fn record_read_document_user_facing(
        &mut self,
        document: &DeveloperDocument,
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

    /// NOTE: returns a page of results. Callers must call record_read_document
    /// for all documents returned from the index stream.
    #[convex_macro::instrument_future]
    pub async fn index_range_user_facing(
        &mut self,
        stable_index_name: &StableIndexName,
        interval: &Interval,
        order: Order,
        mut max_rows: usize,
        version: Option<Version>,
    ) -> anyhow::Result<(
        Vec<(IndexKeyBytes, DeveloperDocument, WriteTimestamp)>,
        Interval,
    )> {
        if interval.is_empty() {
            return Ok((vec![], Interval::empty()));
        }

        max_rows = cmp::min(max_rows, MAX_PAGE_SIZE);

        let tablet_index_name = match stable_index_name {
            StableIndexName::Physical(tablet_index_name) => tablet_index_name,
            StableIndexName::Virtual(index_name, tablet_index_name) => {
                log_virtual_table_query();
                return VirtualTable::new(self)
                    .index_range(
                        index_name,
                        tablet_index_name,
                        interval,
                        order,
                        max_rows,
                        version,
                    )
                    .await;
            },
            StableIndexName::Missing => {
                return Ok((vec![], Interval::empty()));
            },
        };
        let index_name = tablet_index_name
            .clone()
            .map_table(&self.table_mapping().tablet_to_name())?;

        let (results, interval_remaining) = self
            .index
            .range(
                &mut self.reads,
                tablet_index_name,
                &index_name,
                interval,
                order,
                max_rows,
            )
            .await?;
        let developer_results = results
            .into_iter()
            .map(|(key, doc, ts)| {
                let doc = doc.to_developer();
                anyhow::Ok((key, doc, ts))
            })
            .try_collect()?;
        Ok((developer_results, interval_remaining))
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

    /// NOTE: returns a page of results. Callers must call record_read_document
    /// for all documents returned from the index stream.
    #[convex_macro::instrument_future]
    pub async fn index_range(
        &mut self,
        stable_index_name: &StableIndexName,
        interval: &Interval,
        order: Order,
        mut max_rows: usize,
    ) -> anyhow::Result<(
        Vec<(IndexKeyBytes, ResolvedDocument, WriteTimestamp)>,
        Interval,
    )> {
        if interval.is_empty() {
            return Ok((vec![], Interval::empty()));
        }
        let tablet_index_name = match stable_index_name {
            StableIndexName::Physical(tablet_index_name) => tablet_index_name,
            StableIndexName::Virtual(..) => {
                anyhow::bail!(
                    "Can't query virtual tables from index_range (index_range_user_facing can)"
                );
            },
            StableIndexName::Missing => {
                return Ok((vec![], Interval::empty()));
            },
        };
        let index_name = tablet_index_name
            .clone()
            .map_table(&self.table_mapping().tablet_to_name())?;
        max_rows = cmp::min(max_rows, MAX_PAGE_SIZE);

        self.index
            .range(
                &mut self.reads,
                tablet_index_name,
                &index_name,
                interval,
                order,
                max_rows,
            )
            .await
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
            .map(|(id, _)| id.table().table_id)
            .collect();
        Self::validate_memory_index_size(
            table_mapping,
            transaction,
            &modified_tables,
            base_snapshot.search_indexes.in_memory_sizes(),
            search_size_limit,
            "Search",
        )?;
        Self::validate_memory_index_size(
            table_mapping,
            transaction,
            &modified_tables,
            base_snapshot.vector_indexes.in_memory_sizes().into_iter(),
            vector_size_limit,
            "Vector",
        )?;
        Ok(())
    }

    fn validate_memory_index_size<RT: Runtime>(
        table_mapping: &TableMapping,
        transaction: &Transaction<RT>,
        modified_tables: &BTreeSet<TableId>,
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
            let index = transaction
                .index
                .index_registry()
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