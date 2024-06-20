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

    virtual_table_mapping: VirtualTableMapping,
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
    ) -> anyhow::Result<Self> {
        let _timer = bootstrap_table_registry_timer();
        Ok(Self {
            table_mapping,
            tablet_states: table_states,
            persistence_version,
            virtual_table_mapping,
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
                        self.validate_table_number(metadata.number)?;
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
                    if self.virtual_table_mapping.name_exists(&metadata.name) {
                        anyhow::bail!("Tried to create duplicate virtual table {new_value}");
                    }
                    self.validate_table_number(metadata.number)?;
                    virtual_table_creation = Some((metadata.number, metadata.name));
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

    fn validate_table_number(&self, table_number: TableNumber) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self
                .table_mapping
                .iter()
                .any(|(_, _, number, _)| number == table_number),
            "Cannot add a table with table number {table_number} since it already exists in the \
             table mapping"
        );
        anyhow::ensure!(
            !self.virtual_table_mapping.number_exists(table_number),
            "Cannot add a table with table number {table_number} since it already exists in the \
             virtual table mapping"
        );
        Ok(())
    }

    pub fn table_state(&self, tablet_id: TabletId) -> Option<TableState> {
        self.tablet_states.get(&tablet_id).cloned()
    }

    pub fn user_table_names(&self) -> impl Iterator<Item = &TableName> {
        self.table_mapping
            .iter_active_user_tables()
            .map(|(_, _, _, name)| name)
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

    pub fn virtual_table_mapping(&self) -> &VirtualTableMapping {
        &self.virtual_table_mapping
    }

    pub fn all_tables_number_to_name(
        &mut self,
    ) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        let table_mapping = self.table_mapping().clone();
        let virtual_table_mapping = self.virtual_table_mapping().clone();
        move |number| {
            if let Some(table_number) = virtual_table_mapping.name_if_exists(number) {
                return Ok(table_number);
            }
            table_mapping
                .namespace(TableNamespace::by_component_TODO())
                .number_to_name()(number)
        }
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
    virtual_table_creation: Option<(TableNumber, TableName)>,
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
        if let Some((table_number, table_name)) = self.virtual_table_creation.take() {
            self.metadata
                .virtual_table_mapping
                .insert(table_number, table_name);
        }
        self.table_update
    }
}
