#[cfg(test)]
mod tests;

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
        IndexRange,
        IndexRangeExpression,
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
    val,
    FieldPath,
    ResolvedDocumentId,
    TableMapping,
    TableName,
};

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
}

impl<'a, RT: Runtime> SchemaModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn enforce(&mut self, document: &ResolvedDocument) -> anyhow::Result<()> {
        let schema_table_mapping = self.tx.table_mapping().clone();
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
        table_mapping_for_schema: &TableMapping,
    ) -> anyhow::Result<()> {
        let table_name = table_mapping_for_schema.tablet_name(document.table().tablet_id)?;
        if let Some((_id, active_schema)) = self.get_by_state(SchemaState::Active).await? {
            if let Err(schema_error) = active_schema.check_new_document(
                document,
                table_name.clone(),
                table_mapping_for_schema,
                &self.tx.virtual_table_mapping().clone(),
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
                    &self.tx.virtual_table_mapping().clone(),
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
        match state {
            SchemaState::Pending | SchemaState::Validated | SchemaState::Active => {},
            SchemaState::Failed { .. } | SchemaState::Overwritten => anyhow::bail!(
                "Getting schema by state is only permitted for Pending, Validated, or Active \
                 states, since Failed or Overwritten states may have multiple documents."
            ),
        }
        let state_value = val!(state);
        let index_range = IndexRange {
            index_name: SCHEMAS_STATE_INDEX.clone(),
            range: vec![IndexRangeExpression::Eq(
                SCHEMA_STATE_FIELD.clone(),
                state_value.into(),
            )],
            order: Order::Asc,
        };
        let query = Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, query)?;
        let schema = query_stream
            .expect_at_most_one(self.tx)
            .await?
            .map(|doc| {
                Ok::<(ResolvedDocumentId, DatabaseSchema), anyhow::Error>((
                    doc.id().to_owned(),
                    SchemaMetadata::try_from(doc.into_value().into_value())?.schema,
                ))
            })
            .transpose()?;
        Ok(schema)
    }

    pub async fn submit_pending(
        &mut self,
        schema: DatabaseSchema,
    ) -> anyhow::Result<(ResolvedDocumentId, SchemaState)> {
        let mut table_model = TableModel::new(self.tx);
        for name in schema.tables.keys() {
            if !table_model.table_exists(name) {
                table_model.insert_table_metadata(name).await?;
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

        let schema_metadata = SchemaMetadata {
            state: SchemaState::Pending,
            schema,
        };
        let id = SystemMetadataModel::new(self.tx)
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
                SystemMetadataModel::new(self.tx)
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
    ) -> anyhow::Result<(SchemaState, DatabaseSchema)> {
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
            SchemaState::Validated => Ok((SchemaState::Validated, schema.schema)),
            SchemaState::Active => Ok((SchemaState::Active, schema.schema)),
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
        let (state, _) = self.get_validated_or_active(document_id).await?;
        match state {
            // Already active: no-op
            SchemaState::Active => Ok(()),
            // If it's validated, mark as active.
            SchemaState::Validated => {
                self.clear_active().await?;
                SystemMetadataModel::new(self.tx)
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
                SystemMetadataModel::new(self.tx)
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
        let mut query_stream = ResolvedQuery::new(self.tx, query)?;
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
                > self
                    .tx
                    .begin_timestamp()
                    .sub(MAX_TIME_TO_KEEP_FAILED_AND_OVERWRITTEN_SCHEMAS)
                    .context("Should be able to subtract an hour from creation time")?
                    .try_into()?
            {
                break;
            }
            SystemMetadataModel::new(self.tx)
                .delete(schema_doc.id())
                .await?;
            num_deleted += 1;
        }
        Ok(num_deleted)
    }

    async fn mark_overwritten(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        SystemMetadataModel::new(self.tx)
            .patch(
                id,
                patch_value!("state" => Some(SchemaState::Overwritten.try_into()?))?,
            )
            .await?;
        self.delete_old_failed_and_overwritten_schemas().await?;
        Ok(())
    }
}
