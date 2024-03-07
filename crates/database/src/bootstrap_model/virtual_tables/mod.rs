use std::sync::LazyLock;

use common::document::{
    ParsedDocument,
    ResolvedDocument,
};
use value::TableName;

use crate::{
    defaults::{
        SystemIndex,
        SystemTable,
    },
    VirtualTableMetadata,
};

pub mod types;

pub static VIRTUAL_TABLES_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_virtual_tables"
        .parse()
        .expect("Invalid built-in virtual_tables table")
});

pub struct VirtualTablesTable;
impl SystemTable for VirtualTablesTable {
    fn table_name(&self) -> &'static TableName {
        &VIRTUAL_TABLES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<VirtualTableMetadata>::try_from(document).map(|_| ())
    }
}
