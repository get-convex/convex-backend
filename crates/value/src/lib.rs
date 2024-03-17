#![feature(try_blocks)]
#![feature(type_alias_impl_trait)]
#![feature(iter_from_coroutine)]
#![feature(iterator_try_collect)]
#![feature(coroutines)]
#![feature(lazy_cell)]
#![feature(const_trait_impl)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![feature(impl_trait_in_assoc_type)]

mod array;
pub mod base32;
pub mod base64;
mod bytes;
mod document_id;
pub mod export;
mod field_name;
mod field_path;
pub mod id_v6;
mod json;
mod map;
mod metrics;
pub mod numeric;
mod object;
pub mod serde;
mod set;
pub mod sha256;
mod size;
pub mod sorting;
mod string;
mod table_mapping;
mod table_name;
mod virtual_table_mapping;

// Helper modules we'll eventually factor out.
pub mod heap_size;
pub mod utils;

mod macros;
#[cfg(test)]
mod tests;

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    convert::{
        TryFrom,
        TryInto,
    },
    fmt::{
        self,
        Display,
    },
    hash::{
        Hash,
        Hasher,
    },
    string::String,
};

use anyhow::{
    bail,
    Error,
};
use heap_size::HeapSize;
use id_v6::DocumentIdV6;
pub use sync_types::identifier;

pub use crate::{
    array::ConvexArray,
    bytes::ConvexBytes,
    document_id::{
        DeveloperDocumentId,
        GenericDocumentId,
        InternalDocumentId,
        InternalId,
        ResolvedDocumentId,
    },
    field_name::{
        FieldName,
        FieldType,
        IdentifierFieldName,
    },
    field_path::FieldPath,
    json::{
        bytes::JsonBytes,
        float::JsonFloat,
        integer::JsonInteger,
        json_deserialize,
        json_serialize,
    },
    map::ConvexMap,
    object::{
        remove_boolean,
        remove_int64,
        remove_nullable_object,
        remove_nullable_string,
        remove_object,
        remove_string,
        remove_vec,
        remove_vec_of_strings,
        ConvexObject,
        MAX_OBJECT_FIELDS,
    },
    set::ConvexSet,
    size::{
        check_nesting_for_documents,
        check_user_size,
        Size,
        MAX_DOCUMENT_NESTING,
        MAX_NESTING,
        MAX_SIZE,
        MAX_USER_SIZE,
        VALUE_TOO_LARGE_SHORT_MSG,
    },
    sorting::values_to_bytes,
    string::ConvexString,
    table_mapping::{
        TableMapping,
        TableMappingValue,
    },
    table_name::{
        TableId,
        TableIdAndTableNumber,
        TableIdentifier,
        TableName,
        TableNumber,
        TableType,
        METADATA_PREFIX,
    },
    virtual_table_mapping::VirtualTableMapping,
};

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    pub use sync_types::testing::assert_roundtrips;
}

/// The various types that can be stored as a field in an [`Object`].
#[derive(Clone, Debug)]
pub enum ConvexValue {
    /// Sentinel `Null` value.
    Null,

    /// 64-bit signed integer.
    Int64(i64),

    /// IEEE754 double-precision floating point number with NaNs, negative zero,
    /// and subnormal numbers supported.
    Float64(f64),

    /// Boolean value.
    Boolean(bool),

    /// Text strings, represented as a sequence of Unicode scalar values. Scalar
    /// values must be encoded as codepoints without surrogate pairs: The
    /// sequence of codepoints may not contain *any* surrogate pairs, even
    /// if they are correctly paired up.
    String(ConvexString),

    /// Arbitrary binary data.
    Bytes(ConvexBytes),

    /// Arrays of (potentially heterogeneous) [`Value`]s.
    Array(ConvexArray),

    /// Set of (potentially heterogeneous) [`Value`]s.
    Set(ConvexSet),

    /// Map of (potentially heterogenous) keys and values.
    Map(ConvexMap),

    /// Nested object with [`FieldName`] keys and (potentially heterogenous)
    /// values.
    Object(ConvexObject),
}

impl ConvexValue {
    /// Returns a string description of the type of this Value.
    pub fn type_name(&self) -> &'static str {
        match self {
            ConvexValue::Null => "Null",
            ConvexValue::Int64(_) => "Int64",
            ConvexValue::Float64(_) => "Float64",
            ConvexValue::Boolean(_) => "Boolean",
            ConvexValue::String(_) => "String",
            ConvexValue::Bytes(_) => "Bytes",
            ConvexValue::Array(_) => "Array",
            ConvexValue::Set(_) => "Set",
            ConvexValue::Map(_) => "Map",
            ConvexValue::Object(_) => "Object",
        }
    }
}

