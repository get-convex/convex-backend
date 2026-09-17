pub mod legacy;
pub mod types;

use std::sync::LazyLock;

use common::document::CREATION_TIME_FIELD_PATH;
use value::{
    FieldPath,
    TableName,
};

use self::types::SchemaValidationProgressMetadata;
use crate::system_tables::{
    SystemIndex,
    SystemTable,
};

pub const SCHEMA_VALIDATION_PROGRESS_TABLE: TableName =
    TableName::const_new("_schema_validation_progress");
pub static SCHEMA_VALIDATION_PROGRESS_BY_VALIDATION_ID: LazyLock<
    SystemIndex<SchemaValidationProgressTable>,
> = LazyLock::new(|| {
    SystemIndex::new(
        "by_validation_id",
        [&VALIDATION_ID_FIELD, &CREATION_TIME_FIELD_PATH],
    )
    .unwrap()
});
static VALIDATION_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "validationId".parse().expect("invalid validationId field"));

pub static SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID: LazyLock<
    SystemIndex<SchemaValidationProgressTable>,
> = LazyLock::new(|| {
    SystemIndex::new(
        "by_schema_id",
        [&*SCHEMA_ID_FIELD, &CREATION_TIME_FIELD_PATH],
    )
    .unwrap()
});
static SCHEMA_ID_FIELD: LazyLock<FieldPath> = LazyLock::new(|| "schemaId".parse().unwrap());

pub struct SchemaValidationProgressTable;
impl SystemTable for SchemaValidationProgressTable {
    type Metadata = SchemaValidationProgressMetadata;

    const TABLE_NAME: TableName = SCHEMA_VALIDATION_PROGRESS_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![
            SCHEMA_VALIDATION_PROGRESS_BY_VALIDATION_ID.clone(),
            SCHEMA_VALIDATION_PROGRESS_BY_SCHEMA_ID.clone(),
        ]
    }
}
