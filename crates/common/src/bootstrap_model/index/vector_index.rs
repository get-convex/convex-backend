use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    ops::Deref,
    str::FromStr,
};

use anyhow::Context;
use errors::ErrorMetadata;
use sync_types::Timestamp;
use value::{
    obj,
    ConvexObject,
    ConvexValue,
    FieldName,
    FieldPath,
    InternalId,
};

use crate::types::ObjectKey;

pub const MIN_VECTOR_DIMENSIONS: u32 = 2;
pub const MAX_VECTOR_DIMENSIONS: u32 = 4096;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorDimensions(
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "MIN_VECTOR_DIMENSIONS..=MAX_VECTOR_DIMENSIONS")
    )]
    u32,
);

impl From<VectorDimensions> for usize {
    fn from(value: VectorDimensions) -> Self {
        value.0 as usize
    }
}

impl From<VectorDimensions> for u32 {
    fn from(value: VectorDimensions) -> Self {
        value.0
    }
}

impl Deref for VectorDimensions {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<u32> for VectorDimensions {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        anyhow::ensure!(
            (MIN_VECTOR_DIMENSIONS..=MAX_VECTOR_DIMENSIONS).contains(&value),
            ErrorMetadata::bad_request(
                "InvalidVectorDimensionError",
                format!(
                    "Dimensions {} must be between {} and {}.",
                    value, MIN_VECTOR_DIMENSIONS, MAX_VECTOR_DIMENSIONS
                )
            )
        );
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperVectorIndexConfig {
    // Dimensions of the vectors
    pub dimensions: VectorDimensions,

    /// The field to index for vector search.
    pub vector_field: FieldPath,

    /// Other fields to index for equality filtering.
    pub filter_fields: BTreeSet<FieldPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum VectorIndexState {
    Backfilling(VectorIndexBackfillState),
    Backfilled(VectorIndexSnapshot),
    SnapshottedAt(VectorIndexSnapshot),
}

impl VectorIndexState {
    pub fn segments(&self) -> anyhow::Result<&Vec<FragmentedVectorSegment>> {
        match self {
            VectorIndexState::Backfilling(backfill_state) => Ok(&backfill_state.segments),
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                match snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => anyhow::bail!("Unknown snapshot data!"),
                    VectorIndexSnapshotData::MultiSegment(ref segments) => Ok(segments),
                }
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorIndexBackfillState {
    pub segments: Vec<FragmentedVectorSegment>,
    // Both of these variables will be None at the start of backfill.
    // They will be set after the first backfill iteration.
    pub cursor: Option<InternalId>,
    pub backfill_snapshot_ts: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorIndexSnapshot {
    pub data: VectorIndexSnapshotData,
    pub ts: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum VectorIndexSnapshotData {
    // Some future or previous incompatible version. The contained object is the
    // unmodified data that can safely be serialized again without dropping
    // unrecognized fields. Because we expect all data to be rollback
    // compatible, we have to be robust to future formats that might only be
    // recognized by versions ahead of ours.
    Unknown(ConvexObject),
    MultiSegment(Vec<FragmentedVectorSegment>),
}

/// A qdrant Segment that's split into three separate parts, the qdrant Segment
/// which depends on an IdTracker implementation, which depends on a deleted
/// bitset.
///
/// Each file is stored independently, but they're composed to form a queryable
/// segment. The deleted bitset can be written to independently. The id tracker
/// can be queried independently. Using the segment requires all three files.
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentedVectorSegment {
    pub segment_key: ObjectKey,
    pub id_tracker_key: ObjectKey,
    pub deleted_bitset_key: ObjectKey,
    pub num_vectors: u32,
    pub num_deleted: u32,
    // A random UUID that can be used to identify a segment to determine if the
    // segment has changed during non-transactional index changes (compaction).
    pub id: String,
}

impl FragmentedVectorSegment {
    fn extract_key(
        object_fields: &mut BTreeMap<FieldName, ConvexValue>,
        serialized_field_name: &str,
    ) -> anyhow::Result<ObjectKey> {
        match object_fields.remove(serialized_field_name) {
            Some(ConvexValue::String(s)) => s.try_into(),
            _ => anyhow::bail!(
                "Invalid or missing `{serialized_field_name}` field for VectorMultiPartData",
            ),
        }
    }

    pub fn non_deleted_vectors(&self) -> anyhow::Result<u64> {
        let total_vectors = if self.num_vectors < self.num_deleted {
            // Some early segments have been created with num_vectors sent to the initially
            // available point count, which excluded deletes. If sufficient vectors are
            // deleted, that can result in num_deleted exceeding the initial num_vectors.
            // That doesn't strictly mean the segment is empty, but it should be close
            // enough and a backfill to fix these segments is complex.
            Ok(0)
        } else {
            self.num_vectors
                .checked_sub(self.num_deleted)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Failed to subtract {} from {}",
                        self.num_deleted,
                        self.num_vectors
                    )
                })
        };
        total_vectors.map(|value| value as u64)
    }

    /// The estimated size bytes based only on the non-deleted vectors in the
    /// segment.
    ///
    /// The actual size of the segment in s3 will be bigger due both to deleted
    /// vectors excluded from this size estimation and also overhead from the
    /// HNSW index (if present). Index overhead is larger as a percentage for
    /// small dimensional vectors than large dimensional vectors.
    pub fn non_deleted_size_bytes(&self, dimensions: VectorDimensions) -> anyhow::Result<u64> {
        Self::size_bytes(self.non_deleted_vectors()?, dimensions)
    }

    /// The estimated size bytes based on both deleted and non-deleted vectors
    /// in the segment.
    ///
    /// The actual size of the segment in s3 will be bigger due to the overhead
    /// from the HNSW index (if present). Index overhead is larger as a
    /// percentage for small dimensional vectors than large dimensional
    /// vectors.
    pub fn total_size_bytes(&self, dimensions: VectorDimensions) -> anyhow::Result<u64> {
        Self::size_bytes(self.num_vectors as u64, dimensions)
    }

    fn size_bytes(estimated_vectors: u64, dimensions: VectorDimensions) -> anyhow::Result<u64> {
        // A little extra paranoia since all of these numbers are not originally u64 and
        // can overflow u32.
        (estimated_vectors)
            .checked_mul(dimensions.0 as u64)
            .and_then(|value| value.checked_mul(4_u64))
            .context("Overflowed size calculation!")
    }

    pub fn to_paths_proto(self) -> anyhow::Result<pb::searchlight::FragmentedVectorSegmentPaths> {
        Ok(pb::searchlight::FragmentedVectorSegmentPaths {
            segment: Some(pb::searchlight::StorageKey {
                storage_key: self.segment_key.into(),
            }),
            id_tracker: Some(pb::searchlight::StorageKey {
                storage_key: self.id_tracker_key.into(),
            }),
            deleted_bitset: Some(pb::searchlight::StorageKey {
                storage_key: self.deleted_bitset_key.into(),
            }),
        })
    }
}

impl TryFrom<FragmentedVectorSegment> for pb::searchlight::FragmentedVectorSegment {
    type Error = anyhow::Error;

    fn try_from(value: FragmentedVectorSegment) -> Result<Self, Self::Error> {
        Ok(Self {
            segment_key: value.segment_key.try_into()?,
            id_tracker_key: value.id_tracker_key.try_into()?,
            deleted_bitset_key: value.deleted_bitset_key.try_into()?,
            num_vectors: value.num_vectors,
            num_deleted: value.num_deleted,
            id: value.id,
        })
    }
}

impl TryFrom<pb::searchlight::FragmentedVectorSegment> for FragmentedVectorSegment {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::FragmentedVectorSegment) -> Result<Self, Self::Error> {
        Ok(Self {
            segment_key: value.segment_key.try_into()?,
            id_tracker_key: value.id_tracker_key.try_into()?,
            deleted_bitset_key: value.deleted_bitset_key.try_into()?,
            num_vectors: value.num_vectors,
            num_deleted: value.num_deleted,
            id: value.id,
        })
    }
}

impl TryFrom<FragmentedVectorSegment> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: FragmentedVectorSegment) -> Result<Self, Self::Error> {
        obj!(
            "segment_key" => value.segment_key.to_string(),
            "id_tracker_key" => value.id_tracker_key.to_string(),
            "deleted_bitset_key" => value.deleted_bitset_key.to_string(),
            "id" => value.id,
            "num_vectors" => (value.num_vectors as i64),
            "num_deleted" => (value.num_deleted as i64),
        )
    }
}

impl TryFrom<ConvexObject> for FragmentedVectorSegment {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.into();
        let segment_key = Self::extract_key(&mut object_fields, "segment_key")?;
        let id_tracker_key = Self::extract_key(&mut object_fields, "id_tracker_key")?;
        let deleted_bitset_key = Self::extract_key(&mut object_fields, "deleted_bitset_key")?;
        let id = match object_fields.remove("id") {
            Some(ConvexValue::String(s)) => String::from(s),
            _ => anyhow::bail!(
                "Invalid or missing `id` field fo FragmentedVectorSegment: {:?}",
                object_fields
            ),
        };
        let num_vectors = match object_fields.remove("num_vectors") {
            Some(ConvexValue::Int64(i)) => i as u32,
            _ => anyhow::bail!(
                "Invalid or missing `num_vectors` field for FragmentedVectorSegment: {:?}",
                object_fields
            ),
        };
        let num_deleted = match object_fields.remove("num_deleted") {
            Some(ConvexValue::Int64(i)) => i as u32,
            _ => anyhow::bail!(
                "Invalid or missing `num_deleted` field for FragmentedVectorSegment: {:?}",
                object_fields
            ),
        };

