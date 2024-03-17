use std::{
    collections::HashSet,
    convert::TryFrom,
    fmt::Display,
    ops::Deref,
};

use pb::convex_token::FieldPath as FieldPathProto;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    utils::display_sequence,
    ConvexValue,
};

use crate::{
    bootstrap_model::index::{
        index_validation_error,
        MAX_INDEX_FIELDS_SIZE,
    },
    document::{
        CREATION_TIME_FIELD,
        ID_FIELD_PATH,
    },
    paths::FieldPath,
};

/// Ordered list of fields in a multi-column index. This list only contains
/// the user-specified indexes: the system adds the `_id` column at the
/// end to guarantee uniqueness, but this trailing `_id` field isn't
/// included in this type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexedFields(WithHeapSize<Vec<FieldPath>>);

impl IndexedFields {
    pub fn by_id() -> Self {
        IndexedFields(vec![].into())
    }

    pub fn creation_time() -> Self {
        let field_path = FieldPath::new(vec![CREATION_TIME_FIELD.to_owned()])
            .expect("Invalid _creationTime field path");
        IndexedFields(vec![field_path].into())
    }
}

impl HeapSize for IndexedFields {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl Display for IndexedFields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_sequence(f, ["[", "]"], self.0.iter())
    }
}

impl Deref for IndexedFields {
    type Target = Vec<FieldPath>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Vec<FieldPath>> for IndexedFields {
    type Error = anyhow::Error;

    fn try_from(fields: Vec<FieldPath>) -> anyhow::Result<Self> {
        if fields.len() > MAX_INDEX_FIELDS_SIZE {
            anyhow::bail!(index_validation_error::too_many_fields(
                MAX_INDEX_FIELDS_SIZE
            ));
        }

        if fields.contains(&ID_FIELD_PATH) {
            anyhow::bail!(index_validation_error::fields_contain_id())
        }

        let mut seen: HashSet<_> = HashSet::new();
        for field in fields.iter() {
            if !seen.insert(field.clone()) {
                anyhow::bail!(index_validation_error::fields_not_unique_within_index(
                    field
                ));
            }
        }
        Ok(Self(fields.into()))
    }
}

impl From<IndexedFields> for Vec<FieldPath> {
    fn from(fields: IndexedFields) -> Self {
        fields.0.into()
    }
}

impl TryFrom<IndexedFields> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(fields: IndexedFields) -> anyhow::Result<Self> {
        let vec: Vec<_> = fields.0.into();
        vec.try_into()
    }
}

impl TryFrom<ConvexValue> for IndexedFields {
    type Error = anyhow::Error;

    fn try_from(val: ConvexValue) -> anyhow::Result<Self> {
        if let ConvexValue::Array(arr) = val {
            let fields: Vec<FieldPath> = arr
                .iter()
                .cloned()
                .map(FieldPath::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(IndexedFields(fields.into()))
        } else {
            anyhow::bail!("Invalid value for IndexedFields")
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for IndexedFields {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = IndexedFields>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        // Use collection::hash_set to ensure that the fields in the index are unique.
        // Filter out `_id` - because those aren't allowed in indexes. Surprisingly,
        // proptest does randomly generate `_id` once in a while.
        prop::collection::hash_set(
            any::<FieldPath>()
                .prop_filter("_id not allowed in index", |path| path != &*ID_FIELD_PATH),
            1..8,
        )
        .prop_filter_map("Invalid IndexedFields", |set| {
            IndexedFields::try_from(set.into_iter().collect::<Vec<_>>()).ok()
        })
    }
}

impl From<IndexedFields> for Vec<FieldPathProto> {
    fn from(fields: IndexedFields) -> Self {
        Vec::<FieldPath>::from(fields)
            .into_iter()
            .map(|f| f.into())
            .collect()
    }
}
