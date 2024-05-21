use anyhow::Context;
use common::{
    document::ResolvedDocument,
    runtime::Runtime,
};
use value::{
    ConvexObject,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    unauthorized_error,
    PatchValue,
    TableModel,
    Transaction,
};

/// We generally don't let `UserFacingModel` read or write system metadata,
/// so this model is an alternative path for internal use for manipulating
/// these tables.
///
/// Eventually, the goal is to entirely ban system metadata from
/// `UserFacingModel` and strictly require that `SystemMetadataModel` only looks
/// at system metadata.
pub struct SystemMetadataModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    // TODO(lee) pass namespace to transaction methods.
    _namespace: TableNamespace,
}

impl<'a, RT: Runtime> SystemMetadataModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self {
            tx,
            _namespace: namespace,
        }
    }

    /// Helper constructor to create a `SystemMetadataModel` in the Global
    /// namespace. Useful because many system tables only exist in the Global
    /// namespace.
    pub fn new_global(tx: &'a mut Transaction<RT>) -> Self {
        Self {
            tx,
            _namespace: TableNamespace::Global,
        }
    }

    /// Creates a new document with given value in the specified table,
    /// enforcing that the transaction's identity is system or admin.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn insert(
        &mut self,
        table: &TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        anyhow::ensure!(table.is_system());
        if !(self.tx.identity.is_system() || self.tx.identity.is_admin()) {
            anyhow::bail!(unauthorized_error("insert_metadata"));
        }
        let table_id = self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(table)
            .with_context(|| {
                if cfg!(any(test, feature = "testing")) {
                    format!(
                        "Failed to find system table {table} in a test. Try initializing system \
                         tables with:\nDbFixtures::new(&rt).await?.with_model().await?"
                    )
                } else {
                    format!("Failed to find system table {table}")
                }
            })?;
        let id = self.tx.id_generator.generate(&table_id);
        let creation_time = self.tx.next_creation_time.increment()?;
        let document = ResolvedDocument::new(id, creation_time, value)?;
        self.tx.insert_document(document).await
    }

    /// Creates a new document with given value in the specified table without
    /// checking authorization. This also inserts table metadata.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn insert_metadata(
        &mut self,
        table: &TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        TableModel::new(self.tx)
            .insert_table_metadata(table)
            .await?;
        let table_id = self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(table)?;
        let id = self.tx.id_generator.generate(&table_id);
        let creation_time = self.tx.next_creation_time.increment()?;
        let document = ResolvedDocument::new(id, creation_time, value)?;
        self.tx.insert_document(document).await
    }

    /// Merges the existing document with the given object. Will overwrite any
    /// conflicting fields.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn patch(
        &mut self,
        id: ResolvedDocumentId,
        value: PatchValue,
    ) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .is_system(id.table().table_number));

        self.tx.patch_inner(id, value).await
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn replace(
        &mut self,
        id: ResolvedDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .is_system(id.table().table_number));
        self.tx.replace_inner(id, value).await
    }

    /// Delete the document at the given path.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn delete(&mut self, id: ResolvedDocumentId) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .is_system(id.table().table_number));
        self.tx.delete_inner(id).await
    }
}
