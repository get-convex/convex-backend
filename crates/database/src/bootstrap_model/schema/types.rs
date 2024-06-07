use common::schemas::DatabaseSchema;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDiff {
    pub previous_schema: Option<DatabaseSchema>,
    pub next_schema: Option<DatabaseSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSchemaDiff {
    // NOTE: not camel-case.
    pub previous_schema: Option<String>,
    pub next_schema: Option<String>,
}

impl TryFrom<SchemaDiff> for SerializedSchemaDiff {
    type Error = anyhow::Error;

    fn try_from(diff: SchemaDiff) -> Result<Self, Self::Error> {
        Ok(Self {
            previous_schema: diff
                .previous_schema
                .map(|schema| anyhow::Ok(serde_json::to_string(&JsonValue::try_from(schema)?)?))
                .transpose()?,
            next_schema: diff
                .next_schema
                .map(|schema| anyhow::Ok(serde_json::to_string(&JsonValue::try_from(schema)?)?))
                .transpose()?,
        })
    }
}

impl TryFrom<SerializedSchemaDiff> for SchemaDiff {
    type Error = anyhow::Error;

    fn try_from(diff: SerializedSchemaDiff) -> Result<Self, Self::Error> {
        Ok(Self {
            previous_schema: diff
                .previous_schema
                .map(|schema| {
                    let json_value: JsonValue = serde_json::from_str(&schema)?;
                    DatabaseSchema::try_from(json_value)
                })
                .transpose()?,
            next_schema: diff
                .next_schema
                .map(|schema| {
                    let json_value: JsonValue = serde_json::from_str(&schema)?;
                    DatabaseSchema::try_from(json_value)
                })
                .transpose()?,
        })
    }
}
