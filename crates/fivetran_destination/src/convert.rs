use std::{
    collections::BTreeMap,
    str::FromStr,
};

use anyhow::{
    bail,
    ensure,
    Context,
};
use chrono::DateTime;
use common::value::{
    ConvexObject,
    ConvexValue,
    FieldName,
    Namespace,
};
use convex_fivetran_common::fivetran_sdk::value_type::Inner as FivetranValue;
#[cfg(test)]
use convex_fivetran_common::fivetran_sdk::DataType as FivetranDataType;
use convex_fivetran_destination::constants::{
    ID_CONVEX_FIELD_NAME,
    ID_FIVETRAN_FIELD_NAME,
    METADATA_CONVEX_FIELD_NAME,
    SOFT_DELETE_CONVEX_FIELD_NAME,
    SOFT_DELETE_FIVETRAN_FIELD_NAME,
    SYNCED_CONVEX_FIELD_NAME,
    SYNCED_FIVETRAN_FIELD_NAME,
};
use prost_types::Timestamp;
use serde_json::Value as JsonValue;

use crate::file_reader::{
    FileRow,
    FivetranFileValue,
};

fn fivetran_to_convex_value(value: FivetranValue) -> anyhow::Result<ConvexValue> {
    // https://www.notion.so/convex-dev/Fivetran-Destination-Connector-Implementation-bc917ad7f68b483a93212d93dbbf7b0d?pvs=4#b54c641656284be28451f4be06adf0ab
    Ok(match value {
        FivetranValue::Null(_) => ConvexValue::Null,
        FivetranValue::Bool(v) => ConvexValue::Boolean(v),
        FivetranValue::Short(v) => ConvexValue::Float64(v.into()),
        FivetranValue::Int(v) => ConvexValue::Float64(v.into()),
        FivetranValue::Long(v) => ConvexValue::Int64(v),
        FivetranValue::Float(v) => ConvexValue::Float64(v.into()),
        FivetranValue::Double(v) => ConvexValue::Float64(v),
        FivetranValue::NaiveDate(Timestamp { seconds, nanos }) => ConvexValue::String(
            DateTime::from_timestamp(seconds, nanos as u32)
                .context("Invalid datetime value")?
                .naive_utc()
                .date()
                .format("%Y-%m-%d")
                .to_string()
                .try_into()?,
        ),
        FivetranValue::NaiveTime(Timestamp { seconds, nanos }) => ConvexValue::String(
            DateTime::from_timestamp(seconds, nanos as u32)
                .context("Invalid datetime value")?
                .time()
                .format("%H:%M:%S%.f")
                .to_string()
                .try_into()?,
        ),
        FivetranValue::NaiveDatetime(Timestamp { seconds, nanos }) => ConvexValue::String(
            DateTime::from_timestamp(seconds, nanos as u32)
                .context("Invalid datetime value")?
                .naive_utc()
                .to_string()
                .try_into()?,
        ),
        FivetranValue::UtcDatetime(timestamp) => ConvexValue::Float64(timestamp_to_ms(timestamp)),
        FivetranValue::Decimal(v) => ConvexValue::String(v.try_into()?),
        FivetranValue::Binary(v) => ConvexValue::Bytes(v.try_into()?),
        FivetranValue::String(v) => ConvexValue::String(v.try_into()?),
        FivetranValue::Xml(v) => ConvexValue::String(v.try_into()?),
        FivetranValue::Json(v) => {
            let json_value = serde_json::from_str(&v)
                .context("Your data source contains a JSON value which isn’t valid.")?;
            json_to_convex_value(json_value, Level::Top).context("Your data source contains JSON data that isn’t supported by Convex. You can learn more about the values supported by Convex on https://docs.convex.dev/database/types")?
        },
    })
}

#[derive(PartialEq, Eq)]
enum Level {
    Top,
    Nested,
}

