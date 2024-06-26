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

use crate::bootstrap_model::index::text_index::{
    index_snapshot::SerializedFragmentedTextSegment,
    FragmentedTextSegment,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TextIndexBackfillState {
    pub segments: Vec<FragmentedTextSegment>,
    // None at the start of backfill, then set after the first backfill iteration.
    pub cursor: Option<TextBackfillCursor>,
}

impl TextIndexBackfillState {
    pub fn new() -> Self {
        Self {
            segments: vec![],
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TextBackfillCursor {
    pub cursor: InternalId,
    pub backfill_snapshot_ts: Timestamp,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedTextBackfillCursor {
    pub document_cursor: String,
    pub backfill_snapshot_ts: i64,
}

impl From<TextBackfillCursor> for SerializedTextBackfillCursor {
    fn from(value: TextBackfillCursor) -> Self {
        Self {
            document_cursor: value.cursor.to_string(),
            backfill_snapshot_ts: value.backfill_snapshot_ts.into(),
        }
    }
}

impl TryFrom<SerializedTextBackfillCursor> for TextBackfillCursor {
    type Error = anyhow::Error;

    fn try_from(value: SerializedTextBackfillCursor) -> Result<Self, Self::Error> {
        Ok(Self {
            cursor: InternalId::from_str(&value.document_cursor)?,
            backfill_snapshot_ts: Timestamp::try_from(value.backfill_snapshot_ts)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializedTextIndexBackfillState {
    segments: Option<Vec<SerializedFragmentedTextSegment>>,
    cursor: Option<SerializedTextBackfillCursor>,
}

impl TryFrom<TextIndexBackfillState> for SerializedTextIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(backfill_state: TextIndexBackfillState) -> Result<Self, Self::Error> {
        Ok(SerializedTextIndexBackfillState {
            segments: Some(
                backfill_state
                    .segments
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<anyhow::Result<Vec<_>>>()?,
            ),
            cursor: backfill_state
                .cursor
                .map(SerializedTextBackfillCursor::from),
        })
    }
}

impl TryFrom<SerializedTextIndexBackfillState> for TextIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedTextIndexBackfillState) -> Result<Self, Self::Error> {
        Ok(TextIndexBackfillState {
            segments: serialized
                .segments
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.try_into())
                .collect::<anyhow::Result<Vec<_>>>()?,
            cursor: serialized
                .cursor
                .map(TextBackfillCursor::try_from)
                .transpose()?,
        })
    }
}

codegen_convex_serialization!(TextIndexBackfillState, SerializedTextIndexBackfillState);
