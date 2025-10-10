//! Provides an abstract representation of a readable ConvexValue.

use std::fmt::{
    Debug,
    Display,
};

use crate::{
    ConvexArray,
    ConvexBytes,
    ConvexObject,
    ConvexString,
    ConvexValue,
    FieldName,
};

/// A trait abstracting over representations of Convex values, such as
/// `ConvexValue`, &'a ConvexValue`, `PackedValue`, etc.
pub trait ConvexValueWalker {
    type Error: Display + Debug + Send + Sync + Into<anyhow::Error>;
    type String: ConvexStringWalker;
    type Bytes: ConvexBytesWalker;
    type Array: ConvexArrayWalker<Error = Self::Error>;
    type FieldName: ConvexStringWalker;
    type Object: ConvexObjectWalker<Error = Self::Error>;

    fn walk(self) -> Result<ConvexValueType<Self>, Self::Error>;
}

pub enum ConvexValueType<V: ConvexValueWalker + ?Sized> {
    Null,
    Int64(i64),
    Float64(f64),
    Boolean(bool),
    String(V::String),
    Bytes(V::Bytes),
    Array(V::Array),
    Object(V::Object),
}

impl<V: ConvexValueWalker + ?Sized> ConvexValueType<V> {
    pub fn type_name(&self) -> &'static str {
        match self {
            ConvexValueType::Null => "Null",
            ConvexValueType::Int64(_) => "Int64",
            ConvexValueType::Float64(_) => "Float64",
            ConvexValueType::Boolean(_) => "Boolean",
            ConvexValueType::String(_) => "String",
            ConvexValueType::Bytes(_) => "Bytes",
            ConvexValueType::Array(_) => "Array",
            ConvexValueType::Object(_) => "Object",
        }
    }
}

pub trait ConvexStringWalker {
    fn as_str(&self) -> &str;
    fn into_string(self) -> String
    where
        Self: Sized,
    {
        self.as_str().to_owned()
    }
}
pub trait ConvexBytesWalker {
    fn as_bytes(&self) -> &[u8];
    fn into_vec(self) -> Vec<u8>
    where
        Self: Sized,
    {
        self.as_bytes().to_vec()
    }
}

pub trait ConvexArrayWalker {
    type Error;
    type Walker: ConvexValueWalker<Error = Self::Error>;
    fn walk(self) -> impl Iterator<Item = Result<Self::Walker, Self::Error>>;
}
pub trait ConvexObjectWalker {
    type Error;
    type Walker: ConvexValueWalker<Error = Self::Error>;
    fn walk(
        self,
    ) -> impl Iterator<
        Item = Result<(<Self::Walker as ConvexValueWalker>::FieldName, Self::Walker), Self::Error>,
    >;
}

impl ConvexValueWalker for ConvexValue {
    type Array = ConvexArray;
    type Bytes = ConvexBytes;
    type Error = !;
    type FieldName = FieldName;
    type Object = ConvexObject;
    type String = ConvexString;

    fn walk(self) -> Result<ConvexValueType<Self>, !> {
        Ok(match self {
            ConvexValue::Null => ConvexValueType::Null,
            ConvexValue::Int64(i) => ConvexValueType::Int64(i),
            ConvexValue::Float64(f) => ConvexValueType::Float64(f),
            ConvexValue::Boolean(b) => ConvexValueType::Boolean(b),
            ConvexValue::String(string) => ConvexValueType::String(string),
            ConvexValue::Bytes(bytes) => ConvexValueType::Bytes(bytes),
            ConvexValue::Array(array) => ConvexValueType::Array(array),
            ConvexValue::Object(object) => ConvexValueType::Object(object),
        })
    }
}

impl<'a> ConvexValueWalker for &'a ConvexValue {
    type Array = &'a ConvexArray;
    type Bytes = &'a ConvexBytes;
    type Error = !;
    type FieldName = &'a FieldName;
    type Object = &'a ConvexObject;
    type String = &'a ConvexString;

    fn walk(self) -> Result<ConvexValueType<Self>, !> {
        Ok(match self {
            ConvexValue::Null => ConvexValueType::Null,
            ConvexValue::Int64(i) => ConvexValueType::Int64(*i),
            ConvexValue::Float64(f) => ConvexValueType::Float64(*f),
            ConvexValue::Boolean(b) => ConvexValueType::Boolean(*b),
            ConvexValue::String(string) => ConvexValueType::String(string),
            ConvexValue::Bytes(bytes) => ConvexValueType::Bytes(bytes),
            ConvexValue::Array(array) => ConvexValueType::Array(array),
            ConvexValue::Object(object) => ConvexValueType::Object(object),
        })
    }
}

