//! Database metadata. Currently this metadata is just used to store the shape
//! and size for each table.

use common::{
    bootstrap_model::tables::{
        TableMetadata,
        TableState,
        TABLES_TABLE,
    },
    types::{
        PersistenceVersion,
        TableName,
    },
    value::{
        ConvexObject,
        ResolvedDocumentId,
        TableMapping,
        TabletId,
        TabletIdAndTableNumber,
        VirtualTableMapping,
    },
    virtual_system_mapping::{
        all_tables_number_to_name,
        VirtualSystemMapping,
    },
};
use imbl::OrdMap;
use indexing::index_registry::IndexRegistry;
use value::{
    TableNamespace,
    TableNumber,
};

use crate::{
    defaults::bootstrap_system_tables,
    metrics::bootstrap_table_registry_timer,
    VirtualTableMetadata,
    VIRTUAL_TABLES_TABLE,
};

/// This structure is an index over the `_tables` and `_virtual_tables` tables
/// that represents all of the tables in their system and their metadata.
///
/// In addition, it also tracks the current shapes of each table, which reflect
/// all of the data in the system.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRegistry {
    tablet_states: OrdMap<TabletId, TableState>,
    table_mapping: TableMapping,
    persistence_version: PersistenceVersion,

    /// Mapping from virtual table number to name.
    virtual_table_mapping: VirtualTableMapping,
    /// Mapping from virtual table name to corresponding system table name.
    virtual_system_mapping: VirtualSystemMapping,
}

impl TableRegistry {
    /// Fill out all of our table metadata from the latest version of each
    /// document in the `_tables` table. In particular, we expect to find
    /// exactly one record for the `_tables` table.
    pub fn bootstrap(
        table_mapping: TableMapping,
        table_states: OrdMap<TabletId, TableState>,
        persistence_version: PersistenceVersion,
        virtual_table_mapping: VirtualTableMapping,
        virtual_system_mapping: VirtualSystemMapping,
    ) -> anyhow::Result<Self> {
        let _timer = bootstrap_table_registry_timer();
        Ok(Self {
            table_mapping,
            tablet_states: table_states,
            persistence_version,
            virtual_table_mapping,
            virtual_system_mapping,
        })
    }

    pub(crate) fn update(
        &mut self,
        index_registry: &IndexRegistry,
        id: ResolvedDocumentId,
        old_value: Option<&ConvexObject>,
        new_value: Option<&ConvexObject>,
    ) -> anyhow::Result<Option<TableUpdate>> {
        let maybe_table_update = self
            .begin_update(index_registry, id, old_value, new_value)?
            .apply();
        Ok(maybe_table_update)
    }

