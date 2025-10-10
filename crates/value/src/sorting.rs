//! Implementation of `Ord` for `Value` using *sort keys*. We define a total
//! ordering on `Value`s by mapping them to binary strings and then comparing
//! those lexicographically.
//!
//! This encoding of `Value` is self-delimiting and variable-length. Each value
//! has a tag indicating its value (and some additional length information for
//! integers), with the tags carefully chosen to preserve order.
//!
//! This encoding largely follows FoundationDB's [tuple
//! layer](https://github.com/apple/foundationdb/blob/master/design/tuple.md).
//! 1) Values are always prefixed with a tag.
//! 2) Binary strings are stored with `0x0` as a delimiter. Null bytes within
//!    the string are escaped to `0x0 0xFF`, which implies that `0xFF` must
//!    never be a valid tag.
//! 3) 64-bit signed integers are stored as either 1, 2, 4, or 8 bytes, with the
//!    size stored within the tag. There is a negative variant of the tag that
//!    orders negatives before positives.
//! 4) Floats are stored according to IEEE-754 total ordering. See [FoundationDB's notes](https://github.com/apple/foundationdb/blob/master/design/tuple.md#ieee-binary-floating-point)
//!    for an explanation of the algorithm.
//! 5) Compound types, like arrays, are stored sequentially, with a null
//!    terminator at the end.
use std::cmp::Ordering;

use bytes::BufMut;

use crate::{
    walk::{
        ConvexArrayWalker,
        ConvexBytesWalker,
        ConvexObjectWalker,
        ConvexStringWalker,
        ConvexValueType,
        ConvexValueWalker,
    },
    ConvexValue,
};

const UNDEFINED_TAG: u8 = 0x1;

// Legacy ID format (unused).
// const ID_TAG: u8 = 0x2;
const NULL_TAG: u8 = 0x3;

#[cfg(any(test, feature = "testing"))]
const NEG_INT64_8_BYTE_TAG: u8 = 0x4;
#[allow(unused)]
const NEG_INT64_4_BYTE_TAG: u8 = 0x5;
#[allow(unused)]
const NEG_INT64_2_BYTE_TAG: u8 = 0x6;
#[allow(unused)]
const NEG_INT64_1_BYTE_TAG: u8 = 0x7;
const ZERO_INT64_TAG: u8 = 0x8;
#[allow(unused)]
const POS_INT64_1_BYTE_TAG: u8 = 0x9;
#[allow(unused)]
const POS_INT64_2_BYTE_TAG: u8 = 0xA;
#[allow(unused)]
const POS_INT64_4_BYTE_TAG: u8 = 0xB;
#[cfg(any(test, feature = "testing"))]
const POS_INT64_8_BYTE_TAG: u8 = 0xC;

const FLOAT64_TAG: u8 = 0xD;

const FALSE_BOOLEAN_TAG: u8 = 0xE;
const TRUE_BOOLEAN_TAG: u8 = 0xF;

const STRING_TAG: u8 = 0x10;
const BYTES_TAG: u8 = 0x11;
const ARRAY_TAG: u8 = 0x12;
// Deprecated datatypes, now unused.
// const SET_TAG: u8 = 0x13;
// const MAP_TAG: u8 = 0x14;
const OBJECT_TAG: u8 = 0x15;

pub const TERMINATOR_BYTE: u8 = 0x0;
const ESCAPE_BYTE: u8 = 0xFF;

pub fn write_escaped_bytes(buf: &[u8], writer: &mut impl BufMut) {
    for &byte in buf {
        writer.put_u8(byte);
        if byte == TERMINATOR_BYTE {
            writer.put_u8(ESCAPE_BYTE);
        }
    }
    writer.put_u8(TERMINATOR_BYTE);
}

fn write_escaped_string(buf: &str, writer: &mut impl BufMut) {
    write_escaped_bytes(buf.as_bytes(), writer)
}

