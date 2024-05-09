use std::collections::BTreeMap;

use errors::ErrorMetadata;
use imbl::OrdMap;
use serde::Serialize;

use crate::{
    TableName,
    TableNumber,
    TabletId,
    TabletIdAndTableNumber,
};

// This TableMapping contains the mapping between TableNames and
// TabletIdAndTableNumber. This only includes active tables and hidden tables
// (i.e. not deleted tables).
// Use is_active to determine if a table is active.
#[derive(Clone, Debug, PartialEq)]
pub struct TableMapping {
    /// Maps from tablet to number and name exist for all tablets.
    tablet_to_table: OrdMap<TabletId, (TableNumber, TableName)>,

    /// Maps from number and name only exist for active tablets,
    /// because other tablets might have conflicting numbers/names.
    table_name_to_canonical_tablet: OrdMap<TableName, TabletId>,
    table_number_to_canonical_tablet: OrdMap<TableNumber, TabletId>,
}

impl TableMapping {
    pub fn new() -> Self {
        Self {
            tablet_to_table: Default::default(),
            table_name_to_canonical_tablet: Default::default(),
            table_number_to_canonical_tablet: Default::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (TabletId, TableNumber, &TableName)> {
        self.tablet_to_table
            .iter()
            .map(|(table_id, (table_number, table_name))| (*table_id, *table_number, table_name))
    }

    pub fn iter_active_user_tables(
        &self,
    ) -> impl Iterator<Item = (TabletId, TableNumber, &TableName)> {
        self.tablet_to_table
            .iter()
            .filter(|(tablet_id, (_, name))| !name.is_system() && self.is_active(**tablet_id))
            .map(|(table_id, (table_number, table_name))| (*table_id, *table_number, table_name))
    }

    pub fn insert(&mut self, tablet_id: TabletId, table_number: TableNumber, name: TableName) {
        self.insert_tablet(tablet_id, table_number, name.clone());
        self.table_name_to_canonical_tablet.insert(name, tablet_id);
        self.table_number_to_canonical_tablet
            .insert(table_number, tablet_id);
    }

    /// A tablet has mappings ID -> number, and ID -> name but not necessarily
    /// the reverse mappings.
    pub fn insert_tablet(
        &mut self,
        tablet_id: TabletId,
        table_number: TableNumber,
        name: TableName,
    ) {
        self.tablet_to_table.insert(tablet_id, (table_number, name));
    }

    /// Removes tablet for active table.
    pub fn remove(&mut self, tablet_id: TabletId) {
        let Some((number, name)) = self.tablet_to_table.remove(&tablet_id) else {
            panic!("{tablet_id} does not exist");
        };
        if self.table_name_to_canonical_tablet.get(&name) == Some(&tablet_id) {
            self.table_name_to_canonical_tablet.remove(&name);
        }
        if self.table_number_to_canonical_tablet.get(&number) == Some(&tablet_id) {
            self.table_number_to_canonical_tablet.remove(&number);
        }
    }

    pub fn id(&self, name: &TableName) -> anyhow::Result<TabletIdAndTableNumber> {
        self.id_and_number_if_exists(name)
            .ok_or_else(|| anyhow::anyhow!("cannot find table {name:?}"))
    }

    pub fn id_if_exists(&self, name: &TableName) -> Option<TabletId> {
        self.table_name_to_canonical_tablet.get(name).cloned()
    }

    pub fn id_and_number_if_exists(&self, name: &TableName) -> Option<TabletIdAndTableNumber> {
        let table_id = self.id_if_exists(name)?;
        let (number, _) = self.tablet_to_table.get(&table_id)?;
        Some(TabletIdAndTableNumber {
            tablet_id: table_id,
            table_number: *number,
        })
    }

    pub fn id_exists(&self, id: TabletId) -> bool {
        self.tablet_to_table.contains_key(&id)
    }

    pub fn tablet_id_exists(&self, id: TabletId) -> bool {
        self.tablet_to_table.contains_key(&id)
    }

    pub fn table_number_exists(&self) -> impl Fn(TableNumber) -> bool + '_ {
        |n| self.table_number_to_canonical_tablet.contains_key(&n)
    }

    pub fn name_by_number_if_exists(&self, number: TableNumber) -> Option<&TableName> {
        self.table_number_to_canonical_tablet
            .get(&number)
            .and_then(|id| self.tablet_to_table.get(id))
            .map(|(_, name)| name)
    }

    pub fn name_exists(&self, name: &TableName) -> bool {
        self.table_name_to_canonical_tablet.contains_key(name)
    }

    pub fn id_to_name(&self) -> impl Fn(TabletIdAndTableNumber) -> anyhow::Result<TableName> + '_ {
        |id| self.tablet_name(id.tablet_id)
    }

