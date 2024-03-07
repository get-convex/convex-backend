use std::str::FromStr;

use errors::ErrorMetadata;
use serde_json::{
    json,
    Value as JsonValue,
};

use crate::{
    ConvexObject,
    ConvexValue,
};

/// There are multiple ways a client may want their return Values represented.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ValueFormat {
    /// The default, representing values with invertible encoding.
    /// The websocket protocol uses this encoding.
    /// e.g. integers are represented as {"$int": "base64-encoded-int"}
    ConvexEncodedJSON,
    /// Representing values with a lossy, but more human readable encoding.
    /// e.g. integers are represented as stringified "33"
    ///
    /// On import, we'll use the same algorithm as `ConvexEncodedJSON` (hence
    /// lossyness)
    ///
    /// On export, we'll use the clean export algorithm: implemented as
    /// `.export_clean()` in `Value`
    ///
    /// https://www.notion.so/convex-dev/Clean-Export-serialization-c02508b390f54bdfa1cfdf06f9b7f71e.
    ConvexCleanJSON,
}

impl FromStr for ValueFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "convex_encoded_json" => Ok(Self::ConvexEncodedJSON),
            "convex_json" => Ok(Self::ConvexEncodedJSON), // Legacy alias
            "json" => Ok(Self::ConvexCleanJSON),
            "convex_clean_json" => Ok(Self::ConvexCleanJSON), // Legacy alias
            _ => Err(anyhow::anyhow!("unrecognized value format {s:?}").context(
                ErrorMetadata::bad_request(
                    "BadFormat",
                    format!("format param must be one of [`json`]. Got {s}"),
                ),
            )),
        }
    }
}

impl ConvexObject {
    pub fn export(self, value_format: ValueFormat) -> JsonValue {
        let v: serde_json::Map<_, _> = self
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.export(value_format)))
            .collect();
        JsonValue::Object(v)
    }
}

impl ConvexValue {
    pub fn export(self, value_format: ValueFormat) -> JsonValue {
        match value_format {
            ValueFormat::ConvexEncodedJSON => self.into(),
            ValueFormat::ConvexCleanJSON => self.export_clean(),
        }
    }

    /// Converts this value to a JSON value that is more convenient to work with
    /// than the internal representation.
    ///
    /// It is possible for distinct Convex values to be serialized to the same
    /// JSON value by this method. For instance, strings and binary values are
    /// both exported as JSON strings. However, it is possible to convert the
    /// exported value back to a unique Convex value if you also have the `Type`
    /// value associated with the original Convex value (see `roundtrip.rs`).
    ///
    /// # Example
    /// ```
    /// use crate::ConvexValue;
    /// use serde_json::{
    ///     json,
    ///     Value as JsonValue,
    /// };
    ///
    /// let value = ConvexValue::Bytes(vec![0b00000000, 0b00010000, 0b10000011]);
    /// assert_eq!(JsonValue::from(value.clone()), json!({ "$bytes": "ABCD" }));
    /// assert_eq!(value.export(), json!("ABCD"));
    /// ```
    fn export_clean(self) -> JsonValue {
        match self {
            ConvexValue::Null => JsonValue::Null,
            ConvexValue::Int64(value) => JsonValue::String(value.to_string()),
            ConvexValue::Float64(value) => {
                if value.is_nan() {
                    json!("NaN")
                } else if value.is_infinite() {
                    if value.is_sign_positive() {
                        json!("Infinity")
                    } else {
                        json!("-Infinity")
                    }
                } else {
                    value.into()
                }
            },
            ConvexValue::Boolean(value) => JsonValue::Bool(value),
            ConvexValue::String(value) => JsonValue::String(value.to_string()),
            ConvexValue::Bytes(value) => {
                let bytes: Vec<u8> = value.into();
                JsonValue::String(base64::encode(bytes))
            },
            ConvexValue::Array(values) => {
                JsonValue::Array(values.into_iter().map(|x| x.export_clean()).collect())
            },
            // Use the internal representation for deprecated types
            ConvexValue::Set(_) | ConvexValue::Map(_) => self.into(),
            ConvexValue::Object(map) => JsonValue::Object(
                map.into_iter()
                    .map(|(key, value)| (key.to_string(), value.export_clean()))
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::ExcludeSetsAndMaps;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn clean_export_of_server_and_client_values_are_identical(
            server_value in any_with::<ConvexValue>(
                (Default::default(), ExcludeSetsAndMaps(true))
            )
        ) {
            let json_value: JsonValue = server_value.clone().into();
            let client_value: convex::Value = json_value.try_into().unwrap();
            prop_assert_eq!(server_value.export_clean(), client_value.export());
        }
    }

    #[test]
    fn export_of_a_simple_string() {
        let value = ConvexValue::String("Hello world".try_into().unwrap());
        assert_eq!(
            value.export_clean(),
            JsonValue::String("Hello world".to_string())
        );
    }

    #[test]
    fn export_of_a_simple_int64() {
        let value = ConvexValue::Int64(42);
        assert_eq!(value.export_clean(), JsonValue::String("42".to_string()));
    }
}
