use std::collections::{
    BTreeMap,
    HashMap,
};

use anyhow::Context;
use common::{
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexState,
            DeveloperDatabaseIndexConfig,
            IndexedFields,
        },
        index_validation_error,
        search_index::{
            DeveloperSearchIndexConfig,
            SearchIndexState,
        },
        vector_index::{
            DeveloperVectorIndexConfig,
            VectorIndexState,
        },
        DeveloperIndexConfig,
        DeveloperIndexMetadata,
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
        INDEX_TABLE,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        Order,
        Query,
    },
    runtime::Runtime,
    schemas::{
        DatabaseSchema,
        TableDefinition,
    },
    types::{
        IndexDiff,
        IndexId,
        IndexName,
        StableIndexName,
        TableName,
        TabletIndexName,
    },
    value::TableIdentifier,
};
use errors::ErrorMetadata;
use indexing::{
    backend_in_memory_indexes::index_not_a_database_index_error,
    index_registry::{
        index_not_found_error,
        Index,
    },
};
use value::{
    ResolvedDocumentId,
    TableId,
    TableMapping,
};

use crate::{
    defaults::{
        SystemIndex,
        SystemTable,
    },
    query::TableFilter,
    reads::TransactionReadSet,
    transaction_index::TransactionIndex,
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    TableModel,
    Transaction,
};

// NB: This excludes the default index we add to every table.
pub const MAX_USER_INDEXES: usize = 256;

pub struct IndexTable;
impl SystemTable for IndexTable {
    fn table_name(&self) -> &'static TableName {
        &INDEX_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<TabletIndexMetadata>::try_from(document).map(|_| ())
    }
}

