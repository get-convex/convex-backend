use std::{
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
    walk::{
        ConvexArrayWalker,
        ConvexBytesWalker,
        ConvexObjectWalker,
        ConvexStringWalker,
        ConvexValueType,
        ConvexValueWalker,
    },
    ConvexObject,
    ConvexValue,
};

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Invalid type: received {received}, expected {expected}")]
    InvalidType {
        expected: &'static str,
        received: &'static str,
    },

    #[error("ConvexValueType::Int64 was out of range: {0:?}.")]
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

impl Error {
    fn anyhow(e: impl Into<anyhow::Error>) -> Self {
        Self::Anyhow(e.into())
    }
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

struct Deserialize<W>(W);

impl<'de, W: ConvexValueWalker> serde::Deserializer<'de> for Deserialize<W> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Null => visitor.visit_unit(),
            ConvexValueType::Int64(n) => visitor.visit_i64(n),
            ConvexValueType::Float64(n) => visitor.visit_f64(n),
            ConvexValueType::Boolean(b) => visitor.visit_bool(b),
            ConvexValueType::String(s) => visitor.visit_str(s.as_str()),
            ConvexValueType::Bytes(b) => visitor.visit_bytes(b.as_bytes()),
            ConvexValueType::Array(v) => visit_array(v, visitor),
            ConvexValueType::Object(v) => visit_object(v, visitor),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_i8(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_i16(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_i32(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_i64(n),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_u8(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_u16(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_u32(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Int64(n) => visitor.visit_u64(n.try_into()?),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Float64(n) => visitor.visit_f64(n),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Null => visitor.visit_none(),
            v => visitor.visit_some(Deserialize(v)),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Boolean(b) => visitor.visit_bool(b),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::String(s) => visitor.visit_str(s.as_str()),
            v => Err(Error::InvalidType {
                expected: "String",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::String(s) => visitor.visit_string(s.into_string()),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Bytes(b) => visitor.visit_bytes(b.as_bytes()),
            v => Err(Error::InvalidType {
                expected: "Bytes",
                received: v.type_name(),
            }),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Bytes(b) => visitor.visit_byte_buf(b.into_vec()),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Null => visitor.visit_unit(),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Array(v) => visit_array(v, visitor),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Object(v) => visit_object(v, visitor),
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
        match self.0.walk().map_err(Error::anyhow)? {
            ConvexValueType::Object(v) => visit_object(v, visitor),
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

fn visit_array<'de, V>(array: impl ConvexArrayWalker, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let mut deserializer = SeqDeserializer { iter: array.walk() };
    let seq = visitor.visit_seq(&mut deserializer)?;
    if deserializer.iter.next().is_some() {
        return Err(anyhow::anyhow!("Items remaining after deserialization").into());
    }
    Ok(seq)
}

struct SeqDeserializer<I> {
    iter: I,
}

impl<'de, I, W> SeqAccess<'de> for SeqDeserializer<I>
where
    I: Iterator<Item = Result<W, W::Error>>,
    W: ConvexValueWalker,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed
                .deserialize(Deserialize(value.map_err(Error::anyhow)?))
                .map(Some),
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

fn visit_object<'de, V>(object: impl ConvexObjectWalker, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let mut deserializer = MapDeserializer {
        iter: object.walk(),
        value: None,
    };
    let map = visitor.visit_map(&mut deserializer)?;
    if deserializer.iter.next().is_some() {
        return Err(anyhow::anyhow!("Items remaining after deserialization").into());
    }
    Ok(map)
}

struct MapDeserializer<I, W> {
    iter: I,
    value: Option<W>,
}

impl<'de, I, W> MapAccess<'de> for MapDeserializer<I, W>
where
    I: Iterator<Item = Result<(W::FieldName, W), W::Error>>,
    W: ConvexValueWalker,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(r) => {
                let (key, value) = r.map_err(Error::anyhow)?;
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
            Some(value) => seed.deserialize(Deserialize(value)),
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

struct MapKeyDeserializer<S> {
    key: S,
}

impl<'de, S> serde::Deserializer<'de> for MapKeyDeserializer<S>
where
    S: ConvexStringWalker,
{
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
        visitor.visit_str(self.key.as_str())
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key.into_string())
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

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}

pub fn from_object<T: DeserializeOwned>(object: ConvexObject) -> anyhow::Result<T> {
    from_value(ConvexValueType::<ConvexValue>::Object(object))
}

pub fn from_value<V: ConvexValueWalker, T: DeserializeOwned>(value: V) -> anyhow::Result<T> {
    match T::deserialize(Deserialize(value)) {
        Err(Error::Anyhow(e)) => Err(e),
        Err(e) => Err(e.into()),
        Ok(value) => Ok(value),
    }
}