#[allow(clippy::match_overlapping_arm)]
fn write_tagged_int(n: i64, writer: &mut impl BufMut) {
    // Our integer tag values are chosen such that their distance from the zero tag
    // represents how many bytes they should take.
    let tag_diff = match n {
        -128..=127 => 1,
        -32_768..=32_767 => 2,
        -2_147_483_648..=2147483647 => 3,
        -9_223_372_036_854_775_808..=9_223_372_036_854_775_807 => 4,
    };
    let tag = if n < 0 {
        ZERO_INT64_TAG - tag_diff
    } else {
        ZERO_INT64_TAG + tag_diff
    };
    let num_bytes = 1 << (tag_diff - 1);
    let buf = n.to_be_bytes();
    // Check that all of the bytes we're leaving off aren't used.
    let empty = if n < 0 { 0xFF } else { 0 };
    assert!(buf[..(8 - num_bytes)].iter().all(|&b| b == empty));
    writer.put_u8(tag);
    writer.put(&buf[(8 - num_bytes)..]);
}

/// Generate the sort key for a sequence of `Value`s.
pub fn values_to_bytes(values: &[Option<ConvexValue>]) -> Vec<u8> {
    let mut out = vec![];
    for value in values {
        let Ok(()) = write_sort_key_or_undefined(value.as_ref(), &mut out);
    }
    out
}

/// Once a Value or IndexKey has been encoded for sorting, it should not be
/// necessary to decode the Value or IndexKey again. Therefore this is
/// test-only.
#[cfg(any(test, feature = "testing"))]
pub mod sorting_decode {
    use std::{
        cmp,
        collections::BTreeMap,
        io::{
            self,
            Read,
        },
    };

    use anyhow::bail;
    use byteorder::{
        BigEndian,
        ReadBytesExt,
    };

    use super::*;
    use crate::ConvexObject;

    fn read_escaped_string<R: Read>(reader: &mut BytePeeker<R>) -> anyhow::Result<String> {
        Ok(String::from_utf8(read_escaped_bytes(reader)?)?)
    }

    fn read_terminated<R: Read, F>(
        reader: &mut BytePeeker<R>,
        mut read_element: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(&mut BytePeeker<R>) -> anyhow::Result<()>,
    {
        loop {
            if let Some(TERMINATOR_BYTE) = reader.peek()? {
                reader.read_u8()?;
                break;
            }
            read_element(reader)?;
        }
        Ok(())
    }

    fn read_tagged_int<R: Read>(tag: u8, reader: &mut R) -> io::Result<i64> {
        let is_negative = tag < ZERO_INT64_TAG;
        let tag_diff = cmp::max(tag, ZERO_INT64_TAG) - cmp::min(tag, ZERO_INT64_TAG);
        let num_bytes = 1 << (tag_diff - 1);
        let mut buf = [if is_negative { 0xFF } else { 0x0 }; 8];
        reader.read_exact(&mut buf[8 - num_bytes..])?;
        Ok(i64::from_be_bytes(buf))
    }

    /// Parse a `Vec<Value>` from it respective sort keys.
    pub fn bytes_to_values<R: Read>(reader: &mut R) -> anyhow::Result<Vec<Option<ConvexValue>>> {
        let reader = &mut BytePeeker::new(reader);
        let mut values = vec![];
        while reader.peek()?.is_some() {
            let value = ConvexValue::_read_sort_key(reader)?;
            values.push(Some(value));
        }
        Ok(values)
    }

    /// Reader that allow us to peak the next byte.
    pub struct BytePeeker<R: Read> {
        buf: Option<u8>,
        reader: R,
    }

    impl<R: Read> BytePeeker<R> {
        fn new(reader: R) -> Self {
            Self { buf: None, reader }
        }

        pub fn peek(&mut self) -> io::Result<Option<u8>> {
            if let Some(byte) = self.buf {
                return Ok(Some(byte));
            }
            let mut buf = [0];
            let n = self.reader.read(&mut buf)?;
            if n == 0 {
                return Ok(None);
            }
            let byte = buf[0];
            self.buf = Some(byte);
            Ok(Some(byte))
        }
    }

    impl<R: Read> Read for BytePeeker<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if buf.is_empty() {
                return Ok(0);
            }
            if let Some(byte) = self.buf.take() {
                buf[0] = byte;
                return Ok(1 + self.reader.read(&mut buf[1..])?);
            }
            self.reader.read(buf)
        }
    }

    /// Read an escaped, null-terminated byte string from the input stream.
    pub fn read_escaped_bytes<R: Read>(reader: &mut BytePeeker<R>) -> io::Result<Vec<u8>> {
        let mut out = vec![];
        loop {
            let byte = reader.read_u8()?;
            if byte == TERMINATOR_BYTE {
                if let Some(ESCAPE_BYTE) = reader.peek()? {
                    reader.read_u8()?;
                    out.push(TERMINATOR_BYTE);
                } else {
                    break;
                }
            } else {
                out.push(byte);
            }
        }
        Ok(out)
    }

    impl ConvexValue {
        /// Parse a `Value` from a sort key.
        pub fn read_sort_key<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
            Self::_read_sort_key(&mut BytePeeker::new(reader))
        }

        fn _read_sort_key<R: Read>(reader: &mut BytePeeker<R>) -> anyhow::Result<Self> {
            let tag = reader.read_u8()?;
            let r = match tag {
                NULL_TAG => Self::Null,

                ZERO_INT64_TAG => Self::from(0),
                NEG_INT64_8_BYTE_TAG..=POS_INT64_8_BYTE_TAG => {
                    ConvexValue::from(read_tagged_int(tag, reader)?)
                },
                FLOAT64_TAG => {
                    let mut n = reader.read_u64::<BigEndian>()?;
                    // If the sign bit was set, just turn off the sign bit.
                    if n & (1 << 63) != 0 {
                        n &= !(1 << 63);
                    }
                    // Otherwise, flip all of the bits.
                    else {
                        n = !n;
                    }
                    ConvexValue::from(f64::from_bits(n))
                },

                FALSE_BOOLEAN_TAG => ConvexValue::from(false),
                TRUE_BOOLEAN_TAG => ConvexValue::from(true),

                STRING_TAG => {
                    let s = read_escaped_string(reader)?;
                    ConvexValue::try_from(s)?
                },
                BYTES_TAG => {
                    let b = read_escaped_bytes(reader)?;
                    ConvexValue::try_from(b)?
                },

                ARRAY_TAG => {
                    let mut elements = vec![];
                    read_terminated(reader, |reader| {
                        elements.push(Self::_read_sort_key(reader)?);
                        Ok(())
                    })?;
                    ConvexValue::Array(elements.try_into()?)
                },
                OBJECT_TAG => {
                    let mut elements = BTreeMap::new();
                    read_terminated(reader, |reader| {
                        let field = read_escaped_string(reader)?.parse()?;
                        let value = Self::_read_sort_key(reader)?;
                        if elements.insert(field, value).is_some() {
                            anyhow::bail!("Duplicate element in encoded object");
                        }
                        Ok(())
                    })?;
                    ConvexValue::Object(ConvexObject::try_from(elements)?)
                },

                ESCAPE_BYTE => bail!("Escape code used as tag"),
                _ => bail!("Unrecognized tag: {}", tag),
            };
            Ok(r)
        }
    }
}

