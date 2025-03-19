//! Bijection between a subset of JSON values and our [`Value`]s.
//!
//! Notable features:
//!
//! 1) JSON numbers (64-bit floating point) are mapped to `Number`s.
//! 2) Int64 integers are encoded as their little endian representation in
//!    base64: {"$integer": "..."}.
//! 3) Blobs are encoded as base64: {"$binary": "..."}.
//! 4) Objects are not allowed to have keys starting with "$".

pub mod bytes;
pub mod float;
pub mod integer;
pub(crate) mod json_packed_value;

#[cfg(test)]
mod tests;

use std::{
    collections::{
        btree_map::Entry,
        BTreeMap,
        BTreeSet,
    },
    num::FpCategory,
};

use anyhow::{
    anyhow,
    bail,
    Error,
    Result,
};
use serde::{
    ser::SerializeSeq,
    Serialize,
    Serializer,
};
use serde_json::Value as JsonValue;

use crate::{
    json::{
        bytes::JsonBytes,
        float::JsonFloat,
        integer::JsonInteger,
    },
    metrics,
    numeric::is_negative_zero,
    object::ConvexObject,
    ConvexArray,
    ConvexValue,
};

pub struct SerializeValue<'a>(pub &'a ConvexValue);
impl Serialize for SerializeValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        value::serialize(self.0, serializer)
    }
}

pub struct SerializeArray<'a>(pub &'a ConvexArray);
impl Serialize for SerializeArray<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        array::serialize(self.0, serializer)
    }
}

pub struct SerializeObject<'a>(pub &'a ConvexObject);
impl Serialize for SerializeObject<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        object::serialize(self.0, serializer)
    }
}

struct SerializeIter<I>(I);
impl<T: Serialize, I: Clone + Iterator<Item = T> + ExactSizeIterator> Serialize
    for SerializeIter<I>
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0.clone() {
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

pub mod value {
    use std::num::FpCategory;

    use serde::{
        ser::SerializeMap,
        Serializer,
    };

    use crate::{
        numeric::is_negative_zero,
        ConvexValue,
        JsonBytes,
        JsonFloat,
        JsonInteger,
    };

    pub fn serialize<S: Serializer>(value: &ConvexValue, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            ConvexValue::Null => serializer.serialize_unit(),
            ConvexValue::Int64(n) => {
                let mut obj = serializer.serialize_map(Some(1))?;
                obj.serialize_entry("$integer", &JsonInteger::encode(*n))?;
                obj.end()
            },
            ConvexValue::Float64(n) => {
                let mut is_special = is_negative_zero(*n);
                is_special |= match n.classify() {
                    FpCategory::Zero | FpCategory::Normal | FpCategory::Subnormal => false,
                    FpCategory::Infinite | FpCategory::Nan => true,
                };
                if is_special {
                    let mut obj = serializer.serialize_map(Some(1))?;
                    obj.serialize_entry("$float", &JsonFloat::encode(*n))?;
                    obj.end()
                } else {
                    serializer.serialize_f64(*n)
                }
            },
            ConvexValue::Boolean(b) => serializer.serialize_bool(*b),
            ConvexValue::String(s) => serializer.serialize_str(s),
            ConvexValue::Bytes(b) => {
                let mut obj = serializer.serialize_map(Some(1))?;
                obj.serialize_entry("$bytes", &JsonBytes::encode(b))?;
                obj.end()
            },
            ConvexValue::Array(a) => super::array::serialize(a, serializer),
            ConvexValue::Set(s) => {
                crate::metrics::log_serialized_set();
                let mut obj = serializer.serialize_map(Some(1))?;
                obj.serialize_entry(
                    "$set",
                    &super::SerializeIter(s.iter().map(super::SerializeValue)),
                )?;
                obj.end()
            },
            ConvexValue::Map(m) => {
                crate::metrics::log_serialized_map();
                let mut obj = serializer.serialize_map(Some(1))?;
                obj.serialize_entry(
                    "$map",
                    &super::SerializeIter(
                        m.iter()
                            .map(|(k, v)| [super::SerializeValue(k), super::SerializeValue(v)]),
                    ),
                )?;
                obj.end()
            },
            ConvexValue::Object(o) => super::object::serialize(o, serializer),
        }
    }
}

pub mod array {
    use serde::{
        ser::SerializeSeq,
        Serializer,
    };

    use crate::ConvexArray;

    pub fn serialize<S: Serializer>(array: &ConvexArray, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(array.len()))?;
        for value in array.iter() {
            seq.serialize_element(&super::SerializeValue(value))?;
        }
        seq.end()
    }
}

pub mod object {
    use serde::{
        ser::SerializeMap,
        Deserialize,
        Deserializer,
        Serializer,
    };
    use serde_json::Value as JsonValue;

    use crate::ConvexObject;

