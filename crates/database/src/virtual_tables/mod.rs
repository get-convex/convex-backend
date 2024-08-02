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
        let virtual_table_name = self
            .tx
            .virtual_table_mapping()
            .namespace(namespace)
            .name(id.table())?;
        let system_table_name = self
            .tx
            .virtual_system_mapping()
            .virtual_to_system_table(&virtual_table_name)?
            .clone();
        let table_id = self
            .tx
            .table_mapping()
            .namespace(namespace)
            .id(&system_table_name)?;
        let id_ = ResolvedDocumentId::new(
            table_id.tablet_id,
            DeveloperDocumentId::new(table_id.table_number, id.internal_id()),
        );

        // NOTE we intentionally pass `system_table_name` in, which means this
        // `get_inner` doesn't count as bandwidth. It's the caller's
        // responsibility to count bandwidth.
        let result = self.tx.get_inner(id_, system_table_name).await?;
        let table_mapping = self.tx.table_mapping().clone();
        let virtual_table_mapping = self.tx.virtual_table_mapping().clone();
        match result {
            Some((doc, ts)) => {
                let doc = self.tx.virtual_system_mapping().system_to_virtual_doc(
                    doc,
                    &table_mapping,
                    &virtual_table_mapping,
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
        let virtual_table_mapping = self.tx.virtual_table_mapping().clone();
        self.tx.virtual_system_mapping().system_to_virtual_doc(
            doc,
            &table_mapping,
            &virtual_table_mapping,
            version,
        )
    }
}
