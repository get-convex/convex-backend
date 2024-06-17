use std::{
    collections::BTreeSet,
    ops::{
        Deref,
        DerefMut,
    },
    sync::Arc,
};

use value::{
    id_v6::DeveloperDocumentId,
    ResolvedDocumentId,
    TableIdentifier,
    TableMapping,
    TableNamespace,
    TableNumber,
    TabletId,
    TabletIdAndTableNumber,
    VirtualTableMapping,
};

use crate::{
    bootstrap_model::{
        index::INDEX_TABLE,
        tables::{
            TableMetadata,
            TABLES_TABLE,
        },
    },
    document::{
        CreationTime,
        InternalId,
        ResolvedDocument,
    },
    index::IndexKey,
    persistence::{
        ConflictStrategy,
        Persistence,
        PersistenceGlobalKey,
    },
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        TableName,
        Timestamp,
    },
};

/// A simple incrementing IdGenerator for use in tests.
pub struct TestIdGenerator {
    curr: u32,
    curr_table_number: TableNumber,
    table_mapping: TableMapping,
    pub virtual_table_mapping: VirtualTableMapping,
}

impl Deref for TestIdGenerator {
    type Target = TableMapping;

    fn deref(&self) -> &Self::Target {
        &self.table_mapping
    }
}

impl DerefMut for TestIdGenerator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.table_mapping
    }
}

impl TestIdGenerator {
    pub fn new() -> Self {
        Self {
            curr: 0,
            curr_table_number: <TableNumber as TableIdentifier>::min(),
            table_mapping: TableMapping::new(),
            virtual_table_mapping: VirtualTableMapping::new(),
        }
    }

    pub fn generate_table_name(&self) -> TableName {
        format!("test{}", self.curr_table_number.increment().unwrap())
            .parse()
            .unwrap()
    }

    pub fn generate_internal(&mut self) -> InternalId {
        let mut new_id = [0u8; 16];
        new_id[12..].copy_from_slice(&self.curr.to_be_bytes());
        self.curr += 1;
        InternalId(new_id)
    }

    pub fn system_table_id(&mut self, table_name: &TableName) -> TabletIdAndTableNumber {
        assert!(table_name.is_system(), "use user_table_id instead");
        if let Ok(table_id) = self.namespace(TableNamespace::Global).id(table_name) {
            return table_id;
        }
        let tablet_id = TabletId(self.generate_internal());
        let table_number = self.curr_table_number;
        self.curr_table_number = self
            .curr_table_number
            .increment()
            .expect("Could not increment table number");
        self.table_mapping.insert(
            tablet_id,
            TableNamespace::Global,
            table_number,
            table_name.clone(),
        );
        self.system_table_id(&TABLES_TABLE);
        self.system_table_id(&INDEX_TABLE);
        TabletIdAndTableNumber {
            table_number,
            tablet_id,
        }
    }

    // For adding to physical table mapping
    pub fn user_table_id(&mut self, table_name: &TableName) -> TabletIdAndTableNumber {
        assert!(!table_name.is_system(), "use system_table_id instead");
        if let Ok(table_id) = self.namespace(TableNamespace::test_user()).id(table_name) {
            return table_id;
        }
        let tablet_id = TabletId(self.generate_internal());
        let table_number = self.curr_table_number;
        self.curr_table_number = self
            .curr_table_number
            .increment()
            .expect("Could not increment table number");
        self.table_mapping.insert(
            tablet_id,
            TableNamespace::test_user(),
            table_number,
            table_name.clone(),
        );
        self.system_table_id(&TABLES_TABLE);
        self.system_table_id(&INDEX_TABLE);
        TabletIdAndTableNumber {
            table_number,
            tablet_id,
        }
    }

    // For adding to virtual table mapping
    pub fn generate_virtual_table(&mut self, table_name: &TableName) -> TableNumber {
        if let Ok(table_number) = self.virtual_table_mapping.number(table_name) {
            return table_number;
        }
        let num = self.curr_table_number;
        self.curr_table_number = self
            .curr_table_number
            .increment()
            .expect("Could not increment table number");
        self.virtual_table_mapping.insert(num, table_name.clone());
        self.system_table_id(&TABLES_TABLE);
        self.system_table_id(&INDEX_TABLE);
        num
    }

    pub async fn write_tables(&mut self, p: Arc<dyn Persistence>) -> anyhow::Result<()> {
        let tables_by_id = self.generate_internal();
        p.write_persistence_global(
            PersistenceGlobalKey::TablesByIdIndex,
            tables_by_id.to_string().into(),
        )
        .await?;
        let ts = Timestamp::MIN;
        let mut documents = vec![];
        let mut indexes = BTreeSet::new();
        let tables_table_id = self
            .table_mapping
            .namespace(TableNamespace::Global)
            .name_to_id()(TABLES_TABLE.clone())?;
        for (table_id, namespace, table_number, table_name) in self.table_mapping.iter() {
            let table_metadata = TableMetadata::new(namespace, table_name.clone(), table_number);
            let id = ResolvedDocumentId::new(
                tables_table_id.tablet_id,
                tables_table_id.table_number.id(table_id.0),
            );
            let doc = ResolvedDocument::new(id, CreationTime::ONE, table_metadata.try_into()?)?;
            let index_update = DatabaseIndexUpdate {
                index_id: tables_by_id,
                key: IndexKey::new(vec![], id.into()),
                value: DatabaseIndexValue::NonClustered(id),
                is_system_index: false,
            };
            documents.push((ts, doc.id_with_table_id(), Some(doc)));
            indexes.insert((ts, index_update));
        }
        p.write(documents, indexes, ConflictStrategy::Error).await?;
        Ok(())
    }

    pub fn user_generate(&mut self, table_name: &TableName) -> ResolvedDocumentId {
        assert!(!table_name.is_system(), "use system_generate instead");
        let table_id = self.user_table_id(table_name);
        ResolvedDocumentId::new(
            table_id.tablet_id,
            table_id.table_number.id(self.generate_internal()),
        )
    }

    pub fn system_generate(&mut self, table_name: &TableName) -> ResolvedDocumentId {
        assert!(table_name.is_system(), "use user_generate instead");
        let table_id = self.system_table_id(table_name);
        ResolvedDocumentId::new(
            table_id.tablet_id,
            table_id.table_number.id(self.generate_internal()),
        )
    }

    pub fn generate_virtual(&mut self, table_name: &TableName) -> DeveloperDocumentId {
        let table_num = self.generate_virtual_table(table_name);
        table_num.id(self.generate_internal())
    }
}
