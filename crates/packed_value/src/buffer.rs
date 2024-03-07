use std::{
    ops::{
        Deref,
        Range,
    },
    str::{
        self,
        Utf8Error,
    },
};

use bytes::Bytes;
use flexbuffers::Buffer;

#[derive(Clone, Debug)]
pub struct ByteBuffer {
    inner: Bytes,
}

impl<B> From<B> for ByteBuffer
where
    Bytes: From<B>,
{
    fn from(buf: B) -> Self {
        Self::new(Bytes::from(buf))
    }
}

impl ByteBuffer {
    pub fn new(inner: Bytes) -> Self {
        Self { inner }
    }
}

impl Deref for ByteBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.inner.deref()
    }
}

impl Buffer for ByteBuffer {
    type BufferString = StringBuffer;

    fn slice(&self, range: Range<usize>) -> Option<Self> {
        if range.start > range.end || range.end >= self.inner.len() {
            return None;
        }
        Some(Self {
            inner: self.inner.slice(range),
        })
    }

    fn empty() -> Self {
        Self {
            inner: Bytes::new(),
        }
    }

    fn buffer_str(&self) -> Result<Self::BufferString, Utf8Error> {
        str::from_utf8(&self.inner[..])?;
        Ok(StringBuffer {
            inner: self.inner.clone(),
        })
    }

    fn shallow_copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    fn empty_str() -> Self::BufferString {
        StringBuffer {
            inner: Bytes::new(),
        }
    }
}

#[derive(Clone)]
pub struct StringBuffer {
    inner: Bytes,
}

impl Deref for StringBuffer {
    type Target = str;

    fn deref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.inner[..]) }
    }
}

#[test]
fn test_buffer_size() {
    assert_eq!(std::mem::size_of::<ByteBuffer>(), 32);
    assert_eq!(std::mem::size_of::<StringBuffer>(), 32);
}
