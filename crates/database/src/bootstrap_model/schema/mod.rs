#[cfg(test)]
mod tests;
pub mod types;

use std::{
    sync::LazyLock,
    time::Duration,
};

use anyhow::Context;
use async_recursion::async_recursion;
use common::{
    bootstrap_model::schema::{
        SchemaMetadata,
        SchemaState,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
    schemas::{
        DatabaseSchema,
        SchemaValidationError,
    },
    types::IndexName,
};
use errors::ErrorMetadata;
use value::{
    FieldPath,
    NamespacedTableMapping,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use self::types::SchemaDiff;
use crate::{
    defaults::{
        system_index,
        SystemIndex,
        SystemTable,
    },
    patch_value,
    ResolvedQuery,
    SystemMetadataModel,
    TableModel,
    Transaction,
};

pub static SCHEMAS_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_schemas".parse().expect("Invalid built-in schemas table"));

pub static SCHEMAS_STATE_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&SCHEMAS_TABLE, "by_state"));

pub static SCHEMA_STATE_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "state".parse().expect("invalid state field"));

const MAX_TIME_TO_KEEP_FAILED_AND_OVERWRITTEN_SCHEMAS: Duration = Duration::from_secs(60 * 60); // 1 hour

pub struct SchemasTable;
impl SystemTable for SchemasTable {
    fn table_name(&self) -> &'static TableName {
        &SCHEMAS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: SCHEMAS_STATE_INDEX.clone(),
            fields: vec![SCHEMA_STATE_FIELD.clone()].try_into().unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<SchemaMetadata>::try_from(document).map(|_| ())
    }
}

