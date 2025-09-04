use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    DeveloperDocumentId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
/// Schema validation progress is written by the SchemaWorker for `Pending` and
/// `Validated` schemas. `Active`, `Overwritten`, and `Failed` schemas should
/// not have `SchemaValidationProgressMetadata` documents.
pub struct SchemaValidationProgressMetadata {
    /// The ID of the schema being validated. Should correspond to a document in
    /// the _schemas table in `Pending` state.
    pub schema_id: DeveloperDocumentId,
    /// The number of documents that have been validated so far.
    pub num_docs_validated: u64,
    /// The number of total documents that need to be validated. Note this is
    /// approximate because there could be changes since the time we wrote this
    /// value from the table summary when the schema is submitted as pending.
    /// It's possible for num_docs_validated to exceed total_docs.
    /// This field is None if there is no table summary available.
    pub total_docs: Option<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSchemaValidationProgressMetadata {
    pub schema_id: String,
    pub num_docs_validated: i64,
    pub total_docs: Option<i64>,
}

impl From<SchemaValidationProgressMetadata> for SerializedSchemaValidationProgressMetadata {
    fn from(metadata: SchemaValidationProgressMetadata) -> Self {
        SerializedSchemaValidationProgressMetadata {
            schema_id: metadata.schema_id.to_string(),
            num_docs_validated: metadata.num_docs_validated as i64,
            total_docs: metadata.total_docs.map(|x| x as i64),
        }
    }
}

impl TryFrom<SerializedSchemaValidationProgressMetadata> for SchemaValidationProgressMetadata {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedSchemaValidationProgressMetadata) -> anyhow::Result<Self> {
        Ok(SchemaValidationProgressMetadata {
            schema_id: serialized.schema_id.parse()?,
            num_docs_validated: serialized.num_docs_validated as u64,
            total_docs: serialized.total_docs.map(|x| x as u64),
        })
    }
}

codegen_convex_serialization!(
    SchemaValidationProgressMetadata,
    SerializedSchemaValidationProgressMetadata
);
