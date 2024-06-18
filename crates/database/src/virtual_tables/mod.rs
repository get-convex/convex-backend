use std::{
    collections::BTreeMap,
    sync::Arc,
};

use common::{
    document::{
        DeveloperDocument,
        ResolvedDocument,
    },
    runtime::Runtime,
    types::{
        IndexName,
        WriteTimestamp,
    },
    version::Version,
};
use errors::ErrorMetadata;
use imbl::OrdMap;
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableMapping,
    TableName,
    TableNamespace,
    VirtualTableMapping,
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
        id: DeveloperDocumentId,
        version: Option<Version>,
    ) -> anyhow::Result<Option<(DeveloperDocument, WriteTimestamp)>> {
        let virtual_table_name = self.tx.virtual_table_mapping().name(*id.table())?;
        let system_table_name = self
            .tx
            .virtual_system_mapping()
            .virtual_to_system_table(&virtual_table_name)?
            .clone();
        let table_id = self
            .tx
            .table_mapping()
            .namespace(TableNamespace::by_component_TODO())
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

pub trait VirtualSystemDocMapper: Send + Sync {
    fn system_to_virtual_doc(
        &self,
        virtual_system_mapping: &VirtualSystemMapping,
        doc: ResolvedDocument,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
        version: Version,
    ) -> anyhow::Result<DeveloperDocument>;
}

#[derive(Clone, Default)]
pub struct VirtualSystemMapping {
    virtual_to_system: OrdMap<TableName, TableName>,
    system_to_virtual: OrdMap<TableName, TableName>,
    virtual_to_system_indexes: OrdMap<IndexName, IndexName>,
    // system_table_name -> (Fn (SystemDoc) -> VirtualDoc)
    system_to_virtual_doc_mapper: OrdMap<TableName, Arc<dyn VirtualSystemDocMapper>>,
}

impl VirtualSystemMapping {
    pub fn add_table(
        &mut self,
        virt: &TableName,
        system: &TableName,
        indexes: BTreeMap<IndexName, IndexName>,
        mapper: Arc<dyn VirtualSystemDocMapper>,
    ) {
        self.virtual_to_system.insert(virt.clone(), system.clone());
        self.system_to_virtual.insert(system.clone(), virt.clone());
        self.virtual_to_system_indexes.extend(indexes);
        self.system_to_virtual_doc_mapper
            .insert(system.clone(), mapper);
    }

    pub fn is_virtual_table(&self, table_name: &TableName) -> bool {
        self.virtual_to_system.contains_key(table_name)
    }

    pub fn is_virtual_index(&self, index_name: &IndexName) -> bool {
        self.virtual_to_system_indexes.contains_key(index_name)
    }

    pub fn virtual_to_system_index(
        &self,
        virtual_index_name: &IndexName,
    ) -> anyhow::Result<&IndexName> {
        match self.virtual_to_system_indexes.get(virtual_index_name) {
            Some(system_index) => Ok(system_index),
            None => {
                anyhow::bail!("Could not find system index for virtual index {virtual_index_name}")
            },
        }
    }

    pub fn virtual_to_system_table(
        &self,
        virtual_table_name: &TableName,
    ) -> anyhow::Result<&TableName> {
        match self.virtual_to_system.get(virtual_table_name) {
            Some(system_table) => Ok(system_table),
            None => {
                anyhow::bail!("Could not find system table for virtual table {virtual_table_name}")
            },
        }
    }

    // Converts a virtual table DeveloperDocumentId to the system table ResolvedId.
    pub fn virtual_id_v6_to_system_resolved_doc_id(
        &self,
        virtual_id_v6: &DeveloperDocumentId,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let virtual_doc_id = virtual_id_v6.map_table(virtual_table_mapping.number_to_name())?;
        let system_table_name = self.virtual_to_system_table(virtual_doc_id.table())?;
        let system_table_id = table_mapping
            .namespace(TableNamespace::by_component_TODO())
            .id(system_table_name)?;
        Ok(ResolvedDocumentId::new(
            system_table_id.tablet_id,
            DeveloperDocumentId::new(system_table_id.table_number, virtual_id_v6.internal_id()),
        ))
    }

    // Converts a system table ResolvedDocumentId to the equivalent virtual table
    // DeveloperDocumentId by mapping the TableName and using the same InternalId
    pub fn system_resolved_id_to_virtual_developer_id(
        &self,
        system_doc_id: ResolvedDocumentId,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let system_table_name = table_mapping.tablet_name(system_doc_id.tablet_id)?;
        let virtual_table_name = match self.system_to_virtual.get(&system_table_name) {
            Some(virtual_table) => virtual_table.clone(),
            None => {
                anyhow::bail!("Could not find virtual table for system table {system_table_name}")
            },
        };
        let internal_id = system_doc_id.internal_id();
        let virtual_table_number =
            virtual_table_mapping.name_to_number_user_input()(virtual_table_name)?;
        Ok(DeveloperDocumentId::new(virtual_table_number, internal_id))
    }

    fn system_to_virtual_doc(
        &self,
        doc: ResolvedDocument,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
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
        let system_table_name = table_mapping.tablet_name(doc.id().tablet_id)?;
        let Some(mapper) = self.system_to_virtual_doc_mapper.get(&system_table_name) else {
            anyhow::bail!("System document cannot be converted to a virtual document")
        };
        mapper.system_to_virtual_doc(self, doc, table_mapping, virtual_table_mapping, version)
    }
}