pub struct SchemaModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SchemaModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_root_for_test(tx: &'a mut Transaction<RT>) -> Self {
        Self::new(tx, TableNamespace::test_user())
    }

    #[fastrace::trace]
    pub async fn apply(
        &mut self,
        schema_id: Option<ResolvedDocumentId>,
    ) -> anyhow::Result<(Option<SchemaDiff>, Option<DatabaseSchema>)> {
        let previous_schema = self
            .get_by_state(SchemaState::Active)
            .await?
            .map(|(_id, schema)| schema);
        let next_schema = if let Some(schema_id) = schema_id {
            Some(
                self.get_validated_or_active(schema_id)
                    .await?
                    .database_schema()?,
            )
        } else {
            None
        };
        let schema_diff: Option<SchemaDiff> =
            (previous_schema != next_schema).then_some(SchemaDiff {
                previous_schema,
                next_schema: next_schema.clone(),
            });
        if let Some(schema_id) = schema_id {
            self.mark_active(schema_id).await?;
        } else {
            self.clear_active().await?;
        }

        Ok((schema_diff, next_schema))
    }

    #[fastrace::trace]
    pub async fn enforce(&mut self, document: &ResolvedDocument) -> anyhow::Result<()> {
        let schema_table_mapping = self.tx.table_mapping().namespace(self.namespace);
        if schema_table_mapping.is_system_tablet(document.id().tablet_id) {
            // System tables are not subject to schema validation.
            return Ok(());
        }
        self.enforce_with_table_mapping(document, &schema_table_mapping)
            .await
    }

    pub async fn enforce_table_deletion(
        &mut self,
        active_table_to_delete: TableName,
    ) -> anyhow::Result<()> {
        if let Some((_id, active_schema)) = self.get_by_state(SchemaState::Active).await? {
            if let Err(schema_error) =
                active_schema.check_delete_table(active_table_to_delete.clone())
            {
                anyhow::bail!(schema_error.to_error_metadata());
            }
        }
        let pending_schema = self.get_by_state(SchemaState::Pending).await?;
        let validated_schema = self.get_by_state(SchemaState::Validated).await?;
        match (pending_schema, validated_schema) {
            (None, None) => {},
            (Some((id, in_progress_schema)), None) | (None, Some((id, in_progress_schema))) => {
                if let Err(enforcement_error) =
                    in_progress_schema.check_delete_table(active_table_to_delete)
                {
                    self.mark_failed(id, enforcement_error.into()).await?;
                }
            },
            (Some(_), Some(_)) => {
                anyhow::bail!("Invalid schema state: both pending and validated schemas exist")
            },
        }

        Ok(())
    }

    /// You probably want to use `enforce`.
    /// enforce_with_table_mapping allows schema validation to use a custom
    /// TableMapping for validating foreign references, which is useful for
    /// snapshot imports where hidden tables can have foreign references to
    /// other hidden tables in the same import.
    pub async fn enforce_with_table_mapping(
        &mut self,
        document: &ResolvedDocument,
        table_mapping_for_schema: &NamespacedTableMapping,
    ) -> anyhow::Result<()> {
        let table_name = table_mapping_for_schema.tablet_name(document.id().tablet_id)?;
        if let Some((_id, active_schema)) = self.get_by_state(SchemaState::Active).await? {
            if let Err(schema_error) = active_schema.check_new_document(
                document,
                table_name.clone(),
                table_mapping_for_schema,
                self.tx.virtual_system_mapping(),
            ) {
                anyhow::bail!(schema_error.to_error_metadata());
            }
        }
        let pending_schema = self.get_by_state(SchemaState::Pending).await?;
        let validated_schema = self.get_by_state(SchemaState::Validated).await?;
        match (pending_schema, validated_schema) {
            (None, None) => {},
            (Some((id, in_progress_schema)), None) | (None, Some((id, in_progress_schema))) => {
                if let Err(enforcement_error) = in_progress_schema.check_new_document(
                    document,
                    table_name,
                    table_mapping_for_schema,
                    self.tx.virtual_system_mapping(),
                ) {
                    self.mark_failed(id, enforcement_error.into()).await?;
                }
            },
            (Some(_), Some(_)) => {
                anyhow::bail!("Invalid schema state: both pending and validated schemas exist")
            },
        }

        Ok(())
    }

    pub async fn get_by_state(
        &mut self,
        state: SchemaState,
    ) -> anyhow::Result<Option<(ResolvedDocumentId, DatabaseSchema)>> {
        anyhow::ensure!(
            state.is_unique(),
            "Getting schema by state is only permitted for Pending, Validated, or Active states, \
             since Failed or Overwritten states may have multiple documents."
        );
        self.tx.get_schema_by_state(self.namespace, state)
    }

    #[fastrace::trace]
    pub async fn submit_pending(
        &mut self,
        schema: DatabaseSchema,
    ) -> anyhow::Result<(ResolvedDocumentId, SchemaState)> {
        let mut table_model = TableModel::new(self.tx);
        for name in schema.tables.keys() {
            if !table_model.table_exists(self.namespace, name) {
                table_model
                    .insert_table_metadata(self.namespace, name)
                    .await?;
            }
        }
        if let Some((id, active_schema)) = self.get_by_state(SchemaState::Active).await? {
            if active_schema == schema {
                if let Some((id, _pending_schema)) = self.get_by_state(SchemaState::Pending).await?
                {
                    self.mark_overwritten(id).await?;
                }
                if let Some((id, _validated_schema)) =
                    self.get_by_state(SchemaState::Validated).await?
                {
                    self.mark_overwritten(id).await?;
                }
                return Ok((id, SchemaState::Active));
            }
        }
        match (
            self.get_by_state(SchemaState::Pending).await?,
            self.get_by_state(SchemaState::Validated).await?,
        ) {
            (Some(_), Some(_)) => {
                anyhow::bail!("Invalid schema state: both pending and validated schemas exist")
            },
            (Some((id, existing_schema)), None) => {
                if existing_schema == schema {
                    return Ok((id, SchemaState::Pending));
                } else {
                    self.mark_overwritten(id).await?;
                }
            },
            (None, Some((id, existing_schema))) => {
                if existing_schema == schema {
                    return Ok((id, SchemaState::Validated));
                } else {
                    self.mark_overwritten(id).await?;
                }
            },
            (None, None) => {},
        }

        let schema_metadata = SchemaMetadata::new(SchemaState::Pending, schema)?;
        let id = SystemMetadataModel::new(self.tx, self.namespace)
            .insert(&SCHEMAS_TABLE, schema_metadata.try_into()?)
            .await?;
        Ok((id, SchemaState::Pending))
    }

    pub async fn mark_validated(&mut self, document_id: ResolvedDocumentId) -> anyhow::Result<()> {
        let doc = self
            .tx
            .get(document_id)
            .await?
            .context("Schema to mark as validated must exist.")?;
        let schema = SchemaMetadata::try_from(doc.into_value().into_value())?;
        match schema.state {
            SchemaState::Pending => {
                SystemMetadataModel::new(self.tx, self.namespace)
                    .patch(
                        document_id,
                        patch_value!("state" => Some(SchemaState::Validated.try_into()?))?,
                    )
                    .await?;
                Ok(())
            },
            SchemaState::Validated => Err(anyhow::anyhow!("Schema is already validated.")),
            SchemaState::Active => Err(anyhow::anyhow!("Schema is already active.")),
            SchemaState::Failed { error, .. } => Err(ErrorMetadata::bad_request(
                "SchemaAlreadyFailed",
                format!("Schema has already been failed with error: {error}"),
            )
            .into()),
            SchemaState::Overwritten => Err(ErrorMetadata::bad_request(
                "SchemaAlreadyOverwritten",
                "Schema has already been overwritten.",
            )
            .into()),
        }
    }

    pub async fn get_validated_or_active(
        &mut self,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<SchemaMetadata> {
        let doc = self
            .tx
            .get(schema_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No document found for schema ID {schema_id}"))?;
        let schema = SchemaMetadata::try_from(doc.into_value().into_value())?;
        match schema.state {
            SchemaState::Pending => {
                anyhow::bail!("Expected schema to be Validated, but it's Pending {schema_id}")
            },
            SchemaState::Validated => Ok(schema),
            SchemaState::Active => Ok(schema),
            SchemaState::Failed { error, .. } => Err(ErrorMetadata::bad_request(
                "SchemaAlreadyFailed",
                format!("Schema has already been failed with error: {error}"),
            )
            .into()),
            SchemaState::Overwritten => Err(ErrorMetadata::bad_request(
                "SchemaAlreadyOverwritten",
                "Schema has already been overwritten.",
            )
            .into()),
        }
    }

    pub async fn mark_active(&mut self, document_id: ResolvedDocumentId) -> anyhow::Result<()> {
        // Make sure it's already Validated or Active.
        let schema = self.get_validated_or_active(document_id).await?;
        match schema.state {
            // Already active: no-op
            SchemaState::Active => Ok(()),
            // If it's validated, mark as active.
            SchemaState::Validated => {
                self.clear_active().await?;
                SystemMetadataModel::new(self.tx, self.namespace)
                    .patch(
                        document_id,
                        patch_value!("state" => Some(SchemaState::Active.try_into()?))?,
                    )
                    .await?;
                Ok(())
            },
            SchemaState::Overwritten | SchemaState::Pending | SchemaState::Failed { .. } => {
                anyhow::bail!("expected validated or active schema")
            },
        }
    }

    #[async_recursion]
    /// Mark pending or validated schemas as failed. Error if the schema is
    /// already active, and do nothing if it is already overwritten or failed.
    pub async fn mark_failed(
        &mut self,
        document_id: ResolvedDocumentId,
        error: SchemaValidationError,
    ) -> anyhow::Result<()> {
        let doc = self
            .tx
            .get(document_id)
            .await?
            .context("Schema to mark as failed must exist.")?;
        let schema = SchemaMetadata::try_from(doc.into_value().into_value())?;
        match schema.state {
            SchemaState::Pending | SchemaState::Validated => {
                let error_message = error.to_string();
                let table_name = match error {
                    SchemaValidationError::ExistingDocument { table_name, .. } => table_name,
                    SchemaValidationError::NewDocument { table_name, .. } => table_name,
                    SchemaValidationError::TableCannotBeDeleted { table_name } => table_name,
                    SchemaValidationError::ReferencedTableCannotBeDeleted {
                        table_name, ..
                    } => table_name,
                };
                SystemMetadataModel::new(self.tx, self.namespace)
                    .patch(
                        document_id,
                        patch_value!(
                            "state" => Some(
                                SchemaState::Failed {
                                    error: error_message,
                                    table_name: Some(table_name.to_string())
                                }.try_into()?
                            )
                        )?,
                    )
                    .await?;
            },
            SchemaState::Active => {
                anyhow::bail!("Active schemas cannot be marked as failed.")
            },
            SchemaState::Failed { .. } | SchemaState::Overwritten => {},
        }
        self.delete_old_failed_and_overwritten_schemas().await?;
        Ok(())
    }

    pub async fn overwrite_all(&mut self) -> anyhow::Result<bool> {
        let mut is_any_schema_overwritten = false;
        for state in [
            SchemaState::Pending,
            SchemaState::Active,
            SchemaState::Validated,
        ] {
            is_any_schema_overwritten =
                self.overwrite_by_state(state).await? || is_any_schema_overwritten;
        }
        Ok(is_any_schema_overwritten)
    }

    pub async fn clear_active(&mut self) -> anyhow::Result<()> {
        self.overwrite_by_state(SchemaState::Active)
            .await
            .map(|_| ())
    }

    async fn overwrite_by_state(&mut self, state: SchemaState) -> anyhow::Result<bool> {
        if let Some((id, _schema)) = self.get_by_state(state).await? {
            self.mark_overwritten(id).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Deletes failed and overwritten schemas older than an hour, returning the
    /// number of documents deleted. Keeps schemas table small.
    async fn delete_old_failed_and_overwritten_schemas(&mut self) -> anyhow::Result<usize> {
        let query = Query::full_table_scan(SCHEMAS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, self.namespace, query)?;
        let mut num_deleted = 0;
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            let schema_doc: ParsedDocument<SchemaMetadata> = doc.try_into()?;
            // Only delete failed and overwritten schemas
            match schema_doc.state {
                SchemaState::Failed { .. } | SchemaState::Overwritten => {},
                SchemaState::Active | SchemaState::Pending | SchemaState::Validated => continue,
            }
            // Break if the schemas are not old enough to be deleted
            if schema_doc
                .creation_time()
                .context("Missing creation time in document")?
                > (*self
                    .tx
                    .begin_timestamp()
                    .sub(MAX_TIME_TO_KEEP_FAILED_AND_OVERWRITTEN_SCHEMAS)
                    .context("Should be able to subtract an hour from creation time")?)
                .try_into()?
            {
                break;
            }
            SystemMetadataModel::new(self.tx, self.namespace)
                .delete(schema_doc.id())
                .await?;
            num_deleted += 1;
        }
        Ok(num_deleted)
    }

    async fn mark_overwritten(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        SystemMetadataModel::new(self.tx, self.namespace)
            .patch(
                id,
                patch_value!("state" => Some(SchemaState::Overwritten.try_into()?))?,
            )
            .await?;
        self.delete_old_failed_and_overwritten_schemas().await?;
        Ok(())
    }
}
