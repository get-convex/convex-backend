pub mod types;

use std::sync::{
    Arc,
    LazyLock,
};

use common::{
    document::{
        ParseDocument,
        ParsedDocument,
        CREATION_TIME_FIELD_PATH,
    },
    runtime::Runtime,
};
use value::{
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use self::types::ValidationState;
use super::schema_validation_progress::SchemaValidationProgressModel;
use crate::{
    system_tables::{
        SystemIndex,
        SystemTable,
    },
    SchemaValidationMetadata,
    SchemaValidationProgress,
    SystemMetadataModel,
    Transaction,
};

pub const SCHEMA_VALIDATIONS_TABLE: TableName = TableName::const_new("_schema_validations");

pub static SCHEMA_VALIDATIONS_BY_SCHEMA_ID_AND_TABLE_NAME: LazyLock<
    SystemIndex<SchemaValidationTable>,
> = LazyLock::new(|| {
    SystemIndex::new(
        "by_schema_id_and_table_name",
        [
            &SCHEMA_ID_FIELD,
            &TABLE_NAME_FIELD,
            &CREATION_TIME_FIELD_PATH,
        ],
    )
    .unwrap()
});

static SCHEMA_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "schemaId".parse().expect("invalid schemaId field"));

static TABLE_NAME_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "tableName".parse().expect("invalid tableName field"));

pub struct SchemaValidationTable;

impl SystemTable for SchemaValidationTable {
    type Metadata = types::SchemaValidationMetadata;

    const TABLE_NAME: TableName = SCHEMA_VALIDATIONS_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![SCHEMA_VALIDATIONS_BY_SCHEMA_ID_AND_TABLE_NAME.clone()]
    }
}

/// Worker updates apply only to the pending attempt identified by its document
/// ID.
#[derive(Clone)]
pub enum ValidationAttemptUpdate {
    StartWalk {
        total_docs: Option<u64>,
    },
    RecordProgress {
        additional_docs_validated: u64,
        total_docs: Option<u64>,
    },
    MarkValid,
    MarkFailed {
        error: String,
    },
}

pub struct SchemaValidationWithProgress {
    pub validation: SchemaValidationMetadata,
    pub progress: SchemaValidationProgress,
}