    pub(crate) fn begin_update<'a>(
        &'a mut self,
        index_registry: &IndexRegistry,
        id: ResolvedDocumentId,
        old_value: Option<&ConvexObject>,
        new_value: Option<&ConvexObject>,
    ) -> anyhow::Result<Update<'a>> {
        let mut virtual_table_creation = None;

        let table_update = if self
            .table_mapping
            .namespace(TableNamespace::Global)
            .tablet_matches_name(id.tablet_id, &TABLES_TABLE)
        {
            let tablet_id = TabletId(id.internal_id());
            match (old_value, new_value) {
                // Table creation
                (None, Some(new_value)) => {
                    let metadata = TableMetadata::try_from(new_value.clone())?;
                    let table_id_and_code = TabletIdAndTableNumber {
                        tablet_id,
                        table_number: metadata.number,
                    };
                    if metadata.is_active() {
                        if self.table_exists(metadata.namespace, &metadata.name) {
                            anyhow::bail!("Tried to create duplicate table {new_value}");
                        }
                        self.validate_table_number(
                            metadata.namespace,
                            metadata.number,
                            &metadata.name,
                        )?;
                    }
                    Some(TableUpdate {
                        namespace: metadata.namespace,
                        table_id_and_number: table_id_and_code,
                        table_name: metadata.name,
                        state: metadata.state,
                        mode: TableUpdateMode::Create,
                    })
                },
                (Some(_), None) => {
                    anyhow::bail!("_tables delete not allowed, set state to Deleting instead");
                },
                // Table edits, which can delete tables.
                (Some(old_value), Some(new_value)) => {
                    let new_metadata = TableMetadata::try_from(new_value.clone())?;
                    let old_metadata = TableMetadata::try_from(old_value.clone())?;

                    let old_table_id_and_number = TabletIdAndTableNumber {
                        tablet_id,
                        table_number: old_metadata.number,
                    };
                    anyhow::ensure!(
                        old_metadata.name == new_metadata.name,
                        "Table renames currently unsupported: {old_metadata:?} => {new_metadata:?}"
                    );
                    anyhow::ensure!(
                        old_metadata.number == new_metadata.number,
                        "Cannot change the table number in a table edit: {old_metadata:?} => \
                         {new_metadata:?}"
                    );

                    if old_metadata.is_active()
                        && matches!(new_metadata.state, TableState::Deleting)
                    {
                        // Table deletion.
                        anyhow::ensure!(
                            bootstrap_system_tables()
                                .iter()
                                .all(|t| t.table_name() != &new_metadata.name),
                            "cannot delete bootstrap system table"
                        );
                        anyhow::ensure!(index_registry.has_no_indexes(tablet_id));
                        Some(TableUpdate {
                            namespace: old_metadata.namespace,
                            table_id_and_number: old_table_id_and_number,
                            table_name: old_metadata.name,
                            state: new_metadata.state,
                            mode: TableUpdateMode::Drop,
                        })
                    } else if matches!(old_metadata.state, TableState::Hidden)
                        && new_metadata.is_active()
                    {
                        // Table changing from hidden -> active.
                        Some(TableUpdate {
                            namespace: old_metadata.namespace,
                            table_id_and_number: old_table_id_and_number,
                            table_name: old_metadata.name,
                            state: new_metadata.state,
                            mode: TableUpdateMode::Activate,
                        })
                    } else {
                        // Allow updating other fields on TableMetadata.
                        None
                    }
                },
                (None, None) => anyhow::bail!("cannot delete tombstone"),
            }
        } else {
            None
        };

        if self
            .table_mapping
            .namespace(TableNamespace::Global)
            .tablet_matches_name(id.tablet_id, &VIRTUAL_TABLES_TABLE)
        {
            match (old_value, new_value) {
                // Virtual table creation
                (None, Some(new_value)) => {
                    let metadata = VirtualTableMetadata::try_from(new_value.clone())?;
                    self.validate_table_number(
                        metadata.namespace,
                        metadata.number,
                        &metadata.name,
                    )?;
                    virtual_table_creation =
                        Some((metadata.namespace, metadata.number, metadata.name));
                },
                _ => anyhow::bail!("Only inserts are supported on Virtual Tables"),
            }
        }

        let update = Update {
            metadata: self,
            table_update,
            virtual_table_creation,
        };
        Ok(update)
    }

    fn validate_table_number(
        &self,
        namespace: TableNamespace,
        table_number: TableNumber,
        table_name: &TableName,
    ) -> anyhow::Result<()> {
        if let Some(existing_table) = self
            .table_mapping
            .namespace(namespace)
            .name_by_number_if_exists(table_number)
        {
            if self.virtual_system_mapping.is_virtual_table(table_name)
                && self
                    .virtual_system_mapping
                    .virtual_to_system_table(table_name)?
                    == existing_table
            {
                // A virtual table can share a table number with its physical
                // table.
            } else {
                anyhow::ensure!(
                    existing_table == table_name,
                    "Cannot add a table {table_name} with table number {table_number} since it \
                     already exists in the table mapping as {existing_table}"
                );
            }
        }
        Ok(())
    }

    pub fn table_state(&self, tablet_id: TabletId) -> Option<TableState> {
        self.tablet_states.get(&tablet_id).cloned()
    }

    pub fn user_table_names(&self) -> impl Iterator<Item = (TableNamespace, &TableName)> {
        self.table_mapping
            .iter_active_user_tables()
            .map(|(_, namespace, _, name)| (namespace, name))
    }

    pub fn table_exists(&self, namespace: TableNamespace, table: &TableName) -> bool {
        self.table_mapping.namespace(namespace).name_exists(table)
    }

    pub fn iter_active_user_tables(
        &self,
    ) -> impl Iterator<Item = (TabletId, TableNamespace, TableNumber, &TableName)> {
        self.table_mapping.iter_active_user_tables()
    }

    pub fn iter_active_system_tables(
        &self,
    ) -> impl Iterator<Item = (TabletId, TableNamespace, TableNumber, &TableName)> {
        self.table_mapping
            .iter()
            .filter(|(table_id, _, _, table_name)| {
                table_name.is_system()
                    && matches!(self.tablet_states.get(table_id), Some(TableState::Active))
            })
    }

    pub fn table_mapping(&self) -> &TableMapping {
        &self.table_mapping
    }

    pub(crate) fn tablet_states(&self) -> &OrdMap<TabletId, TableState> {
        &self.tablet_states
    }

    pub(crate) fn virtual_table_mapping(&self) -> &VirtualTableMapping {
        &self.virtual_table_mapping
    }

    pub fn all_tables_number_to_name(
        &mut self,
        namespace: TableNamespace,
    ) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        let table_mapping = self.table_mapping().namespace(namespace);
        all_tables_number_to_name(&table_mapping, &self.virtual_system_mapping)
    }

    pub fn persistence_version(&self) -> PersistenceVersion {
        self.persistence_version
    }
}

