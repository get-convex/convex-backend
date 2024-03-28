use std::collections::BTreeMap;

use errors::ErrorMetadata;
use imbl::OrdMap;
use serde::Serialize;

use crate::{
    TableId,
    TableIdAndTableNumber,
    TableName,
    TableNumber,
};

// This TableMapping contains the mapping between TableNames and
// TableIdAndTableNumber. This only includes active tables and hidden tables
// (i.e. not deleted tables).
// Use is_active to determine if a table is active.
#[derive(Clone, Debug, PartialEq)]
pub struct TableMapping {
    /// Maps from tablet to number and name exist for all tablets.
    table_id_to_number_and_name: OrdMap<TableId, (TableNumber, TableName)>,

    /// Maps from number and name only exist for active tablets,
    /// because other tablets might have conflicting numbers/names.
    table_name_to_canonical_id: OrdMap<TableName, TableId>,
    table_number_to_canonical_id: OrdMap<TableNumber, TableId>,
}

impl TableMapping {
    pub fn new() -> Self {
        Self {
            table_id_to_number_and_name: Default::default(),
            table_name_to_canonical_id: Default::default(),
            table_number_to_canonical_id: Default::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (TableId, TableNumber, &TableName)> {
        self.table_id_to_number_and_name
            .iter()
            .map(|(table_id, (table_number, table_name))| (*table_id, *table_number, table_name))
    }

    pub fn iter_active_user_tables(
        &self,
    ) -> impl Iterator<Item = (TableId, TableNumber, &TableName)> {
        self.table_id_to_number_and_name
            .iter()
            .filter(|(tablet_id, (_, name))| !name.is_system() && self.is_active(**tablet_id))
            .map(|(table_id, (table_number, table_name))| (*table_id, *table_number, table_name))
    }

    pub fn insert(&mut self, table_id: TableId, table_number: TableNumber, name: TableName) {
        self.insert_tablet(table_id, table_number, name.clone());
        self.table_name_to_canonical_id.insert(name, table_id);
        self.table_number_to_canonical_id
            .insert(table_number, table_id);
    }

    /// A tablet has mappings ID -> number, and ID -> name but not necessarily
    /// the reverse mappings.
    pub fn insert_tablet(&mut self, table_id: TableId, table_number: TableNumber, name: TableName) {
        self.table_id_to_number_and_name
            .insert(table_id, (table_number, name));
    }

    /// Removes tablet for active table.
    pub fn remove(&mut self, id: TableId) {
        let Some((number, name)) = self.table_id_to_number_and_name.remove(&id) else {
            panic!("{id} does not exist");
        };
        if self.table_name_to_canonical_id.get(&name) == Some(&id) {
            self.table_name_to_canonical_id.remove(&name);
        }
        if self.table_number_to_canonical_id.get(&number) == Some(&id) {
            self.table_number_to_canonical_id.remove(&number);
        }
    }

    pub fn id(&self, name: &TableName) -> anyhow::Result<TableIdAndTableNumber> {
        self.id_and_number_if_exists(name)
            .ok_or_else(|| anyhow::anyhow!("cannot find table {name:?}"))
    }

    pub fn id_if_exists(&self, name: &TableName) -> Option<TableId> {
        self.table_name_to_canonical_id.get(name).cloned()
    }

    pub fn id_and_number_if_exists(&self, name: &TableName) -> Option<TableIdAndTableNumber> {
        let table_id = self.id_if_exists(name)?;
        let (number, _) = self.table_id_to_number_and_name.get(&table_id)?;
        Some(TableIdAndTableNumber {
            table_id,
            table_number: *number,
        })
    }

    pub fn id_exists(&self, id: TableId) -> bool {
        self.table_id_to_number_and_name.contains_key(&id)
    }

    pub fn table_id_exists(&self, id: TableId) -> bool {
        self.table_id_to_number_and_name.contains_key(&id)
    }

    pub fn table_number_exists(&self) -> impl Fn(TableNumber) -> bool + '_ {
        |n| self.table_number_to_canonical_id.contains_key(&n)
    }

    pub fn name_by_number_if_exists(&self, number: TableNumber) -> Option<&TableName> {
        self.table_number_to_canonical_id
            .get(&number)
            .and_then(|id| self.table_id_to_number_and_name.get(id))
            .map(|(_, name)| name)
    }

    pub fn name_exists(&self, name: &TableName) -> bool {
        self.table_name_to_canonical_id.contains_key(name)
    }

    pub fn id_to_name(&self) -> impl Fn(TableIdAndTableNumber) -> anyhow::Result<TableName> + '_ {
        |id| self.tablet_name(id.table_id)
    }

