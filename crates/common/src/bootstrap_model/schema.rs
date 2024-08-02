use std::str::FromStr;

use errors::ErrorMetadata;
use value::{
    id_v6::DeveloperDocumentId,
    InternalDocumentId,
    ResolvedDocumentId,
    TableMapping,
    TableNamespace,
};

pub use super::{
    schema_metadata::SchemaMetadata,
    schema_state::SchemaState,
};

pub fn parse_schema_id(
    schema_id: &str,
    table_mapping: &TableMapping,
    namespace: TableNamespace,
) -> anyhow::Result<ResolvedDocumentId> {
    // Try parsing as a document ID with TableId first
    match InternalDocumentId::from_str(schema_id) {
        Ok(s) => Ok(s.to_resolved(table_mapping.tablet_number(s.table())?)),
        Err(_) => {
            // Try parsing as an IDv6 ID
            let id = DeveloperDocumentId::decode(schema_id)?;
            id.to_resolved(&table_mapping.namespace(namespace).number_to_tablet())
        },
    }
}

pub fn invalid_schema_id(schema_id: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidSchemaId",
        format!("Invalid schema id: {}", schema_id),
    )
}
