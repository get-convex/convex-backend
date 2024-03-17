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
    pub schema: DatabaseSchema,
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
            schema: serde_json::to_string(&JsonValue::try_from(s.schema)?)?,
        })
    }
}

impl TryFrom<SerializedSchemaMetadata> for SchemaMetadata {
    type Error = anyhow::Error;

    fn try_from(s: SerializedSchemaMetadata) -> anyhow::Result<Self> {
        let deserialized_value: JsonValue = serde_json::from_str(&s.schema)?;
        Ok(Self {
            state: s.state.try_into()?,
            schema: DatabaseSchema::try_from(deserialized_value)?,
        })
    }
}

codegen_convex_serialization!(SchemaMetadata, SerializedSchemaMetadata);
