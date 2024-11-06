use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::codegen_convex_serialization;

use super::schema_state::{
    SchemaState,
    SerializedSchemaState,
};
use crate::schemas::DatabaseSchema;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SchemaMetadata {
    pub state: SchemaState,
    pub raw_schema: String,
}

impl SchemaMetadata {
    pub fn database_schema(&self) -> anyhow::Result<DatabaseSchema> {
        let deserialized_value: JsonValue = serde_json::from_str(&self.raw_schema)?;
        DatabaseSchema::try_from(deserialized_value)
    }

    pub fn new(state: SchemaState, schema: DatabaseSchema) -> anyhow::Result<Self> {
        let json_schema: JsonValue = schema.try_into()?;
        let raw_schema = serde_json::to_string(&json_schema)?;
        Ok(Self { state, raw_schema })
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializedSchemaMetadata {
    state: SerializedSchemaState,
    schema: String,
}

impl TryFrom<SchemaMetadata> for SerializedSchemaMetadata {
    type Error = anyhow::Error;

    fn try_from(s: SchemaMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            state: s.state.try_into()?,
            schema: s.raw_schema,
        })
    }
}

impl TryFrom<SerializedSchemaMetadata> for SchemaMetadata {
    type Error = anyhow::Error;

    fn try_from(s: SerializedSchemaMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            state: s.state.try_into()?,
            raw_schema: s.schema,
        })
    }
}

codegen_convex_serialization!(SchemaMetadata, SerializedSchemaMetadata);
