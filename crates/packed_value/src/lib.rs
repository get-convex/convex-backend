#![feature(type_alias_impl_trait)]
#![feature(iterator_try_collect)]
#![feature(impl_trait_in_assoc_type)]
#![feature(try_blocks)]
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Deref,
};

use flexbuffers::{
    Blob,
    Buffer,
    Builder,
    FlexBufferType,
    MapReader,
    Reader,
    VectorReader,
};
use value::{
    heap_size::HeapSize,
    ConvexValue,
    FieldPath,
};

mod buffer;
mod debug;
mod flexbuilder;
mod json;

#[cfg(test)]
mod tests;

pub use self::buffer::{
    ByteBuffer,
    StringBuffer,
};
use self::flexbuilder::FlexBuilder;

pub struct PackedValue<B: Buffer>
where
    B::BufferString: Clone,
{
    buf: B,
}

impl<B: Buffer> Clone for PackedValue<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.shallow_copy(),
        }
    }
}

impl<B: Buffer> PackedValue<B>
where
    B::BufferString: Clone,
{
    pub fn new(buf: B) -> Self {
        Self { buf }
    }

    pub fn open(self) -> anyhow::Result<OpenedValue<B>> {
        OpenedValue::new(Reader::get_root(self.buf)?)
    }

    pub fn size(&self) -> usize {
        self.buf.len()
    }

    /// Get a shared reference to the PackedValue, so it can be opened multiple
    /// times without cloning the underlying buffer.
    pub fn as_ref(&self) -> PackedValue<&[u8]> {
        PackedValue::new(&self.buf)
    }

    pub fn open_path(self, field_path: &FieldPath) -> Option<OpenedValue<B>> {
        field_path.fields().first()?; // return None if empty path
        let mut v = self.open().expect("failed to open packed value");
        for field in field_path.fields().iter() {
            match v {
                OpenedValue::Object(o) => {
                    v = o.get(field).expect("failed to open packed object")?;
                },
                _ => return None,
            }
        }
        Some(v)
    }

    /// Same behavior as Value::get_path but doesn't fully unpack.
    pub fn get_path(&self, field_path: &FieldPath) -> Option<ConvexValue> {
        self.as_ref()
            .open_path(field_path)
            .map(ConvexValue::try_from)
            .transpose()
            .expect("failed to unpack opened value")
    }
}

impl<B: Buffer> HeapSize for PackedValue<B>
where
    B::BufferString: Clone,
{
    fn heap_size(&self) -> usize {
        self.buf.len()
    }
}

impl PackedValue<ByteBuffer> {
    pub fn pack(value: &ConvexValue) -> Self {
        let mut builder = Builder::default();
        Self::_pack(value, &mut builder);
        Self {
            buf: builder.take_buffer().into(),
        }
    }

    fn _pack(value: &ConvexValue, builder: &mut impl FlexBuilder) {
        match value {
            ConvexValue::Null => {
                builder.push(());
            },
            ConvexValue::Int64(i) => {
                builder.push(*i);
            },
            ConvexValue::Float64(f) => {
                builder.push(*f);
            },
            ConvexValue::Boolean(b) => {
                builder.push(*b);
            },
            ConvexValue::String(s) => {
                builder.push(&s[..]);
            },
            ConvexValue::Bytes(b) => {
                builder.push(Blob(&b[..]));
            },
            ConvexValue::Array(ref values) => {
                let mut vector = builder.start_vector();
                for value in values {
                    Self::_pack(value, &mut vector);
                }
                vector.end_vector();
            },
            ConvexValue::Set(ref values) => {
                let mut map = builder.start_map();
                let mut vector = map.start_vector("$set");
                for value in values {
                    Self::_pack(value, &mut vector);
                }
                vector.end_vector();
                map.end_map();
            },
            ConvexValue::Map(ref values) => {
                let mut map = builder.start_map();
                let mut vector = map.start_vector("$map");
                for (key, value) in values {
                    Self::_pack(key, &mut vector);
                    Self::_pack(value, &mut vector);
                }
                vector.end_vector();
                map.end_map();
            },
            ConvexValue::Object(ref fields) => {
                let mut map = builder.start_map();
                for (field, value) in fields.iter() {
                    let mut builder = (&field[..], &mut map);
                    Self::_pack(value, &mut builder);
                }
                map.end_map();
            },
        }
    }
}

pub enum OpenedValue<B: Buffer = ByteBuffer>
where
    B::BufferString: Clone,
{
    Null,
    Int64(i64),
    Float64(f64),
    Boolean(bool),
    String(OpenedString<B>),
    Bytes(OpenedBytes<B>),
    Array(OpenedArray<B>),
    Set(OpenedSet<B>),
    Map(OpenedMap<B>),
    Object(OpenedObject<B>),
}

