use std::{
    collections::BTreeMap,
    fmt::{
        self,
        Display,
    },
    num::TryFromIntError,
};

use serde::de::{
    DeserializeOwned,
    DeserializeSeed,
    Error as SerdeError,
    MapAccess,
    SeqAccess,
    Visitor,
};

use crate::{
    ConvexArray,
    ConvexObject,
    ConvexValue,
    FieldName,
};

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Invalid type: received {received}, expected {expected}")]
    InvalidType {
        expected: &'static str,
        received: &'static str,
    },

    #[error("ConvexValue::Int64 was out of range: {0:?}.")]
    IntegerOutofRange(#[from] TryFromIntError),

    #[error("f32s aren't supported, use an f64 instead.")]
    Float32Unsupported,

    #[error("chars aren't supported, use a string instead.")]
    CharUnsupported,

    #[error("Tuple structs aren't supported.")]
    TupleStructsUnsupported,

    #[error("Unit structs aren't supported.")]
    UnitStructUnsupported,

    #[error("Newtype structs aren't supported.")]
    NewtypeStructUnsupported,

    #[error("Deserializing object field into invalid type {field_type}")]
    InvalidField { field_type: &'static str },

    #[error("Ignored any unsupported.")]
    IgnoredAnyUnsupported,

    #[error("Direct enum unsupported, use #[serde(tag = \"type\")] instead.")]
    EnumUnsupported,

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("{0}")]
    Custom(String),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl SerdeError for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

impl<'de> serde::Deserializer<'de> for ConvexValue {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Null => visitor.visit_unit(),
            ConvexValue::Int64(n) => visitor.visit_i64(n),
            ConvexValue::Float64(n) => visitor.visit_f64(n),
            ConvexValue::Boolean(b) => visitor.visit_bool(b),
            ConvexValue::String(s) => visitor.visit_string(s.into()),
            ConvexValue::Bytes(b) => visitor.visit_byte_buf(b.into()),
            ConvexValue::Array(v) => visit_array(v, visitor),
            ConvexValue::Object(v) => visit_object(v, visitor),
            v => Err(anyhow::anyhow!("Unsupported value: {v}").into()),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_i8(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_i16(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_i32(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_i64(n),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_u8(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_u16(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_u32(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Int64(n) => visitor.visit_u64(n.try_into()?),
            v => Err(Error::InvalidType {
                expected: "Int64",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::Float32Unsupported)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Float64(n) => visitor.visit_f64(n),
            v => Err(Error::InvalidType {
                expected: "Float",
                received: v.type_name(),
            }),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::EnumUnsupported)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NewtypeStructUnsupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Boolean(b) => visitor.visit_bool(b),
            v => Err(Error::InvalidType {
                expected: "Boolean",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::CharUnsupported)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::String(s) => visitor.visit_string(s.into()),
            v => Err(Error::InvalidType {
                expected: "String",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Bytes(b) => visitor.visit_byte_buf(b.into()),
            v => Err(Error::InvalidType {
                expected: "Bytes",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Null => visitor.visit_unit(),
            v => Err(Error::InvalidType {
                expected: "Null",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnitStructUnsupported)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Array(v) => visit_array(v, visitor),
            v => Err(Error::InvalidType {
                expected: "Array",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::TupleStructsUnsupported)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Object(v) => visit_object(v, visitor),
            v => Err(Error::InvalidType {
                expected: "Object",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            ConvexValue::Object(v) => visit_object(v, visitor),
            v => Err(Error::InvalidType {
                expected: "Object",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

fn visit_array<'de, V>(array: ConvexArray, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let _len = array.len();
    let mut deserializer = SeqDeserializer {
        iter: Vec::from(array).into_iter(),
    };
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining != 0 {
        return Err(anyhow::anyhow!("Items remaining after deserialization").into());
    }
    Ok(seq)
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<ConvexValue>,
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

fn visit_object<'de, V>(object: ConvexObject, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let _len = object.len();
    let mut deserializer = MapDeserializer {
        iter: BTreeMap::from(object).into_iter(),
        value: None,
    };
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining != 0 {
        return Err(anyhow::anyhow!("Items remaining after deserialization").into());
    }
    Ok(map)
}

struct MapDeserializer {
    iter: <BTreeMap<FieldName, ConvexValue> as IntoIterator>::IntoIter,
    value: Option<ConvexValue>,
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_de = MapKeyDeserializer { key };
                Ok(Some(seed.deserialize(key_de)?))
            },
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(anyhow::anyhow!("value is missing").into()),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

struct MapKeyDeserializer {
    key: FieldName,
}

impl<'de> serde::Deserializer<'de> for MapKeyDeserializer {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "i8" })
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "i16" })
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "i32" })
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "i64" })
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "u8" })
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "u16" })
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "u32" })
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "u64" })
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "f32" })
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "f64" })
    }

    #[inline]
    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "Option<T>",
        })
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "enum" })
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "newtype struct",
        })
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "bool" })
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "char" })
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key.into())
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "bytes",
        })
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "bytes",
        })
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "unit" })
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "unit struct",
        })
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "sequence",
        })
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "tuple",
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "tuple struct",
        })
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField { field_type: "map" })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "struct",
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::InvalidField {
            field_type: "ignored any",
        })
    }
}

pub fn from_value<T: DeserializeOwned>(value: ConvexValue) -> anyhow::Result<T> {
    match T::deserialize(value) {
        Err(Error::Anyhow(e)) => Err(e),
        Err(e) => Err(e.into()),
        Ok(value) => Ok(value),
    }
}

pub fn from_object<T: DeserializeOwned>(value: ConvexObject) -> anyhow::Result<T> {
    match T::deserialize(ConvexValue::Object(value)) {
        Err(Error::Anyhow(e)) => Err(e),
        Err(e) => Err(e.into()),
        Ok(value) => Ok(value),
    }
}
