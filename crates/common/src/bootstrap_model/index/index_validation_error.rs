use errors::ErrorMetadata;
use value::{
    TableIdentifier,
    TableName,
};

use crate::{
    paths::FieldPath,
    schemas::IndexSchema,
    types::IndexDescriptor,
};

pub fn empty_index(table_name: &TableName, index: &IndexSchema) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "EmptyIndex",
        format!("In table \"{table_name}\" index \"{index}\" must have at least one field."),
    )
}
pub fn fields_not_unique_within_index(field: &FieldPath) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "FieldsNotUniqueWithinIndex",
        format!("Duplicate field {field}. Index fields must be unique within an index."),
    )
}
pub fn index_not_unique(
    table_name: &TableName,
    index1: &IndexDescriptor,
    index2: &IndexDescriptor,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexNotUnique",
        format!(
            "In table \"{table_name}\" index \"{index1}\" and index \"{index2}\" have the same \
             fields. Indexes must be unique within a table."
        ),
    )
}
// IndexFieldsContainId is a more specific version of
// IndexFieldNameReserved. It provides a more actionable error
// message.
pub fn fields_contain_id() -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexFieldsContainId",
        "`_id` is not a valid index field. To load documents by ID, use `db.get(id)`.",
    )
}
// IndexFieldsContainCreationTime is a more specific version of
// IndexFieldNameReserved. It provides a more actionable error message.
pub fn fields_contain_creation_time() -> ErrorMetadata {
    ErrorMetadata::bad_request("IndexFieldsContainCreationTime",
                               "`_creationTime` is automatically added to the end of each index. It should not \
                                be added explicitly in the index definition. See https://docs.convex.dev/using/indexes \
                                for more details."
    )
}
pub fn field_name_reserved() -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexFieldNameReserved",
        "Reserved fields (starting with `_`) are not allowed in indexes.",
    )
}
pub fn search_field_not_unique(
    table_name: &TableName,
    index1: &IndexDescriptor,
    index2: &IndexDescriptor,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "SearchIndexFieldNotUnique",
        format!(
            "In table \"{table_name}\" search index \"{index1}\" and search index \"{index2}\" \
             have the same `searchField`. Search index fields must be unique within a table. You \
             should combine the
             indexes with the same `searchField` into one index containing all `filterField`s and \
             then use different subsets of the `filterField`s at query time."
        ),
    )
}
pub fn vector_field_not_unique(
    table_name: &TableName,
    index1: &IndexDescriptor,
    index2: &IndexDescriptor,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "VectorIndexFieldNotUnique",
        format!(
            "In table \"{table_name}\" vector index \"{index1}\" and vector index \"{index2}\" \
             have the same `vectorField`. Vector index fields must be unique within a table. You \
             should combine the
             indexes with the same `vectorField` into one index containing all `filterField`s and \
             then use different subsets of the `filterField`s at query time."
        ),
    )
}
pub fn name_reserved<T: TableIdentifier>(table_name: &T, name: &IndexDescriptor) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexNameReserved",
        format!(
            "In table \"{table_name}\" cannot name an index \"{name}\" because the name is \
             reserved. Indexes may not start with an underscore or be named \"by_id\" or \
             \"by_creation_time\"."
        ),
    )
}
pub fn names_not_unique(table_name: &TableName, index: &IndexDescriptor) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexNamesNotUnique",
        format!("Table \"{table_name}\" has two or more definitions of index \"{index}\"."),
    )
}
pub fn invalid_index_name(descriptor: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidIndexName",
        format!(
            "Invalid index name: \"{descriptor}\". Identifiers must be 64 characters or less, \
             start with a letter, and only contain letters, digits, underscores."
        ),
    )
}
pub fn invalid_index_field(descriptor: &IndexDescriptor, field: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidIndexField",
        format!("In index \"{descriptor}\": Invalid index field: \"{field}\""),
    )
}

// TODO - move elsewhere (near table names) - it's not indexing related
pub fn invalid_table_name(table_name: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidTableName",
        format!(
            "Invalid table name: \"{table_name}\". Identifiers must start with a letter and can \
             only contain letters, digits, and underscores."
        ),
    )
}
pub fn not_enough_name_components(index_name: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexNotEnoughNameComponents",
        format!("Insufficient components in index name {index_name}"),
    )
}
pub fn too_many_fields(num_fields: usize) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexTooManyFields",
        format!("Indexes may have up to {num_fields} fields."),
    )
}
pub fn too_many_filter_fields(num_fields: usize) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexTooManyFilterFields",
        format!("Search indexes may have up to {num_fields} filter fields."),
    )
}
pub fn too_many_indexes(table_name: &TableName, num_indexes: usize) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "TooManyIndexes",
        format!("Table \"{table_name}\" cannot have more than {num_indexes} indexes."),
    )
}

pub fn too_many_name_components(index_name: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "IndexTooManyNameComponents",
        format!("Too many components in index name {index_name}"),
    )
}

// TODO move elsewhere (near table names) - it's not indexing related
pub fn table_name_reserved(table_name: &TableName) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "TableNameReserved",
        format!("{table_name} is a reserved table name."),
    )
}

// TODO move elsewhere - it's not indexing related
pub fn too_many_tables(num_tables: usize) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "TooManyTables",
        format!("Number of tables cannot exceed {num_tables}."),
    )
}
