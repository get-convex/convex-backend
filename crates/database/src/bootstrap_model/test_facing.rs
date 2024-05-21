use common::{
    document::ResolvedDocument,
    runtime::Runtime,
};
use value::{
    ConvexObject,
    ResolvedDocumentId,
    TableName,
};

use crate::{
    SystemMetadataModel,
    Transaction,
};

pub struct TestFacingModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> TestFacingModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    /// Insert a new document and immediately read it. Prefer using `insert`
    /// unless you need to read the creation time.
    #[convex_macro::instrument_future]
    pub async fn insert(
        &mut self,
        table: &TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocumentId> {
        SystemMetadataModel::new_global(self.tx)
            .insert_metadata(table, value)
            .await
    }

    /// Insert a new document and immediately read it. Prefer using `insert`
    /// unless you need to read the creation time.
    #[convex_macro::instrument_future]
    pub async fn insert_and_get(
        &mut self,
        table: TableName,
        value: ConvexObject,
    ) -> anyhow::Result<ResolvedDocument> {
        let id = self.insert(&table, value).await?;
        self.tx
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document with id {id} must exist"))
    }
}