/// Converts a JSON value to a Convex value.
///
/// Unlike the default JsonValue → ConvexValue conversion, this conversion does
/// not support internal representation for types such as binary.
///
/// This conversion only supports arrays, objects and null at the top level. We
/// do so because it forces JSON columns to be marked in the Convex schema with
/// a type that Fivetran will recognize as representing a JSON column.
fn json_to_convex_value(value: JsonValue, level: Level) -> anyhow::Result<ConvexValue> {
    Ok(match value {
        JsonValue::Null => ConvexValue::Null,
        JsonValue::Bool(b) => {
            anyhow::ensure!(
                level != Level::Top,
                "Booleans aren’t supported as top-level JSON values"
            );
            ConvexValue::from(b)
        },
        JsonValue::Number(n) => {
            anyhow::ensure!(
                level != Level::Top,
                "Numbers aren’t supported as top-level JSON values"
            );
            let n = n
                .as_f64()
                .context("Arbitrary precision JSON integers unsupported")?;
            ConvexValue::from(n)
        },
        JsonValue::String(s) => {
            anyhow::ensure!(
                level != Level::Top,
                "Strings aren’t supported as top-level JSON values"
            );
            ConvexValue::try_from(s)?
        },
        JsonValue::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for a in arr {
                out.push(json_to_convex_value(a, Level::Nested)?);
            }
            ConvexValue::try_from(out)?
        },
        JsonValue::Object(map) => {
            let mut fields = BTreeMap::new();
            for (key, value) in map {
                let field_name = FieldName::from_str(&key)?;
                fields.insert(field_name, json_to_convex_value(value, Level::Nested)?);
            }
            ConvexValue::try_from(fields)?
        },
    })
}

fn timestamp_to_ms(ts: Timestamp) -> f64 {
    ts.seconds as f64 * 1000.0 + ((ts.nanos as f64) / 1_000_000.0)
}

/// Converts a Fivetran file row to a Convex object.
///
/// The values marked as unmodified in the Fivetran row are omitted from the
/// Convex object.
///
/// The Fivetran metadata columns (`_fivetran_synced`, `_fivetran_id`, and
/// `_fivetran_deleted`) become nested attributes of the `fivetran` attribute in
/// Convex.
///
/// For instance, the following Convex row…
/// ```no_run
/// ┌──────────────┬─────┬──────────────┬────────────────────────────────┬───────────────────┐
/// │     name     │ age │ _fivetran_id │        _fivetran_synced        │ _fivetran_deleted │
/// ├──────────────┼─────┼──────────────┼────────────────────────────────┼───────────────────┤
/// │ [Unmodified] │  21 │           42 │ 2024-01-09T04:10:19.156057706Z │ false             │
/// └──────────────┴─────┴──────────────┴────────────────────────────────┴───────────────────┘
/// ```
///
/// …is converted to the following Convex object:
/// ```no_run
/// {
///   "age": 21,
///   "fivetran": {
///     "id": 42,
///     "synced": 1704773419156,
///     "deleted": false
///   }
/// }
/// ```
impl TryInto<ConvexObject> for FileRow {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ConvexObject, Self::Error> {
        let mut row: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();
        let mut metadata: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();

        for (field_name, value) in self.0 {
            // Ignore unmodified values
            let FivetranFileValue::Value(value) = value else {
                continue;
            };

            if field_name == *SYNCED_FIVETRAN_FIELD_NAME {
                let FivetranValue::UtcDatetime(timestamp) = value else {
                    bail!("Unexpected value for _fivetran_synced");
                };

                metadata.insert(
                    SYNCED_CONVEX_FIELD_NAME.clone().into(),
                    ConvexValue::Float64(timestamp_to_ms(timestamp)),
                );
            } else if field_name == *SOFT_DELETE_FIVETRAN_FIELD_NAME {
                metadata.insert(
                    SOFT_DELETE_CONVEX_FIELD_NAME.clone().into(),
                    fivetran_to_convex_value(value)?,
                );
            } else if field_name == *ID_FIVETRAN_FIELD_NAME {
                metadata.insert(
                    ID_CONVEX_FIELD_NAME.clone().into(),
                    fivetran_to_convex_value(value)?,
                );
            } else {
                let field_name = FieldName::from_str(&field_name)
                    .context("Invalid field name in the source data")?;
                ensure!(
                    !field_name.is_system(),
                    "System field name in the source data"
                );
                row.insert(field_name, fivetran_to_convex_value(value)?);
            }
        }

        row.insert(
            METADATA_CONVEX_FIELD_NAME.clone().into(),
            ConvexValue::Object(ConvexObject::try_from(metadata)?),
        );
        ConvexObject::try_from(row)
    }
}