        Ok(Self {
            segment_key,
            id_tracker_key,
            deleted_bitset_key,
            id,
            num_vectors,
            num_deleted,
        })
    }
}

impl VectorIndexSnapshotData {
    pub fn is_version_current(&self) -> bool {
        let result = matches!(self, VectorIndexSnapshotData::MultiSegment(_));
        if !result {
            tracing::warn!(
                "Vector version mismatch, stored: {:?}, current: MultiSegment",
                self,
            );
        }
        result
    }
}

impl TryFrom<&ConvexObject> for VectorIndexSnapshotData {
    type Error = anyhow::Error;

    fn try_from(value: &ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.clone().into();
        let data_type: String = match object_fields.remove("data_type") {
            Some(ConvexValue::String(s)) => String::from(s),
            _ => anyhow::bail!(
                "Invalid or missing `data_type` field for VectorIndexSnapshotData: {:?}",
                object_fields
            ),
        };
        if data_type == "MultiSegment" {
            let parts = match object_fields.remove("segments") {
                Some(ConvexValue::Array(values)) => values
                    .into_iter()
                    .map(|value| ConvexObject::try_from(value)?.try_into())
                    .try_collect::<Vec<_>>()?,
                _ => anyhow::bail!(
                    "Invalid or missing `parts` field for VectorIndexSnapshotData::MultiSegment: \
                     {:?}",
                    object_fields
                ),
            };
            return Ok(VectorIndexSnapshotData::MultiSegment(parts));
        }
        anyhow::bail!(
            "Unrecognized vector index snapshot data: {:?}",
            object_fields
        );
    }
}

