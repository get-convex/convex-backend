use std::collections::BTreeMap;

use common::components::ComponentPath;
use events::usage::TableDatabaseStorage;
use itertools::{
    Either,
    Itertools,
};
use value::TableName;

/// Counts the amount of storage used by documents and indexes in a table.
#[derive(Debug)]
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
#[derive(Debug)]
pub struct TablesUsage<T>(pub BTreeMap<T, TableUsage>);

#[derive(Debug, PartialEq)]
pub struct DatabaseStorage {
    pub user_tables: Vec<TableDatabaseStorage>,
    pub system_tables: Vec<TableDatabaseStorage>,
}

impl TablesUsage<(ComponentPath, TableName)> {
    pub fn partition(self) -> DatabaseStorage {
        let (user_tables, system_tables) =
            self.0
                .into_iter()
                .partition_map(|((component_path, name), table_usage)| {
                    let component_path = component_path.serialize();
                    let table_name = (*name).into();
                    if name.is_system() {
                        Either::Right(TableDatabaseStorage {
                            component_path,
                            table_name,
                            total_document_size: table_usage.document_size,
                            total_index_size: table_usage.index_size
                                + table_usage.system_index_size,
                        })
                    } else {
                        // TODO: system indexes aren't reported anywhere for user tables
                        Either::Left(TableDatabaseStorage {
                            component_path,
                            table_name,
                            total_document_size: table_usage.document_size,
                            total_index_size: table_usage.index_size,
                        })
                    }
                });
        DatabaseStorage {
            user_tables,
            system_tables,
        }
    }
}

#[test]
fn test_partition_usage() {
    use std::str::FromStr;

    let usage = TablesUsage(
        [
            (
                (
                    ComponentPath::root(),
                    TableName::from_str("user_table").unwrap(),
                ),
                TableUsage {
                    document_size: 1,
                    index_size: 2,
                    system_index_size: 3,
                },
            ),
            (
                (
                    ComponentPath::root(),
                    TableName::from_str("_system_table").unwrap(),
                ),
                TableUsage {
                    document_size: 4,
                    index_size: 5,
                    system_index_size: 6,
                },
            ),
            (
                (
                    ComponentPath::test_user(),
                    TableName::from_str("component_user_table").unwrap(),
                ),
                TableUsage {
                    document_size: 7,
                    index_size: 8,
                    system_index_size: 9,
                },
            ),
            (
                (
                    ComponentPath::test_user(),
                    TableName::from_str("_component_system_table").unwrap(),
                ),
                TableUsage {
                    document_size: 10,
                    index_size: 11,
                    system_index_size: 12,
                },
            ),
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(
        usage.partition(),
        DatabaseStorage {
            user_tables: vec![
                TableDatabaseStorage {
                    component_path: None,
                    table_name: "component_user_table".to_owned(),
                    total_document_size: 7,
                    // system index size not counted
                    total_index_size: 8,
                },
                TableDatabaseStorage {
                    component_path: None,
                    table_name: "user_table".to_owned(),
                    total_document_size: 1,
                    total_index_size: 2,
                },
            ],
            system_tables: vec![
                TableDatabaseStorage {
                    component_path: None,
                    table_name: "_component_system_table".to_owned(),
                    total_document_size: 10,
                    // system index size is counted
                    total_index_size: 11 + 12,
                },
                TableDatabaseStorage {
                    component_path: None,
                    table_name: "_system_table".to_owned(),
                    total_document_size: 4,
                    total_index_size: 5 + 6,
                },
            ],
        }
    );
}