impl ConvexValue {
    /// Generate the sort key for a given `Value`.
    pub fn sort_key(&self) -> Vec<u8> {
        let mut out = vec![];
        self.write_sort_key(&mut out);
        out
    }

    pub fn write_sort_key(&self, writer: &mut impl BufMut) {
        let Ok(()) = write_sort_key(self, writer);
    }
}

/// Write a `Value`'s sort key out to a writer.
pub fn write_sort_key<V: ConvexValueWalker>(
    value: V,
    writer: &mut impl BufMut,
) -> Result<(), V::Error> {
    match value.walk()? {
        ConvexValueType::Null => {
            writer.put_u8(NULL_TAG);
        },
        ConvexValueType::Int64(0) => {
            writer.put_u8(ZERO_INT64_TAG);
        },
        ConvexValueType::Int64(i) => {
            write_tagged_int(i, writer);
        },
        ConvexValueType::Float64(f) => {
            let mut f = f.to_bits();
            // Flip all of the bits if the sign bit is set.
            if f & (1 << 63) != 0 {
                f = !f;
            }
            // Otherwise, just flip the sign bit.
            else {
                f |= 1 << 63;
            }
            writer.put_u8(FLOAT64_TAG);
            writer.put_u64(f); // N.B.: always big-endian
        },
        ConvexValueType::Boolean(false) => {
            writer.put_u8(FALSE_BOOLEAN_TAG);
        },
        ConvexValueType::Boolean(true) => {
            writer.put_u8(TRUE_BOOLEAN_TAG);
        },
        ConvexValueType::String(s) => {
            writer.put_u8(STRING_TAG);
            write_escaped_string(s.as_str(), writer);
        },
        ConvexValueType::Bytes(b) => {
            writer.put_u8(BYTES_TAG);
            write_escaped_bytes(b.as_bytes(), writer);
        },
        ConvexValueType::Array(array) => {
            writer.put_u8(ARRAY_TAG);
            for element in array.walk() {
                write_sort_key(element?, writer)?;
            }
            writer.put_u8(TERMINATOR_BYTE);
        },
        ConvexValueType::Object(object) => {
            writer.put_u8(OBJECT_TAG);
            for pair in object.walk() {
                let (field, value) = pair?;
                write_escaped_string(field.as_str(), writer);
                write_sort_key(value, writer)?;
            }
            writer.put_u8(TERMINATOR_BYTE);
        },
    }
    Ok(())
}

