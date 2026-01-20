use std::collections::BTreeMap;

use common::components::ComponentPath;
use derive_more::{
    Add,
    AddAssign,
};
use events::usage::{
    TableDatabaseStorage,
    UsageEvent,
};
use value::{
    TableName,
    TableNamespace,
};

/// Counts the amount of storage used by documents and indexes in a table.
#[derive(Debug, Copy, Clone, Eq, PartialEq, AddAssign, Add)]
pub struct TableUsage {
    /// Bytes used by documents in this table
    pub document_size: u64,
    /// Bytes used by user-owned indexes on this table
    pub index_size: u64,
    /// Bytes used by system-owned indexes on this table,
    /// including `by_id` and `creation_time`.
    /// For system tables, this is all indexes.
    pub system_index_size: u64,
}

/// `TableUsage` for all tables in a database. `T` is the fully qualified name
/// of a table.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TablesUsage {
    pub user_tables: BTreeMap<(TableNamespace, TableName), (TableUsage, ComponentPath)>,
    pub system_tables: BTreeMap<(TableNamespace, TableName), (TableUsage, ComponentPath)>,
    pub virtual_tables: BTreeMap<(TableNamespace, TableName), (TableUsage, ComponentPath)>,
    pub orphaned_tables: BTreeMap<(TableNamespace, TableName), TableUsage>,
}

impl From<TablesUsage> for UsageEvent {
    fn from(
        TablesUsage {
            user_tables,
            system_tables,
            orphaned_tables: _, // orphaned tables don't count for usage
            virtual_tables,
        }: TablesUsage,
    ) -> UsageEvent {
        let mapper = |((_namespace, name), (table_usage, component_path)): (
            (TableNamespace, TableName),
            (TableUsage, ComponentPath),
        )| {
            let component_path = component_path.serialize();
            let table_name = (*name).into();
            TableDatabaseStorage {
                component_path,
                table_name,
                total_document_size: table_usage.document_size,
                total_index_size: table_usage.index_size,
                total_system_index_size: table_usage.system_index_size,
            }
        };
        let user_tables = user_tables.into_iter().map(mapper).collect();
        let system_tables = system_tables.into_iter().map(mapper).collect();
        let virtual_tables = virtual_tables.into_iter().map(mapper).collect();

        UsageEvent::CurrentDatabaseStorage {
            user_tables,
            system_tables,
            virtual_tables,
        }
    }
}
