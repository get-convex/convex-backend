use database::system_tables::{
    SystemIndex,
    SystemTable,
};
use value::TableName;

pub mod types;

use types::PersistedBackendState;

pub static BACKEND_STATE_TABLE: TableName = TableName::const_new("_backend_state");

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
