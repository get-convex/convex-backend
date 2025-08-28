pub mod types;

use std::sync::LazyLock;

use common::document::CREATION_TIME_FIELD_PATH;
use database::system_tables::{
    SystemIndex,
    SystemTable,
};
use value::{
    FieldPath,
    TableName,
};

pub static SCHEMA_VALIDATION_PROGRESS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_schema_validation_progress"
        .parse()
        .expect("Invalid built-in _schema_validation_progress table")
});

pub static SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID: LazyLock<
    SystemIndex<SchemaValidationProgressTable>,
> = LazyLock::new(|| {
    SystemIndex::new(
        "by_schema_id",
        [&SCHEMA_ID_FIELD, &CREATION_TIME_FIELD_PATH],
    )
    .unwrap()
});

static SCHEMA_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "schemaId".parse().expect("invalid schemaId field"));

pub struct SchemaValidationProgressTable;

impl SystemTable for SchemaValidationProgressTable {
    type Metadata = types::SchemaValidationProgressMetadata;

    fn table_name() -> &'static TableName {
        &SCHEMA_VALIDATION_PROGRESS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID.clone()]
    }
}
