use std::sync::LazyLock;

use database::system_tables::{
    SystemIndex,
    SystemTable,
};
use value::TableName;

pub mod types;

use types::PersistedBackendState;

pub static BACKEND_STATE_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_backend_state"
        .parse()
        .expect("Invalid built-in backend_state table")
});

pub struct BackendStateTable;
impl SystemTable for BackendStateTable {
    type Metadata = PersistedBackendState;

    const FOR_MIGRATION: bool = true;

    fn table_name() -> &'static TableName {
        &BACKEND_STATE_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}
