use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::{
    codegen_convex_serialization,
    InternalId,
};

use super::segment::{
    FragmentedVectorSegment,
    SerializedFragmentedVectorSegment,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorIndexBackfillState {
    pub segments: Vec<FragmentedVectorSegment>,
    // Both of these variables will be None at the start of backfill.
    // They will be set after the first backfill iteration.
    pub cursor: Option<InternalId>,
    pub backfill_snapshot_ts: Option<Timestamp>,
    pub staged: bool,
}

impl VectorIndexBackfillState {
    pub fn new(staged: bool) -> Self {
        Self {
            segments: vec![],
            cursor: None,
            backfill_snapshot_ts: None,
            staged,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct SerializedVectorIndexBackfillState {
    segments: Option<Vec<SerializedFragmentedVectorSegment>>,
    document_cursor: Option<String>,
    backfill_snapshot_ts: Option<i64>,
    staged: Option<bool>,
}

impl TryFrom<VectorIndexBackfillState> for SerializedVectorIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(backfill_state: VectorIndexBackfillState) -> Result<Self, Self::Error> {
        Ok(SerializedVectorIndexBackfillState {
            segments: Some(
                backfill_state
                    .segments
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<anyhow::Result<Vec<_>>>()?,
            ),
            document_cursor: backfill_state.cursor.map(|id| id.to_string()),
            backfill_snapshot_ts: backfill_state.backfill_snapshot_ts.map(|ts| ts.into()),
            staged: Some(backfill_state.staged),
        })
    }
}

impl TryFrom<SerializedVectorIndexBackfillState> for VectorIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedVectorIndexBackfillState) -> Result<Self, Self::Error> {
        // The fields cursor, backfill_snapshot_ts, and segments are not present in old
        // indexes in Backfilling state. Thus, these all support being deserialized when
        // missing using empty defaults (None or vec![]). This allows backfilling to be
        // backwards-compatible.
        Ok(VectorIndexBackfillState {
            segments: serialized
                .segments
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.try_into())
                .collect::<anyhow::Result<Vec<_>>>()?,
            cursor: serialized
                .document_cursor
                .map(|id| InternalId::from_str(&id))
                .transpose()?,
            backfill_snapshot_ts: serialized
                .backfill_snapshot_ts
                .map(Timestamp::try_from)
                .transpose()?,
            staged: serialized.staged.unwrap_or_default(),
        })
    }
}

codegen_convex_serialization!(VectorIndexBackfillState, SerializedVectorIndexBackfillState);
