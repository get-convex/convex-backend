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
use crate::bootstrap_model::index::search_index::{
    BackfillState,
    SearchBackfillCursor,
};

pub type VectorIndexBackfillState = BackfillState<FragmentedVectorSegment>;

#[derive(Serialize, Deserialize)]
pub struct SerializedVectorIndexBackfillState {
    segments: Option<Vec<SerializedFragmentedVectorSegment>>,
    document_cursor: Option<String>,
    backfill_snapshot_ts: Option<i64>,
    table_scan_cursor: Option<Vec<u8>>,
    last_segment_ts: Option<i64>,
    staged: Option<bool>,
}

impl TryFrom<VectorIndexBackfillState> for SerializedVectorIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(backfill_state: VectorIndexBackfillState) -> Result<Self, Self::Error> {
        let (document_cursor, backfill_snapshot_ts, table_scan_cursor, last_segment_ts) =
            match backfill_state.cursor {
                Some(SearchBackfillCursor::AtSnapshot {
                    backfill_snapshot_ts,
                    cursor,
                }) => (
                    Some(cursor.to_string()),
                    Some(backfill_snapshot_ts.into()),
                    None,
                    None,
                ),
                Some(SearchBackfillCursor::WalkingForwards {
                    last_segment_ts,
                    table_scan_cursor,
                }) => (
                    None,
                    None,
                    Some(table_scan_cursor),
                    Some(last_segment_ts.into()),
                ),
                None => (None, None, None, None),
            };
        Ok(SerializedVectorIndexBackfillState {
            segments: Some(
                backfill_state
                    .segments
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<anyhow::Result<Vec<_>>>()?,
            ),
            document_cursor,
            backfill_snapshot_ts,
            table_scan_cursor,
            last_segment_ts,
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
        let document_cursor = serialized
            .document_cursor
            .map(|id| InternalId::from_str(&id))
            .transpose()?;
        let backfill_snapshot_ts = serialized
            .backfill_snapshot_ts
            .map(Timestamp::try_from)
            .transpose()?;
        let cursor = match (document_cursor, backfill_snapshot_ts) {
            (Some(cursor), Some(backfill_snapshot_ts)) => Some(SearchBackfillCursor::AtSnapshot {
                backfill_snapshot_ts,
                cursor,
            }),
            (None, None) => {
                let table_scan_cursor = serialized.table_scan_cursor;
                let last_segment_ts = serialized
                    .last_segment_ts
                    .map(Timestamp::try_from)
                    .transpose()?;
                match (table_scan_cursor, last_segment_ts) {
                    (Some(table_scan_cursor), Some(last_segment_ts)) => {
                        Some(SearchBackfillCursor::WalkingForwards {
                            last_segment_ts,
                            table_scan_cursor,
                        })
                    },
                    (None, None) => None,
                    _ => anyhow::bail!(
                        "VectorIndexBackfillState must have both table_scan_cursor and \
                         last_segment_ts"
                    ),
                }
            },
            _ => anyhow::bail!(
                "VectorIndexBackfillState must have both document_cursor and backfill_snapshot_ts"
            ),
        };
        Ok(VectorIndexBackfillState {
            segments: serialized
                .segments
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.try_into())
                .collect::<anyhow::Result<Vec<_>>>()?,
            cursor,
            staged: serialized.staged.unwrap_or_default(),
        })
    }
}

codegen_convex_serialization!(VectorIndexBackfillState, SerializedVectorIndexBackfillState);