impl<B: Buffer> Clone for OpenedValue<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        match self {
            OpenedValue::Null => OpenedValue::Null,
            OpenedValue::Int64(i) => OpenedValue::Int64(*i),
            OpenedValue::Float64(f) => OpenedValue::Float64(*f),
            OpenedValue::Boolean(b) => OpenedValue::Boolean(*b),
            OpenedValue::String(ref s) => OpenedValue::String(s.clone()),
            OpenedValue::Bytes(ref b) => OpenedValue::Bytes(b.clone()),
            OpenedValue::Array(ref a) => OpenedValue::Array(a.clone()),
            OpenedValue::Set(ref s) => OpenedValue::Set(s.clone()),
            OpenedValue::Map(ref m) => OpenedValue::Map(m.clone()),
            OpenedValue::Object(ref o) => OpenedValue::Object(o.clone()),
        }
    }
}

impl<B: Buffer> OpenedValue<B>
where
    B::BufferString: Clone,
{
    fn new(reader: Reader<B>) -> anyhow::Result<Self> {
        let result = match reader.flexbuffer_type() {
            FlexBufferType::Null => OpenedValue::Null,
            FlexBufferType::Int | FlexBufferType::IndirectInt => {
                OpenedValue::Int64(reader.get_i64()?)
            },
            FlexBufferType::Float | FlexBufferType::IndirectFloat => {
                OpenedValue::Float64(reader.get_f64()?)
            },
            FlexBufferType::Bool => OpenedValue::Boolean(reader.get_bool()?),
            FlexBufferType::String => OpenedValue::String(OpenedString {
                buf: reader.get_str()?,
            }),
            FlexBufferType::Blob => OpenedValue::Bytes(OpenedBytes {
                buf: reader.get_blob()?.0,
            }),
            FlexBufferType::Map => {
                let reader = reader.get_map()?;
                if let Some(ix) = reader.index_key("$set") {
                    anyhow::ensure!(reader.len() == 1);
                    let reader = reader.index(ix)?.get_vector()?;
                    OpenedValue::Set(OpenedSet { reader })
                } else if let Some(ix) = reader.index_key("$map") {
                    anyhow::ensure!(reader.len() == 1);
                    let reader = reader.index(ix)?.get_vector()?;
                    anyhow::ensure!(reader.len() % 2 == 0);
                    OpenedValue::Map(OpenedMap { reader })
                } else {
                    OpenedValue::Object(OpenedObject { reader })
                }
            },
            // NB: Maps also satisfy `is_vector`, so be sure to check those first above.
            ty if ty.is_vector() => OpenedValue::Array(OpenedArray {
                reader: reader.get_vector()?,
            }),
            ty => anyhow::bail!("Unexpected buffer type: {ty:?}"),
        };
        Ok(result)
    }
}

pub struct OpenedString<B: Buffer>
where
    B::BufferString: Clone,
{
    buf: B::BufferString,
}

impl<B: Buffer> Clone for OpenedString<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
        }
    }
}

impl<B: Buffer> Deref for OpenedString<B>
where
    B::BufferString: Clone,
{
    type Target = str;

    fn deref(&self) -> &str {
        &self.buf[..]
    }
}

pub struct OpenedBytes<B: Buffer>
where
    B::BufferString: Clone,
{
    buf: B,
}

impl<B: Buffer> Clone for OpenedBytes<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.shallow_copy(),
        }
    }
}

impl<B: Buffer> Deref for OpenedBytes<B>
where
    B::BufferString: Clone,
{
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buf[..]
    }
}

pub struct OpenedArray<B: Buffer>
where
    B::BufferString: Clone,
{
    reader: VectorReader<B>,
}

impl<B: Buffer> Clone for OpenedArray<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
        }
    }
}

impl<B: Buffer> OpenedArray<B>
where
    B::BufferString: Clone,
{
    pub fn len(&self) -> usize {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    pub fn index(&self, i: usize) -> anyhow::Result<OpenedValue<B>> {
        OpenedValue::new(self.reader.index(i)?)
    }

    pub fn iter(&self) -> impl Iterator<Item = anyhow::Result<OpenedValue<B>>> + '_ {
        (0..self.reader.len()).map(|i| OpenedValue::new(self.reader.index(i)?))
    }
}

pub struct OpenedSet<B: Buffer>
where
    B::BufferString: Clone,
{
    // Invariant: items are written in ascending `Value` order.
    reader: VectorReader<B>,
}