    pub fn serialize<S: Serializer>(
        object: &ConvexObject,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(object.len()))?;
        for (key, value) in object.iter() {
            map.serialize_entry(key, &super::SerializeValue(value))?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ConvexObject, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = JsonValue::deserialize(deserializer)?;
        ConvexObject::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<JsonValue> for ConvexValue {
    type Error = Error;

    #[allow(clippy::float_cmp)]
    fn try_from(value: JsonValue) -> Result<Self> {
        let r = match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(b) => Self::from(b),
            JsonValue::Number(n) => {
                // TODO/WTF: JSON supports arbitrary precision numbers?
                let n = n
                    .as_f64()
                    .ok_or_else(|| anyhow!("Arbitrary precision JSON integers unsupported"))?;
                ConvexValue::from(n)
            },
            JsonValue::String(s) => Self::try_from(s)?,
            JsonValue::Array(arr) => {
                let mut out = Vec::with_capacity(arr.len());
                for a in arr {
                    out.push(ConvexValue::try_from(a)?);
                }
                ConvexValue::Array(out.try_into()?)
            },
            JsonValue::Object(map) => {
                if map.len() == 1 {
                    let (key, value) = map.into_iter().next().unwrap();
                    match &key[..] {
                        "$bytes" => {
                            let i: String = serde_json::from_value(value)?;
                            Self::Bytes(JsonBytes::decode(i)?)
                        },
                        "$integer" => {
                            let i: String = serde_json::from_value(value)?;
                            Self::from(JsonInteger::decode(i)?)
                        },
                        "$float" => {
                            let i: String = serde_json::from_value(value)?;
                            let n = JsonFloat::decode(i)?;
                            // Float64s encoded as a $float object must not fit into a regular
                            // `number`.
                            if !is_negative_zero(n) {
                                if let FpCategory::Normal | FpCategory::Subnormal = n.classify() {
                                    bail!("Float64 {} should be encoded as a number", n);
                                }
                            }
                            Self::from(n)
                        },
                        "$set" => {
                            metrics::log_deserialized_set();
                            let items = match value {
                                JsonValue::Array(items) => items,
                                _ => bail!("$set must have an array value"),
                            };
                            let mut set = BTreeSet::new();
                            for item in items {
                                if let Some(old_value) = set.replace(Self::try_from(item)?) {
                                    anyhow::bail!("Duplicate value {old_value} in set");
                                }
                            }
                            Self::Set(set.try_into()?)
                        },
                        "$map" => {
                            metrics::log_deserialized_map();
                            let entries: Vec<[JsonValue; 2]> = serde_json::from_value(value)?;
                            let mut out = BTreeMap::new();
                            for [k, v] in entries {
                                match out.entry(ConvexValue::try_from(k)?) {
                                    Entry::Vacant(e) => {
                                        e.insert(ConvexValue::try_from(v)?);
                                    },
                                    Entry::Occupied(e) => {
                                        anyhow::bail!("Duplicate key {} in map", e.key())
                                    },
                                }
                            }
                            Self::Map(out.try_into()?)
                        },
                        _ => Self::Object(ConvexObject::for_value(
                            key.parse()?,
                            Self::try_from(value)?,
                        )?),
                    }
                } else {
                    let mut fields = BTreeMap::new();
                    for (key, value) in map {
                        fields.insert(key.parse()?, Self::try_from(value)?);
                    }
                    Self::Object(fields.try_into()?)
                }
            },
        };
        Ok(r)
    }
}

impl TryFrom<JsonValue> for ConvexArray {
    type Error = Error;

    fn try_from(object: JsonValue) -> Result<Self> {
        Self::try_from(ConvexValue::try_from(object)?)
    }
}

impl TryFrom<JsonValue> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(object: JsonValue) -> anyhow::Result<Self> {
        ConvexValue::try_from(object)?.try_into()
    }
}

impl From<ConvexValue> for JsonValue {
    fn from(value: ConvexValue) -> Self {
        value.to_internal_json()
    }
}

impl ConvexValue {
    pub fn to_internal_json(&self) -> JsonValue {
        value::serialize(self, serde_json::value::Serializer)
            .expect("Failed to serialize to JsonValue")
    }

    pub fn json_serialize(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&SerializeValue(self))?)
    }
}

impl From<ConvexObject> for JsonValue {
    fn from(value: ConvexObject) -> Self {
        value.to_internal_json()
    }
}

impl ConvexObject {
    pub fn to_internal_json(&self) -> JsonValue {
        object::serialize(self, serde_json::value::Serializer)
            .expect("Failed to serialize to JsonValue")
    }

    pub fn json_serialize(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&SerializeObject(self))?)
    }
}

impl ConvexArray {
    pub fn to_internal_json(&self) -> JsonValue {
        array::serialize(self, serde_json::value::Serializer)
            .expect("Failed to serialize to JsonValue")
    }

    pub fn json_serialize(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&SerializeArray(self))?)
    }
}

pub fn json_deserialize(s: &str) -> anyhow::Result<ConvexValue> {
    let v: serde_json::Value = serde_json::from_str(s)?;
    v.try_into()
}