// Covers &str, String, and ConvexString
impl<T> ConvexStringWalker for T
where
    T: AsRef<str> + Into<String>,
{
    fn as_str(&self) -> &str {
        self.as_ref()
    }

    fn into_string(self) -> String {
        self.into()
    }
}
impl ConvexStringWalker for &ConvexString {
    fn as_str(&self) -> &str {
        self
    }
}
impl ConvexStringWalker for FieldName {
    fn as_str(&self) -> &str {
        self
    }
}
impl ConvexStringWalker for &FieldName {
    fn as_str(&self) -> &str {
        self
    }
}

// Convers &[u8], Vec<u8>, and ConvexBytes
impl<T> ConvexBytesWalker for T
where
    T: AsRef<[u8]> + Into<Vec<u8>>,
{
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    fn into_vec(self) -> Vec<u8> {
        self.into()
    }
}
impl ConvexBytesWalker for &ConvexBytes {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl ConvexArrayWalker for ConvexArray {
    type Error = !;
    type Walker = ConvexValue;

    fn walk(self) -> impl Iterator<Item = Result<Self::Walker, !>> {
        self.into_iter().map(Ok)
    }
}

impl<'a> ConvexArrayWalker for &'a ConvexArray {
    type Error = !;
    type Walker = &'a ConvexValue;

    fn walk(self) -> impl Iterator<Item = Result<Self::Walker, !>> {
        self.into_iter().map(Ok)
    }
}

impl ConvexObjectWalker for ConvexObject {
    type Error = !;
    type Walker = ConvexValue;

    fn walk(self) -> impl Iterator<Item = Result<(FieldName, Self::Walker), !>> {
        self.into_iter().map(Ok)
    }
}

impl<'a> ConvexObjectWalker for &'a ConvexObject {
    type Error = !;
    type Walker = &'a ConvexValue;

    fn walk(self) -> impl Iterator<Item = Result<(&'a FieldName, Self::Walker), !>> {
        self.iter().map(Ok)
    }
}

// This impl is useful for callers that already know the concrete type of their
// ConvexValue (e.g. when holding a `&'a ConvexObject`)
impl<V: ConvexValueWalker> ConvexValueWalker for ConvexValueType<V> {
    type Array = V::Array;
    type Bytes = V::Bytes;
    type Error = V::Error;
    type FieldName = V::FieldName;
    type Object = V::Object;
    type String = V::String;

    fn walk(self) -> Result<ConvexValueType<Self>, Self::Error> {
        Ok(match self {
            ConvexValueType::Null => ConvexValueType::Null,
            ConvexValueType::Int64(i) => ConvexValueType::Int64(i),
            ConvexValueType::Float64(f) => ConvexValueType::Float64(f),
            ConvexValueType::Boolean(b) => ConvexValueType::Boolean(b),
            ConvexValueType::String(string) => ConvexValueType::String(string),
            ConvexValueType::Bytes(bytes) => ConvexValueType::Bytes(bytes),
            ConvexValueType::Array(array) => ConvexValueType::Array(array),
            ConvexValueType::Object(object) => ConvexValueType::Object(object),
        })
    }
}

impl<'a> ConvexValueWalker for &'a str {
    type Array = ConvexArray;
    type Bytes = ConvexBytes;
    type Error = !;
    type FieldName = FieldName;
    type Object = ConvexObject;
    type String = &'a str;

    fn walk(self) -> Result<ConvexValueType<Self>, !> {
        Ok(ConvexValueType::String(self))
    }
}

impl ConvexValueWalker for ! {
    type Array = &'static ConvexArray;
    type Bytes = &'static [u8];
    type Error = !;
    type FieldName = &'static str;
    type Object = &'static ConvexObject;
    type String = &'static str;

    fn walk(self) -> Result<ConvexValueType<Self>, Self::Error> {
        self
    }
}

impl ConvexValueWalker for i64 {
    type Array = &'static ConvexArray;
    type Bytes = &'static [u8];
    type Error = !;
    type FieldName = &'static str;
    type Object = &'static ConvexObject;
    type String = &'static str;

    fn walk(self) -> Result<ConvexValueType<Self>, Self::Error> {
        Ok(ConvexValueType::Int64(self))
    }
}

impl ConvexValueWalker for f64 {
    type Array = &'static ConvexArray;
    type Bytes = &'static [u8];
    type Error = !;
    type FieldName = &'static str;
    type Object = &'static ConvexObject;
    type String = &'static str;

    fn walk(self) -> Result<ConvexValueType<Self>, Self::Error> {
        Ok(ConvexValueType::Float64(self))
    }
}