impl TryFrom<VectorIndexSnapshotData> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: VectorIndexSnapshotData) -> anyhow::Result<Self> {
        match value {
            VectorIndexSnapshotData::MultiSegment(parts) => obj!(
                "data_type" => "MultiSegment",
                "segments" => ConvexValue::Array(
                    parts.into_iter().map(|value| value.try_into().map(ConvexValue::Object))
                        .try_collect::<Vec<_>>()?
                        .try_into()?
                ),
            ),
            // If we're written back, restore whatever data we originally read.
            VectorIndexSnapshotData::Unknown(obj) => Ok(obj),
        }
    }
}

impl From<ConvexObject> for VectorIndexSnapshotData {
    fn from(value: ConvexObject) -> Self {
        match Self::try_from(&value) {
            Ok(result) => result,
            Err(e) => {
                // Fallback to an unknown value that will trigger a rebuild and that can
                // pass through the unknown data without modifying it.
                tracing::error!("Unrecognized vector index snapshot data: {:?}", e);
                VectorIndexSnapshotData::Unknown(value)
            },
        }
    }
}

impl TryFrom<VectorIndexState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(state: VectorIndexState) -> Result<Self, Self::Error> {
        match state {
            VectorIndexState::Backfilling(VectorIndexBackfillState {
                segments,
                cursor,
                backfill_snapshot_ts,
            }) => {
                let backfill_snapshot_ts = backfill_snapshot_ts
                    .map(|ts| anyhow::Ok(ConvexValue::Int64(ts.try_into()?)))
                    .transpose()?
                    .unwrap_or(ConvexValue::Null);
                let segments = ConvexValue::Array(
                    segments
                        .into_iter()
                        .map(|value| value.try_into().map(ConvexValue::Object))
                        .try_collect::<Vec<_>>()?
                        .try_into()?,
                );
                obj!(
                    "state" => "backfilling",
                    "document_cursor" => cursor.map(|c| ConvexValue::try_from(c.to_string())).transpose()?.unwrap_or(ConvexValue::Null),
                    "backfill_snapshot_ts" => backfill_snapshot_ts,
                    "segments" => segments,
                )
            },
            VectorIndexState::Backfilled(snapshot) => snapshot_to_object("backfilled", snapshot),
            VectorIndexState::SnapshottedAt(snapshot) => {
                snapshot_to_object("snapshotted", snapshot)
            },
        }
    }
}