pub struct IndexModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> IndexModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    /// Register given index.
    /// Indexes won't be backfilled and available for queries until after the
    /// transaction has committed.
    pub async fn add_application_index(
        &mut self,
        index: IndexMetadata<TableName>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        anyhow::ensure!(
            self.tx.identity().is_admin() || self.tx.identity().is_system(),
            unauthorized_error("add_index")
        );
        anyhow::ensure!(!index.name.is_system_owned(), "Can't change system indexes");
        let num_user_indexes = self.get_application_indexes().await?.len();
        anyhow::ensure!(
            num_user_indexes < MAX_USER_INDEXES,
            index_validation_error::too_many_total_user_indexes(MAX_USER_INDEXES),
        );
        self._add_index(index).await
    }

    /// Add system index.
    /// Indexes won't be backfilled and available for queries until
    /// after the transaction has committed.
    pub async fn add_system_index(
        &mut self,
        index: IndexMetadata<TableName>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.tx.identity().is_admin() || self.tx.identity().is_system(),
            unauthorized_error("add_system_index")
        );
        self._add_index(index).await?;
        Ok(())
    }

    async fn _add_index(
        &mut self,
        index: IndexMetadata<TableName>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        // Make sure the table exists before creating the index.
        TableModel::new(self.tx)
            .insert_table_metadata(index.name.table())
            .await?;
        let index: TabletIndexMetadata = index
            .map_table(&self.tx.table_mapping().name_to_id())?
            .into();
        SystemMetadataModel::new(self.tx)
            .insert_metadata(&INDEX_TABLE, index.try_into()?)
            .await
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn enable_index_for_testing(&mut self, index: &IndexName) -> anyhow::Result<()> {
        let metadata = self
            .pending_index_metadata(index)?
            .ok_or_else(|| anyhow::anyhow!("Failed to find pending index: {}", index))?;
        self.enable_index(&metadata.into_value()).await
    }

    async fn enable_index(&mut self, backfilled_index: &TabletIndexMetadata) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.tx.identity().is_admin() || self.tx.identity().is_system(),
            unauthorized_error("enable_index")
        );
        anyhow::ensure!(
            !backfilled_index.name.is_by_id_or_creation_time(),
            "Can't change system indexes"
        );

        let mut doc: ParsedDocument<TabletIndexMetadata> = self
            .pending_resolved_index_metadata(&backfilled_index.name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing pending data for index: {:?}",
                    backfilled_index.name
                )
            })?;
        match doc.config {
            IndexConfig::Database {
                ref mut on_disk_state,
                ..
            } => match on_disk_state {
                DatabaseIndexState::Backfilling(_) | DatabaseIndexState::Enabled => {
                    anyhow::bail!(
                        "Expected backfilled index, but found: {:?} for {:?}",
                        on_disk_state,
                        backfilled_index.name.descriptor()
                    )
                },
                DatabaseIndexState::Backfilled => {
                    *on_disk_state = DatabaseIndexState::Enabled;
                },
            },
            IndexConfig::Search {
                ref mut on_disk_state,
                ..
            } => match on_disk_state {
                SearchIndexState::Backfilled(snapshot) => {
                    *on_disk_state = SearchIndexState::SnapshottedAt(snapshot.clone());
                },
                SearchIndexState::Backfilling | SearchIndexState::SnapshottedAt(_) => {
                    anyhow::bail!(
                        "Expected backfilled index, but found: {on_disk_state:?} for {:?}",
                        backfilled_index.name.descriptor()
                    )
                },
            },
            IndexConfig::Vector {
                ref mut on_disk_state,
                ..
            } => match on_disk_state {
                VectorIndexState::Backfilled(snapshot) => {
                    *on_disk_state = VectorIndexState::SnapshottedAt(snapshot.clone());
                },
                VectorIndexState::Backfilling(_) | VectorIndexState::SnapshottedAt(_) => {
                    anyhow::bail!(
                        "Expected backfilled index, but found: {on_disk_state:?} for {:?}",
                        backfilled_index.name.descriptor()
                    )
                },
            },
        };

        let id = doc.id();
        let table_id_metadata: TabletIndexMetadata = doc.into_value();
        SystemMetadataModel::new(self.tx)
            .replace(id, table_id_metadata.try_into()?)
            .await?;

        Ok(())
    }

    // This method assumes it's being called in apply_config, or at least after
    // indexes have been added and backfilled.
    pub async fn commit_indexes_for_schema(
        &mut self,
        tables_in_schema: &BTreeMap<TableName, TableDefinition>,
    ) -> anyhow::Result<IndexDiff> {
        let index_diff: IndexDiff = self.get_index_diff(tables_in_schema).await?;

        let IndexDiff {
            added,
            identical,
            dropped,
        } = &index_diff;

        // New indexes should all have been added in prepare_schema, something has gone
        // wrong if we're trying to commit and realize there's a new index.
        anyhow::ensure!(
            added.is_empty(),
            "Trying to add new indexes when committing"
        );

        let only_dropped_tables = LegacyIndexDiff {
            added: vec![],
            dropped: dropped.clone(),
        };
        self.apply_index_diff(&only_dropped_tables).await?;

        // Added indexes should have backfilled via build_indexes
        // (for < 0.14.0 CLIs) or in apply_config (for >= 0.14.0 CLIs).
        let indexes_to_enable = identical
            .iter()
            .filter(|index| !index.config.is_enabled())
            .map(|doc| doc.clone().into_value())
            .collect();
        self.enable_backfilled_indexes(indexes_to_enable).await?;

        Ok(index_diff)
    }

    // Enables the given set of indexes if they're backfilled.
    // Asserts that the given indexes are database indexes (not search indexes)
    // and that they are in the Backfilled state.
    pub async fn enable_backfilled_indexes(
        &mut self,
        indexes: Vec<TabletIndexMetadata>,
    ) -> anyhow::Result<()> {
        for index in indexes {
            self.enable_index(&index).await?;
        }
        Ok(())
    }

    pub async fn apply_index_diff(&mut self, diff: &LegacyIndexDiff) -> anyhow::Result<()> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("modify_indexes"));
        }
        for index in &diff.dropped {
            self.drop_index(index.id()).await?;
        }
        for index in &diff.added {
            self.add_application_index(index.clone()).await?;
        }

        Ok(())
    }

    ///  Given a set of tables from a not yet fully committed schema,
    ///  returns the difference between the indexes in those not yet committed
    ///  tables and the indexes in storage. We compare only the developer config
    ///  and pending/enablded state of the indexes to determine the diff.
    pub async fn get_index_diff(
        &mut self,
        tables_in_schema: &BTreeMap<TableName, TableDefinition>,
    ) -> anyhow::Result<IndexDiff> {
        let mut indexes_in_schema: Vec<IndexMetadata<TableName>> = Vec::new();
        for (table_name, table_schema) in tables_in_schema {
            // Collect the database indexes.
            for (index_descriptor, index_schema) in &table_schema.indexes {
                let index_name = IndexName::new(table_name.clone(), index_descriptor.clone())?;
                indexes_in_schema.push(IndexMetadata::new_backfilling(
                    index_name.clone(),
                    index_schema.fields.clone(),
                ))
            }

            // Collect the search indexes.
            for (index_descriptor, index_schema) in &table_schema.search_indexes {
                let index_name = IndexName::new(table_name.clone(), index_descriptor.clone())?;
                indexes_in_schema.push(IndexMetadata::new_backfilling_search_index(
                    index_name.clone(),
                    index_schema.search_field.clone(),
                    index_schema.filter_fields.clone(),
                ))
            }
            for (index_descriptor, index_schema) in &table_schema.vector_indexes {
                let index_name = IndexName::new(table_name.clone(), index_descriptor.clone())?;
                indexes_in_schema.push(IndexMetadata::new_backfilling_vector_index(
                    index_name.clone(),
                    index_schema.vector_field.clone(),
                    index_schema.dimension,
                    index_schema.filter_fields.clone(),
                ));
            }
        }

        let mut diff = IndexDiff::default();

        let mut remaining_indexes: HashMap<IndexName, Vec<ParsedDocument<DeveloperIndexMetadata>>> =
            HashMap::new();
        for index in self.get_application_indexes().await? {
            remaining_indexes
                .entry(index.name.clone())
                .or_default()
                .push(index);
        }

        for new_index in indexes_in_schema {
            remaining_indexes.remove(&new_index.name);

            match self.compare_new_and_existing_indexes(new_index)? {
                IndexComparison::Added(index) => diff.added.push(index),
                IndexComparison::Identical(index) => diff.identical.push(index),
                IndexComparison::Replaced {
                    replaced,
                    replacement,
                } => {
                    for doc in replaced {
                        diff.dropped
                            .push(TableModel::new(self.tx).doc_table_id_to_name(doc)?);
                    }
                    match replacement {
                        ReplacementIndex::NewOrUpdated(index) => diff.added.push(index),
                        ReplacementIndex::Identical(index) => diff.identical.push(index),
                    }
                },
            }
        }

        for (name, mut indexes) in remaining_indexes {
            anyhow::ensure!(
                !name.is_system_owned(),
                "Preparing to drop a system index: {:?}",
                name,
            );
            diff.dropped.append(&mut indexes);
        }

        Ok(diff)
    }

    fn compare_new_and_existing_indexes(
        &mut self,
        new_index: DeveloperIndexMetadata,
    ) -> anyhow::Result<IndexComparison> {
        let pending_index = self.pending_index_metadata(&new_index.name)?;
        let enabled_index = self.enabled_index_metadata(&new_index.name)?;

        fn identical_or_replaced(
            existing_index: ParsedDocument<TabletIndexMetadata>,
            new_index: DeveloperIndexMetadata,
        ) -> IndexComparison {
            if identical_dev_configs(&existing_index, &new_index) {
                IndexComparison::Identical(existing_index)
            } else {
                IndexComparison::Replaced {
                    replaced: vec![existing_index],
                    replacement: ReplacementIndex::NewOrUpdated(new_index),
                }
            }
        }

        Ok(match (enabled_index, pending_index) {
            (None, None) => IndexComparison::Added(new_index),
            (Some(enabled_index), None) => identical_or_replaced(enabled_index, new_index),
            (None, Some(pending_index)) => identical_or_replaced(pending_index, new_index),
            (Some(enabled_index), Some(pending_index)) => {
                if identical_dev_configs(&pending_index, &new_index) {
                    IndexComparison::Replaced {
                        replaced: vec![enabled_index],
                        replacement: ReplacementIndex::Identical(pending_index),
                    }
                } else {
                    IndexComparison::Replaced {
                        replaced: vec![enabled_index, pending_index],
                        replacement: ReplacementIndex::NewOrUpdated(new_index),
                    }
                }
            },
        })
    }

    /// Inserts new and updated mutated indexes so they can be backfilled.
    /// Returns the complete index diff, even though only the additions are
    /// immediately applied (the rest will be applied in apply_config)
    pub async fn prepare_new_and_mutated_indexes(
        &mut self,
        schema: &DatabaseSchema,
    ) -> anyhow::Result<IndexDiff> {
        let diff: IndexDiff = self.get_index_diff(&schema.tables).await?;

        // If an index is currently pending and we're mutating it, we need to drop the
        // currently pending index immediately so that we avoid having multiple pending
        // indexes with the same name. This is still atomic(ish) because users cannot be
        // using the pending indexes we drop here.
        let dropped: Vec<ParsedDocument<IndexMetadata<TableName>>> = diff
            .dropped
            .iter()
            .filter(|index| !index.config.is_enabled())
            .cloned()
            .collect();

        let added = diff.added.clone();

        let only_new_and_mutated = LegacyIndexDiff { added, dropped };
        tracing::info!(
            "Preparing new and mutated indexes, overall diff: {diff:?}, applying right now: \
             {only_new_and_mutated:?}"
        );
        // Dropped will be removed in apply_config when the rest of the schema is
        // committed.
        if !only_new_and_mutated.is_empty() {
            self.apply_index_diff(&only_new_and_mutated).await?;
        }
        Ok(diff)
    }

    pub async fn build_indexes(
        &mut self,
        schema: &DatabaseSchema,
    ) -> anyhow::Result<LegacyIndexDiff> {
        let diff: LegacyIndexDiff = self.get_index_diff(&schema.tables).await?.into();

        if diff.is_empty() {
            return Ok(diff);
        }
        self.apply_index_diff(&diff).await?;
        Ok(diff)
    }

    pub fn indexed_fields(
        &mut self,
        stable_index_name: &StableIndexName,
        printable_index_name: &IndexName,
    ) -> anyhow::Result<IndexedFields> {
        let resolved_index_name = match stable_index_name {
            StableIndexName::Physical(index_name) => index_name,
            StableIndexName::Virtual(_, index_name) => index_name,
            StableIndexName::Missing => anyhow::bail!(index_not_found_error(printable_index_name)),
        };
        let metadata =
            self.require_enabled_index_metadata(printable_index_name, resolved_index_name)?;
        match metadata.config.clone() {
            IndexConfig::Database {
                developer_config: DeveloperDatabaseIndexConfig { fields },
                ..
            } => Ok(fields),
            _ => anyhow::bail!(index_not_a_database_index_error(printable_index_name)),
        }
    }

    /// Returns the index metadata for the given name if it's enabled or fails
    /// with a descriptive error if the index is either missing or not
    /// enabled.
    ///
    /// Queries and most other use cases should use this method or
    /// enable_index_metadata isntead of pending_index_metadata.
    fn require_enabled_index_metadata(
        &mut self,
        printable_index_name: &IndexName,
        resolved_index_name: &TabletIndexName,
    ) -> anyhow::Result<ParsedDocument<TabletIndexMetadata>> {
        // Because require_enabled does not mutate table_mapping, we can clone it here
        // to avoid duplicate mutable references to self.tx. If require_enabled
        // ever does start mutating the mapping, then this would be unsafe.
        Ok(self
            .tx
            .index
            .require_enabled(
                &mut self.tx.reads,
                resolved_index_name,
                printable_index_name,
            )?
            .metadata()
            .clone())
    }

    /// Returns the index metadata for the given name if it's in the enabled
    /// state or None if the index either can't be found or is not enabled.
    ///
    /// Queries and most other use cases should use
    /// require_enabled_index_metadata or this method instead of
    /// pending_index_metadata.
    pub fn enabled_index_metadata(
        &mut self,
        index_name: &IndexName,
    ) -> anyhow::Result<Option<ParsedDocument<TabletIndexMetadata>>> {
        self._index_metadata(index_name, |indexes, reads, index_name| {
            indexes.get_enabled(reads, &index_name)
        })
    }

    /// Returns the index metadata for the given name if it's not yet enabled or
    /// None if the index either can't be found or is enabled.
    ///
    /// Only use this method when you're mutating indexes or their state.
    /// Queries and most other use cases should use enabled_index_metadata
    /// or require_enabled_index_metadata instead.
    pub fn pending_index_metadata(
        &mut self,
        index_name: &IndexName,
    ) -> anyhow::Result<Option<ParsedDocument<TabletIndexMetadata>>> {
        self._index_metadata(index_name, |indexes, reads, index_name| {
            indexes.get_pending(reads, &index_name)
        })
    }

    fn pending_resolved_index_metadata(
        &mut self,
        index_name: &TabletIndexName,
    ) -> Option<ParsedDocument<TabletIndexMetadata>> {
        let index = self.tx.index.get_pending(&mut self.tx.reads, index_name)?;
        Some(index.metadata.clone())
    }

    fn _index_metadata<'b>(
        &'b mut self,
        index_name: &IndexName,
        getter: impl FnOnce(
            &'b mut TransactionIndex,
            &'b mut TransactionReadSet,
            TabletIndexName,
        ) -> Option<&'b Index>,
    ) -> anyhow::Result<Option<ParsedDocument<TabletIndexMetadata>>> {
        if !self.tx.table_mapping().name_exists(index_name.table()) {
            return Ok(None);
        }
        let index_name = self.resolve_index_name(index_name)?;
        Ok(getter(&mut self.tx.index, &mut self.tx.reads, index_name)
            .map(|index| index.metadata.clone()))
    }

    pub fn stable_index_name(
        &mut self,
        index_name: &IndexName,
        table_filter: TableFilter,
    ) -> anyhow::Result<StableIndexName> {
        if self
            .tx
            .virtual_table_mapping()
            .name_exists(index_name.table())
        {
            let physical_index_name = self
                .tx
                .virtual_system_mapping()
                .virtual_to_system_index(index_name)?
                .clone();
            Ok(StableIndexName::Virtual(
                index_name.clone(),
                self.resolve_index_name(&physical_index_name)?,
            ))
        } else if self.tx.table_mapping().name_exists(index_name.table()) {
            match table_filter {
                TableFilter::IncludePrivateSystemTables => Ok(StableIndexName::Physical(
                    self.resolve_index_name(index_name)?,
                )),
                TableFilter::ExcludePrivateSystemTables => {
                    if index_name.table().is_system() {
                        Ok(StableIndexName::Missing)
                    } else {
                        Ok(StableIndexName::Physical(
                            self.resolve_index_name(index_name)?,
                        ))
                    }
                },
            }
        } else {
            Ok(StableIndexName::Missing)
        }
    }

    fn resolve_index_name(&mut self, index_name: &IndexName) -> anyhow::Result<TabletIndexName> {
        let resolved = index_name
            .clone()
            .map_table(&self.tx.table_mapping().name_to_id())?;
        Ok(resolved.into())
    }

    /// Returns by_id indexes for *all tablets*, including hidden ones.
    pub async fn by_id_indexes(&mut self) -> anyhow::Result<BTreeMap<TableId, IndexId>> {
        let all_indexes = self.get_all_indexes().await?;
        Ok(all_indexes
            .into_iter()
            .filter(|index| index.name.is_by_id())
            .map(|index| (*index.name.table(), index.id().internal_id()))
            .collect())
    }

    pub async fn by_id_index_metadata(
        &mut self,
        table_id: TableId,
    ) -> anyhow::Result<ParsedDocument<TabletIndexMetadata>> {
        self.all_indexes_on_table(table_id)
            .await?
            .into_iter()
            .find(|index| index.name.is_by_id())
            .ok_or_else(|| anyhow::anyhow!("by_id index missing for {table_id}"))
    }

    /// All indexes (system and developer-defined and
    /// backfilling/backfilled/enabled) for a single table.
    pub async fn all_indexes_on_table(
        &mut self,
        table_id: TableId,
    ) -> anyhow::Result<Vec<ParsedDocument<TabletIndexMetadata>>> {
        let all_indexes = self.get_all_indexes().await?;
        Ok(all_indexes
            .into_iter()
            .filter(|index| *index.name.table() == table_id)
            .collect())
    }

    /// Returns all registered indexes (both system and developer-defined)
    /// including both pending and enabled indexes.
    ///
    /// Because of mutated indexes, there may be up to two indexes with the same
    /// name (but different configurations).
    pub async fn get_all_indexes(
        &mut self,
    ) -> anyhow::Result<Vec<ParsedDocument<TabletIndexMetadata>>> {
        // Index doesn't have `by_creation_time` index, and thus can't be queried via
        // collect.
        let index_query = Query::index_range(IndexRange {
            index_name: IndexName::by_id(INDEX_TABLE.clone()),
            range: vec![],
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, index_query)?;

        let mut indexes = vec![];
        while let Some(document) = query_stream.next(self.tx, None).await? {
            let index = TabletIndexMetadata::from_document(document)?;
            indexes.push(index);
        }
        Ok(indexes)
    }

    /// Returns the index metadata matching the given id or fails if the
    /// document is missing or not an index.
    pub async fn require_index_by_id(
        &mut self,
        index_id: ResolvedDocumentId,
    ) -> anyhow::Result<ParsedDocument<TabletIndexMetadata>> {
        let document = self
            .tx
            .get(index_id)
            .await?
            .with_context(|| format!("Missing document for index id {index_id:?}"))?;
        TabletIndexMetadata::from_document(document)
    }

    /// Returns all registered indexes that are system owned including both
    /// pending and enabled indexes.
    ///
    /// Because of mutated indexes, there may be up to two indexes with the same
    /// name (but different configurations).
    pub async fn get_system_indexes(
        &mut self,
    ) -> anyhow::Result<Vec<ParsedDocument<DeveloperIndexMetadata>>> {
        self.get_indexes(IndexCategory::System).await
    }

    /// Returns all registered indexes that aren't system owned including both
    /// pending and enabled indexes.
    ///
    /// Because of mutated indexes, there may be up to two indexes with the same
    /// name (but different configurations).
    pub async fn get_application_indexes(
        &mut self,
    ) -> anyhow::Result<Vec<ParsedDocument<DeveloperIndexMetadata>>> {
        self.get_indexes(IndexCategory::Application).await
    }

    async fn get_indexes(
        &mut self,
        category: IndexCategory,
    ) -> anyhow::Result<Vec<ParsedDocument<DeveloperIndexMetadata>>> {
        let indexes_with_id = self.get_all_indexes().await?;
        let table_mapping = self.tx.table_mapping();
        indexes_with_id
            .into_iter()
            .filter(|doc| category.belongs(doc, table_mapping))
            .map(|doc: ParsedDocument<TabletIndexMetadata>| {
                doc.map(|metadata| metadata.map_table(&table_mapping.tablet_to_name()))
            })
            .try_collect()
    }

    pub async fn drop_index(&mut self, index_id: ResolvedDocumentId) -> anyhow::Result<()> {
        SystemMetadataModel::new(self.tx).delete(index_id).await?;
        Ok(())
    }

    pub async fn drop_system_index(&mut self, index_name: IndexName) -> anyhow::Result<()> {
        anyhow::ensure!(index_name.table().is_system());
        let system_index = self
            .get_system_indexes()
            .await?
            .into_iter()
            .find(|index| index.name == index_name);
        if let Some(system_index) = system_index {
            tracing::info!("Dropping system index {index_name}");
            self.drop_index(system_index.id()).await?;
        }
        Ok(())
    }

    pub async fn copy_indexes_to_table(
        &mut self,
        source_table: &TableName,
        target_table: TableId,
    ) -> anyhow::Result<()> {
        // Copy over enabled indexes from existing active table, if any.
        let Some(active_table_id) = self.tx.table_mapping().id_if_exists(source_table) else {
            return Ok(());
        };
        for index in IndexModel::new(self.tx)
            .all_indexes_on_table(active_table_id)
            .await?
        {
            anyhow::ensure!(
                !index.config.is_backfilling(),
                ErrorMetadata::bad_request(
                    "InvalidImport",
                    format!(
                        "{source_table} is still backfilling indexes, so it cannot be replaced. \
                         Wait for indexes to complete backfilling"
                    )
                )
            );
            if !index.config.is_enabled() {
                // Only copy Enabled indexes, otherwise we might get naming conflicts.
                continue;
            }
            if index.name.is_by_id_or_creation_time() {
                // by_id and by_creation_time already created.
                continue;
            }
            let index_name = TabletIndexName::new(target_table, index.name.descriptor().clone())?;
            let metadata = match index.into_value().config {
                // Table is empty, so it's okay to create indexes in state Enabled.
                IndexConfig::Database {
                    developer_config: DeveloperDatabaseIndexConfig { fields },
                    ..
                } => IndexMetadata::new_backfilling(index_name, fields),
                IndexConfig::Search {
                    developer_config:
                        DeveloperSearchIndexConfig {
                            search_field,
                            filter_fields,
                        },
                    ..
                } => IndexMetadata::new_backfilling_search_index(
                    index_name,
                    search_field,
                    filter_fields,
                ),
                IndexConfig::Vector {
                    developer_config:
                        DeveloperVectorIndexConfig {
                            dimensions,
                            vector_field,
                            filter_fields,
                        },
                    ..
                } => IndexMetadata::new_backfilling_vector_index(
                    index_name,
                    vector_field,
                    dimensions,
                    filter_fields,
                ),
            };
            SystemMetadataModel::new(self.tx)
                .insert_metadata(&INDEX_TABLE, metadata.try_into()?)
                .await?;
        }
        Ok(())
    }
}

