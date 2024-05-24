use std::collections::BTreeMap;

use common::schemas::DatabaseSchema;
use serde_json::Value as JsonValue;
use value::{
    obj,
    val,
    ConvexObject,
    ConvexValue,
};

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDiff {
    pub previous_schema: Option<DatabaseSchema>,
    pub next_schema: Option<DatabaseSchema>,
}
impl TryFrom<SchemaDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(
        SchemaDiff {
            previous_schema,
            next_schema,
        }: SchemaDiff,
    ) -> Result<Self, Self::Error> {
        obj!(
            "previous_schema" => match previous_schema {
            Some(previous_schema) => val!(
                serde_json::to_string(&JsonValue::try_from(previous_schema)?)?
            ),
                None => val!(null),
            },
            "next_schema" => match next_schema {
                Some(next_schema) => val!(
                    serde_json::to_string(&JsonValue::try_from(next_schema)?)?
                ),
                None => val!(null),
            },
        )
    }
}

impl TryFrom<ConvexObject> for SchemaDiff {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = o.into();
        let previous_schema = match fields.remove("previous_schema") {
            Some(ConvexValue::String(s)) => {
                let json_value: JsonValue = serde_json::from_str(&s)?;
                Some(DatabaseSchema::try_from(json_value)?)
            },
            Some(ConvexValue::Null) => None,
            _ => anyhow::bail!("Invalid previous_schema field"),
        };
        let next_schema = match fields.remove("next_schema") {
            Some(ConvexValue::String(s)) => {
                let json_value: JsonValue = serde_json::from_str(&s)?;
                Some(DatabaseSchema::try_from(json_value)?)
            },
            Some(ConvexValue::Null) => None,
            _ => anyhow::bail!("Invalid next_schema field"),
        };
        Ok(Self {
            previous_schema,
            next_schema,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use crate::bootstrap_model::schema::types::SchemaDiff;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_schema_diff_roundtrip(v in any::<SchemaDiff>()) {
            assert_roundtrips::<SchemaDiff, ConvexObject>(v);
        }
    }
}
