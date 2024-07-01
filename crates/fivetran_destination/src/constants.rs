use std::sync::LazyLock;

use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document::CREATION_TIME_FIELD_PATH,
    types::IndexDescriptor,
    value::{
        FieldPath,
        IdentifierFieldName,
    },
};

use crate::api_types::FivetranFieldName;

/// The name of the field used in Convex tables to store the Fivetran metadata.
pub static METADATA_CONVEX_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "fivetran".parse().unwrap());

pub static SYNCED_FIVETRAN_FIELD_NAME: LazyLock<FivetranFieldName> =
    LazyLock::new(|| "_fivetran_synced".parse().unwrap());
pub static SOFT_DELETE_FIVETRAN_FIELD_NAME: LazyLock<FivetranFieldName> =
    LazyLock::new(|| "_fivetran_deleted".parse().unwrap());
pub static ID_FIVETRAN_FIELD_NAME: LazyLock<FivetranFieldName> =
    LazyLock::new(|| "_fivetran_id".parse().unwrap());

pub static SYNCED_CONVEX_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "synced".parse().unwrap());
pub static SOFT_DELETE_CONVEX_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "deleted".parse().unwrap());
pub static ID_CONVEX_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "id".parse().unwrap());
pub static UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "columns".parse().unwrap());

pub static PRIMARY_KEY_INDEX_DESCRIPTOR: LazyLock<IndexDescriptor> =
    LazyLock::new(|| "by_primary_key".parse().unwrap());

pub static SOFT_DELETE_FIELD_PATH: LazyLock<FieldPath> = LazyLock::new(|| {
    FieldPath::new(vec![
        METADATA_CONVEX_FIELD_NAME.clone(),
        SOFT_DELETE_CONVEX_FIELD_NAME.clone(),
    ])
    .expect("Invalid field path")
});

pub static SYNCED_FIELD_PATH: LazyLock<FieldPath> = LazyLock::new(|| {
    FieldPath::new(vec![
        METADATA_CONVEX_FIELD_NAME.clone(),
        SYNCED_CONVEX_FIELD_NAME.clone(),
    ])
    .expect("Invalid field path")
});

pub static ID_FIELD_PATH: LazyLock<FieldPath> = LazyLock::new(|| {
    FieldPath::new(vec![
        METADATA_CONVEX_FIELD_NAME.clone(),
        ID_CONVEX_FIELD_NAME.clone(),
    ])
    .expect("Invalid field path")
});

pub static FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS: LazyLock<IndexedFields> =
    LazyLock::new(|| {
        IndexedFields::try_from(vec![
            SYNCED_FIELD_PATH.clone(),
            CREATION_TIME_FIELD_PATH.clone(),
        ])
        .expect("Invalid IndexedFields")
    });

pub static FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS: LazyLock<IndexedFields> =
    LazyLock::new(|| {
        IndexedFields::try_from(vec![
            SOFT_DELETE_FIELD_PATH.clone(),
            SYNCED_FIELD_PATH.clone(),
            CREATION_TIME_FIELD_PATH.clone(),
        ])
        .expect("Invalid IndexedFields")
    });