impl From<ConvexObject> for ConvexValue {
    fn from(o: ConvexObject) -> ConvexValue {
        ConvexValue::Object(o)
    }
}

impl From<ResolvedDocumentId> for ConvexValue {
    fn from(value: ResolvedDocumentId) -> Self {
        DocumentIdV6::try_from(value)
            .expect("Could not create IDV6 from DocumentId")
            .into()
    }
}

impl From<DocumentIdV6> for ConvexValue {
    fn from(value: DocumentIdV6) -> Self {
        ConvexValue::String(
            value
                .encode()
                .try_into()
                .expect("Could not create Value::String for ID string"),
        )
    }
}

impl TryFrom<ConvexValue> for DocumentIdV6 {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        if let ConvexValue::String(s) = value {
            DocumentIdV6::decode(&s).map_err(|e| anyhow::anyhow!(e))
        } else {
            Err(anyhow::anyhow!("Value is not an ID"))
        }
    }
}

impl From<i64> for ConvexValue {
    fn from(i: i64) -> Self {
        Self::Int64(i)
    }
}

impl From<f64> for ConvexValue {
    fn from(i: f64) -> Self {
        Self::Float64(i)
    }
}

impl From<bool> for ConvexValue {
    fn from(i: bool) -> Self {
        Self::Boolean(i)
    }
}

impl TryFrom<String> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: String) -> anyhow::Result<Self> {
        Ok(Self::String(ConvexString::try_from(i)?))
    }
}

impl<'a> TryFrom<&'a str> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: &'a str) -> anyhow::Result<Self> {
        Ok(Self::String(i.to_owned().try_into()?))
    }
}

impl TryFrom<Vec<u8>> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: Vec<u8>) -> anyhow::Result<Self> {
        Ok(Self::Bytes(ConvexBytes::try_from(i)?))
    }
}

impl TryFrom<Vec<ConvexValue>> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: Vec<ConvexValue>) -> anyhow::Result<Self> {
        Ok(Self::Array(i.try_into()?))
    }
}

impl TryFrom<BTreeSet<ConvexValue>> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: BTreeSet<ConvexValue>) -> anyhow::Result<Self> {
        Ok(Self::Set(i.try_into()?))
    }
}

impl TryFrom<BTreeMap<ConvexValue, ConvexValue>> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: BTreeMap<ConvexValue, ConvexValue>) -> anyhow::Result<Self> {
        Ok(Self::Map(i.try_into()?))
    }
}

impl TryFrom<BTreeMap<FieldName, ConvexValue>> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(i: BTreeMap<FieldName, ConvexValue>) -> anyhow::Result<Self> {
        Ok(Self::Object(i.try_into()?))
    }
}

impl TryFrom<ConvexValue> for bool {
    type Error = Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::Boolean(b) => Ok(b),
            _ => bail!("Value must be a Boolean"),
        }
    }
}

impl TryFrom<ConvexValue> for i64 {
    type Error = Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::Int64(i) => Ok(i),
            _ => bail!("Value must be an integer"),
        }
    }
}

impl TryFrom<ConvexValue> for ConvexString {
    type Error = Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::String(s) => Ok(s),
            _ => bail!("Value must be a string"),
        }
    }
}

impl TryFrom<ConvexValue> for String {
    type Error = Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        Ok(ConvexString::try_from(v)?.into())
    }
}

impl TryFrom<ConvexValue> for ConvexArray {
    type Error = Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::Array(a) => Ok(a),
            _ => bail!("Value must be an Array"),
        }
    }
}

impl TryFrom<ConvexValue> for ConvexObject {
    type Error = Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::Object(o) => Ok(o),
            _ => bail!("Value must be an Object"),
        }
    }
}

impl Display for ConvexValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConvexValue::Null => write!(f, "null"),
            ConvexValue::Int64(n) => write!(f, "{}", n),
            ConvexValue::Float64(n) => write!(f, "{:?}", n),
            ConvexValue::Boolean(b) => write!(f, "{:?}", b),
            ConvexValue::String(s) => write!(f, "{:?}", s),
            ConvexValue::Bytes(b) => write!(f, "{}", b),
            ConvexValue::Array(arr) => write!(f, "{}", arr),
            ConvexValue::Set(set) => write!(f, "{}", set),
            ConvexValue::Map(map) => write!(f, "{}", map),
            ConvexValue::Object(m) => write!(f, "{}", m),
        }
    }
}

impl Size for ConvexValue {
    fn size(&self) -> usize {
        match self {
            ConvexValue::Null => 1,
            ConvexValue::Int64(_) => 1 + 8,
            ConvexValue::Float64(_) => 1 + 8,
            ConvexValue::Boolean(_) => 1,
            ConvexValue::String(s) => s.size(),
            ConvexValue::Bytes(b) => b.size(),
            ConvexValue::Array(arr) => arr.size(),
            ConvexValue::Set(set) => set.size(),
            ConvexValue::Map(map) => map.size(),
            ConvexValue::Object(m) => m.size(),
        }
    }

