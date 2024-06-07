use std::{
    cmp,
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
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    interval::Interval,
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        GenericIndexName,
        IndexName,
        TableName,
        TabletIndexName,
    },
    value::{
        TableIdentifier,
        TabletIdAndTableNumber,
    },
};
use errors::ErrorMetadata;
use value::{
    FieldPath,
    TableNamespace,
    TableNumber,
    TabletId,
};

use crate::{
    bootstrap_model::virtual_tables::types::VirtualTableMetadata,
    defaults::{
        bootstrap_system_tables,
        system_index,
        SystemIndex,
        SystemTable,
    },
    IndexModel,
    ResolvedQuery,
    SchemaModel,
    SystemMetadataModel,
    Transaction,
    VIRTUAL_TABLES_TABLE,
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

pub static TABLES_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&TABLES_TABLE, "by_name"));

pub static NAME_FIELD_PATH: LazyLock<FieldPath> =
    LazyLock::new(|| "name".parse().expect("Invalid built-in field"));

pub struct TablesTable;
impl SystemTable for TablesTable {
    fn table_name(&self) -> &'static TableName {
        &TABLES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: TABLES_INDEX.clone(),
            fields: vec![NAME_FIELD_PATH.clone()].try_into().unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<TableMetadata>::try_from(document).map(|_| ())
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
    ) -> anyhow::Result<u64> {
        let count = if let Some(tablet_id) = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .id_if_exists(table)
        {
            // Get table count at the beginning of the transaction, then add the delta from
            // the transaction so far.
            let snapshot_count = self.tx.count_snapshot.count(tablet_id).await?;
            let transaction_delta = self.tx.table_count_deltas.get(&tablet_id).unwrap_or(&0);
            if *transaction_delta < 0 {
                snapshot_count
                    .checked_sub(transaction_delta.unsigned_abs())
                    .context("Count underflow")?
            } else {
                snapshot_count
                    .checked_add(*transaction_delta as u64)
                    .context("Count overflow")?
            }
        } else {
            0
        };

        // Add read dependency on the entire table.
        // But we haven't explicitly read the documents, so don't record_read_documents.
        if self.table_exists(namespace, table) {
            let table_id = self.tx.table_mapping().namespace(namespace).id(table)?;
            self.tx.reads.record_indexed_directly(
                TabletIndexName::by_id(table_id.tablet_id),
                IndexedFields::by_id(),
                Interval::all(),
            )?;
        }

        Ok(count)
    }

    pub(crate) fn doc_table_id_to_name(
        &mut self,
        doc: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<ParsedDocument<DeveloperIndexMetadata>> {
        doc.map(|metadata| metadata.map_table(&self.tx.table_mapping().tablet_to_name()))
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

    pub async fn delete_table(
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
        self.delete_table_by_id(table_id_and_number.tablet_id).await
    }

    pub async fn delete_hidden_table(&mut self, tablet_id: TabletId) -> anyhow::Result<()> {
        let table_metadata = self.get_table_metadata(tablet_id).await?;
        // We don't need to validate hidden table with the schema.
        anyhow::ensure!(table_metadata.state == TableState::Hidden);
        self.delete_table_by_id(tablet_id).await
    }

    async fn delete_table_by_id(&mut self, tablet_id: TabletId) -> anyhow::Result<()> {
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
        Ok(())
    }

    async fn get_table_metadata(
        &mut self,
        tablet_id: TabletId,
    ) -> anyhow::Result<ParsedDocument<TableMetadata>> {
        let table_doc_id = self.tx.bootstrap_tables().tables_id.id(tablet_id.0);
        self.tx
            .get(table_doc_id)
            .await?
            .context(format!("Couldn't find table metadata for {tablet_id}"))?
            .try_into()
    }

    pub async fn table_is_empty(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<bool> {
        Ok(self.count(namespace, table).await? == 0)
    }

    // Checks both _tables and _virtual_tables to find a non-conflicting table
    // number
    pub async fn next_user_table_number(&mut self) -> anyhow::Result<TableNumber> {
        self.next_table_number(false).await
    }

    pub async fn next_system_table_number(&mut self) -> anyhow::Result<TableNumber> {
        self.next_table_number(true).await
    }

    async fn next_table_number(&mut self, is_system: bool) -> anyhow::Result<TableNumber> {
        let tables_query = Query::full_table_scan(TABLES_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, tables_query)?;
        let mut max_table_number = TableNumber::try_from(if is_system {
            NUM_RESERVED_LEGACY_TABLE_NUMBERS
        } else {
            NUM_RESERVED_SYSTEM_TABLE_NUMBERS
        })?;
        while let Some(table_metadata) = query_stream.next(self.tx, None).await? {
            let parsed_metadata: ParsedDocument<TableMetadata> = table_metadata.try_into()?;
            max_table_number = cmp::max(max_table_number, parsed_metadata.number);
        }

        let virtual_tables_query = Query::full_table_scan(VIRTUAL_TABLES_TABLE.clone(), Order::Asc);
        let mut virtual_query_stream =
            ResolvedQuery::new(self.tx, TableNamespace::Global, virtual_tables_query)?;
        while let Some(table_metadata) = virtual_query_stream.next(self.tx, None).await? {
            let parsed_metadata: ParsedDocument<VirtualTableMetadata> =
                table_metadata.try_into()?;
            max_table_number = cmp::max(max_table_number, parsed_metadata.number);
        }

        let next_number = max_table_number.increment()?;
        Ok(next_number)
    }

    /// Checks for conflicts when replacing table, e.g. snapshot import.
    /// A table with the same name can be replaced with a different table
    /// number, but if a different table has the same table number then we have
    /// a problem.
    fn check_can_overwrite(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
        table_number: Option<TableNumber>,
        tables_in_import: &BTreeSet<TableName>,
    ) -> anyhow::Result<()> {
        let Some(table_number) = table_number else {
            return Ok(());
        };
        if self.tx.virtual_table_mapping().table_number_exists()(table_number) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "TableConflict",
                format!("New table {table} has IDs that conflict with existing system table")
            ));
        }
        if let Some(existing_table_by_number) = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .name_by_number_if_exists(table_number)
        {
            // Don't mess with creating conflicts with bootstrap system tables.
            anyhow::ensure!(
                bootstrap_system_tables()
                    .iter()
                    .all(|t| t.table_name() != existing_table_by_number),
                "Conflict with bootstrap system table {existing_table_by_number}",
            );
            if existing_table_by_number == table {
                // Overwriting in-place, same table name and number.
                return Ok(());
            }
            if tables_in_import.contains(existing_table_by_number) {
                // Overwriting would create a table number conflict with an
                // existing table, but that existing table is also being
                // overwritten.
                return Ok(());
            }
            if existing_table_by_number.is_system() {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "TableConflict",
                    format!(
                        "New table {table} has IDs that conflict with existing internal table. \
                         Consider importing this table without `_id` fields or import into a new \
                         deployment."
                    )
                ));
            }
            anyhow::bail!(ErrorMetadata::bad_request(
                "TableConflict",
                format!(
                    "New table {table} has IDs that conflict with existing table \
                     {existing_table_by_number}"
                )
            ));
        }
        Ok(())
    }

    pub async fn activate_table(
        &mut self,
        tablet_id: TabletId,
        table_name: &TableName,
        table_number: TableNumber,
        tables_in_import: &BTreeSet<TableName>,
    ) -> anyhow::Result<u64> {
        let mut documents_deleted = 0;
        let table_metadata = self.get_table_metadata(tablet_id).await?;
        match table_metadata.state {
            TableState::Active => return Ok(0),
            TableState::Deleting => anyhow::bail!("cannot activate {table_name} which is deleting"),
            TableState::Hidden => {},
        }
        let namespace = table_metadata.namespace;
        self.check_can_overwrite(namespace, table_name, Some(table_number), tables_in_import)?;
        if self.table_exists(namespace, table_name) {
            let existing_table_by_name = self
                .tx
                .table_mapping()
                .namespace(namespace)
                .id(table_name)?;
            documents_deleted += self.count(namespace, table_name).await?;
            self.delete_table_by_id(existing_table_by_name.tablet_id)
                .await?;
        }
        let table_metadata = TableMetadata::new_with_state(
            namespace,
            table_name.clone(),
            table_number,
            TableState::Active,
        );
        let table_doc_id = self.tables_table_id()?.id(tablet_id.0);
        SystemMetadataModel::new_global(self.tx)
            .replace(table_doc_id, table_metadata.try_into()?)
            .await?;
        Ok(documents_deleted)
    }

    fn tables_table_id(&mut self) -> anyhow::Result<TabletIdAndTableNumber> {
        self.tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .name_to_id()(TABLES_TABLE.clone())
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

    pub async fn insert_table_for_import(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
        table_number: Option<TableNumber>,
        tables_in_import: &BTreeSet<TableName>,
    ) -> anyhow::Result<TabletIdAndTableNumber> {
        anyhow::ensure!(
            bootstrap_system_tables()
                .iter()
                .all(|t| t.table_name() != table),
            "Conflict with bootstrap system table {table}",
        );
        let existing_table_by_name = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .id_and_number_if_exists(table)
            .map(|id| id.table_number);
        let table_number = table_number.or(existing_table_by_name);
        self.check_can_overwrite(namespace, table, table_number, tables_in_import)?;
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
            anyhow::ensure!(
                self.count_user_tables() < MAX_USER_TABLES,
                index_validation_error::too_many_tables(MAX_USER_TABLES)
            );
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
                self.next_user_table_number().await?
            };
            let table_metadata =
                TableMetadata::new_with_state(namespace, table.clone(), table_number, state);
            let table_doc_id = SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&TABLES_TABLE, table_metadata.try_into()?)
                .await?;
            let table_id = TabletIdAndTableNumber {
                tablet_id: TabletId(table_doc_id.internal_id()),
                table_number,
            };

            // Add the system defined indexes for the newly created table. Since the newly
            // created table is empty, we can start these indexes as `Enabled`.
            let metadata = IndexMetadata::new_enabled(
                GenericIndexName::by_id(table_id.tablet_id),
                IndexedFields::by_id(),
            );
            SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            let metadata = IndexMetadata::new_enabled(
                GenericIndexName::by_creation_time(table_id.tablet_id),
                IndexedFields::creation_time(),
            );
            SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&INDEX_TABLE, metadata.try_into()?)
                .await?;
            Ok(table_id)
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn insert_table_metadata_for_test(
        &mut self,
        namespace: TableNamespace,
        table: &TableName,
    ) -> anyhow::Result<()> {
        self._insert_table_metadata(namespace, table, None, TableState::Active)
            .await?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use common::{
        bootstrap_model::schema::{
            SchemaMetadata,
            SchemaState,
        },
        db_schema,
        document::ParsedDocument,
        object_validator,
        schemas::{
            validator::{
                FieldValidator,
                Validator,
            },
            DatabaseSchema,
            DocumentSchema,
        },
    };
    use must_let::must_let;
    use runtime::testing::TestRuntime;
    use value::{
        TableName,
        TableNamespace,
    };

    use crate::{
        bootstrap_model::table::NUM_RESERVED_SYSTEM_TABLE_NUMBERS,
        test_helpers::new_tx,
        SchemaModel,
        TableModel,
        Transaction,
    };

    #[convex_macro::test_runtime]
    async fn delete_table_with_missing_table_does_nothing(rt: TestRuntime) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;
        let mut model = TableModel::new(&mut tx);
        model
            .delete_table(TableNamespace::test_user(), TableName::from_str("missing")?)
            .await
    }

    #[convex_macro::test_runtime]
    async fn insert_table_metadata_inserts_table(rt: TestRuntime) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;
        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        assert!(model.table_exists(TableNamespace::test_user(), &table_name));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn delete_table_with_valid_table_deletes_it(rt: TestRuntime) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;
        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        model
            .delete_table(TableNamespace::test_user(), table_name.clone())
            .await?;
        assert!(!model.table_exists(TableNamespace::test_user(), &table_name));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn delete_table_with_table_not_in_active_schema_deletes_it(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;

        set_active_schema(&mut tx, db_schema!()).await?;

        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        model
            .delete_table(TableNamespace::test_user(), table_name.clone())
            .await?;
        assert!(!model.table_exists(TableNamespace::test_user(), &table_name));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn delete_table_with_other_table_in_active_schema_deletes_it(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;

        set_active_schema(&mut tx, db_schema!("other_table" => DocumentSchema::Any)).await?;

        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        model
            .delete_table(TableNamespace::test_user(), table_name.clone())
            .await?;
        assert!(!model.table_exists(TableNamespace::test_user(), &table_name));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn delete_table_with_table_in_active_schema_fails(rt: TestRuntime) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;

        set_active_schema(&mut tx, db_schema!("my_table" => DocumentSchema::Any)).await?;

        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        let result = model
            .delete_table(TableNamespace::test_user(), table_name.clone())
            .await;
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Failed to delete table \"my_table\" because it appears in the schema"),
            "{error}"
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn delete_table_with_table_in_active_schema_as_reference_fails(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;

        set_active_schema(&mut tx, db_schema!("other_table" =>
            DocumentSchema::Union(vec![object_validator!(
                "field" => FieldValidator::required_field_type(Validator::Id("my_table".parse()?))
            )])
        )).await?;

        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        let result = model
            .delete_table(TableNamespace::test_user(), table_name.clone())
            .await;
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains(
                "Failed to delete table \"my_table\" because `v.id(\"my_table\")` appears in the \
                 schema of table \"other_table\""
            ),
            "{error}"
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn delete_table_with_table_in_validated_schema_fails_schema(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;

        let mut schema_model = SchemaModel::new_root_for_test(&mut tx);
        let (schema_id, _state) = schema_model
            .submit_pending(db_schema!("my_table" => DocumentSchema::Any))
            .await?;
        schema_model.mark_validated(schema_id).await?;

        let mut model = TableModel::new(&mut tx);
        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        model
            .delete_table(TableNamespace::test_user(), table_name.clone())
            .await?;

        let mut schema_model = SchemaModel::new_root_for_test(&mut tx);
        assert!(schema_model
            .get_by_state(SchemaState::Validated)
            .await?
            .is_none());
        let schema = ParsedDocument::<SchemaMetadata>::try_from(tx.get(schema_id).await?.unwrap())?;
        must_let!(let SchemaState::Failed { error, table_name } = &schema.state);
        assert_eq!(table_name, &Some("my_table".to_string()));
        assert!(
            error
                .to_string()
                .contains("Failed to delete table \"my_table\" because it appears in the schema"),
            "{error}"
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_next_table_number(rt: TestRuntime) -> anyhow::Result<()> {
        let mut tx = new_tx(rt).await?;
        let mut model = TableModel::new(&mut tx);

        let next_table_number: u32 = model.next_user_table_number().await?.into();
        assert_eq!(
            (NUM_RESERVED_SYSTEM_TABLE_NUMBERS + 1) as usize,
            next_table_number as usize
        );

        let table_name = TableName::from_str("my_table")?;
        model
            .insert_table_metadata(TableNamespace::test_user(), &table_name)
            .await?;
        let new_next_table_number: u32 = model.next_user_table_number().await?.into();
        assert_eq!(next_table_number + 1, new_next_table_number);
        Ok(())
    }

    async fn set_active_schema(
        tx: &mut Transaction<TestRuntime>,
        schema: DatabaseSchema,
    ) -> anyhow::Result<()> {
        let mut schema_model = SchemaModel::new_root_for_test(tx);
        let (schema_id, _state) = schema_model.submit_pending(schema).await?;
        schema_model.mark_validated(schema_id).await?;
        schema_model.mark_active(schema_id).await?;
        Ok(())
    }
}
