pub mod database_index;
mod developer_index_config;
mod index_config;
mod index_metadata;
pub mod index_validation_error;
pub mod text_index;
pub mod vector_index;

use std::sync::LazyLock;

use value::IdentifierFieldName;

pub use self::{
    developer_index_config::DeveloperIndexConfig,
    index_config::IndexConfig,
    index_metadata::{
        index_metadata_serialize_tablet_id,
        DeveloperIndexMetadata,
        IndexMetadata,
        TabletIndexMetadata,
    },
};
use crate::{
    paths::FieldPath,
    types::TableName,
};

/// Table name for Index data.
pub static INDEX_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_index".parse().expect("Invalid built-in index table"));
/// Field for an indexed table's name in `IndexMetadata`.
pub static TABLE_ID_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "table_id".parse().expect("Invalid built-in field"));
pub static TABLE_ID_FIELD_PATH: LazyLock<FieldPath> =
    LazyLock::new(|| FieldPath::new(vec![TABLE_ID_FIELD_NAME.clone()]).unwrap());

pub const MAX_INDEX_FIELDS_SIZE: usize = 16;
pub const MAX_TEXT_INDEX_FILTER_FIELDS_SIZE: usize = 16;
pub const MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE: usize = 16;