pub struct SchemaValidationModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SchemaValidationModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    pub async fn validations_for_schema(
        &mut self,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<Vec<ParsedDocument<SchemaValidationMetadata>>> {
        Ok(self
            .tx
            .query_system(
                self.namespace,
                &*SCHEMA_VALIDATIONS_BY_SCHEMA_ID_AND_TABLE_NAME,
            )?
            .eq(&[schema_id.developer_id.encode_into(&mut Default::default())])?
            .all()
            .await?
            .into_iter()
            .map(|validation| (*validation).clone())
            .collect())
    }

    /// The validation for one table under `schema_id`, if any.
    pub async fn validation_metadata_for_table(
        &mut self,
        schema_id: ResolvedDocumentId,
        table_name: &TableName,
    ) -> anyhow::Result<Option<ParsedDocument<SchemaValidationMetadata>>> {
        Ok(self
            .tx
            .query_system(
                self.namespace,
                &*SCHEMA_VALIDATIONS_BY_SCHEMA_ID_AND_TABLE_NAME,
            )?
            .eq(&[
                schema_id.developer_id.encode_into(&mut Default::default()),
                table_name,
            ])?
            .unique()
            .await?
            .map(Arc::unwrap_or_clone))
    }

    /// Create (or reset) the validation for one table under `schema_id`, in
    /// state `Pending` with zero progress. Its fresh document ID fences
    /// workers holding a previous attempt's snapshot.
    pub async fn start_table_validation(
        &mut self,
        schema_id: ResolvedDocumentId,
        table_name: TableName,
        validator_hash: Option<String>,
        total_docs: Option<u64>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        if let Some(existing) = self
            .validation_metadata_for_table(schema_id, &table_name)
            .await?
        {
            self.delete_attempt(existing.id()).await?;
        }
        let metadata = SchemaValidationMetadata {
            schema_id: schema_id.developer_id,
            table_name,
            validator_hash,
            state: ValidationState::Pending,
        };
        let id = SystemMetadataModel::new(self.tx, self.namespace)
            .insert(&SCHEMA_VALIDATIONS_TABLE, metadata.try_into()?)
            .await?;
        SchemaValidationProgressModel::new(self.tx, self.namespace)
            .create(id, 0, total_docs)
            .await?;
        Ok(id)
    }

    /// The document ID binds the update to the snapshot captured by the worker.
    /// A retry replaces the validation, so stale workers return false. Counter
    /// flushes read this validation and write only progress: a committed
    /// failure cancels an in-flight flush without counter updates
    /// invalidating failure writes.
    pub async fn update_attempt(
        &mut self,
        attempt_id: ResolvedDocumentId,
        update: ValidationAttemptUpdate,
    ) -> anyhow::Result<bool> {
        let Some(doc) = self.tx.get(attempt_id).await? else {
            return Ok(false);
        };
        let validation: ParsedDocument<SchemaValidationMetadata> = doc.parse()?;
        let (id, mut metadata) = validation.into_id_and_value();
        match metadata.state {
            ValidationState::Pending => {},
            ValidationState::Valid | ValidationState::Failed { .. } => return Ok(false),
        }
        match update {
            ValidationAttemptUpdate::StartWalk { total_docs } => {
                SchemaValidationProgressModel::new(self.tx, self.namespace)
                    .reset(id, total_docs)
                    .await?;
                return Ok(true);
            },
            ValidationAttemptUpdate::RecordProgress {
                additional_docs_validated,
                total_docs,
            } => {
                SchemaValidationProgressModel::new(self.tx, self.namespace)
                    .record(id, additional_docs_validated, total_docs)
                    .await?;
                return Ok(true);
            },
            ValidationAttemptUpdate::MarkValid => metadata.state = ValidationState::Valid,
            ValidationAttemptUpdate::MarkFailed { error } => {
                metadata.state = ValidationState::Failed { error }
            },
        }
        SystemMetadataModel::new(self.tx, self.namespace)
            .replace(id, metadata.try_into()?)
            .await?;
        Ok(true)
    }

    pub async fn progress(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<SchemaValidationProgress> {
        Ok(SchemaValidationProgressModel::new(self.tx, self.namespace)
            .must_get(id)
            .await?
            .into_value())
    }

    pub async fn validations_with_progress(
        &mut self,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<Vec<SchemaValidationWithProgress>> {
        let mut result = vec![];
        for validation in self.validations_for_schema(schema_id).await? {
            let counters = self.progress(validation.id()).await?;
            result.push(SchemaValidationWithProgress {
                validation: validation.into_value(),
                progress: counters,
            });
        }
        Ok(result)
    }

    async fn delete_attempt(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        SchemaValidationProgressModel::new(self.tx, self.namespace)
            .delete(id)
            .await?;
        SystemMetadataModel::new(self.tx, self.namespace)
            .delete(id)
            .await?;
        Ok(())
    }

    /// Write-time invalidation applies to both pending and valid attempts.
    /// An already-failed validation keeps its first error; a missing validation
    /// is canceled.
    pub async fn mark_failed(
        &mut self,
        schema_id: ResolvedDocumentId,
        table_name: &TableName,
        error: String,
    ) -> anyhow::Result<bool> {
        let Some(validation) = self
            .validation_metadata_for_table(schema_id, table_name)
            .await?
        else {
            return Ok(false);
        };
        let (id, mut metadata) = validation.into_id_and_value();
        match metadata.state {
            ValidationState::Failed { .. } => {},
            ValidationState::Pending | ValidationState::Valid => {
                metadata.state = ValidationState::Failed { error };
                SystemMetadataModel::new(self.tx, self.namespace)
                    .replace(id, metadata.try_into()?)
                    .await?;
            },
        }
        Ok(true)
    }

    pub async fn delete_validations_for_schema(
        &mut self,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        for validation in self.validations_for_schema(schema_id).await? {
            self.delete_attempt(validation.id()).await?;
        }
        Ok(())
    }
}
