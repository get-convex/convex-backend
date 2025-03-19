use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    index_snapshot::SerializedTextIndexSnapshot,
    TextIndexSnapshot,
};
use crate::bootstrap_model::index::text_index::backfill_state::{
    SerializedTextIndexBackfillState,
    TextIndexBackfillState,
};

/// The state of a text search index.
/// Text search indexes begin in `Backfilling`. Once they finish backfilling,
/// but before they've been committed, they'll be in a `Backfilled` state with a
/// snapshot and timestamp that moves continually forward. Once the index change
/// is committed by the user, they advance to the `SnapshottedAt` state and can
/// be used in queries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum TextIndexState {
    Backfilling(TextIndexBackfillState),
    Backfilled(TextIndexSnapshot),
    SnapshottedAt(TextIndexSnapshot),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SerializedTextIndexState {
    Backfilling,
    Backfilling2(SerializedTextIndexBackfillState),
    Backfilled(SerializedTextIndexSnapshot),
    Snapshotted(SerializedTextIndexSnapshot),
}

impl TryFrom<TextIndexState> for SerializedTextIndexState {
    type Error = anyhow::Error;

    fn try_from(state: TextIndexState) -> Result<Self, Self::Error> {
        Ok(match state {
            TextIndexState::Backfilling(state) => {
                // Maintain rollback compatibility with the old format by writing empty
                // backfilling states using the old format. Since we don't
                // currently use the new format, all states should be empty, so
                // we should always write the old format. TODO(CX-6465): Clean
                // this up.
                if state.segments.is_empty() && state.cursor.is_none() {
                    SerializedTextIndexState::Backfilling
                } else {
                    SerializedTextIndexState::Backfilling2(state.try_into()?)
                }
            },
            TextIndexState::Backfilled(snapshot) => {
                SerializedTextIndexState::Backfilled(snapshot.try_into()?)
            },
            TextIndexState::SnapshottedAt(snapshot) => {
                SerializedTextIndexState::Snapshotted(snapshot.try_into()?)
            },
        })
    }
}

impl TryFrom<SerializedTextIndexState> for TextIndexState {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedTextIndexState) -> Result<Self, Self::Error> {
        Ok(match serialized {
            SerializedTextIndexState::Backfilling => {
                TextIndexState::Backfilling(TextIndexBackfillState::new())
            },
            SerializedTextIndexState::Backfilling2(backfill_state) => {
                TextIndexState::Backfilling(backfill_state.try_into()?)
            },
            SerializedTextIndexState::Backfilled(snapshot) => {
                TextIndexState::Backfilled(snapshot.try_into()?)
            },
            SerializedTextIndexState::Snapshotted(snapshot) => {
                TextIndexState::SnapshottedAt(snapshot.try_into()?)
            },
        })
    }
}

codegen_convex_serialization!(TextIndexState, SerializedTextIndexState);