enum IndexCategory {
    System,
    Application,
}

impl IndexCategory {
    fn belongs(
        &self,
        index: &ParsedDocument<TabletIndexMetadata>,
        table_mapping: &TableMapping,
    ) -> bool {
        let is_system = index.name.descriptor().is_reserved()
            || table_mapping.is_system_table_id(*index.name.table());
        let is_active = table_mapping.is_active(*index.name.table());
        is_active
            && match self {
                Self::System => is_system,
                Self::Application => !is_system,
            }
    }
}

fn identical_dev_configs<T: TableIdentifier, Y: TableIdentifier>(
    existing: &ParsedDocument<IndexMetadata<T>>,
    new: &IndexMetadata<Y>,
) -> bool {
    DeveloperIndexConfig::from(existing.config.clone())
        == DeveloperIndexConfig::from(new.config.clone())
}

enum IndexComparison {
    Added(DeveloperIndexMetadata),
    Identical(ParsedDocument<TabletIndexMetadata>),
    Replaced {
        replaced: Vec<ParsedDocument<TabletIndexMetadata>>,
        replacement: ReplacementIndex,
    },
}

enum ReplacementIndex {
    /// This replacement index is not yet in storage or its definition has
    /// changed.
    NewOrUpdated(DeveloperIndexMetadata),
    /// The replacement index is in storage and its definition has not changed.
    Identical(ParsedDocument<TabletIndexMetadata>),
}

// A LegacyIndexDiff includes mutated indexes in both added and dropped. We need
// to eventually migrate away from this behavior and special case mutations.
// For now that means we need to retain this legacy diffing behavior.
#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct LegacyIndexDiff {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::vec(
            proptest::prelude::any::<IndexMetadata<TableName>>(),
            0..4,
        )")
    )]
    pub added: Vec<IndexMetadata<TableName>>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::vec(
            proptest::prelude::any::<ParsedDocument<IndexMetadata<TableName>>>(),
            0..4,
        )")
    )]
    pub dropped: Vec<ParsedDocument<IndexMetadata<TableName>>>,
}

impl LegacyIndexDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.dropped.is_empty()
    }
}

impl From<IndexDiff> for LegacyIndexDiff {
    fn from(diff: IndexDiff) -> Self {
        Self {
            added: diff.added,
            dropped: diff.dropped,
        }
    }
}
