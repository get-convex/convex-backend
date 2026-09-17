pub mod types;

use std::sync::LazyLock;

use common::document::CREATION_TIME_FIELD_PATH;
use value::{
    FieldPath,
    TableName,
};

use crate::system_tables::{
    SystemIndex,
    SystemTable,
};

pub const SCHEMA_VALIDATIONS_TABLE: TableName = TableName::const_new("_schema_validations");

pub static SCHEMA_VALIDATIONS_BY_SCHEMA_ID_AND_TABLE_NAME: LazyLock<
    SystemIndex<SchemaValidationTable>,
> = LazyLock::new(|| {
    SystemIndex::new(
        "by_schema_id_and_table_name",
        [
            &SCHEMA_ID_FIELD,
            &TABLE_NAME_FIELD,
            &CREATION_TIME_FIELD_PATH,
        ],
    )
    .unwrap()
});

static SCHEMA_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "schemaId".parse().expect("invalid schemaId field"));

static TABLE_NAME_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "tableName".parse().expect("invalid tableName field"));

pub struct SchemaValidationTable;

impl SystemTable for SchemaValidationTable {
    type Metadata = types::SchemaValidationMetadata;

    const TABLE_NAME: TableName = SCHEMA_VALIDATIONS_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![SCHEMA_VALIDATIONS_BY_SCHEMA_ID_AND_TABLE_NAME.clone()]
    }
}
