use std::sync::LazyLock;

use value::TableName;

// Keep this around for a push so we can drop the table - table is unused.
pub static VIRTUAL_TABLES_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_virtual_tables"
        .parse()
        .expect("Invalid built-in virtual_tables table")
});
