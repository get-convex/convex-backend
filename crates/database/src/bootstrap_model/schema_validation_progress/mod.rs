pub mod types;

use std::sync::{
    Arc,
    LazyLock,
};

use common::{
    document::{
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

use crate::{
    system_tables::{
        SystemIndex,
        SystemTable,
    },
    SchemaValidationProgressMetadata,
    SystemMetadataModel,
    Transaction,
};

pub static SCHEMA_VALIDATION_PROGRESS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_schema_validation_progress"
        .parse()
        .expect("Invalid built-in _schema_validation_progress table")
});

pub static SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID: LazyLock<
    SystemIndex<SchemaValidationProgressTable>,
> = LazyLock::new(|| {
    SystemIndex::new(
        "by_schema_id",
        [&SCHEMA_ID_FIELD, &CREATION_TIME_FIELD_PATH],
    )
    .unwrap()
});

static SCHEMA_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "schemaId".parse().expect("invalid schemaId field"));

pub struct SchemaValidationProgressTable;

impl SystemTable for SchemaValidationProgressTable {
    type Metadata = types::SchemaValidationProgressMetadata;

    fn table_name() -> &'static TableName {
        &SCHEMA_VALIDATION_PROGRESS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID.clone()]
    }
}

pub struct SchemaValidationProgressModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SchemaValidationProgressModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    pub async fn existing_schema_validation_progress(
        &mut self,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<Arc<ParsedDocument<SchemaValidationProgressMetadata>>>> {
        self.tx
            .query_system(self.namespace, &*SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID)?
            .eq(&[schema_id.developer_id.encode_into(&mut Default::default())])?
            .unique()
            .await
    }

    pub async fn initialize_schema_validation_progress(
        &mut self,
        schema_id: ResolvedDocumentId,
        total_docs: Option<u64>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let maybe_existing_metadata = self.existing_schema_validation_progress(schema_id).await?;
        let mut system_model = SystemMetadataModel::new(self.tx, self.namespace);
        let new_metadata = SchemaValidationProgressMetadata {
            schema_id: schema_id.developer_id,
            total_docs,
            num_docs_validated: 0,
        };
        if let Some(existing_metadata) = maybe_existing_metadata {
            system_model
                .replace(existing_metadata.id(), new_metadata.try_into()?)
                .await?;
            Ok(existing_metadata.id())
        } else {
            system_model
                .insert(&SCHEMA_VALIDATION_PROGRESS_TABLE, new_metadata.try_into()?)
                .await
        }
    }

    /// Update the schema validation progress for a schema, adding
    /// `num_docs_validated` to the existing progress.
    /// Returns false if there is no existing progress metadata to update.
    pub async fn update_schema_validation_progress(
        &mut self,
        schema_id: ResolvedDocumentId,
        num_docs_validated: u64,
        // Only used if total_docs is missing
        total_docs: Option<u64>,
    ) -> anyhow::Result<bool> {
        let maybe_existing_metadata = self.existing_schema_validation_progress(schema_id).await?;
        let mut system_model = SystemMetadataModel::new(self.tx, self.namespace);
        let Some(existing_metadata) = maybe_existing_metadata else {
            return Ok(false);
        };

        let num_docs_validated = existing_metadata.num_docs_validated + num_docs_validated;
        let new_metadata = SchemaValidationProgressMetadata {
            schema_id: schema_id.developer_id,
            total_docs: existing_metadata.total_docs.or(total_docs),
            num_docs_validated,
        };
        system_model
            .replace(existing_metadata.id(), new_metadata.try_into()?)
            .await?;
        Ok(true)
    }

    pub async fn delete_schema_validation_progress(
        &mut self,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        if let Some(existing_metadata) = self.existing_schema_validation_progress(schema_id).await?
        {
            SystemMetadataModel::new_global(self.tx)
                .delete(existing_metadata.id())
                .await?;
        }
        Ok(())
    }
}
