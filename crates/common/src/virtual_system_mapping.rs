use std::{
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context;
use imbl::OrdMap;
use semver::Version;
use tonic::async_trait;
use value::{
    DeveloperDocumentId,
    NamespacedTableMapping,
    ResolvedDocumentId,
    TableMapping,
    TableName,
    TableNamespace,
    TableNumber,
};

use crate::{
    document::{
        DeveloperDocument,
        ResolvedDocument,
    },
    types::IndexName,
};

pub trait VirtualSystemDocMapper: Send + Sync {
    fn system_to_virtual_doc(
        &self,
        tx: &mut dyn GetDocument,
        virtual_system_mapping: &VirtualSystemMapping,
        doc: ResolvedDocument,
        table_mapping: &TableMapping,
        version: Version,
    ) -> anyhow::Result<DeveloperDocument>;
}

/// This trait is used for dependency injection, exposing get_document
/// (implemented by `Transaction`) to convert system documents joined across
/// multiple system tables to virtual documents in `VirtualSystemDocMapper`.
#[async_trait]
pub trait GetDocument {
    async fn get_document(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<ResolvedDocument>>;
}

#[cfg(any(test, feature = "testing"))]
pub struct NoopDocMapper;

#[cfg(any(test, feature = "testing"))]
pub mod test_virtual_system_mapping {
    use value::TableMapping;

    use super::NoopDocMapper;
    use crate::{
        document::{
            DeveloperDocument,
            ResolvedDocument,
        },
        version::Version,
        virtual_system_mapping::{
            GetDocument,
            VirtualSystemDocMapper,
            VirtualSystemMapping,
        },
    };

    impl VirtualSystemDocMapper for NoopDocMapper {
        fn system_to_virtual_doc(
            &self,
            _tx: &mut dyn GetDocument,
            _virtual_system_mapping: &VirtualSystemMapping,
            doc: ResolvedDocument,
            _table_mapping: &TableMapping,
            _version: Version,
        ) -> anyhow::Result<DeveloperDocument> {
            Ok(doc.to_developer())
        }
    }
}

#[derive(Clone, Default)]
pub struct VirtualSystemMapping {
    virtual_to_system: OrdMap<TableName, TableName>,
    system_to_virtual: OrdMap<TableName, TableName>,
    virtual_to_system_indexes: OrdMap<IndexName, IndexName>,
    // system_table_name -> (Fn (SystemDoc) -> VirtualDoc)
    pub system_to_virtual_doc_mapper: OrdMap<TableName, Arc<dyn VirtualSystemDocMapper>>,
}

impl std::fmt::Debug for VirtualSystemMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualSystemMapping")
            .field("virtual_to_system", &self.virtual_to_system)
            .field("virtual_to_system_indexes", &self.virtual_to_system_indexes)
            .finish()
    }
}

impl PartialEq for VirtualSystemMapping {
    fn eq(&self, other: &Self) -> bool {
        self.virtual_to_system == other.virtual_to_system
            && self.virtual_to_system_indexes == other.virtual_to_system_indexes
    }
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

    pub fn system_to_virtual_table(&self, system_table_name: &TableName) -> Option<&TableName> {
        self.system_to_virtual.get(system_table_name)
    }

    // Converts a virtual table DeveloperDocumentId to the system table ResolvedId.
    pub fn virtual_id_v6_to_system_resolved_doc_id(
        &self,
        namespace: TableNamespace,
        virtual_id_v6: &DeveloperDocumentId,
        table_mapping: &TableMapping,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let table_number = virtual_id_v6.table();
        let tablet_id = table_mapping.namespace(namespace).number_to_tablet()(table_number)
            .with_context(|| {
                format!("cannot find table with id {table_number} in {namespace:?}")
            })?;
        Ok(ResolvedDocumentId::new(tablet_id, *virtual_id_v6))
    }

    // Converts a system table ResolvedDocumentId to the equivalent virtual table
    // DeveloperDocumentId by mapping the TableName and using the same InternalId
    pub fn system_resolved_id_to_virtual_developer_id(
        &self,
        system_doc_id: ResolvedDocumentId,
    ) -> anyhow::Result<DeveloperDocumentId> {
        Ok(system_doc_id.developer_id)
    }
}

// Checks both virtual tables and tables to get the table number to name mapping
pub fn all_tables_number_to_name(
    table_mapping: &NamespacedTableMapping,
    virtual_system_mapping: &VirtualSystemMapping,
) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + use<> {
    let table_mapping = table_mapping.clone();
    let virtual_system_mapping = virtual_system_mapping.clone();
    move |number| {
        let physical_name = table_mapping.number_to_name()(number)?;
        if let Some(virtual_name) = virtual_system_mapping.system_to_virtual.get(&physical_name) {
            Ok(virtual_name.clone())
        } else {
            Ok(physical_name)
        }
    }
}

// Checks both virtual tables and tables to get the table name to number mapping
pub fn all_tables_name_to_number(
    namespace: TableNamespace,
    table_mapping: &TableMapping,
    virtual_system_mapping: &VirtualSystemMapping,
) -> impl Fn(TableName) -> anyhow::Result<TableNumber> + use<> {
    let table_mapping = table_mapping.clone();
    let virtual_system_mapping = virtual_system_mapping.clone();
    move |name| {
        let name = if let Some(physical_table) = virtual_system_mapping.virtual_to_system.get(&name)
        {
            physical_table.clone()
        } else {
            name
        };
        table_mapping
            .namespace(namespace)
            .name_to_number_user_input()(name)
    }
}
