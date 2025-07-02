use std::sync::LazyLock;

use common::document::CREATION_TIME_FIELD_PATH;
use value::{
    FieldPath,
    TableName,
};

use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static INDEX_BACKFILLS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_index_backfills"
        .parse()
        .expect("Invalid built-in index_backfills table")
});

pub static INDEX_BACKFILLS_BY_INDEX_ID: LazyLock<SystemIndex<IndexBackfillTable>> =
    LazyLock::new(|| {
        SystemIndex::new("by_index_id", [&INDEX_ID_FIELD, &CREATION_TIME_FIELD_PATH]).unwrap()
    });

static INDEX_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "indexId".parse().expect("invalid indexId field"));

pub struct IndexBackfillTable;

impl SystemTable for IndexBackfillTable {
    type Metadata = types::IndexBackfillMetadata;

    fn table_name() -> &'static TableName {
        &INDEX_BACKFILLS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![INDEX_BACKFILLS_BY_INDEX_ID.clone()]
    }
}
