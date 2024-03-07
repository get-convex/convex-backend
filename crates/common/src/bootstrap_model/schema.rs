use std::{
    collections::BTreeMap,
    str::FromStr,
};

use errors::ErrorMetadata;
use serde_json::Value as JsonValue;
use value::{
    id_v6::DocumentIdV6,
    obj,
    val,
    ConvexObject,
    ConvexValue,
    GenericDocumentId,
    ResolvedDocumentId,
    TableId,
    TableMapping,
};

use crate::schemas::DatabaseSchema;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SchemaMetadata {
    pub state: SchemaState,
    pub schema: DatabaseSchema,
}

/// SchemaState state machine:
/// ```text
/// +----------+-----------------|  
/// | Pending  |-+               |  
/// +---+------+ |  +--------+   |  
///     |        +->| Failed |   |  
///     v           +--------+   |  
/// +----------+         ^       |  
/// |Validated |---------+       |  
/// +---+------+         |       |  
///     |                |       |  
///     v                v       v  
/// +------+           +-----------+
/// |Active|---------->|Overwritten|
/// +------+           +-----------+
/// ```
/// Invariants:
/// 1. At most one schema can be in the `Pending` or `Validated` state at a
/// time.
///
/// 2. At most one schema can be in the `Active` state at a time.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SchemaState {
    Pending,
    Validated,
    Active,
    Failed {
        error: String,
        table_name: Option<String>,
    },
    Overwritten,
}

impl TryFrom<SchemaState> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(s: SchemaState) -> anyhow::Result<ConvexValue> {
        let object = match s {
            SchemaState::Pending => obj!("state" => "pending"),
            SchemaState::Validated => obj!("state" => "validated"),
            SchemaState::Active => obj!("state" => "active"),
            SchemaState::Failed { error, table_name } => {
                obj!(
                    "state" => "failed",
                    "error" => error.as_str(),
                    "table_name" => if let Some(table_name) = table_name {
                        val!(table_name)
                    } else {
                        val!(null)
                    }
                )
            },
            SchemaState::Overwritten => obj!("state" => "overwritten"),
        }?;
        Ok(ConvexValue::Object(object))
    }
}

impl TryFrom<ConvexValue> for SchemaState {
    type Error = anyhow::Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<SchemaState> {
        let o = if let ConvexValue::Object(o) = v {
            Ok(o)
        } else {
            Err(anyhow::anyhow!("Schema state must be an object"))
        }?;
        let mut fields: BTreeMap<_, _> = o.into();
        match fields.remove("state") {
            Some(ConvexValue::String(s)) => match s.to_string().as_str() {
                "pending" => Ok(SchemaState::Pending),
                "validated" => Ok(SchemaState::Validated),
                "active" => Ok(SchemaState::Active),
                "failed" => {
                    let table_name = fields.remove("table_name").and_then(|table_name| {
                        if let ConvexValue::String(s) = table_name {
                            Some(s.into())
                        } else {
                            None
                        }
                    });
                    match fields.remove("error") {
                        Some(ConvexValue::String(e)) => Ok(SchemaState::Failed {
                            error: e.to_string(),
                            table_name,
                        }),
                        _ => Err(anyhow::anyhow!("Failed schema is missing error")),
                    }
                },
                "overwritten" => Ok(SchemaState::Overwritten),
                _ => Err(anyhow::anyhow!("Invalid schema state: {s}")),
            },
            _ => Err(anyhow::anyhow!(
                "Schema state object is missing state field."
            )),
        }
    }
}

impl TryFrom<ConvexObject> for SchemaMetadata {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> anyhow::Result<Self> {
        let mut fields: BTreeMap<_, _> = o.into();
        let state = fields
            .remove("state")
            .map(SchemaState::try_from)
            .ok_or_else(|| anyhow::anyhow!("Schema is missing state field."))??;
        let schema = match fields.remove("schema") {
            Some(ConvexValue::String(s)) => {
                let deserialized_value: JsonValue = serde_json::from_str(&s)?;
                DatabaseSchema::try_from(deserialized_value)
            },
            None => Err(anyhow::anyhow!("Schema is missing schema field.")),
            _ => Err(anyhow::anyhow!("Schema is not serialized as a string")),
        }?;
        Ok(SchemaMetadata { state, schema })
    }
}

impl TryFrom<SchemaMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(SchemaMetadata { state, schema }: SchemaMetadata) -> anyhow::Result<Self> {
        let serialized_schema = serde_json::to_string(&JsonValue::try_from(schema)?)?;
        obj!("state" => state, "schema" => serialized_schema)
    }
}

pub fn parse_schema_id(
    schema_id: &str,
    table_mapping: &TableMapping,
) -> anyhow::Result<ResolvedDocumentId> {
    // Try parsing as a document ID with TableId first
    match GenericDocumentId::<TableId>::from_str(schema_id) {
        Ok(s) => s.map_table(table_mapping.inject_table_number()),
        Err(_) => {
            // Try parsing as an IDv6 ID
            let id = DocumentIdV6::decode(schema_id)?;
            id.to_resolved(&table_mapping.inject_table_id())
        },
    }
}

pub fn invalid_schema_id(schema_id: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidSchemaId",
        format!("Invalid schema id: {}", schema_id),
    )
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::{
        testing::assert_roundtrips,
        ConvexObject,
    };

    use crate::bootstrap_model::schema::SchemaMetadata;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_schema_roundtrip(v in any::<SchemaMetadata>()) {
            assert_roundtrips::<SchemaMetadata, ConvexObject>(v);
        }
    }
}