    fn nesting(&self) -> usize {
        match self {
            ConvexValue::Null => 0,
            ConvexValue::Int64(_) => 0,
            ConvexValue::Float64(_) => 0,
            ConvexValue::Boolean(_) => 0,
            ConvexValue::String(_) => 0,
            ConvexValue::Bytes(_) => 0,
            ConvexValue::Array(arr) => arr.nesting(),
            ConvexValue::Set(set) => set.nesting(),
            ConvexValue::Map(map) => map.nesting(),
            ConvexValue::Object(m) => m.nesting(),
        }
    }
}

impl HeapSize for ConvexValue {
    fn heap_size(&self) -> usize {
        match self {
            ConvexValue::Null => 0,
            ConvexValue::Int64(_) => 0,
            ConvexValue::Float64(_) => 0,
            ConvexValue::Boolean(_) => 0,
            ConvexValue::String(s) => s.heap_size(),
            ConvexValue::Bytes(b) => b.heap_size(),
            ConvexValue::Array(arr) => arr.heap_size(),
            ConvexValue::Set(set) => set.heap_size(),
            ConvexValue::Map(map) => map.heap_size(),
            ConvexValue::Object(m) => m.heap_size(),
        }
    }
}

/// Encode to bytes that can be hashed for quick equality checks, where
/// if values compare as equal with Eq, their v.encode_for_hash() will match.
/// They are not portable, so don't store them durably. They are not ordered
/// the same as impl PartialOrd.
pub mod encode_for_hash {
    use std::io::{
        self,
        Write,
    };

    use byteorder::WriteBytesExt;

    use crate::{
        sorting::write_escaped_bytes,
        ConvexObject,
        ConvexValue,
    };

    impl ConvexValue {
        pub fn encode_for_hash<W: Write>(&self, w: &mut W) -> io::Result<()> {
            match self {
                ConvexValue::Null => {
                    w.write_u8(2)?;
                },
                ConvexValue::Int64(i) => {
                    w.write_u8(3)?;
                    write_escaped_bytes(&i.to_le_bytes(), w)?;
                },
                ConvexValue::Float64(f) => {
                    w.write_u8(4)?;
                    write_escaped_bytes(&f.to_le_bytes(), w)?;
                },
                ConvexValue::Boolean(b) => {
                    w.write_u8(5)?;
                    write_escaped_bytes(&[*b as u8], w)?;
                },
                ConvexValue::String(s) => {
                    w.write_u8(6)?;
                    write_escaped_bytes(s.as_bytes(), w)?;
                },
                ConvexValue::Bytes(b) => {
                    w.write_u8(7)?;
                    write_escaped_bytes(b, w)?;
                },
                ConvexValue::Array(a) => {
                    w.write_u8(8)?;
                    for v in a {
                        v.encode_for_hash(w)?;
                    }
                },
                ConvexValue::Set(s) => {
                    w.write_u8(9)?;
                    for v in s {
                        v.encode_for_hash(w)?;
                    }
                },
                ConvexValue::Map(m) => {
                    w.write_u8(10)?;
                    for (k, v) in m {
                        k.encode_for_hash(w)?;
                        v.encode_for_hash(w)?;
                    }
                },
                ConvexValue::Object(o) => {
                    w.write_u8(11)?;
                    o.encode_for_hash(w)?;
                },
            }
            Ok(())
        }
    }

    impl ConvexObject {
        pub fn encode_for_hash<W: Write>(&self, w: &mut W) -> io::Result<()> {
            for (k, v) in self.iter() {
                write_escaped_bytes(k.as_bytes(), w)?;
                v.encode_for_hash(w)?;
            }
            Ok(())
        }
    }
}

impl Hash for ConvexValue {
    /// f64 doesn't implement `hash` so we need to manually implement `hash` for
    /// `Value`. Must be compatible with our manual implementation of `cmp`.
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        let mut bytes = vec![];
        self.encode_for_hash(&mut bytes)
            .expect("failed to write to memory");
        bytes.hash(hasher)
    }
}

impl Hash for ConvexObject {
    /// f64 doesn't implement `hash` so we need to manually implement `hash` for
    /// `ConvexObject`. Must be compatible with our manual implementation of
    /// `cmp`.
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        let mut bytes = vec![];
        self.encode_for_hash(&mut bytes)
            .expect("failed to write to memory");
        bytes.hash(hasher)
    }
}

