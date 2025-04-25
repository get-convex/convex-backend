#![feature(try_blocks)]
#![feature(never_type)]
#![feature(type_alias_impl_trait)]
#![feature(iter_from_coroutine)]
#![feature(iterator_try_collect)]
#![feature(coroutines)]
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
pub mod walk;

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
    fmt::{
        self,
        Display,
    },
    hash::{
        Hash,
        Hasher,
    },
};

use anyhow::{
    bail,
    Error,
};
use heap_size::HeapSize;
pub use paste::paste;
pub use sync_types::identifier;
use walk::ConvexValueWalker;

pub use crate::{
    array::ConvexArray,
    bytes::ConvexBytes,
    document_id::{
        DeveloperDocumentId,
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
        json_packed_value::JsonPackedValue,
        object as json_object,
        value as json_value,
    },
    map::ConvexMap,
    object::{
        remove_boolean,
        remove_int64,
        remove_nullable_int64,
        remove_nullable_object,
        remove_nullable_string,
        remove_nullable_vec,
        remove_nullable_vec_of_strings,
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
        NamespacedTableMapping,
        TableMapping,
        TableMappingValue,
        TableNamespace,
    },
    table_name::{
        TableName,
        TableNumber,
        TableType,
        TabletId,
        TabletIdAndTableNumber,
        METADATA_PREFIX,
    },
};

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    pub use sync_types::testing::assert_roundtrips;
}

/// The various types that can be stored as a field in a [`ConvexObject`].
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

    /// Arrays of (potentially heterogeneous) [`ConvexValue`]s.
    Array(ConvexArray),

    /// Set of (potentially heterogeneous) [`ConvexValue`]s.
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
        let Ok(ty) = self.walk();
        ty.type_name()
    }
}

impl From<ConvexObject> for ConvexValue {
    fn from(o: ConvexObject) -> ConvexValue {
        ConvexValue::Object(o)
    }
}

impl From<ResolvedDocumentId> for ConvexValue {
    fn from(value: ResolvedDocumentId) -> Self {
        DeveloperDocumentId::from(value).into()
    }
}

impl From<DeveloperDocumentId> for ConvexValue {
    fn from(value: DeveloperDocumentId) -> Self {
        ConvexValue::String(
            value
                .encode()
                .try_into()
                .expect("Could not create Value::String for ID string"),
        )
    }
}

impl TryFrom<ConvexValue> for DeveloperDocumentId {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        if let ConvexValue::String(s) = value {
            DeveloperDocumentId::decode(&s).map_err(|e| anyhow::anyhow!(e))
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

impl<T: TryInto<ConvexValue, Error = anyhow::Error>> TryFrom<Option<T>> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(v: Option<T>) -> anyhow::Result<Self> {
        Ok(match v {
            None => ConvexValue::Null,
            Some(v) => v.try_into()?,
        })
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

/// f64 doesn't implement `hash` so we need to manually implement `hash` for
/// `ConvexValue`. Must be compatible with our manual implementation of `cmp`.
impl Hash for ConvexValue {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match self {
            ConvexValue::Null => {
                h.write_u8(2);
            },
            ConvexValue::Int64(i) => {
                h.write_u8(3);
                i.hash(h);
            },
            ConvexValue::Float64(f) => {
                h.write_u8(4);
                f.to_le_bytes().hash(h);
            },
            ConvexValue::Boolean(b) => {
                h.write_u8(5);
                b.hash(h);
            },
            ConvexValue::String(s) => {
                h.write_u8(6);
                s.hash(h);
            },
            ConvexValue::Bytes(b) => {
                h.write_u8(7);
                b.hash(h);
            },
            ConvexValue::Array(a) => {
                h.write_u8(8);
                a.hash(h);
            },
            ConvexValue::Set(s) => {
                h.write_u8(9);
                s.hash(h);
            },
            ConvexValue::Map(m) => {
                h.write_u8(10);
                m.hash(h);
            },
            ConvexValue::Object(o) => {
                h.write_u8(11);
                o.hash(h);
            },
        }
    }
}

#[cfg(test)]
mod hash_tests {
    use std::hash::{
        Hash,
        Hasher,
    };

    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use crate::ConvexValue;

    struct SaveHasher(Vec<u8>);
    impl Hasher for SaveHasher {
        fn finish(&self) -> u64 {
            unimplemented!()
        }

