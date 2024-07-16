use std::collections::BTreeMap;

use anyhow::Context;
#[cfg(test)]
use convex::ExportContext;
use convex::Value as ConvexValue;
use convex_fivetran_common::fivetran_sdk::value_type::Inner as FivetranValue;
use prost_types::Timestamp;
use serde_json::Value as JsonValue;

fn timestamp_from_ms(ms_since_unix_epoch: f64) -> Timestamp {
    let ms_in_s = 1000.0;
    let ns_in_ms = 1_000_000.0;

    Timestamp {
        seconds: (ms_since_unix_epoch / ms_in_s) as i64,
        nanos: ((ms_since_unix_epoch % ms_in_s) * ns_in_ms) as i32,
    }
}

fn to_fivetran_value(value: ConvexValue) -> FivetranValue {
    match value {
        ConvexValue::Null => FivetranValue::Null(true),
        ConvexValue::Int64(value) => FivetranValue::Long(value),
        ConvexValue::Float64(value) => FivetranValue::Double(value),
        ConvexValue::Boolean(value) => FivetranValue::Bool(value),
        ConvexValue::String(value) => FivetranValue::String(value),
        ConvexValue::Bytes(value) => FivetranValue::Binary(value),
        ConvexValue::Array(_) | ConvexValue::Object(_) => {
            FivetranValue::Json(value.export().to_string())
        },
    }
}

/// Converts a Fivetran value to the original Convex value, given its export
/// context. This is only used in proptests to ensure that the conversion isn’t
/// lossy.
#[cfg(test)]
fn roundtrip_fivetran_value(
    value: FivetranValue,
    export_context: ExportContext,
) -> anyhow::Result<ConvexValue> {
    Ok(match value {
        FivetranValue::Null(_) => ConvexValue::Null,
        FivetranValue::Bool(value) => ConvexValue::Boolean(value),
        FivetranValue::Long(value) => ConvexValue::Int64(value),
        FivetranValue::Double(value) => ConvexValue::Float64(value),
        FivetranValue::Binary(value) => ConvexValue::Bytes(value),
        FivetranValue::String(value) => ConvexValue::String(value),
        FivetranValue::Json(value) => {
            let json: JsonValue = serde_json::from_str(&value)?;
            (json, &export_context).try_into()?
        },

        FivetranValue::Float(_)
        | FivetranValue::Short(_)
        | FivetranValue::Int(_)
        | FivetranValue::UtcDatetime(_)
        // | FivetranValue::NaiveTime(_)
        | FivetranValue::NaiveDate(_)
        | FivetranValue::NaiveDatetime(_)
        | FivetranValue::Decimal(_)
        | FivetranValue::Xml(_) => anyhow::bail!("Unsupported Fivetran value: {:?}", value),
    })
}

/// Converts a Convex document field to a Fivetran field.
/// Returns None if the field is skipped in Fivetran.
fn to_fivetran_field(
    (field_name, field_value): (String, JsonValue),
) -> anyhow::Result<Option<(String, FivetranValue)>> {
    let result =
        // Skip most system fields
        if field_name.starts_with('_') && field_name != "_id" && field_name != "_creationTime" {
            None
        } else {
            let fivetran_value: FivetranValue = if field_name == "_creationTime" {
                let JsonValue::Number(milliseconds) = field_value else {
                    anyhow::bail!("Unexpected _creationTime value: {:?}", field_value);
                };
                let milliseconds = milliseconds.as_f64().context(
                    "Unexpected arbitrary-precision floating-point number found in _creationTime"
                )?;
                FivetranValue::UtcDatetime(timestamp_from_ms(milliseconds))
            } else {
                let convex_value = ConvexValue::try_from(field_value).context("Invalid Convex value")?;
                to_fivetran_value(convex_value)
            };

            Some((field_name, fivetran_value))
        };
    anyhow::Result::Ok(result)
}

pub fn to_fivetran_row(
    convex_document: BTreeMap<String, JsonValue>,
) -> anyhow::Result<BTreeMap<String, FivetranValue>> {
    let possible_object_entries: Vec<Option<(String, FivetranValue)>> = convex_document
        .into_iter()
        .map(to_fivetran_field)
        .try_collect()?;
    Ok(possible_object_entries.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use maplit::btreemap;
    use proptest::prelude::*;
    use serde_json::json;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn value_to_fivetran_roundtrips(value in any::<ConvexValue>()) {
            let fivetran_value: FivetranValue = to_fivetran_value(value.clone());
            let export_context = ExportContext::of(&value);
            assert_eq!(value, roundtrip_fivetran_value(fivetran_value, export_context).unwrap());
        }
    }

    #[test]
    fn ignores_system_fields_except_id_and_creation_time() -> anyhow::Result<()> {
        let result = to_fivetran_row(btreemap! {
            "_id".to_string() => json!("2rsfck4e88mvyb011h9k7znq9h1mb00"),
            "_creationTime".to_string() => json!(1686799242010.5989),
            "_other_system_field".to_string() => json!("hidden"),
            "normalField".to_string() => json!("Hello world"),
        })?;

        assert!(result.contains_key("_id"));
        assert!(result.contains_key("_creationTime"));
        assert!(!result.contains_key("_other_system_field"));
        assert!(result.contains_key("normalField"));

        Ok(())
    }

    #[test]
    fn can_convert_id() -> anyhow::Result<()> {
        assert_eq!(
            to_fivetran_row(btreemap! {
                "_id".to_string() => json!("2rsfck4e88mvyb011h9k7znq9h1mb00"),
            })?,
            btreemap! {
                "_id".to_string() => FivetranValue::String("2rsfck4e88mvyb011h9k7znq9h1mb00".to_string()),
            }
        );

        Ok(())
    }

    #[test]
    fn can_convert_creation_time() -> anyhow::Result<()> {
        assert_eq!(
            to_fivetran_row(btreemap! {
                "_creationTime".to_string() => json!(1686799242010.5),
            })?,
            btreemap! {
                "_creationTime".to_string() => FivetranValue::UtcDatetime(Timestamp::date_time_nanos(2023, 6, 15, 3, 20, 42, 10500000).unwrap()),
            }
        );

        Ok(())
    }
}