#[cfg(test)]
mod hash_tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use crate::ConvexValue;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1024 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn proptest_value_encode_for_hash(
            v1 in any::<ConvexValue>(),
            v2 in any::<ConvexValue>(),
        ) {
            let mut v1_encoded = vec![];
            let mut v2_encoded = vec![];
            v1.encode_for_hash(&mut v1_encoded).unwrap();
            v2.encode_for_hash(&mut v2_encoded).unwrap();
            assert_eq!(
                v1 == v2,
                v1_encoded == v2_encoded
            );
        }
    }
}

pub trait Namespace {
    fn is_system(&self) -> bool;
}

#[cfg(any(test, feature = "testing"))]
pub mod proptest {
    use proptest::prelude::*;

    use super::{
        bytes::ConvexBytes,
        string::ConvexString,
        ConvexValue,
    };
    use crate::field_name::FieldName;

    impl Arbitrary for ConvexValue {
        type Parameters = (<FieldName as Arbitrary>::Parameters, ExcludeSetsAndMaps);

        type Strategy = impl Strategy<Value = ConvexValue>;

        fn arbitrary_with(
            (field_params, exclude_sets_and_maps): Self::Parameters,
        ) -> Self::Strategy {
            resolved_value_strategy(
                move || any_with::<FieldName>(field_params),
                4,
                32,
                8,
                exclude_sets_and_maps,
            )
        }
    }

    pub fn float64_strategy() -> impl Strategy<Value = f64> {
        prop::num::f64::ANY | prop::num::f64::SIGNALING_NAN
    }

    #[derive(Default)] // default to include sets and maps
    pub struct ExcludeSetsAndMaps(pub bool);

    pub fn resolved_value_strategy<F, S>(
        field_strategy: F,
        depth: usize,
        node_target: usize,
        branching: usize,
        exclude_sets_and_maps: ExcludeSetsAndMaps,
    ) -> impl Strategy<Value = ConvexValue>
    where
        F: Fn() -> S + 'static,
        S: Strategy<Value = FieldName> + 'static,
    {
        use crate::{
            id_v6::DocumentIdV6,
            resolved_object_strategy,
            ConvexArray,
            ConvexMap,
            ConvexSet,
        };

        // https://altsysrq.github.io/proptest-book/proptest/tutorial/recursive.html
        let leaf = prop_oneof![
            1 => any::<DocumentIdV6>()
                .prop_map(|id| {
                    let s = id.encode().try_into().expect("Could not create String value from ID");
                    ConvexValue::String(s)
                }),
            1 => Just(ConvexValue::Null),
            1 => any::<i64>().prop_map(ConvexValue::from),
            1 => (prop::num::f64::ANY | prop::num::f64::SIGNALING_NAN)
                .prop_map(ConvexValue::from),
            1 => any::<bool>().prop_map(ConvexValue::from),
            1 => any::<ConvexString>().prop_filter_map("String ID", |s| match DocumentIdV6::decode(&s) {
                Ok(_) => None,
                Err(_) => Some(ConvexValue::String(s))
            }),
            1 => any::<ConvexBytes>().prop_map(ConvexValue::Bytes),
        ];
        let map_set_weight = if exclude_sets_and_maps.0 { 0 } else { 1 };
        leaf.prop_recursive(
            depth as u32,
            node_target as u32,
            branching as u32,
            move |inner| {
                prop_oneof![
                    // Manually create the strategies here rather than using the `Arbitrary`
                    // implementations on `Array`, etc. This lets us explicitly pass `inner`
                    // through rather than starting the `Value` strategy from
                    // scratch at each tree level.
                    1 => prop::collection::vec(inner.clone(), 0..branching)
                        .prop_filter_map("Vec wasn't a valid Convex value", |v| {
                            ConvexArray::try_from(v).ok()
                        })
                        .prop_map(ConvexValue::Array),
                    map_set_weight => prop::collection::btree_set(inner.clone(), 0..branching)
                        .prop_filter_map("BTreeSet wasn't a valid Convex value", |v| {
                            ConvexSet::try_from(v).ok()
                        })
                        .prop_map(ConvexValue::Set),
                    map_set_weight => prop::collection::btree_map(
                        inner.clone(),
                        inner.clone(),
                        0..branching,
                    )
                        .prop_filter_map("BTreeMap wasn't a valid Convex value", |v| {
                            ConvexMap::try_from(v).ok()
                        })
                        .prop_map(ConvexValue::Map),
                    1 => resolved_object_strategy(field_strategy(), inner, 0..branching)
                        .prop_map(ConvexValue::Object),
                ]
            },
        )
    }
}
#[cfg(any(test, feature = "testing"))]
pub use self::{
    object::resolved_object_strategy,
    proptest::resolved_value_strategy,
    proptest::ExcludeSetsAndMaps,
};
