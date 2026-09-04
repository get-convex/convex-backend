use std::{
    collections::BTreeSet,
    sync::LazyLock,
};

use anyhow::Context;
use async_recursion::async_recursion;
use common::{
    bootstrap_model::{
        index::{
            database_index::IndexedFields,
            index_validation_error,
            DeveloperIndexMetadata,
            IndexMetadata,
            TabletIndexMetadata,
            INDEX_TABLE,
        },
        tables::{
            TableMetadata,
            TableState,
            TABLES_TABLE,
        },
    },
    components::ComponentPath,
    document::{
        ParseDocument,
        ParsedDocument,
    },
    interval::Interval,
    runtime::Runtime,
    types::{
        GenericIndexName,
        TableName,
        TabletIndexName,
    },
    value::TabletIdAndTableNumber,
    virtual_system_mapping::VirtualSystemMapping,
};
use errors::ErrorMetadata;
use value::{
    FieldPath,
    TableNamespace,
    TableNumber,
    TabletId,
};

use crate::{
    defaults::bootstrap_system_tables,
    system_tables::{
        SystemIndex,
        SystemTable,
    },
    table_summary::table_summary_bootstrapping_error,
    BootstrapComponentsModel,
    IndexModel,
    SchemaModel,
    SystemMetadataModel,
    Transaction,
};

/// Each instance is limited to a certain number of user tables.
pub const MAX_USER_TABLES: usize = 10000;

/// Reserve the first 10K for system tables. User tables start at 10K
/// Older instances (before ~12/2023) may have lower numbered user tables.
pub const NUM_RESERVED_SYSTEM_TABLE_NUMBERS: u32 = 10000;
/// Reserve the first 512 table numbers for legacy tables, before we created
/// default system table numbers and reserving numbers for system tables.
/// Instances created before ~01/2024 may have lower numbered user or system
/// tables, but instances created after will have all tables >512.
pub const NUM_RESERVED_LEGACY_TABLE_NUMBERS: u32 = 512;

pub static TABLES_BY_NAME_INDEX: LazyLock<SystemIndex<TablesTable>> =
    LazyLock::new(|| SystemIndex::new("by_name", [&NAME_FIELD_PATH]).unwrap());

pub static NAME_FIELD_PATH: LazyLock<FieldPath> =
    LazyLock::new(|| "name".parse().expect("Invalid built-in field"));

pub struct TablesTable;
impl SystemTable for TablesTable {
    type Metadata = TableMetadata;

    const TABLE_NAME: TableName = TABLES_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![TABLES_BY_NAME_INDEX.clone()]
    }
}