pub fn snapshot_to_object(
    state: &str,
    snapshot: VectorIndexSnapshot,
) -> anyhow::Result<ConvexObject> {
    match snapshot.data {
        VectorIndexSnapshotData::MultiSegment(_) => obj!(
            "state" => state,
            "ts" => ConvexValue::Int64(snapshot.ts.into()),
            "data" => ConvexValue::Object(snapshot.data.try_into()?),
        ),
        VectorIndexSnapshotData::Unknown(obj) => obj!(
            "state" => state,
            "ts" => ConvexValue::Int64(snapshot.ts.into()),
            "data" => ConvexValue::Object(obj),
        ),
    }
}

pub(crate) fn snapshot_from_object(
    mut object_fields: BTreeMap<FieldName, ConvexValue>,
) -> anyhow::Result<VectorIndexSnapshot> {
    let data = match object_fields.remove("data") {
        Some(ConvexValue::Object(obj)) => obj.into(),
        _ => anyhow::bail!(
            "Invalid or missing `data` field for VectorIndexSnapshot: {:?}",
            object_fields
        ),
    };
    let ts: Timestamp = match object_fields.remove("ts") {
        Some(ConvexValue::Int64(i)) => i.try_into()?,
        _ => anyhow::bail!(
            "Invalid or missing `ts` field for VectorIndexSnapshot: {:?}",
            object_fields
        ),
    };
    Ok(VectorIndexSnapshot { data, ts })
}

impl TryFrom<ConvexObject> for VectorIndexState {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = object.into();
        let state = match object_fields.remove("state") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing `state` field for VectorIndexState: {:?}",
                object_fields
            ),
        };
        Ok(match state.to_string().as_str() {
            "backfilling" => {
                // The fields cursor, backfill_snapshot_ts, and segments are not present in old
                // indexes in Backfilling state. Thus, these all support being deserialized when
                // missing using empty defaults (None or vec![]). This allows backfilling to be
                // backwards-compatible
                let cursor: Option<InternalId> = match object_fields.remove("document_cursor") {
                    None | Some(ConvexValue::Null) => None,
                    Some(ConvexValue::String(v)) => Some(InternalId::from_str(&v)?),
                    Some(_) => anyhow::bail!("expected document_cursor to be string"),
                };
                let segments = match object_fields.remove("segments") {
                    Some(ConvexValue::Array(values)) => values
                        .into_iter()
                        .map(|value| ConvexObject::try_from(value)?.try_into())
                        .try_collect::<Vec<_>>()?,
                    None => vec![],
                    v => anyhow::bail!("Invalid `segments` field for VectorIndexState: {:?}", v),
                };
                let backfill_snapshot_ts = match object_fields.remove("backfill_snapshot_ts") {
                    Some(ConvexValue::Int64(ts)) => Some(Timestamp::try_from(ts)?),
                    None | Some(ConvexValue::Null) => None,
                    v => anyhow::bail!(
                        "Invalid `backfill_snapshot_ts` field for VectorIndexState: {:?}",
                        v
                    ),
                };

                VectorIndexState::Backfilling(VectorIndexBackfillState {
                    cursor,
                    segments,
                    backfill_snapshot_ts,
                })
            },
            "backfilled" => {
                let snapshot = snapshot_from_object(object_fields)?;
                VectorIndexState::Backfilled(snapshot)
            },
            "snapshotted" => {
                let snapshot = snapshot_from_object(object_fields)?;
                VectorIndexState::SnapshottedAt(snapshot)
            },
            _ => anyhow::bail!(
                "Invalid `state` field for VectorIndexState: {:?}",
                object_fields
            ),
        })
    }
}