impl<B: Buffer> Clone for OpenedSet<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
        }
    }
}

impl<B: Buffer> OpenedSet<B>
where
    B::BufferString: Clone,
{
    pub fn len(&self) -> usize {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = anyhow::Result<OpenedValue<B>>> + '_ {
        (0..self.reader.len()).map(|i| OpenedValue::new(self.reader.index(i)?))
    }
}

pub struct OpenedMap<B: Buffer>
where
    B::BufferString: Clone,
{
    // Invariant: alternate keys and values are written in ascending `Value` key order.
    reader: VectorReader<B>,
}

impl<B: Buffer> Clone for OpenedMap<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
        }
    }
}

impl<B: Buffer> OpenedMap<B>
where
    B::BufferString: Clone,
{
    pub fn len(&self) -> usize {
        self.reader.len() / 2
    }

    pub fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = anyhow::Result<(OpenedValue<B>, OpenedValue<B>)>> + '_ {
        assert_eq!(self.reader.len() % 2, 0);
        (0..(self.reader.len() / 2)).map(move |i| {
            let key = OpenedValue::new(self.reader.index(2 * i)?)?;
            let value = OpenedValue::new(self.reader.index(2 * i + 1)?)?;
            Ok((key, value))
        })
    }
}

pub struct OpenedObject<B: Buffer>
where
    B::BufferString: Clone,
{
    reader: MapReader<B>,
}

impl<B: Buffer> Clone for OpenedObject<B>
where
    B::BufferString: Clone,
{
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
        }
    }
}

impl<B: Buffer> OpenedObject<B>
where
    B::BufferString: Clone,
{
    pub fn len(&self) -> usize {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    pub fn get(&self, field: &str) -> anyhow::Result<Option<OpenedValue<B>>> {
        let Some(i) = self.reader.index_key(field) else {
            return Ok(None);
        };
        let reader = self.reader.index(i)?;
        Ok(Some(OpenedValue::new(reader)?))
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = anyhow::Result<(B::BufferString, OpenedValue<B>)>> + '_ {
        self.reader
            .iter_keys()
            .zip(self.reader.iter_values())
            .map(|(key, value)| Ok((key, OpenedValue::new(value)?)))
    }
}

impl<B: Buffer> TryFrom<PackedValue<B>> for ConvexValue
where
    B::BufferString: Clone,
{
    type Error = anyhow::Error;

    fn try_from(value: PackedValue<B>) -> anyhow::Result<Self> {
        value.open()?.try_into()
    }
}

impl<B: Buffer> TryFrom<OpenedValue<B>> for ConvexValue
where
    B::BufferString: Clone,
{
    type Error = anyhow::Error;

    fn try_from(value: OpenedValue<B>) -> anyhow::Result<Self> {
        let result = match value {
            OpenedValue::Null => Self::Null,
            OpenedValue::Int64(i) => Self::from(i),
            OpenedValue::Float64(f) => Self::from(f),
            OpenedValue::Boolean(b) => Self::from(b),
            OpenedValue::String(s) => Self::try_from(s[..].to_owned())?,
            OpenedValue::Bytes(b) => Self::try_from(b[..].to_owned())?,
            OpenedValue::Array(packed_values) => {
                let values = packed_values
                    .iter()
                    .map(|r| Self::try_from(r?))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Self::Array(values.try_into()?)
            },
            OpenedValue::Set(packed_values) => {
                let values = packed_values
                    .iter()
                    .map(|r| Self::try_from(r?))
                    .collect::<anyhow::Result<BTreeSet<_>>>()?;
                Self::Set(values.try_into()?)
            },
            OpenedValue::Map(packed_values) => {
                let values = packed_values
                    .iter()
                    .map(|r| {
                        let (k, v) = r?;
                        Ok((Self::try_from(k)?, Self::try_from(v)?))
                    })
                    .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
                Self::Map(values.try_into()?)
            },
            OpenedValue::Object(packed_values) => {
                let values = packed_values
                    .iter()
                    .map(|r| {
                        let (k, v) = r?;
                        Ok((k.parse()?, Self::try_from(v)?))
                    })
                    .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
                Self::Object(values.try_into()?)
            },
        };
        Ok(result)
    }
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::prelude::*;
    use value::ConvexValue;

    use super::{
        buffer::ByteBuffer,
        PackedValue,
    };

    impl Arbitrary for PackedValue<ByteBuffer> {
        type Parameters = ();

        type Strategy = impl Strategy<Value = PackedValue<ByteBuffer>>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<ConvexValue>().prop_map(|v| PackedValue::pack(&v))
        }
    }
}
