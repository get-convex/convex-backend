use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    DeveloperDocumentId,
    TableName,
};

/// One row per (schema, table) tracking the validation of that table's
/// documents against a validator: the enforced validator while the schema is
/// `Pending` (rows are deleted when the schema resolves), or a staged
/// validator while the schema is `Active`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaValidationMetadata {
    /// The schema whose validator this row tracks.
    pub schema_id: DeveloperDocumentId,
    /// The table being validated.
    pub table_name: TableName,
    /// Staged validator hash. Matching validators carry nonfailed state into a
    /// fresh attempt on activation; failed validators restart.
    pub validator_hash: Option<String>,
    pub state: ValidationState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ValidationState {
    /// Not all documents have been checked yet.
    Pending,
    /// Every document conforms; staged validators remain valid through
    /// write-time checks until a violation fails the attempt.
    Valid,
    /// A document failed the validator.
    Failed { error: String },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSchemaValidationMetadata {
    pub schema_id: String,
    pub table_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator_hash: Option<String>,
    pub state: ValidationState,
}

impl TryFrom<SchemaValidationMetadata> for SerializedSchemaValidationMetadata {
    type Error = anyhow::Error;

    fn try_from(metadata: SchemaValidationMetadata) -> anyhow::Result<Self> {
        Ok(SerializedSchemaValidationMetadata {
            schema_id: metadata.schema_id.to_string(),
            table_name: metadata.table_name.to_string(),
            validator_hash: metadata.validator_hash,
            state: metadata.state,
        })
    }
}

impl TryFrom<SerializedSchemaValidationMetadata> for SchemaValidationMetadata {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedSchemaValidationMetadata) -> anyhow::Result<Self> {
        Ok(SchemaValidationMetadata {
            schema_id: serialized.schema_id.parse()?,
            table_name: serialized.table_name.parse()?,
            validator_hash: serialized.validator_hash,
            state: serialized.state,
        })
    }
}

codegen_convex_serialization!(SchemaValidationMetadata, SerializedSchemaValidationMetadata);
