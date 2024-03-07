use errors::ErrorMetadata;
use imbl::OrdMap;

use crate::{
    TableName,
    TableNumber,
};

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualTableMapping {
    table_name_to_table_number: OrdMap<TableName, TableNumber>,
    table_number_to_table_name: OrdMap<TableNumber, TableName>,
}

impl VirtualTableMapping {
    pub fn new() -> Self {
        Self {
            table_name_to_table_number: Default::default(),
            table_number_to_table_name: Default::default(),
        }
    }

    pub fn insert(&mut self, table_number: TableNumber, table_name: TableName) {
        self.table_name_to_table_number
            .insert(table_name.clone(), table_number);
        self.table_number_to_table_name
            .insert(table_number, table_name);
    }

    pub fn name_exists(&self, name: &TableName) -> bool {
        self.table_name_to_table_number.contains_key(name)
    }

    pub fn number_exists(&self, number: &TableNumber) -> bool {
        self.table_number_to_table_name.contains_key(number)
    }

    pub fn table_number_exists(&self) -> impl Fn(TableNumber) -> bool + '_ {
        |n| self.number_exists(&n)
    }

    pub fn name(&self, number: TableNumber) -> anyhow::Result<TableName> {
        self.name_if_exists(number)
            .ok_or_else(|| anyhow::anyhow!("cannot find table name for table number {number:?}"))
    }

    pub fn number(&self, name: &TableName) -> anyhow::Result<TableNumber> {
        self.number_if_exists(name)
            .ok_or_else(|| anyhow::anyhow!("cannot find table number for table name {name:?}"))
    }

    pub fn number_if_exists(&self, name: &TableName) -> Option<TableNumber> {
        self.table_name_to_table_number.get(name).cloned()
    }

    pub fn name_if_exists(&self, number: TableNumber) -> Option<TableName> {
        self.table_number_to_table_name.get(&number).cloned()
    }

    /// When the user inputs a TableName and we don't know whether it exists,
    /// throw a developer error if it doesn't exist.
    pub fn name_to_number_user_input(
        &self,
    ) -> impl Fn(TableName) -> anyhow::Result<TableNumber> + '_ {
        |name| {
            let Some(table_number) = self.table_name_to_table_number.get(&name) else {
                anyhow::bail!(table_does_not_exist(&name));
            };
            Ok(*table_number)
        }
    }

    pub fn number_to_name(&self) -> impl Fn(TableNumber) -> anyhow::Result<TableName> + '_ {
        |number| {
            let Some(table_name) = self.table_number_to_table_name.get(&number) else {
                anyhow::bail!("Could not find table name for table number {number:?}");
            };
            Ok(table_name.clone())
        }
    }
}

fn table_does_not_exist(table: &TableName) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "SystemTableDoesNotExist",
        format!("System table '{table}' not found"),
    )
}
