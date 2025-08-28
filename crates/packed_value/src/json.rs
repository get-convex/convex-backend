use std::{
    num::FpCategory,
    str,
};

use flexbuffers::Buffer;
use serde::{
    ser::{
        Error as SerdeError,
        SerializeMap,
        SerializeSeq,
        Serializer,
    },
    Serialize,
};
use value::numeric::is_negative_zero;

use crate::{
    OpenedMap,
    OpenedSet,
    OpenedValue,
};

#[allow(dead_code)] // TODO: remove
struct JsonOpenedValue<'a, B: Buffer>(&'a OpenedValue<B>)
where
    B::BufferString: Clone;

impl<B: Buffer> Serialize for JsonOpenedValue<'_, B>
where
    B::BufferString: Clone,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let result = match self.0 {
            OpenedValue::Null => serializer.serialize_unit()?,
            OpenedValue::Int64(i) => {
                let mut map = serializer.serialize_map(Some(1))?;
                let mut out = [0u8; 12];
                assert_eq!(
                    base64::encode_config_slice(i.to_le_bytes(), base64::STANDARD, &mut out),
                    12,
                );
                let out = str::from_utf8(&out).expect("Encoded base64 wasn't valid UTF8?");
                map.serialize_entry("$integer", out)?;
                map.end()?
            },
            OpenedValue::Float64(f) => {
                let mut is_special = is_negative_zero(*f);
                is_special |= match f.classify() {
                    FpCategory::Zero | FpCategory::Normal | FpCategory::Subnormal => false,
                    FpCategory::Infinite | FpCategory::Nan => true,
                };
                if is_special {
                    let mut map = serializer.serialize_map(Some(1))?;
                    let mut out = [0u8; 12];
                    assert_eq!(
                        base64::encode_config_slice(f.to_le_bytes(), base64::STANDARD, &mut out),
                        12,
                    );
                    let out = str::from_utf8(&out).expect("Encoded base64 wasn't valid UTF8?");
                    map.serialize_entry("$float", out)?;
                    map.end()?
                } else {
                    serializer.serialize_f64(*f)?
                }
            },
            OpenedValue::Boolean(b) => serializer.serialize_bool(*b)?,
            OpenedValue::String(s) => serializer.serialize_str(&s[..])?,
            OpenedValue::Bytes(b) => {
                let mut map = serializer.serialize_map(Some(1))?;
                let out = base64::encode(&b[..]);
                map.serialize_entry("$bytes", &out[..])?;
                map.end()?
            },
            OpenedValue::Array(ref values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value_r in values.iter() {
                    let value = value_r.map_err(SerdeError::custom)?;
                    seq.serialize_element(&JsonOpenedValue(&value))?;
                }
                seq.end()?
            },
            OpenedValue::Set(ref values) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("$set", &JsonOpenedSet(values))?;
                map.end()?
            },
            OpenedValue::Map(ref values) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("$map", &JsonOpenedMap(values))?;
                map.end()?
            },
            OpenedValue::Object(ref fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for r in fields.iter() {
                    let (field, value) = r.map_err(SerdeError::custom)?;
                    map.serialize_entry(&field[..], &JsonOpenedValue(&value))?;
                }
                map.end()?
            },
        };
        Ok(result)
    }
}

#[allow(dead_code)] // TODO: remove
pub struct JsonOpenedSet<'a, B: Buffer>(&'a OpenedSet<B>)
where
    B::BufferString: Clone;

impl<B: Buffer> Serialize for JsonOpenedSet<'_, B>
where
    B::BufferString: Clone,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for value_r in self.0.iter() {
            let value = value_r.map_err(SerdeError::custom)?;
            seq.serialize_element(&JsonOpenedValue(&value))?;
        }
        seq.end()
    }
}

#[allow(dead_code)] // TODO: remove
pub struct JsonOpenedMap<'a, B: Buffer>(&'a OpenedMap<B>)
where
    B::BufferString: Clone;

impl<B: Buffer> Serialize for JsonOpenedMap<'_, B>
where
    B::BufferString: Clone,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for r in self.0.iter() {
            let (key, value) = r.map_err(SerdeError::custom)?;
            seq.serialize_element(&JsonOpenedMapPair {
                key: &key,
                value: &value,
            })?;
        }
        seq.end()
    }
}

#[allow(dead_code)] // TODO: remove
pub struct JsonOpenedMapPair<'a, B: Buffer>
where
    B::BufferString: Clone,
{
    key: &'a OpenedValue<B>,
    value: &'a OpenedValue<B>,
}

impl<B: Buffer> Serialize for JsonOpenedMapPair<'_, B>
where
    B::BufferString: Clone,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&JsonOpenedValue(self.key))?;
        seq.serialize_element(&JsonOpenedValue(self.value))?;
        seq.end()
    }
}
