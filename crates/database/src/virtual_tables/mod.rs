use async_trait::async_trait;
use common::{
    document::{
        DeveloperDocument,
        ResolvedDocument,
    },
    runtime::Runtime,
    types::WriteTimestamp,
    version::Version,
    virtual_system_mapping::GetDocument,
};
use errors::ErrorMetadata;
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
};

use crate::Transaction;

#[async_trait]
impl<RT: Runtime> GetDocument for Transaction<RT> {
    async fn get_document(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<ResolvedDocument>> {
        self.get(id).await
    }
}

pub struct VirtualTable<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> VirtualTable<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[fastrace::trace]
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
        match result {
            Some((doc, ts)) => {
                let doc = self.system_to_virtual_doc(doc, version)?;
                Ok(Some((doc, ts)))
            },
            None => Ok(None),
        }
    }

    pub fn system_to_virtual_doc(
        &mut self,
        doc: ResolvedDocument,
        version: Option<Version>,
    ) -> anyhow::Result<DeveloperDocument> {
        if version.is_none() {
            return Err(ErrorMetadata::bad_request(
                "InvalidClientVersion",
                "Upgrade to NPM version 1.6.1 or above to access system tables",
            )
            .into());
        }
        let version = version.unwrap();
        let virtual_system_mapping = self.tx.virtual_system_mapping().clone();
        let table_mapping = self.tx.table_mapping().clone();
        let system_table_name = table_mapping.tablet_name(doc.id().tablet_id)?;
        let Some(mapper) = virtual_system_mapping
            .system_to_virtual_doc_mapper
            .get(&system_table_name)
        else {
            anyhow::bail!("System document cannot be converted to a virtual document")
        };
        mapper.system_to_virtual_doc(
            self.tx,
            &virtual_system_mapping,
            doc,
            &table_mapping,
            version,
        )
    }
}
