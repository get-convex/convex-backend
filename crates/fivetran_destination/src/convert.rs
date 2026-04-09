use std::{
    collections::BTreeMap,
    str::FromStr,
};

use anyhow::{
    bail,
    Context,
};
use chrono::DateTime;
use common::value::{
    ConvexObject,
    ConvexValue,
    FieldName,
};
use fivetran_common::fivetran_sdk::value_type::Inner as FivetranValue;
use prost_types::Timestamp;
use serde_json::Value as JsonValue;

use crate::{
    constants::{
        ID_CONVEX_FIELD_NAME,
        ID_FIVETRAN_FIELD_NAME,
        METADATA_CONVEX_FIELD_NAME,
        SOFT_DELETE_CONVEX_FIELD_NAME,
        SOFT_DELETE_FIVETRAN_FIELD_NAME,
        SYNCED_CONVEX_FIELD_NAME,
        SYNCED_FIVETRAN_FIELD_NAME,
        UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME,
    },
    file_reader::{
        FileRow,
        FivetranFileValue,
    },
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
/// Other columns starting by an underscore become nested fields in
/// `fivetran.columns`. This is done because their names are reserved in Convex,
/// while such names are frequently used by Fivetran data sources.
///
/// For instance, the following Convex row…
/// ```no_run
/// ┌──────────────┬─────┬─────────┬──────────────┬────────────────────────────────┬───────────────────┐
/// │     name     │ age │ _secret │ _fivetran_id │        _fivetran_synced        │ _fivetran_deleted │
/// ├──────────────┼─────┼─────────┼──────────────┼────────────────────────────────┼───────────────────┤
/// │ [Unmodified] │  21 │ true    │           42 │ 2024-01-09T04:10:19.156057706Z │ false             │
/// └──────────────┴─────┴─────────┴──────────────┴────────────────────────────────┴───────────────────┘
/// ```
///
/// …is converted to the following Convex object:
/// ```no_run
/// {
///   age: 21,
///   fivetran: {
///     id: 42,
///     synced: 1704773419156,
///     deleted: false,
///     columns: {
///       secret: true
///     }
///   }
/// }
/// ```
impl TryInto<ConvexObject> for FileRow {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ConvexObject, Self::Error> {
        let mut row: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();
        let mut metadata: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();
        let mut underscore_columns: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();

        for (field_name, value) in self.0 {
            // Ignore unmodified values
            let FivetranFileValue::Value(value) = value else {
                continue;
            };

            if field_name == *SYNCED_FIVETRAN_FIELD_NAME {
                let FivetranValue::UtcDatetime(timestamp) = value else {
                    bail!("Unexpected value for _fivetran_synced: {:?}", value);
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
            } else if let Some(field_name) = field_name.strip_prefix('_') {
                let field_name = FieldName::from_str(field_name)
                    .context("Invalid field name in the source data")?;

                underscore_columns.insert(field_name, fivetran_to_convex_value(value)?);
            } else {
                let field_name = FieldName::from_str(&field_name)
                    .context("Invalid field name in the source data")?;
                row.insert(field_name, fivetran_to_convex_value(value)?);
            }
        }

        if !underscore_columns.is_empty() {
            metadata.insert(
                UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.clone().into(),
                ConvexValue::Object(ConvexObject::try_from(underscore_columns)?),
            );
        }

        row.insert(
            METADATA_CONVEX_FIELD_NAME.clone().into(),
            ConvexValue::Object(ConvexObject::try_from(metadata)?),
        );
        ConvexObject::try_from(row)
    }
}
