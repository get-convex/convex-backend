use anyhow::Context;
use common::{
    document::ResolvedDocument,
    runtime::Runtime,
};
use value::{
    ConvexObject,
    DeveloperDocumentId,
    InternalId,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
    TabletIdAndTableNumber,
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
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SystemMetadataModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    /// Helper constructor to create a `SystemMetadataModel` in the Global
    /// namespace. Useful because many system tables only exist in the Global
    /// namespace.
    pub fn new_global(tx: &'a mut Transaction<RT>) -> Self {
        Self {
            tx,
            namespace: TableNamespace::Global,
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
        let table_id = self.lookup_table_id(table)?;
        let id = ResolvedDocumentId::new(
            table_id.tablet_id,
            DeveloperDocumentId::new(
                table_id.table_number,
                self.tx.id_generator.generate_internal(),
            ),
        );
        let creation_time = self.tx.next_creation_time.increment()?;
        let document = ResolvedDocument::new(id, creation_time, value)?;
        self.tx.insert_document(document).await
    }

    pub fn allocate_internal_id(&mut self) -> anyhow::Result<InternalId> {
        Ok(self.tx.id_generator.generate_internal())
    }

    /// Create a new document with a predetermined internal ID.
    pub async fn insert_with_internal_id(
        &mut self,
        table: &TableName,
        internal_id: InternalId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        anyhow::ensure!(table.is_system());
        if !(self.tx.identity.is_system() || self.tx.identity.is_admin()) {
            anyhow::bail!(unauthorized_error("insert_metadata"));
        }
        let table_id = self.lookup_table_id(table)?;
        let document_id = ResolvedDocumentId::new(
            table_id.tablet_id,
            DeveloperDocumentId::new(table_id.table_number, internal_id),
        );
        let creation_time = self.tx.next_creation_time.increment()?;
        let document = ResolvedDocument::new(document_id, creation_time, value)?;
        self.tx.insert_document(document).await
    }

    fn lookup_table_id(&mut self, table: &TableName) -> anyhow::Result<TabletIdAndTableNumber> {
        self.tx
            .table_mapping()
            .namespace(self.namespace)
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
            })
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
            .insert_table_metadata(self.namespace, table)
            .await?;
        let table_id = self
            .tx
            .table_mapping()
            .namespace(self.namespace)
            .id(table)?;
        let id = self.tx.id_generator.generate_resolved(table_id);
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
        anyhow::ensure!(self.tx.table_mapping().is_system_tablet(id.tablet_id));

        self.tx.patch_inner(id, value).await
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn replace(
        &mut self,
        id: ResolvedDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self.tx.table_mapping().is_system_tablet(id.tablet_id));
        self.tx.replace_inner(id, value).await
    }

    /// Delete the document at the given path.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn delete(&mut self, id: ResolvedDocumentId) -> anyhow::Result<ResolvedDocument> {
        anyhow::ensure!(self.tx.table_mapping().is_system_tablet(id.tablet_id));
        self.tx.delete_inner(id).await
    }
}