    pub fn name_to_id(&self) -> impl Fn(TableName) -> anyhow::Result<TableIdAndTableNumber> + '_ {
        |name| self.id(&name)
    }

    pub fn tablet_name(&self, id: TableId) -> anyhow::Result<TableName> {
        self.table_id_to_number_and_name
            .get(&id)
            .map(|(_, name)| name.clone())
            .ok_or_else(|| anyhow::anyhow!("cannot find table {id:?}"))
    }

    pub fn tablet_to_name(&self) -> impl Fn(TableId) -> anyhow::Result<TableName> + '_ {
        |id| self.tablet_name(id)
    }

    /// When the user inputs a TableName and we don't know whether it exists,
    /// throw a developer error if it doesn't exist.
    pub fn name_to_id_user_input(
        &self,
    ) -> impl Fn(TableName) -> anyhow::Result<TableIdAndTableNumber> + '_ {
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
        for (table_id, (table_number, table_name)) in other.table_id_to_number_and_name.into_iter()
        {
            self.insert(table_id, table_number, table_name);
        }
    }

    /// Assuming all system tables are in the table mapping,
    /// does a table id correspond to a system table?
    pub fn is_system(&self, table_number: TableNumber) -> bool {
        match self.table_number_to_canonical_id.get(&table_number) {
            Some(id) => self.is_system_table_id(*id),
            None => false,
        }
    }

    /// Assuming all system tables are in the table mapping,
    /// does a table id correspond to a system table?
    pub fn is_system_table_id(&self, table_id: TableId) -> bool {
        match self.table_id_to_number_and_name.get(&table_id) {
            Some((_, t)) => t.is_system(),
            None => false,
        }
    }

    pub fn is_active(&self, tablet_id: TableId) -> bool {
        let Some((table_number, _)) = self.table_id_to_number_and_name.get(&tablet_id) else {
            return false;
        };
        let Some(active_tablet_id) = self.table_number_to_canonical_id.get(table_number) else {
            return false;
        };
        tablet_id == *active_tablet_id
    }

    pub fn number_matches_name(&self, table_number: TableNumber, name: &TableName) -> bool {
        match self.table_name_to_canonical_id.get(name) {
            Some(table_id) => match self.table_id_to_number_and_name.get(table_id) {
                Some((number, _)) => *number == table_number,
                None => false,
            },
            None => false,
        }
    }

    pub fn number_to_name(&self) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        |table_number| {
            self.table_number_to_canonical_id
                .get(&table_number)
                .and_then(|id| self.table_id_to_number_and_name.get(id))
                .map(|(_, name)| name.clone())
                .ok_or_else(|| anyhow::anyhow!("cannot find table {table_number:?}"))
        }
    }

    pub fn inject_table_number(
        &self,
    ) -> impl Fn(TableId) -> anyhow::Result<TableIdAndTableNumber> + '_ {
        |table_id| {
            self.table_id_to_number_and_name
                .get(&table_id)
                .ok_or_else(|| anyhow::anyhow!("could not find table id {table_id}"))
                .map(|(table_number, _)| TableIdAndTableNumber {
                    table_number: *table_number,
                    table_id,
                })
        }
    }

    pub fn inject_table_id(
        &self,
    ) -> impl Fn(TableNumber) -> anyhow::Result<TableIdAndTableNumber> + '_ {
        |table_number| {
            self.table_number_to_canonical_id
                .get(&table_number)
                .ok_or_else(|| anyhow::anyhow!("cannot find table with id {table_number}"))
                .map(|table_id| TableIdAndTableNumber {
                    table_number,
                    table_id: *table_id,
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