pub struct TableModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> TableModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    /// Returns the number of documents in the table, up-to-date with the
    /// current transaction.
    pub async fn count(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<Option<u64>> {
        let count = if let Some(tablet_id) = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .id_if_exists(table)
        {
            self.count_tablet(tablet_id).await?
        } else {
            Some(0)
        };

        Ok(count)
    }

    /// Returns the number of documents in the table, up-to-date with the
    /// current transaction.
    pub async fn must_count(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<u64> {
        self.count(namespace, table).await?.ok_or_else(|| {
            table_summary_bootstrapping_error(Some("Table count unavailable while bootstrapping"))
        })
    }

    /// Records a read on the entire by_id index of `tablet_id`.
    pub fn add_dependency_on_tablet(&mut self, tablet_id: TabletId) -> anyhow::Result<()> {
        self.tx.reads.record_indexed_directly(
            TabletIndexName::by_id(tablet_id),
            IndexedFields::by_id(),
            Interval::all(),
            &self.tx.limits,
        )?;
        Ok(())
    }

    pub async fn count_tablet(&mut self, tablet_id: TabletId) -> anyhow::Result<Option<u64>> {
        // Add read dependency on the entire table.
        // But we haven't explicitly read the documents, so don't record_read_documents.
        self.add_dependency_on_tablet(tablet_id)?;
        // Get table count at the beginning of the transaction, then add the delta from
        // the transaction so far.
        let snapshot_count = self.tx.count_snapshot.count(tablet_id).await?;
        let Some(snapshot_count) = snapshot_count else {
            return Ok(None);
        };
        let transaction_delta = self.tx.table_count_deltas.get(&tablet_id).unwrap_or(&0);
        let result = if *transaction_delta < 0 {
            snapshot_count
                .checked_sub(transaction_delta.unsigned_abs())
                .context("Count underflow")
        } else {
            snapshot_count
                .checked_add(*transaction_delta as u64)
                .context("Count overflow")
        };
        Ok(Some(result?))
    }

    pub async fn must_count_tablet(&mut self, tablet_id: TabletId) -> anyhow::Result<u64> {
        self.count_tablet(tablet_id).await?.ok_or_else(|| {
            table_summary_bootstrapping_error(Some("Table count unavailable while bootstrapping"))
        })
    }

    pub(crate) fn doc_table_id_to_name(
        &mut self,
        doc: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<ParsedDocument<DeveloperIndexMetadata>> {
        doc.map(|metadata| metadata.map_table(&self.tx.table_mapping().tablet_to_name()))
    }

    pub(crate) fn doc_table_name_to_id(
        &mut self,
        namespace: TableNamespace,
        doc: ParsedDocument<DeveloperIndexMetadata>,
    ) -> anyhow::Result<ParsedDocument<TabletIndexMetadata>> {
        doc.map(|metadata| {
            metadata.map_table(
                &self
                    .tx
                    .table_mapping()
                    .namespace(namespace)
                    .name_to_tablet(),
            )
        })
    }

    pub fn table_exists(&mut self, namespace: TableNamespace, table: &TableName) -> bool {
        self.tx
            .table_mapping()
            .namespace(namespace)
            .name_exists(table)
    }

    pub fn count_user_tables(&mut self) -> usize {
        self.tx.table_mapping().iter_active_user_tables().count()
    }

    pub async fn delete_active_table(
        &mut self,
        namespace: TableNamespace,
        table_name: TableName,
    ) -> anyhow::Result<()> {
        if !self.table_exists(namespace, &table_name) {
            return Ok(());
        }
        SchemaModel::new(self.tx, namespace)
            .enforce_table_deletion(table_name.clone())
            .await?;

        let table_id_and_number = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .id(&table_name)?;
        self.delete_table_by_id_bypassing_schema_enforcement(table_id_and_number.tablet_id)
            .await?;
        Ok(())
    }

    pub async fn delete_hidden_table(&mut self, tablet_id: TabletId) -> anyhow::Result<()> {
        let table_metadata = self.get_table_metadata(tablet_id).await?;
        // We don't need to validate hidden table with the schema.
        anyhow::ensure!(table_metadata.state == TableState::Hidden);
        self.delete_table_by_id_bypassing_schema_enforcement(tablet_id)
            .await?;
        Ok(())
    }

    pub async fn delete_table_by_id_bypassing_schema_enforcement(
        &mut self,
        tablet_id: TabletId,
    ) -> anyhow::Result<TableNumber> {
        for index in IndexModel::new(self.tx)
            .all_indexes_on_table(tablet_id)
            .await?
        {
            let index_id = index.id();
            SystemMetadataModel::new_global(self.tx)
                .delete(index_id)
                .await?;
        }
        let table_metadata = self.get_table_metadata(tablet_id).await?;
        let table_doc_id = table_metadata.id();
        let table_metadata = table_metadata.into_value();
        let updated_table_metadata = TableMetadata {
            name: table_metadata.name,
            number: table_metadata.number,
            state: TableState::Deleting,
            namespace: table_metadata.namespace,
        };
        SystemMetadataModel::new_global(self.tx)
            .replace(table_doc_id, updated_table_metadata.try_into()?)
            .await?;
        Ok(table_metadata.number)
    }

    /// Removes the `_tables` metadata document for a tablet whose documents
    /// document retention has finished deleting, finalizing the drop.
    ///
    /// This must not gate on the table mapping. A tablet in `Deleting` state is
    /// removed from the mapping in the commit that starts the drop and is never
    /// re-added (bootstrap skips `Deleting` tablets), so
    /// `table_mapping().tablet_id_exists(tablet_id)` is always false here and
    /// gating on it makes the whole method a no-op. Resolve the `_tables`
    /// document directly instead. A missing document means the tablet was
    /// already finalized -- the cleanup worker can be handed the same tablet
    /// more than once (e.g. re-swept by retention after a restart) -- which is a
    /// no-op so the call stays idempotent.
    ///
    /// Only call this once document retention has emptied the tablet.
    pub async fn hard_delete_tablet_document(&mut self, tablet_id: TabletId) -> anyhow::Result<()> {
        let table_doc_id = self.tx.bootstrap_tables().table_resolved_doc_id(tablet_id);
        let Some(doc) = self.tx.get(table_doc_id).await? else {
            return Ok(());
        };
        let doc: ParsedDocument<TableMetadata> = doc.parse()?;
        anyhow::ensure!(
            doc.state == TableState::Deleting,
            "Cannot hard-delete tablet {tablet_id} in state {:?}, expected Deleting",
            doc.state,
        );
        SystemMetadataModel::new(self.tx, TableNamespace::Global)
            .delete(doc.id())
            .await?;
        Ok(())
    }

    pub async fn get_table_metadata(
        &mut self,
        tablet_id: TabletId,
    ) -> anyhow::Result<ParsedDocument<TableMetadata>> {
        let table_doc_id = self.tx.bootstrap_tables().table_resolved_doc_id(tablet_id);
        self.tx
            .get(table_doc_id)
            .await?
            .context(format!("Couldn't find table metadata for {tablet_id}"))?
            .parse()
    }

    // Checks both _tables and _virtual_tables to find a non-conflicting table
    // number
    pub async fn next_user_table_number(
        &mut self,
        namespace: TableNamespace,
    ) -> anyhow::Result<TableNumber> {
        self.next_table_number(false, namespace).await
    }

    pub async fn next_system_table_number(
        &mut self,
        namespace: TableNamespace,
    ) -> anyhow::Result<TableNumber> {
        self.next_table_number(true, namespace).await
    }

    async fn next_table_number(
        &mut self,
        is_system: bool,
        namespace: TableNamespace,
    ) -> anyhow::Result<TableNumber> {
        let occupied_table_numbers: BTreeSet<TableNumber> = self
            .tx
            .query_system(
                TableNamespace::Global,
                &SystemIndex::<TablesTable>::by_creation_time(),
            )?
            .all()
            .await?
            .into_iter()
            .filter(|table_metadata| table_metadata.namespace == namespace)
            .map(|table_metadata| table_metadata.number)
            .collect();

        let mut candidate_table_number = TableNumber::try_from(if is_system {
            NUM_RESERVED_LEGACY_TABLE_NUMBERS
        } else {
            NUM_RESERVED_SYSTEM_TABLE_NUMBERS
        })?
        .increment()?;
        while occupied_table_numbers.contains(&candidate_table_number) {
            candidate_table_number = candidate_table_number.increment()?;
        }

        Ok(candidate_table_number)
    }

    /// Checks for table number conflicts when activating a table, e.g. snapshot
    /// import. No two active tables in a namespace can have the same table
    /// number.
    fn check_can_activate(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
        table_number: TableNumber,
    ) -> anyhow::Result<()> {
        let Some(existing_table_by_number) = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .name_by_number_if_exists(table_number)
            .cloned()
        else {
            // No existing table with this number.
            return Ok(());
        };
        let component_path = BootstrapComponentsModel::new(self.tx)
            .get_component_path(namespace.into())
            .unwrap_or_default();
        Err(Self::table_conflict_error(
            &self.tx.virtual_system_mapping,
            &component_path,
            table,
            &existing_table_by_number,
        )
        .into())
    }

    pub fn table_conflict_error(
        virtual_system_mapping: &VirtualSystemMapping,
        component_path: &ComponentPath,
        table: &TableName,
        existing_table: &TableName,
    ) -> ErrorMetadata {
        let in_component = component_path.in_component_str();
        let msg = if virtual_system_mapping
            .associated_virtual_table_name(existing_table)
            .is_some()
        {
            format!(
                "New table `{table}`{in_component} has IDs that conflict with existing system \
                 table"
            )
        } else if existing_table.is_system() {
            format!(
                "New table `{table}`{in_component} has IDs that conflict with existing internal \
                 table. Consider importing this table without `_id` fields or import into a new \
                 deployment."
            )
        } else {
            format!(
                "New table `{table}`{in_component} has IDs that conflict with existing table \
                 `{existing_table}`. To delete all existing tables, import with `npx convex \
                 import --replace-all`."
            )
        };
        ErrorMetadata::bad_request("TableConflict", msg)
    }

    pub async fn activate_tables(
        &mut self,
        tablet_ids: impl IntoIterator<Item = TabletId>,
    ) -> anyhow::Result<u64> {
        let mut documents_deleted = 0;
        let mut table_metadatas = vec![];
        // Delete all existing tables before activating the new ones.
        // This ensures that we never have duplicate table numbers, even temporarily.
        for tablet_id in tablet_ids {
            let table_metadata = self.get_table_metadata(tablet_id).await?;
            match table_metadata.state {
                TableState::Active => continue,
                TableState::Deleting => {
                    anyhow::bail!("cannot activate {} which is deleting", table_metadata.name)
                },
                TableState::Hidden => {},
            }
            let namespace = table_metadata.namespace;
            if self.table_exists(namespace, &table_metadata.name) {
                let existing_table_by_name = self
                    .tx
                    .table_mapping()
                    .namespace(namespace)
                    .id(&table_metadata.name)?;
                documents_deleted += self.must_count(namespace, &table_metadata.name).await?;
                self.delete_table_by_id_bypassing_schema_enforcement(
                    existing_table_by_name.tablet_id,
                )
                .await?;
            }
            table_metadatas.push(table_metadata);
        }
        for table_metadata in table_metadatas {
            self.check_can_activate(
                table_metadata.namespace,
                &table_metadata.name,
                table_metadata.number,
            )?;
            let (table_doc_id, mut table_metadata) = table_metadata.into_id_and_value();
            table_metadata.state = TableState::Active;
            SystemMetadataModel::new_global(self.tx)
                .replace(table_doc_id, table_metadata.try_into()?)
                .await?;
        }
        Ok(documents_deleted)
    }

    #[async_recursion]
    pub async fn insert_table_metadata(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<()> {
        // Don't implicitly create table metadata for system tables.
        if table.is_system() {
            return Ok(());
        }
        self._insert_table_metadata(namespace, table, None, TableState::Active)
            .await?;

        Ok(())
    }

    pub async fn replace_with_empty_table<S: SystemTable>(
        &mut self,
        _system_table: S,
        namespace: TableNamespace,
    ) -> anyhow::Result<()> {
        let tablet_id = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .name_to_tablet()(S::TABLE_NAME)?;

        let table_number = self
            .delete_table_by_id_bypassing_schema_enforcement(tablet_id)
            .await?;
        self._insert_table_metadata(
            namespace,
            &S::TABLE_NAME,
            Some(table_number),
            TableState::Active,
        )
        .await?;
        for index in S::indexes() {
            let index_metadata = IndexMetadata::new_enabled(
                index
                    .name
                    .map_table(&|_| anyhow::Ok(S::TABLE_NAME.clone()))?,
                index.fields,
            );
            IndexModel::new(self.tx)
                .add_system_index(namespace, index_metadata)
                .await?;
        }
        Ok(())
    }

    pub async fn insert_table_for_import(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
        table_number: Option<TableNumber>,
    ) -> anyhow::Result<TabletIdAndTableNumber> {
        anyhow::ensure!(
            bootstrap_system_tables()
                .iter()
                .all(|t| t.table_name() != *table),
            "Conflict with bootstrap system table {table}",
        );
        self._insert_table_metadata(namespace, table, table_number, TableState::Hidden)
            .await
    }

    async fn _insert_table_metadata(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
        table_number: Option<TableNumber>,
        state: TableState,
    ) -> anyhow::Result<TabletIdAndTableNumber> {
        if state == TableState::Active && self.table_exists(namespace, table) {
            let table_id = self.tx.table_mapping().namespace(namespace).id(table)?;
            anyhow::ensure!(table_number.is_none() || table_number == Some(table_id.table_number));
            Ok(table_id)
        } else {
            if !table.is_system() {
                anyhow::ensure!(
                    self.count_user_tables() < MAX_USER_TABLES,
                    index_validation_error::too_many_tables(MAX_USER_TABLES)
                );
            }
            let table_number = if let Some(table_number) = table_number {
                anyhow::ensure!(
                    state == TableState::Hidden
                        || !self
                            .tx
                            .table_mapping()
                            .namespace(namespace)
                            .table_number_exists()(table_number),
                    ErrorMetadata::bad_request(
                        "InvalidId",
                        format!("conflict trying to create {table} with number {table_number}")
                    )
                );
                table_number
            } else {
                self.next_user_table_number(namespace).await?
            };
            let table_metadata =
                TableMetadata::new_with_state(namespace, table.clone(), table_number, state);
            let table_doc_id = SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&TABLES_TABLE, table_metadata.try_into()?)
                .await?;
            let tablet_id = TabletId(table_doc_id.internal_id());

            // Add the system defined indexes for the newly created table. Since the newly
            // created table is empty, we can start these indexes as `Enabled`.
            let by_id = GenericIndexName::by_id(tablet_id);
            let metadata = IndexMetadata::new_enabled(by_id, IndexedFields::by_id());
            SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            let by_creation_time = GenericIndexName::by_creation_time(tablet_id);
            let metadata =
                IndexMetadata::new_enabled(by_creation_time, IndexedFields::creation_time());
            SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            Ok(TabletIdAndTableNumber {
                tablet_id,
                table_number,
            })
        }
    }
}

/// Regression tests for `hard_delete_tablet_document` (get-convex/convex-backend#515).
///
/// Gated on `feature = "testing"` (like `indexing`'s `shuttle_tests`) so the
/// default `cargo test -p database` build stays unaffected. Requires Convex's
/// internal test harness -- `DbFixtures`, `TestFacingModel` and
/// `#[convex_macro::test_runtime]` -- which is compiled under that feature; see
/// SPEC §C6.
#[cfg(all(test, feature = "testing"))]
mod tests {
    use common::{
        bootstrap_model::tables::{
            TableMetadata,
            TableState,
        },
        document::{
            ParseDocument,
            ParsedDocument,
        },
        types::TableName,
    };
    use keybroker::Identity;
    use runtime::testing::TestRuntime;
    use value::{
        assert_obj,
        TableNamespace,
        TabletId,
    };

    use crate::{
        test_helpers::DbFixtures,
        TableModel,
        TestFacingModel,
        Transaction,
    };

    /// State of a tablet's `_tables` metadata document, or `None` once it has
    /// been hard-deleted. Resolves the document directly, never via the table
    /// mapping (which never contains `Deleting` tablets).
    async fn tablet_doc_state(
        tx: &mut Transaction<TestRuntime>,
        tablet_id: TabletId,
    ) -> anyhow::Result<Option<TableState>> {
        let id = tx.bootstrap_tables().table_resolved_doc_id(tablet_id);
        let Some(doc) = tx.get(id).await? else {
            return Ok(None);
        };
        let doc: ParsedDocument<TableMetadata> = doc.parse()?;
        Ok(Some(doc.state))
    }

    #[convex_macro::test_runtime]
    async fn hard_delete_removes_deleting_tablet_metadata(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let ns = TableNamespace::test_user();
        let table: TableName = "widgets".parse()?;

        // Create the table with a couple of documents.
        let mut tx = db.begin(Identity::system()).await?;
        TestFacingModel::new(&mut tx)
            .insert(&table, assert_obj!())
            .await?;
        TestFacingModel::new(&mut tx)
            .insert(&table, assert_obj!())
            .await?;
        let tablet_id = tx.table_mapping().namespace(ns).id(&table)?.tablet_id;
        db.commit(tx).await?;

        // Drop it: the `_tables` doc moves to `Deleting` and the tablet is
        // removed from the mapping in the same commit.
        let mut tx = db.begin(Identity::system()).await?;
        TableModel::new(&mut tx)
            .delete_active_table(ns, table.clone())
            .await?;
        db.commit(tx).await?;

        // The precondition that made the old `tablet_id_exists` guard a
        // permanent no-op: tablet absent from the mapping, `_tables` doc still
        // present in `Deleting`.
        let mut tx = db.begin(Identity::system()).await?;
        assert!(!tx.table_mapping().tablet_id_exists(tablet_id));
        assert_eq!(
            tablet_doc_state(&mut tx, tablet_id).await?,
            Some(TableState::Deleting),
        );
        drop(tx);

        // Finalize the drop -- what `SystemTableCleanupWorker` does once
        // retention has emptied the tablet.
        let mut tx = db.begin(Identity::system()).await?;
        TableModel::new(&mut tx)
            .hard_delete_tablet_document(tablet_id)
            .await?;
        db.commit(tx).await?;

        // V1 / V5: the `_tables` doc is gone, so `tablets_to_delete()` no longer
        // re-collects this tablet on the next boot.
        let mut tx = db.begin(Identity::system()).await?;
        assert_eq!(tablet_doc_state(&mut tx, tablet_id).await?, None);
        drop(tx);

        // V2: idempotent -- re-delivering the same tablet is a no-op, not an
        // error.
        let mut tx = db.begin(Identity::system()).await?;
        TableModel::new(&mut tx)
            .hard_delete_tablet_document(tablet_id)
            .await?;
        db.commit(tx).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn hard_delete_refuses_non_deleting_tablet(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let ns = TableNamespace::test_user();
        let table: TableName = "gadgets".parse()?;

        let mut tx = db.begin(Identity::system()).await?;
        TestFacingModel::new(&mut tx)
            .insert(&table, assert_obj!())
            .await?;
        let tablet_id = tx.table_mapping().namespace(ns).id(&table)?.tablet_id;
        db.commit(tx).await?;

        // V3: tablet is `Active`, not `Deleting` -- refuse rather than delete a
        // live table's metadata document.
        let mut tx = db.begin(Identity::system()).await?;
        let err = TableModel::new(&mut tx)
            .hard_delete_tablet_document(tablet_id)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("expected Deleting"),
            "unexpected error: {err}"
        );

        Ok(())
    }
}
