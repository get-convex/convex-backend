//! Default state to initialize the database with.

use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{
        Arc,
        LazyLock,
    },
};

use common::{
    bootstrap_model::{
        index::{
            database_index::IndexedFields,
            INDEX_TABLE,
        },
        tables::TABLES_TABLE,
    },
    document::ResolvedDocument,
    types::{
        IndexName,
        TableName,
    },
    value::{
        TableMapping,
        TabletIdAndTableNumber,
    },
};
use maplit::btreemap;
use value::{
    TableNumber,
    TabletId,
};

use crate::{
    bootstrap_model::{
        components::{
            definition::COMPONENT_DEFINITIONS_TABLE,
            COMPONENTS_TABLE,
        },
        index::IndexTable,
        index_workers::IndexWorkerMetadataTable,
        schema::SchemasTable,
        table::TablesTable,
        virtual_tables::VirtualTablesTable,
    },
    ComponentDefinitionsTable,
    ComponentsTable,
    VirtualSystemDocMapper,
    INDEX_WORKER_METADATA_TABLE,
    NUM_RESERVED_LEGACY_TABLE_NUMBERS,
    SCHEMAS_TABLE,
    VIRTUAL_TABLES_TABLE,
};

pub fn system_index(table: &impl Deref<Target = TableName>, name: &'static str) -> IndexName {
    IndexName::new(
        table.deref().clone(),
        name.parse().expect("Invalid system index descriptor"),
    )
    .expect("Invalid system index")
}

pub trait SystemTable: Send + Sync {
    /// Table name for this system table. Must begin with `_`
    fn table_name(&self) -> &'static TableName;
    /// List of indexes for the system table
    fn indexes(&self) -> Vec<SystemIndex>;
    fn virtual_table(
        &self,
    ) -> Option<(
        &'static TableName,
        BTreeMap<IndexName, IndexName>,
        Arc<dyn VirtualSystemDocMapper>,
    )> {
        None
    }

    /// Check that a document is valid for this system table.
    /// We can't return the parsed document struct because its type might not
    /// be accessible from db-verifier.
    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()>;
}

pub struct SystemIndex {
    pub name: IndexName,
    pub fields: IndexedFields,
}

pub fn bootstrap_system_tables() -> Vec<&'static dyn SystemTable> {
    vec![
        &TablesTable,
        &IndexTable,
        &SchemasTable,
        &VirtualTablesTable,
        &IndexWorkerMetadataTable,
        &ComponentDefinitionsTable,
        &ComponentsTable,
    ]
}

pub static DEFAULT_BOOTSTRAP_TABLE_NUMBERS: LazyLock<BTreeMap<TableName, TableNumber>> =
    LazyLock::new(|| {
        let tn = |tn| TableNumber::try_from(NUM_RESERVED_LEGACY_TABLE_NUMBERS + tn).unwrap();
        btreemap! {
            TABLES_TABLE.clone() => tn(1),
            INDEX_TABLE.clone() => tn(2),
            SCHEMAS_TABLE.clone() => tn(20),
            VIRTUAL_TABLES_TABLE.clone() => tn(26),
            INDEX_WORKER_METADATA_TABLE.clone() => tn(30),
            COMPONENT_DEFINITIONS_TABLE.clone() => tn(31),
            COMPONENTS_TABLE.clone() => tn(32),
            // To add a bootstrap system table, first add to model/src/lib and then
            // replicate that table number to here.
        }
    });

#[cfg(test)]
mod test_bootstrap_system_tables {
    use std::collections::BTreeSet;

    use super::{
        bootstrap_system_tables,
        DEFAULT_BOOTSTRAP_TABLE_NUMBERS,
    };

    #[test]
    fn test_ensure_consistent() {
        assert_eq!(
            bootstrap_system_tables()
                .into_iter()
                .map(|t| t.table_name())
                .collect::<BTreeSet<_>>(),
            DEFAULT_BOOTSTRAP_TABLE_NUMBERS
                .keys()
                .collect::<BTreeSet<_>>(),
        );
    }
}

/// Contains the table_id and index_id that never change after initializing the
/// backend database. We prefer to pass this around instead of the full
/// TableMapping so don't worry about passing around a reference to the
/// "authoritative" table mapping.
#[derive(Clone, Copy)]
pub struct BootstrapTableIds {
    pub tables_id: TabletIdAndTableNumber,
    pub index_id: TabletIdAndTableNumber,
}

impl BootstrapTableIds {
    pub fn new(table_mapping: &TableMapping) -> Self {
        let tables_id = table_mapping
            .id(&TABLES_TABLE)
            .expect("_tables should exist");
        let index_id = table_mapping.id(&INDEX_TABLE).expect("_index should exist");
        Self {
            tables_id,
            index_id,
        }
    }

    pub fn is_index_table(&self, table_id: TabletIdAndTableNumber) -> bool {
        self.index_id == table_id
    }

    pub fn is_tables_table(&self, table_id: TabletIdAndTableNumber) -> bool {
        self.tables_id == table_id
    }

    pub fn is_index_tablet_id(&self, tablet_id: TabletId) -> bool {
        self.index_id.tablet_id == tablet_id
    }

    pub fn is_tables_tablet_id(&self, tablet_id: TabletId) -> bool {
        self.tables_id.tablet_id == tablet_id
    }
}
