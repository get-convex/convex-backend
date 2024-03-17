use std::{
    collections::BTreeMap,
    fmt::{
        self,
        Display,
    },
    num::TryFromIntError,
};

use serde::{
    ser::{
        Error as SerdeError,
        Impossible,
    },
    Serialize,
};

use crate::{
    ConvexObject,
    ConvexValue,
    FieldName,
};

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Integer isn't in range for ConvexValue::Int64: {0:?}.")]
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

    #[error("Invalid field {field} for Convex object: {err}")]
    InvalidField { field: String, err: String },

    #[error(
        "Struct enum variants unsupported. Set #[serde(tag = \"type\")] to serialize as a regular \
         object."
    )]
    StructVariantsUnsupported,

    #[error(
        "Unit enum variants unsupported. Set #[serde(tag = \"type\")] to serialize as a regular \
         object."
    )]
    EnumVariantsUnsupported,

    #[error(
        "Newtype enum variants unsupported. Set #[serde(tag = \"type\")] to serialize as a \
         regular object."
    )]
    NewtypeVariantsUnsupported,

    #[error(
        "Tuple enum variants unsupported. Set #[serde(tag = \"type\")] to serialize as a regular \
         object."
    )]
    TupleVariantsUnsupported,

    #[error("{0}")]
    Custom(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
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

type Result<T> = std::result::Result<T, Error>;

struct Serializer;

impl serde::Serializer for Serializer {
    type Error = Error;
    type Ok = ConvexValue;
    type SerializeMap = SerializeObject;
    type SerializeSeq = SerializeVec;
    type SerializeStruct = SerializeObject;
    type SerializeStructVariant = Impossible<ConvexValue, Error>;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = Impossible<ConvexValue, Error>;
    type SerializeTupleVariant = Impossible<ConvexValue, Error>;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<ConvexValue> {
        Ok(ConvexValue::Boolean(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value as i64))
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value as i64))
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value as i64))
    }

    fn serialize_i64(self, value: i64) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value))
    }

    fn serialize_i128(self, value: i128) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value.try_into()?))
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value as i64))
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value as i64))
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value as i64))
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value.try_into()?))
    }

    fn serialize_u128(self, value: u128) -> Result<ConvexValue> {
        Ok(ConvexValue::Int64(value.try_into()?))
    }

    #[inline]
    fn serialize_f32(self, _float: f32) -> Result<ConvexValue> {
        // We don't serialize `f32` so we don't have to worry about roundtripping from
        // f32 to f64 to f32.
        Err(Error::Float32Unsupported)
    }

    #[inline]
    fn serialize_f64(self, float: f64) -> Result<ConvexValue> {
        Ok(ConvexValue::Float64(float))
    }

    #[inline]
    fn serialize_char(self, _value: char) -> Result<ConvexValue> {
        Err(Error::CharUnsupported)
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<ConvexValue> {
        Ok(ConvexValue::String(value.try_into()?))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<ConvexValue> {
        Ok(ConvexValue::Bytes(value.to_vec().try_into()?))
    }

    #[inline]
    fn serialize_unit(self) -> Result<ConvexValue> {
        Ok(ConvexValue::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<ConvexValue> {
        Err(Error::UnitStructUnsupported)
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<ConvexValue> {
        Err(Error::EnumVariantsUnsupported)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<ConvexValue>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NewtypeStructUnsupported)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<ConvexValue>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NewtypeVariantsUnsupported)
    }

    #[inline]
    fn serialize_none(self) -> Result<ConvexValue> {
        Ok(ConvexValue::Null)
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<ConvexValue>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::TupleStructsUnsupported)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::TupleVariantsUnsupported)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeObject {
            fields: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(SerializeObject {
            fields: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::StructVariantsUnsupported)
    }

    fn collect_str<T>(self, value: &T) -> Result<ConvexValue>
    where
        T: ?Sized + Display,
    {
        Ok(ConvexValue::String(value.to_string().try_into()?))
    }
}

struct SerializeVec {
    vec: Vec<ConvexValue>,
}

impl serde::ser::SerializeSeq for SerializeVec {
    type Error = Error;
    type Ok = ConvexValue;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<ConvexValue> {
        Ok(ConvexValue::Array(self.vec.try_into()?))
    }
}

impl serde::ser::SerializeTuple for SerializeVec {
    type Error = Error;
    type Ok = ConvexValue;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<ConvexValue> {
        Ok(ConvexValue::Array(self.vec.try_into()?))
    }
}

struct SerializeObject {
    fields: BTreeMap<FieldName, ConvexValue>,
    next_key: Option<FieldName>,
}

impl serde::ser::SerializeMap for SerializeObject {
    type Error = Error;
    type Ok = ConvexValue;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        assert!(
            self.next_key.is_none(),
            "serialize_key called twice without serialize_value"
        );
        self.next_key = Some(key.serialize(FieldSerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .next_key
            .take()
            .expect("serialize_value called without preceding serialize_key");
        self.fields.insert(key, to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<ConvexValue> {
        Ok(ConvexValue::Object(self.fields.try_into()?))
    }
}

struct FieldSerializer;

impl serde::Serializer for FieldSerializer {
    type Error = Error;
    type Ok = FieldName;
    type SerializeMap = Impossible<FieldName, Error>;
    type SerializeSeq = Impossible<FieldName, Error>;
    type SerializeStruct = Impossible<FieldName, Error>;
    type SerializeStructVariant = Impossible<FieldName, Error>;
    type SerializeTuple = Impossible<FieldName, Error>;
    type SerializeTupleStruct = Impossible<FieldName, Error>;
    type SerializeTupleVariant = Impossible<FieldName, Error>;

    #[inline]
    fn serialize_str(self, value: &str) -> Result<FieldName> {
        value.parse::<FieldName>().map_err(|e| Error::InvalidField {
            field: value.to_string(),
            err: e.to_string(),
        })
    }

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_i64(self, value: i64) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_i128(self, value: i128) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_u128(self, value: u128) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: value.to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: "bytes".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_unit(self) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: "unit".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: "unit struct".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: "unit variant".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<FieldName>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::InvalidField {
            field: "newtype struct".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<FieldName>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::InvalidField {
            field: "newtype variant".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_none(self) -> Result<FieldName> {
        Err(Error::InvalidField {
            field: "None".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    #[inline]
    fn serialize_some<T>(self, _value: &T) -> Result<FieldName>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::InvalidField {
            field: "Some".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::InvalidField {
            field: "seq".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::InvalidField {
            field: "tuple".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::InvalidField {
            field: "tuple struct".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::InvalidField {
            field: "tuple variant".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::InvalidField {
            field: "map".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::InvalidField {
            field: "struct".to_string(),
            err: "fields must be strings".to_string(),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::InvalidField {
            field: "struct variant".to_string(),
            err: "fields must be strings".to_string(),
        })
    }
}

impl serde::ser::SerializeStruct for SerializeObject {
    type Error = Error;
    type Ok = ConvexValue;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        field: &'static str,
        value: &T,
    ) -> Result<()> {
        self.fields.insert(field.parse()?, to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<ConvexValue> {
        Ok(ConvexValue::Object(self.fields.try_into()?))
    }
}

pub fn to_value<T: Serialize>(value: T) -> Result<ConvexValue> {
    value.serialize(Serializer)
}

pub fn to_object<T: Serialize>(value: T) -> Result<ConvexObject> {
    Ok(to_value(value)?.try_into()?)
}
