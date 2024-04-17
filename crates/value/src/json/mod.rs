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
pub mod object;

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
use serde_json::{
    json,
    Value as JsonValue,
};

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

impl From<ConvexValue> for JsonValue {
    fn from(value: ConvexValue) -> Self {
        match value {
            ConvexValue::Null => JsonValue::Null,
            ConvexValue::Int64(n) => json!({ "$integer": JsonInteger::encode(n) }),
            ConvexValue::Float64(n) => {
                let mut is_special = is_negative_zero(n);
                is_special |= match n.classify() {
                    FpCategory::Zero | FpCategory::Normal | FpCategory::Subnormal => false,
                    FpCategory::Infinite | FpCategory::Nan => true,
                };
                if is_special {
                    json!({ "$float": JsonFloat::encode(n) })
                } else {
                    json!(n)
                }
            },
            ConvexValue::Boolean(b) => json!(b),
            ConvexValue::String(s) => json!(String::from(s)),
            ConvexValue::Bytes(b) => json!({ "$bytes": JsonBytes::encode(&b) }),
            ConvexValue::Array(a) => JsonValue::from(a),
            ConvexValue::Set(s) => {
                metrics::log_serialized_set();
                let items: Vec<_> = s.into_iter().map(JsonValue::from).collect();
                json!({
                    "$set": items,
                })
            },
            ConvexValue::Map(m) => {
                metrics::log_serialized_map();
                let items: Vec<_> = m
                    .into_iter()
                    .map(|(k, v)| [JsonValue::from(k), JsonValue::from(v)])
                    .collect();
                json!({
                    "$map": items,
                })
            },
            ConvexValue::Object(o) => JsonValue::from(o),
        }
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

impl From<ConvexArray> for JsonValue {
    fn from(object: ConvexArray) -> Self {
        let v: Vec<_> = Vec::from(object).into_iter().map(JsonValue::from).collect();
        json!(v)
    }
}

pub fn json_serialize(t: impl Into<ConvexValue>) -> anyhow::Result<String> {
    let v = serde_json::Value::from(t.into());
    Ok(serde_json::to_string(&v)?)
}

pub fn json_deserialize(s: &str) -> anyhow::Result<ConvexValue> {
    let v: serde_json::Value = serde_json::from_str(s)?;
    v.try_into()
}
