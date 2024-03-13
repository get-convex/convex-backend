use anyhow::Context;
use common::{
    document::{
        CreationTime,
        ResolvedDocument,
        CREATION_TIME_FIELD,
        ID_FIELD,
    },
    runtime::Runtime,
};
use errors::ErrorMetadata;
use value::{
    check_user_size,
    id_v6::DocumentIdV6,
    ConvexObject,
    ConvexValue,
    DeveloperDocumentId,
    FieldName,
    Size,
    TableIdAndTableNumber,
    TableIdentifier,
    TableMapping,
    TableName,
};

use crate::{
    defaults::bootstrap_system_tables,
    SchemaModel,
    Transaction,
};

pub struct ImportFacingModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ImportFacingModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    /// Inserts a new document as part of a snapshot import.
    /// This is like `UserFacingModel::insert` with a few differences:
    /// - the table for insertion is chosen by table id, not table name or
    ///   number.
    /// - nonexistent tables won't be created implicitly.
    /// - the _creationTime may be user-specified.
    /// - only admin/system auth is allowed.
    #[convex_macro::instrument_future]
    pub async fn insert(
        &mut self,
        table_id: TableIdAndTableNumber,
        table_name: &TableName,
        value: ConvexObject,
        table_mapping_for_schema: &TableMapping,
    ) -> anyhow::Result<DeveloperDocumentId> {
        if self
            .tx
            .virtual_system_mapping()
            .is_virtual_table(table_name)
        {
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
        if !(self.tx.identity.is_admin() || self.tx.identity.is_system()) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnauthorizedImport",
                "Import requires admin auth"
            ));
        }

        check_user_size(value.size())?;
        self.tx.retention_validator.fail_if_falling_behind()?;
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
            self.tx.id_generator.generate_internal()
        };
        let id = table_id.id(internal_id);

        let creation_time_field = FieldName::from(CREATION_TIME_FIELD.clone());
        let creation_time = if let Some(ConvexValue::Float64(f)) = value.get(&creation_time_field) {
            CreationTime::try_from(*f)?
        } else {
            self.tx.next_creation_time.increment()?
        };

        let document = ResolvedDocument::new(id, creation_time, value)?;
        SchemaModel::new(self.tx)
            .enforce_with_table_mapping(&document, table_mapping_for_schema)
            .await?;
        self.tx.apply_validated_write(id, None, Some(document))?;

        Ok(id.into())
    }
}