#[cfg(test)]
pub fn fivetran_data_type(value: &FivetranValue) -> Option<FivetranDataType> {
    match value {
        FivetranValue::Null(_) => None,
        FivetranValue::Bool(_) => Some(FivetranDataType::Boolean),
        FivetranValue::Short(_) => Some(FivetranDataType::Short),
        FivetranValue::Int(_) => Some(FivetranDataType::Int),
        FivetranValue::Long(_) => Some(FivetranDataType::Long),
        FivetranValue::Float(_) => Some(FivetranDataType::Float),
        FivetranValue::Double(_) => Some(FivetranDataType::Double),
        FivetranValue::NaiveDate(_) => Some(FivetranDataType::NaiveDate),
        FivetranValue::NaiveTime(_) => Some(FivetranDataType::NaiveTime),
        FivetranValue::NaiveDatetime(_) => Some(FivetranDataType::NaiveDatetime),
        FivetranValue::UtcDatetime(_) => Some(FivetranDataType::UtcDatetime),
        FivetranValue::Decimal(_) => Some(FivetranDataType::Decimal),
        FivetranValue::Binary(_) => Some(FivetranDataType::Binary),
        FivetranValue::String(_) => Some(FivetranDataType::String),
        FivetranValue::Json(_) => Some(FivetranDataType::Json),
        FivetranValue::Xml(_) => Some(FivetranDataType::Xml),
    }
}

#[cfg(test)]
fn timestamp_from_ms(ms_since_unix_epoch: f64) -> Timestamp {
    let ms_in_s = 1000.0;
    let ns_in_ms = 1_000_000.0;

    Timestamp {
        seconds: f64::div_euclid(ms_since_unix_epoch, ms_in_s) as i64,
        nanos: (ms_since_unix_epoch.rem_euclid(ms_in_s) * ns_in_ms) as i32,
    }
}