    pub fn name_to_id(&self) -> impl Fn(TableName) -> anyhow::Result<TabletIdAndTableNumber> + '_ {
        |name| self.id(&name)
    }

    pub fn tablet_name(&self, id: TabletId) -> anyhow::Result<TableName> {
        self.tablet_to_table
            .get(&id)
            .map(|(_, name)| name.clone())
            .ok_or_else(|| anyhow::anyhow!("cannot find table {id:?}"))
    }

    pub fn tablet_to_name(&self) -> impl Fn(TabletId) -> anyhow::Result<TableName> + '_ {
        |id| self.tablet_name(id)
    }

    /// When the user inputs a TableName and we don't know whether it exists,
    /// throw a developer error if it doesn't exist.
    pub fn name_to_id_user_input(
        &self,
    ) -> impl Fn(TableName) -> anyhow::Result<TabletIdAndTableNumber> + '_ {
        |name| {
            let Some(id) = self.id_and_number_if_exists(&name) else {
                anyhow::bail!(table_does_not_exist(&name));
            };
            Ok(id)
        }
    }

    /// When the user inputs a TableName and we don't know whether it exists,
    /// throw a developer error if it doesn't exist.
    pub fn name_to_number_user_input(
        &self,
    ) -> impl Fn(TableName) -> anyhow::Result<TableNumber> + '_ {
        |name| {
            let table_id = self.name_to_id_user_input()(name)?;
            Ok(table_id.table_number)
        }
    }

    pub fn update(&mut self, other: TableMapping) {
        for (table_id, (table_number, table_name)) in other.tablet_to_table.into_iter() {
            self.insert(table_id, table_number, table_name);
        }
    }

    /// Assuming all system tables are in the table mapping,
    /// does a table id correspond to a system table?
    pub fn is_system(&self, table_number: TableNumber) -> bool {
        match self.table_number_to_canonical_tablet.get(&table_number) {
            Some(id) => self.is_system_table_id(*id),
            None => false,
        }
    }

    /// Assuming all system tables are in the table mapping,
    /// does a table id correspond to a system table?
    pub fn is_system_table_id(&self, table_id: TabletId) -> bool {
        match self.tablet_to_table.get(&table_id) {
            Some((_, t)) => t.is_system(),
            None => false,
        }
    }

    pub fn is_active(&self, tablet_id: TabletId) -> bool {
        let Some((table_number, _)) = self.tablet_to_table.get(&tablet_id) else {
            return false;
        };
        let Some(active_tablet_id) = self.table_number_to_canonical_tablet.get(table_number) else {
            return false;
        };
        tablet_id == *active_tablet_id
    }

    pub fn number_matches_name(&self, table_number: TableNumber, name: &TableName) -> bool {
        match self.table_name_to_canonical_tablet.get(name) {
            Some(table_id) => match self.tablet_to_table.get(table_id) {
                Some((number, _)) => *number == table_number,
                None => false,
            },
            None => false,
        }
    }

    pub fn number_to_name(&self) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        |table_number| {
            self.table_number_to_canonical_tablet
                .get(&table_number)
                .and_then(|id| self.tablet_to_table.get(id))
                .map(|(_, name)| name.clone())
                .ok_or_else(|| anyhow::anyhow!("cannot find table {table_number:?}"))
        }
    }

    pub fn inject_table_number(
        &self,
    ) -> impl Fn(TabletId) -> anyhow::Result<TabletIdAndTableNumber> + '_ {
        |table_id| {
            self.tablet_to_table
                .get(&table_id)
                .ok_or_else(|| anyhow::anyhow!("could not find table id {table_id}"))
                .map(|(table_number, _)| TabletIdAndTableNumber {
                    table_number: *table_number,
                    tablet_id: table_id,
                })
        }
    }

    pub fn inject_table_id(
        &self,
    ) -> impl Fn(TableNumber) -> anyhow::Result<TabletIdAndTableNumber> + '_ {
        |table_number| {
            self.table_number_to_canonical_tablet
                .get(&table_number)
                .ok_or_else(|| anyhow::anyhow!("cannot find table with id {table_number}"))
                .map(|table_id| TabletIdAndTableNumber {
                    table_number,
                    tablet_id: *table_id,
                })
        }
    }
}

fn table_does_not_exist(table: &TableName) -> ErrorMetadata {
    ErrorMetadata::bad_request("TableDoesNotExist", format!("Table '{table}' not found"))
}

/// The table mapping that is sent to the dashboard through the
/// `getTableMapping` operation. It omits system tables.
#[derive(Serialize)]
pub struct TableMappingValue(BTreeMap<TableNumber, TableName>);

impl From<TableMapping> for TableMappingValue {
    fn from(table_mapping: TableMapping) -> Self {
        TableMappingValue(
            table_mapping
                .iter()
                .filter(|(_, _, name)| !name.is_system())
                .map(|(_, number, name)| (number, name.clone()))
                .collect(),
        )
    }
}
