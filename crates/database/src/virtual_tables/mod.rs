use common::{
    document::{
        DeveloperDocument,
        ResolvedDocument,
    },
    runtime::Runtime,
    types::WriteTimestamp,
    version::Version,
};
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
};

use crate::Transaction;

pub struct VirtualTable<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> VirtualTable<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[minitrace::trace]
    pub async fn get(
        &mut self,
        namespace: TableNamespace,
        id: DeveloperDocumentId,
        version: Option<Version>,
    ) -> anyhow::Result<Option<(DeveloperDocument, WriteTimestamp)>> {
        let tablet_id = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .number_to_tablet()(id.table())?;
        let id_ = ResolvedDocumentId::new(tablet_id, id);
        let system_table_name = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .tablet_name(tablet_id)?;

        // NOTE we intentionally pass `system_table_name` in, which means this
        // `get_inner` doesn't count as bandwidth. It's the caller's
        // responsibility to count bandwidth.
        let result = self.tx.get_inner(id_, system_table_name).await?;
        let table_mapping = self.tx.table_mapping().clone();
        match result {
            Some((doc, ts)) => {
                let doc = self.tx.virtual_system_mapping().system_to_virtual_doc(
                    doc,
                    &table_mapping,
                    version,
                )?;
                Ok(Some((doc, ts)))
            },
            None => Ok(None),
        }
    }

    pub fn map_system_doc_to_virtual_doc(
        &mut self,
        doc: ResolvedDocument,
        version: Option<Version>,
    ) -> anyhow::Result<DeveloperDocument> {
        let table_mapping = self.tx.table_mapping().clone();
        self.tx
            .virtual_system_mapping()
            .system_to_virtual_doc(doc, &table_mapping, version)
    }
}