/// Converts a Convex value to the original Fivetran value value, given the type
/// it was originally converted from.
#[cfg(test)]
fn roundtrip_converted_value(
    value: ConvexValue,
    original_type: FivetranDataType,
) -> anyhow::Result<FivetranValue> {
    use chrono::{
        NaiveDate,
        NaiveDateTime,
        NaiveTime,
    };

    Ok(match (value, original_type) {
        (ConvexValue::Boolean(v), FivetranDataType::Boolean) => FivetranValue::Bool(v),
        (ConvexValue::Float64(v), FivetranDataType::Short) => FivetranValue::Short(v as i32),
        (ConvexValue::Float64(v), FivetranDataType::Int) => FivetranValue::Int(v as i32),
        (ConvexValue::Int64(v), FivetranDataType::Long) => FivetranValue::Long(v),
        (ConvexValue::String(v), FivetranDataType::Decimal) => FivetranValue::Decimal(v.into()),
        (ConvexValue::Float64(v), FivetranDataType::Float) => FivetranValue::Float(v as f32),
        (ConvexValue::Float64(v), FivetranDataType::Double) => FivetranValue::Double(v),
        (ConvexValue::String(v), FivetranDataType::NaiveTime) => {
            let dt = NaiveDateTime::new(
                NaiveDate::default(),
                NaiveTime::parse_from_str(&v, "%H:%M:%S%.f")?,
            )
            .and_utc();
            FivetranValue::NaiveTime(Timestamp {
                seconds: dt.timestamp(),
                nanos: dt.timestamp_subsec_nanos() as i32,
            })
        },
        (ConvexValue::String(v), FivetranDataType::NaiveDate) => {
            let dt = NaiveDateTime::new(
                NaiveDate::parse_from_str(&v, "%Y-%m-%d")?,
                NaiveTime::default(),
            )
            .and_utc();
            FivetranValue::NaiveDate(Timestamp {
                seconds: dt.timestamp(),
                nanos: dt.timestamp_subsec_nanos() as i32,
            })
        },
        (ConvexValue::String(v), FivetranDataType::NaiveDatetime) => {
            let dt = NaiveDateTime::parse_from_str(&v, "%Y-%m-%d %H:%M:%S%.f")?.and_utc();
            FivetranValue::NaiveDatetime(Timestamp {
                seconds: dt.timestamp(),
                nanos: dt.timestamp_subsec_nanos() as i32,
            })
        },
        (ConvexValue::Float64(v), FivetranDataType::UtcDatetime) => {
            FivetranValue::UtcDatetime(timestamp_from_ms(v))
        },
        (ConvexValue::Bytes(v), FivetranDataType::Binary) => FivetranValue::Binary(v.into()),
        (ConvexValue::String(v), FivetranDataType::Xml) => FivetranValue::Xml(v.into()),
        (ConvexValue::String(v), FivetranDataType::String) => FivetranValue::String(v.into()),
        (val, FivetranDataType::Json) => {
            let (ConvexValue::Object(_) | ConvexValue::Array(_)) = val else {
                bail!("Unexpected JSON value")
            };
            FivetranValue::Json(JsonValue::from(val).to_string())
        },
        (val, _) => {
            bail!("Unexpected value {val:?} for the given Fivetran type {original_type:?}",)
        },
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use common::{
        assert_obj,
        value::{
            ConvexObject,
            ConvexValue,
        },
    };
    use convex_fivetran_common::fivetran_sdk::value_type::Inner as FivetranValue;
    use convex_fivetran_destination::api_types::FivetranFieldName;
    use maplit::btreemap;
    use proptest::prelude::*;
    use prost_types::Timestamp;

    use super::fivetran_to_convex_value;
    use crate::{
        convert::{
            fivetran_data_type,
            roundtrip_converted_value,
            timestamp_from_ms,
        },
        file_reader::{
            FileRow,
            FivetranFileValue,
        },
    };

    #[test]
    fn convert_file_row_into_convex_object() -> anyhow::Result<()> {
        let actual: ConvexObject = FileRow(btreemap! {
            FivetranFieldName::from_str("name")? => FivetranFileValue::Value(FivetranValue::String("Nicolas".to_string())),
            FivetranFieldName::from_str("null_attribute")? => FivetranFileValue::Value(FivetranValue::Null(true)),
            FivetranFieldName::from_str("unmodified_attribute")? => FivetranFileValue::Unmodified,
            FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                seconds: 1715700652,
                nanos: 563000000,
            })),
        }).try_into()?;
        let expected = assert_obj!(
            "name" => "Nicolas",
            "null_attribute" => ConvexValue::Null,
            // unmodified values are not included in the result
            "fivetran" => assert_obj!(
                "synced" => 1715700652563.0,
            ),
        );

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn convert_file_row_wiht_all_metadata_fields() -> anyhow::Result<()> {
        let actual: ConvexObject = FileRow(btreemap! {
            FivetranFieldName::from_str("name")? => FivetranFileValue::Value(FivetranValue::String("Nicolas".to_string())),
            FivetranFieldName::from_str("_fivetran_id")? => FivetranFileValue::Value(FivetranValue::Int(42)),
            FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(false)),
            FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                seconds: 1715700652,
                nanos: 563000000,
            })),
        }).try_into()?;
        let expected = assert_obj!(
            "name" => "Nicolas",
            "fivetran" => assert_obj!(
                "id" => 42.0,
                "deleted" => false,
                "synced" => 1715700652563.0,
            ),
        );

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn booleans_are_converted_directly() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Bool(false))?,
            ConvexValue::Boolean(false)
        );
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Bool(true))?,
            ConvexValue::Boolean(true)
        );
        Ok(())
    }

    #[test]
    fn small_integer_types_are_converted_to_v_number() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Short(42))?,
            ConvexValue::Float64(42.0)
        );
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Int(-4))?,
            ConvexValue::Float64(-4.0)
        );
        Ok(())
    }

    #[test]
    fn longs_are_converted_directly() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Long(i64::MAX))?,
            ConvexValue::Int64(i64::MAX)
        );
        Ok(())
    }

    #[test]
    fn decimals_are_represented_as_strings() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Decimal("123.456".to_string()))?,
            ConvexValue::String("123.456".try_into()?)
        );
        Ok(())
    }

    #[test]
    fn floats_are_converted_to_doubles() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Float(1.0))?,
            ConvexValue::Float64(1.0)
        );
        Ok(())
    }

    #[test]
    fn doubles_are_converted_directly() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Double(std::f64::consts::PI))?,
            ConvexValue::Float64(std::f64::consts::PI)
        );
        Ok(())
    }

    #[test]
    fn naive_date_is_converted_to_strings() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::NaiveDate(Timestamp {
                seconds: 1196640000,
                nanos: 0
            }))?,
            ConvexValue::String("2007-12-03".try_into()?)
        );
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::NaiveDate(Timestamp {
                seconds: 0,
                nanos: 0
            }))?,
            ConvexValue::String("1970-01-01".try_into()?)
        );
        Ok(())
    }

    #[test]
    fn naive_time_is_converted_to_strings() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::NaiveTime(Timestamp {
                seconds: 19 * 60 * 60 + 41 * 60 + 30,
                nanos: 0
            }))?,
            ConvexValue::String("19:41:30".try_into()?)
        );
        Ok(())
    }

    #[test]
    fn naive_datetime_is_converted_to_strings() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::NaiveDatetime(Timestamp {
                seconds: 1196676930,
                nanos: 0
            }))?,
            ConvexValue::String("2007-12-03 10:15:30".try_into()?)
        );
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::NaiveDatetime(Timestamp {
                seconds: 1196676930,
                nanos: 1_000_000
            }))?,
            ConvexValue::String("2007-12-03 10:15:30.001".try_into()?)
        );
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::NaiveDatetime(Timestamp {
                seconds: 0,
                nanos: 0
            }))?,
            ConvexValue::String("1970-01-01 00:00:00".try_into()?)
        );
        Ok(())
    }

    #[test]
    fn utc_datetimes_are_converted_to_ms_timestamps() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::UtcDatetime(Timestamp {
                seconds: 1196676930,
                nanos: 123_000_000,
            }))?,
            ConvexValue::Float64(1196676930.0 * 1000.0 + 123.0)
        );
        Ok(())
    }

    #[test]
    fn binary_is_converted_directly() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Binary(vec![0, 255, 1, 2, 3]))?,
            ConvexValue::Bytes(vec![0, 255, 1, 2, 3].try_into()?)
        );
        Ok(())
    }

    #[test]
    fn xml_is_converted_to_string() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Xml(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><document/>".to_string()
            ))?,
            ConvexValue::String(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><document/>".try_into()?
            )
        );
        Ok(())
    }

    #[test]
    fn string_is_converted_directly() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::String("Hello world".to_string()))?,
            ConvexValue::String("Hello world".try_into()?)
        );
        Ok(())
    }

    #[test]
    fn json_objects_are_converted() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Json(
                "{
                    \"null\": null,
                    \"bool\": false,
                    \"number\": 42,
                    \"string\": \"Hello world\",
                    \"array\": [-1],
                    \"object\": { \"nested\": true }
                }"
                .to_string()
            ))?,
            ConvexValue::Object(assert_obj!(
                "null" => ConvexValue::Null,
                "bool" => false,
                "number" => 42.0,
                "string" => "Hello world",
                "array" => ConvexValue::Array(vec![ConvexValue::Float64(-1.0)].try_into()?),
                "object" => ConvexValue::Object(assert_obj!(
                    "nested" => true,
                )),
            )),
        );
        Ok(())
    }

    #[test]
    fn json_arrays_are_converted() -> anyhow::Result<()> {
        assert_eq!(
            fivetran_to_convex_value(FivetranValue::Json("[1,2,3]".to_string()))?,
            ConvexValue::Array(
                vec![
                    ConvexValue::Float64(1.0),
                    ConvexValue::Float64(2.0),
                    ConvexValue::Float64(3.0),
                ]
                .try_into()?
            )
        );
        Ok(())
    }

    #[test]
    fn other_json_values_are_not_converted() -> anyhow::Result<()> {
        fivetran_to_convex_value(FivetranValue::Json("42".to_string())).unwrap_err();
        Ok(())
    }

    #[test]
    fn json_conversions_can_fail() -> anyhow::Result<()> {
        fivetran_to_convex_value(FivetranValue::Json("{\"$reserved\": true}".to_string()))
            .unwrap_err();
        Ok(())
    }

    #[test]
    fn ms_to_timestamp_conversion() {
        assert_eq!(
            timestamp_from_ms(1234.0),
            Timestamp {
                seconds: 1,
                nanos: 234_000_000
            }
        );
        assert_eq!(
            timestamp_from_ms(-1.0),
            Timestamp {
                seconds: -1,
                nanos: 999_000_000
            }
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn test_fivetran_to_convex_conversion_roundtrips(value in any::<FivetranValue>()) {
            // Skipping FivetranValue::Json since a JSON value might have different string
            // representation due to whitespace, and serde_json’s equality doesn’t match JavaScript
            // value equality (e.g. 0.0 != 0 when comparing the equality of JsonValue)
            if let FivetranValue::Null(_) | FivetranValue::Json(_) = value {
                return Ok(());
            }

            let original_data_type = fivetran_data_type(&value)
                .expect("The original value has no data type");
            let Ok(converted_value) = fivetran_to_convex_value(value.clone()) else {
                panic!("Can’t serialize the value {value:?}");
            };
            let roundtripped_value = roundtrip_converted_value(converted_value, original_data_type)
                .expect("Couldn’t roundtrip the value");
            prop_assert_eq!(
                fivetran_data_type(&roundtripped_value)
                    .expect("The roundtripped value has no data type"),
                original_data_type
            );
            prop_assert_eq!(roundtripped_value, value);
        }
    }
}
