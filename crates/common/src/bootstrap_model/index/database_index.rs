use std::{
    collections::{
        BTreeMap,
        HashSet,
    },
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
    obj,
    utils::display_sequence,
    ConvexObject,
    ConvexValue,
};

use super::MAX_INDEX_FIELDS_SIZE;
use crate::{
    bootstrap_model::index::index_validation_error,
    document::{
        CREATION_TIME_FIELD,
        ID_FIELD_PATH,
    },
    paths::FieldPath,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperDatabaseIndexConfig {
    /// Ordered field(s) to index. The "unindexed" primary key ordering of
    /// documents by [`DocumentId`] is represented by an empty vector.
    pub fields: IndexedFields,
}

/// Represents the state of an index.
/// Table scan index for a newly created table starts at `Enabled`. All
/// other indexes start at `Backfilling` state and are transitioned to
/// `Enabled` by the index backfill routine. Disabled indexes are not
/// implicitly transitioned to any other state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum DatabaseIndexState {
    // We are backfilling this index. All new writes should update the index.
    Backfilling(DatabaseIndexBackfillState),
    // The index is fully backfilled, but hasn't yet been committed and is not
    // yet available for reads.
    Backfilled,
    // Index is fully backfilled and ready to serve reads.
    Enabled,
}

impl TryFrom<DatabaseIndexState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(state: DatabaseIndexState) -> Result<Self, Self::Error> {
        match state {
            DatabaseIndexState::Backfilling(backfill_state) => obj!(
                "type" => "Backfilling",
                "backfillState" => ConvexValue::Object(backfill_state.try_into()?),
            ),
            DatabaseIndexState::Enabled => obj!("type" => "Enabled"),
            // Use Backfilled2 to distinguish between records impacted by CX-3897
            DatabaseIndexState::Backfilled => obj!("type" => "Backfilled2"),
        }
    }
}

impl TryFrom<ConvexObject> for DatabaseIndexState {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = o.into();

        let t = match object_fields.get("type") {
            Some(ConvexValue::String(s)) => s,
            Some(..) => {
                anyhow::bail!("Invalid `type` field for IndexState {:?}", object_fields)
            },
            None => anyhow::bail!("Missing `type` field for IndexState {:?}", object_fields),
        };

        match t.as_ref() {
            "Backfilling" => {
                let backfill_state = match object_fields.remove("backfillState") {
                    Some(ConvexValue::Object(backfill_state)) => backfill_state.try_into()?,
                    _ => anyhow::bail!(
                        "Missing or invalid backfill_state field for IndexState: {:?}",
                        object_fields
                    ),
                };
                Ok(DatabaseIndexState::Backfilling(backfill_state))
            },
            // We have historical records with Disabled state.
            "Disabled" => Ok(DatabaseIndexState::Backfilling(DatabaseIndexBackfillState)),
            "Backfilled2" => Ok(DatabaseIndexState::Backfilled),
            "Enabled" => Ok(DatabaseIndexState::Enabled),
            _ => anyhow::bail!("Invalid index type {}", t),
        }
    }
}

/// Represents state of currently backfilling index.
/// We currently do not checkpoint. Will extend the struct when we do.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DatabaseIndexBackfillState;

impl From<DatabaseIndexBackfillState> for ConvexObject {
    fn from(_state: DatabaseIndexBackfillState) -> Self {
        ConvexObject::empty()
    }
}

impl TryFrom<ConvexObject> for DatabaseIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        anyhow::ensure!(o.is_empty(), "Non-empty object {:?}", o);
        Ok(DatabaseIndexBackfillState)
    }
}

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

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use value::{
        obj,
        ConvexObject,
    };

    use super::*;

    #[test]
    fn test_backfilled_metadata_is_deserialized_as_backfilled() -> anyhow::Result<()> {
        let object: ConvexObject = obj!("type" => "Backfilled2")?;
        let index_state: DatabaseIndexState = object.try_into()?;
        assert_matches!(index_state, DatabaseIndexState::Backfilled);
        Ok(())
    }

    #[test]
    fn test_backfilled_metadata_is_serialized_as_backfilled() -> anyhow::Result<()> {
        let index_state = DatabaseIndexState::Backfilled;
        let object: ConvexObject = index_state.try_into()?;
        let index_state: DatabaseIndexState = object.try_into()?;
        assert_matches!(index_state, DatabaseIndexState::Backfilled);
        Ok(())
    }
}