/// Writes `value`'s sort key, interpreting `None` as `undefined` (which is,
/// notably, a different value than `null`)
pub fn write_sort_key_or_undefined<V: ConvexValueWalker>(
    value: Option<V>,
    writer: &mut impl BufMut,
) -> Result<(), V::Error> {
    match value {
        Some(value) => write_sort_key(value, writer),
        None => {
            writer.put_u8(UNDEFINED_TAG);
            Ok(())
        },
    }
}

// Manual implementation of `Ord` that is proptested to be equivalent to
// comparing sort keys.
impl Ord for ConvexValue {
    fn cmp(&self, other: &Self) -> Ordering {
        // This function is structured to make it hard to add another variant without
        // adding its case here. To that end, we avoid using wildcard matches.
        fn type_tag(v: &ConvexValue) -> usize {
            match v {
                ConvexValue::Null => 1,
                ConvexValue::Int64(..) => 2,
                ConvexValue::Float64(..) => 3,
                ConvexValue::Boolean(..) => 4,
                ConvexValue::String(..) => 5,
                ConvexValue::Bytes(..) => 6,
                ConvexValue::Array(..) => 7,
                ConvexValue::Object(..) => 10,
            }
        }
        let tag_cmp = type_tag(self).cmp(&type_tag(other));
        if !tag_cmp.is_eq() {
            return tag_cmp;
        }
        match self {
            ConvexValue::Null => {
                let ConvexValue::Null = other else {
                    panic!("Invalid value: {other:?}");
                };
                Ordering::Equal
            },
            ConvexValue::Int64(self_) => {
                let ConvexValue::Int64(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.cmp(other_)
            },
            ConvexValue::Float64(self_) => {
                let ConvexValue::Float64(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.total_cmp(other_)
            },
            ConvexValue::Boolean(self_) => {
                let ConvexValue::Boolean(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.cmp(other_)
            },
            ConvexValue::String(self_) => {
                let ConvexValue::String(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.cmp(other_)
            },
            ConvexValue::Bytes(self_) => {
                let ConvexValue::Bytes(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.cmp(other_)
            },
            ConvexValue::Array(self_) => {
                let ConvexValue::Array(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.cmp(other_)
            },
            ConvexValue::Object(self_) => {
                let ConvexValue::Object(other_) = other else {
                    panic!("Invalid value: {other:?}");
                };
                self_.cmp(other_)
            },
        }
    }
}

impl PartialOrd for ConvexValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ConvexValue {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ConvexValue {}

#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TotalOrdF64(f64);

impl Ord for TotalOrdF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl PartialOrd for TotalOrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for TotalOrdF64 {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Equal)
    }
}
impl Eq for TotalOrdF64 {}
impl From<TotalOrdF64> for ConvexValue {
    fn from(f: TotalOrdF64) -> ConvexValue {
        ConvexValue::from(f.0)
    }
}
impl From<f64> for TotalOrdF64 {
    fn from(f: f64) -> Self {
        Self(f)
    }
}

impl From<TotalOrdF64> for f64 {
    fn from(f: TotalOrdF64) -> f64 {
        f.0
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fmt::Debug,
    };

    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use crate::{
        id_v6::DeveloperDocumentId,
        sorting::{
            sorting_decode::bytes_to_values,
            TotalOrdF64,
        },
        values_to_bytes,
        ConvexArray,
        ConvexBytes,
        ConvexObject,
        ConvexString,
        ConvexValue,
        InternalId,
        ResolvedDocumentId,
        TableNumber,
        TabletId,
    };

    #[test]
    fn test_roundtrip_trophies() -> anyhow::Result<()> {
        // The random portion of this ID starts with the 0xFF byte which
        // used to break our sorting serialization.
        let id_str = "074wwt1x3qmwz35bvscy44eq2yngrt8";
        let id = DeveloperDocumentId::decode(id_str)?;
        let trophies = vec![ConvexValue::from(-1), ConvexValue::from(id)];
        for v in trophies {
            assert_eq!(
                ConvexValue::read_sort_key(&mut &v.sort_key()[..]).unwrap(),
                v
            );
        }
        Ok(())
    }

    fn test_compatible_with_ord<F: Ord + TryInto<ConvexValue>>(l: F, r: F)
    where
        <F as TryInto<ConvexValue>>::Error: Debug,
    {
        let ord1 = l.cmp(&r);

        let lv: ConvexValue = l.try_into().unwrap();
        let rv: ConvexValue = r.try_into().unwrap();

        let ord2 = lv.sort_key().cmp(&rv.sort_key());
        assert_eq!(ord1, ord2);
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_roundtrips(v in any::<ConvexValue>(),) {
            assert_eq!(ConvexValue::read_sort_key(&mut &v.sort_key()[..], ).unwrap(), v);
        }

        #[test]
        fn test_vector_roundtrips(v in any::<Vec<ConvexValue>>()) {
            let values: Vec<_> = v.clone().into_iter().map(Some).collect();
            let bytes = values_to_bytes(&values);
            assert_eq!(
                bytes_to_values(&mut &bytes[..]).unwrap(),
                v.into_iter().map(Some).collect::<Vec<_>>(),
            );
        }

        #[test]
        fn test_integer_roundtrips(v in any::<i64>()) {
            let v = ConvexValue::from(v);
            assert_eq!(ConvexValue::read_sort_key(&mut &v.sort_key()[..]).unwrap(), v);
        }

        #[test]
        fn test_id_roundtrips(v in any::<DeveloperDocumentId>()) {
            let v: ConvexValue = v.into();
            assert_eq!(ConvexValue::read_sort_key(&mut &v.sort_key()[..]).unwrap(), v);
        }


        #[test]
        fn test_compatible_with_manual_impl(
            l in any::<ConvexValue>(),
            r in any::<ConvexValue>(),
        ) {
            let ord1 = l.cmp(&r);
            let ord2 = l.sort_key().cmp(&r.sort_key());
            assert_eq!(ord1, ord2);
        }

        #[test]
        fn test_compatible_with_float(l in any::<f64>(), r in any::<f64>()) {
            test_compatible_with_ord(TotalOrdF64(l), TotalOrdF64(r));
        }

        #[test]
        fn test_compatible_with_bool(l in any::<bool>(), r in any::<bool>())  {
            test_compatible_with_ord(l, r)
        }

        #[test]
        fn test_compatible_with_str(l in any::<ConvexString>(), r in any::<ConvexString>())  {
            test_compatible_with_ord(String::from(l), String::from(r))
        }

        #[test]
        fn test_compatible_with_bytes(l in any::<ConvexBytes>(), r in any::<ConvexBytes>())  {
            test_compatible_with_ord(Vec::from(l), Vec::from(r))
        }

        #[test]
        fn test_compatible_with_id_string(
            l in any::<DeveloperDocumentId>(),
            r in any::<DeveloperDocumentId>(),
        )  {
            test_compatible_with_ord(l.encode(), r.encode())
        }

        #[test]
        fn test_compatible_with_internal_id(l in any::<InternalId>(), r in any::<InternalId>())  {
            let tablet_id = TabletId::MIN;
            let table_number = TableNumber::MIN;
            let l = ResolvedDocumentId {
                tablet_id,
                developer_id: DeveloperDocumentId::new(table_number, l),
            };
            let r = ResolvedDocumentId {
                tablet_id,
                developer_id: DeveloperDocumentId::new(table_number, r),
            };
            test_compatible_with_ord(ConvexValue::from(l), ConvexValue::from(r))
        }

    }

    proptest! {
        #![proptest_config(
            ProptestConfig {
                cases: 16 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
                failure_persistence: None,
                .. ProptestConfig::default()
            }
        )]

        #[test]
        fn test_compatible_with_arr(l in any::<ConvexArray>(), r in any::<ConvexArray>())  {
            test_compatible_with_ord(Vec::from(l), Vec::from(r))
        }

        #[test]
        fn test_compatible_with_object(
            l in any::<ConvexObject>(),
            r in any::<ConvexObject>(),
        )  {
            test_compatible_with_ord(BTreeMap::from(l), BTreeMap::from(r))
        }
    }
}