        fn write(&mut self, bytes: &[u8]) {
            self.0.extend_from_slice(bytes);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1024 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn proptest_value_encode_for_hash(
            v1 in any::<ConvexValue>(),
            v2 in any::<ConvexValue>(),
        ) {
            let mut v1_encoded = SaveHasher(vec![]);
            let mut v2_encoded = SaveHasher(vec![]);
            v1.hash(&mut v1_encoded);
            v2.hash(&mut v2_encoded);
            assert_eq!(
                v1 == v2,
                v1_encoded.0 == v2_encoded.0
            );
        }
    }
}

pub trait Namespace {
    fn is_system(&self) -> bool;
}

#[cfg(any(test, feature = "testing"))]
pub mod proptest {
    use core::f64;

    use proptest::prelude::*;

    use super::{
        bytes::ConvexBytes,
        string::ConvexString,
        ConvexValue,
    };
    use crate::field_name::FieldName;

    impl Arbitrary for ConvexValue {
        type Parameters = (
            <FieldName as Arbitrary>::Parameters,
            ValueBranching,
            ExcludeSetsAndMaps,
            RestrictNaNs,
        );

        type Strategy = impl Strategy<Value = ConvexValue>;

        fn arbitrary_with(
            (field_params, branching, exclude_sets_and_maps, restrict_nans): Self::Parameters,
        ) -> Self::Strategy {
            resolved_value_strategy(
                move || any_with::<FieldName>(field_params),
                branching,
                exclude_sets_and_maps,
                restrict_nans,
            )
        }
    }

    pub fn float64_strategy() -> impl Strategy<Value = f64> {
        prop::num::f64::ANY | prop::num::f64::SIGNALING_NAN
    }

    #[derive(Default)] // default to include sets and maps
    pub struct ExcludeSetsAndMaps(pub bool);

    // Whether to allow any `NaN` value (e.g. negative `NaN`s) or not.
    // Defaults to including any valid `NaN` value.
    #[derive(Default)]
    pub struct RestrictNaNs(pub bool);

    pub struct ValueBranching {
        pub depth: usize,
        pub node_target: usize,
        pub branching: usize,
    }

    impl ValueBranching {
        pub fn new(depth: usize, node_target: usize, branching: usize) -> Self {
            Self {
                depth,
                node_target,
                branching,
            }
        }

        pub fn small() -> Self {
            Self::new(4, 4, 4)
        }

        pub fn medium() -> Self {
            Self::new(4, 32, 8)
        }

        pub fn large() -> Self {
            Self::new(8, 64, 16)
        }
    }

    impl Default for ValueBranching {
        fn default() -> Self {
            Self::medium()
        }
    }

    pub fn resolved_value_strategy<F, S>(
        field_strategy: F,
        branching: ValueBranching,
        exclude_sets_and_maps: ExcludeSetsAndMaps,
        restrict_nans: RestrictNaNs,
    ) -> impl Strategy<Value = ConvexValue>
    where
        F: Fn() -> S + 'static,
        S: Strategy<Value = FieldName> + 'static,
    {
        use crate::{
            id_v6::DeveloperDocumentId,
            resolved_object_strategy,
            ConvexArray,
            ConvexMap,
            ConvexSet,
        };

        // https://altsysrq.github.io/proptest-book/proptest/tutorial/recursive.html
        let leaf = prop_oneof![
            1 => any::<DeveloperDocumentId>()
                .prop_map(|id| {
                    let s = id.encode().try_into().expect("Could not create String value from ID");
                    ConvexValue::String(s)
                }),
            1 => Just(ConvexValue::Null),
            1 => any::<i64>().prop_map(ConvexValue::from),
            1 => (prop::num::f64::ANY | prop::num::f64::SIGNALING_NAN)
                .prop_map(move |f| {
                    if restrict_nans.0 && f.is_nan() {
                        ConvexValue::Float64(f64::NAN)
                    } else {
                        ConvexValue::Float64(f)
                    }
                }),
            1 => any::<bool>().prop_map(ConvexValue::from),
            1 => any::<ConvexString>().prop_filter_map("String ID", |s| match DeveloperDocumentId::decode(&s) {
                Ok(_) => None,
                Err(_) => Some(ConvexValue::String(s))
            }),
            1 => any::<ConvexBytes>().prop_map(ConvexValue::Bytes),
        ];
        let map_set_weight = if exclude_sets_and_maps.0 { 0 } else { 1 };
        let ValueBranching {
            depth,
            node_target,
            branching,
        } = branching;
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
