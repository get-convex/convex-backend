use flexbuffers::Buffer;
use value::walk::{
    ConvexArrayWalker,
    ConvexBytesWalker,
    ConvexObjectWalker,
    ConvexStringWalker,
    ConvexValueType,
    ConvexValueWalker,
};

use crate::{
    OpenedArray,
    OpenedBytes,
    OpenedObject,
    OpenedString,
    OpenedValue,
};

impl<B: Buffer> ConvexValueWalker for OpenedValue<B>
where
    B::BufferString: Clone,
{
    type Array = OpenedArray<B>;
    type Bytes = OpenedBytes<B>;
    type Error = anyhow::Error;
    type FieldName = OpenedString<B>;
    type Object = OpenedObject<B>;
    type String = OpenedString<B>;

    fn walk(self) -> anyhow::Result<ConvexValueType<Self>> {
        Ok(match self {
            OpenedValue::Null => ConvexValueType::Null,
            OpenedValue::Int64(v) => ConvexValueType::Int64(v),
            OpenedValue::Float64(v) => ConvexValueType::Float64(v),
            OpenedValue::Boolean(v) => ConvexValueType::Boolean(v),
            OpenedValue::String(string) => ConvexValueType::String(string),
            OpenedValue::Bytes(bytes) => ConvexValueType::Bytes(bytes),
            OpenedValue::Array(array) => ConvexValueType::Array(array),
            OpenedValue::Object(object) => ConvexValueType::Object(object),
        })
    }
}

impl<B: Buffer> ConvexBytesWalker for OpenedBytes<B>
where
    B::BufferString: Clone,
{
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl<B: Buffer> ConvexStringWalker for OpenedString<B>
where
    B::BufferString: Clone,
{
    fn as_str(&self) -> &str {
        self
    }
}

impl<B: Buffer> ConvexArrayWalker for OpenedArray<B>
where
    B::BufferString: Clone,
{
    type Error = anyhow::Error;
    type Walker = OpenedValue<B>;

    fn walk(self) -> impl Iterator<Item = anyhow::Result<Self::Walker>> {
        (0..self.reader.len()).map(move |i| OpenedValue::new(self.reader.index(i)?))
    }
}
impl<B: Buffer> ConvexObjectWalker for OpenedObject<B>
where
    B::BufferString: Clone,
{
    type Error = anyhow::Error;
    type Walker = OpenedValue<B>;

    fn walk(self) -> impl Iterator<Item = anyhow::Result<(OpenedString<B>, Self::Walker)>> {
        self.iter().map(|v| {
            let (key, value) = v?;
            Ok((OpenedString { buf: key }, value))
        })
    }
}