impl TryFrom<pb::searchlight::VectorIndexConfig> for DeveloperVectorIndexConfig {
    type Error = anyhow::Error;

    fn try_from(proto: pb::searchlight::VectorIndexConfig) -> anyhow::Result<Self> {
        Ok(DeveloperVectorIndexConfig {
            dimensions: VectorDimensions::try_from(proto.dimension)?,
            vector_field: proto
                .vector_field_path
                .ok_or_else(|| anyhow::format_err!("Missing vector_field_path"))?
                .try_into()?,
            filter_fields: proto
                .filter_fields
                .into_iter()
                .map(|i| i.try_into())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect(),
        })
    }
}

impl From<DeveloperVectorIndexConfig> for pb::searchlight::VectorIndexConfig {
    fn from(config: DeveloperVectorIndexConfig) -> Self {
        pb::searchlight::VectorIndexConfig {
            dimension: u32::from(config.dimensions),
            vector_field_path: Some(config.vector_field.into()),
            filter_fields: config
                .filter_fields
                .into_iter()
                .map(|f| f.into())
                .collect::<Vec<_>>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use must_let::must_let;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::assert_obj;

    use super::*;

    fn serialized_index_state_name_having_data() -> impl Strategy<Value = String> {
        prop::string::string_regex("backfilled|snapshotted").unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_developer_vector_index_config_roundtrips(v in any::<DeveloperVectorIndexConfig>()) {
            assert_roundtrips::<
                DeveloperVectorIndexConfig,
                pb::searchlight::VectorIndexConfig
            >(v);
        }

        #[test]
        fn vector_index_state_roundtrips(v in any::<VectorIndexState>()) {
            assert_roundtrips::<VectorIndexState, ConvexObject>(v)
        }

        #[test]
        fn from_legacy_resolved_object_fails(
            key in any::<ObjectKey>(),
            ts in any::<Timestamp>(),
            serialized_index_state_name in serialized_index_state_name_having_data(),
        ) {
            let legacy_object = assert_obj!(
                "state" => serialized_index_state_name.as_str(),
                "index" => key.to_string(),
                "ts" => ConvexValue::Int64(ts.into()),
                "version" => 0,
            );
            // We don't have an unknown field at the state level, only for data, so we have to let
            // this error.
            assert!(VectorIndexState::try_from(legacy_object).is_err());
        }

        #[test]
        fn missing_data_type_defaults_to_unknown(
            ts in any::<Timestamp>(),
            serialized_index_state_name in serialized_index_state_name_having_data(),
        ) {
            let legacy_object = assert_obj!(
                "state" => serialized_index_state_name.as_str(),
                "data" => {"something" => "invalid"},
                "ts" => ConvexValue::Int64(ts.into()),
            );
            let state: VectorIndexState = legacy_object.try_into().unwrap();
            let snapshot = extract_snapshot(serialized_index_state_name, state);

            must_let!(let VectorIndexSnapshotData::Unknown(_) = snapshot.data);
        }

        #[test]
        fn unrecognized_data_type_defaults_to_unknown(
            ts in any::<Timestamp>(),
            serialized_index_state_name in serialized_index_state_name_having_data(),
        ) {
            let legacy_object = assert_obj!(
                "state" => serialized_index_state_name.as_str(),
                "data" => {"data_type" => "invalid"},
                "ts" => ConvexValue::Int64(ts.into()),
            );
            let state: VectorIndexState = legacy_object.try_into().unwrap();
            let snapshot = extract_snapshot(serialized_index_state_name, state);

            must_let!(let VectorIndexSnapshotData::Unknown(_) = snapshot.data);
        }
    }

    fn extract_snapshot(
        expected_index_state: String,
        state: VectorIndexState,
    ) -> VectorIndexSnapshot {
        if expected_index_state == "backfilled" {
            must_let!(let VectorIndexState::Backfilled(snapshot) = state);
            snapshot
        } else {
            must_let!(let VectorIndexState::SnapshottedAt(snapshot) = state);
            snapshot
        }
    }
}