pub(crate) struct TableUpdate {
    pub namespace: TableNamespace,
    pub table_id_and_number: TabletIdAndTableNumber,
    pub table_name: TableName,
    pub state: TableState,
    pub mode: TableUpdateMode,
}

impl TableUpdate {
    fn activates(&self) -> bool {
        matches!(self.mode, TableUpdateMode::Activate)
            || (matches!(self.mode, TableUpdateMode::Create)
                && matches!(self.state, TableState::Active))
    }
}

pub(crate) enum TableUpdateMode {
    Create,
    Activate,
    Drop,
}

pub(crate) struct Update<'a> {
    metadata: &'a mut TableRegistry,
    table_update: Option<TableUpdate>,
    virtual_table_creation: Option<(TableNamespace, TableNumber, TableName)>,
}

impl<'a> Update<'a> {
    pub(crate) fn apply(mut self) -> Option<TableUpdate> {
        if let Some(ref table_update) = self.table_update {
            if table_update.activates() {
                self.metadata.table_mapping.insert(
                    table_update.table_id_and_number.tablet_id,
                    table_update.namespace,
                    table_update.table_id_and_number.table_number,
                    table_update.table_name.clone(),
                );
            }
            let TableUpdate {
                namespace,
                table_id_and_number,
                table_name,
                state,
                mode,
            } = table_update;
            match mode {
                TableUpdateMode::Activate => {},
                TableUpdateMode::Create => {
                    self.metadata.table_mapping.insert_tablet(
                        table_id_and_number.tablet_id,
                        *namespace,
                        table_id_and_number.table_number,
                        table_name.clone(),
                    );
                },
                TableUpdateMode::Drop => {
                    self.metadata
                        .table_mapping
                        .remove(table_id_and_number.tablet_id);
                },
            }
            self.metadata
                .tablet_states
                .insert(table_id_and_number.tablet_id, *state);
        }
        if let Some((namespace, table_number, table_name)) = self.virtual_table_creation.take() {
            self.metadata
                .virtual_table_mapping
                .insert(namespace, table_number, table_name);
        }
        self.table_update
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::Arc,
    };

    use common::{
        bootstrap_model::{
            index::{
                database_index::IndexedFields,
                TabletIndexMetadata,
                INDEX_TABLE,
            },
            tables::{
                TableMetadata,
                TABLES_TABLE,
            },
        },
        document::{
            CreationTime,
            ResolvedDocument,
        },
        testing::TestIdGenerator,
        types::{
            PersistenceVersion,
            TabletIndexName,
        },
        virtual_system_mapping::{
            NoopDocMapper,
            VirtualSystemMapping,
        },
    };
    use imbl::OrdMap;
    use indexing::index_registry::IndexRegistry;
    use value::{
        TableNamespace,
        VirtualTableMapping,
    };

    use crate::{
        TableRegistry,
        VirtualTableMetadata,
        VIRTUAL_TABLES_TABLE,
    };

    fn make_registries() -> anyhow::Result<(TableRegistry, IndexRegistry, TestIdGenerator)> {
        let mut id_generator = TestIdGenerator::new();
        let index_tablet = id_generator.system_table_id(&INDEX_TABLE).tablet_id;
        let index_by_id = id_generator.system_generate(&INDEX_TABLE);
        id_generator.system_table_id(&TABLES_TABLE);
        id_generator.system_table_id(&VIRTUAL_TABLES_TABLE);
        let by_id_index_metadata = TabletIndexMetadata::new_enabled(
            TabletIndexName::by_id(index_tablet),
            IndexedFields::by_id(),
        );
        let mut virtual_system_mapping = VirtualSystemMapping::default();
        virtual_system_mapping.add_table(
            &"_storage".parse()?,
            &"_file_storage".parse()?,
            BTreeMap::new(),
            Arc::new(NoopDocMapper),
        );
        let table_registry = TableRegistry::bootstrap(
            id_generator.clone(),
            OrdMap::new(),
            PersistenceVersion::V5,
            VirtualTableMapping::new(),
            virtual_system_mapping,
        )?;
        let index_documents = vec![ResolvedDocument::new(
            index_by_id,
            CreationTime::ONE,
            by_id_index_metadata.try_into()?,
        )?];
        let index_registry = IndexRegistry::bootstrap(
            &id_generator,
            index_documents.iter(),
            PersistenceVersion::V5,
        )?;
        Ok((table_registry, index_registry, id_generator))
    }

    #[test]
    fn test_create_system_then_virtual_table_same_number() -> anyhow::Result<()> {
        let (mut table_registry, index_registry, mut id_generator) = make_registries()?;

        let table_number = 530.try_into()?;
        let system_table_id = id_generator.system_generate(&TABLES_TABLE);
        let virtual_table_id = id_generator.system_generate(&VIRTUAL_TABLES_TABLE);

        let table_metadata = TableMetadata::new(
            TableNamespace::test_user(),
            "_file_storage".parse()?,
            table_number,
        );
        table_registry.update(
            &index_registry,
            system_table_id,
            None,
            Some(&table_metadata.try_into()?),
        )?;
        let virtual_table_metadata = VirtualTableMetadata::new(
            TableNamespace::test_user(),
            "_storage".parse()?,
            table_number,
        );
        table_registry.update(
            &index_registry,
            virtual_table_id,
            None,
            Some(&virtual_table_metadata.try_into()?),
        )?;
        Ok(())
    }

    #[test]
    fn test_create_virtual_then_system_table_same_number() -> anyhow::Result<()> {
        let (mut table_registry, index_registry, mut id_generator) = make_registries()?;

        let table_number = 530.try_into()?;
        let system_table_id = id_generator.system_generate(&TABLES_TABLE);
        let virtual_table_id = id_generator.system_generate(&VIRTUAL_TABLES_TABLE);

        let virtual_table_metadata = VirtualTableMetadata::new(
            TableNamespace::test_user(),
            "_storage".parse()?,
            table_number,
        );
        table_registry.update(
            &index_registry,
            virtual_table_id,
            None,
            Some(&virtual_table_metadata.try_into()?),
        )?;
        let table_metadata = TableMetadata::new(
            TableNamespace::test_user(),
            "_file_storage".parse()?,
            table_number,
        );
        table_registry.update(
            &index_registry,
            system_table_id,
            None,
            Some(&table_metadata.try_into()?),
        )?;
        Ok(())
    }
}
