use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    index_snapshot::SerializedTextIndexSnapshot,
    TextIndexSnapshot,
};
use crate::bootstrap_model::index::search_index::backfill_state::{
    SerializedTextIndexBackfillState,
    TextIndexBackfillState,
};

/// The state of a search index.
/// Search indexes begin in `Backfilling`.
/// Once the backfill completes, we'll have a snapshot at a timestamp which
/// continually moves forward.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SearchIndexState {
    Backfilling(TextIndexBackfillState),
    Backfilled(TextIndexSnapshot),
    SnapshottedAt(TextIndexSnapshot),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SerializedSearchIndexState {
    Backfilling,
    Backfilling2 {
        #[serde(flatten)]
        backfill_state: SerializedTextIndexBackfillState,
    },
    Backfilled {
        #[serde(flatten)]
        snapshot: SerializedTextIndexSnapshot,
    },
    Snapshotted {
        #[serde(flatten)]
        snapshot: SerializedTextIndexSnapshot,
    },
}

impl TryFrom<SearchIndexState> for SerializedSearchIndexState {
    type Error = anyhow::Error;

    fn try_from(state: SearchIndexState) -> Result<Self, Self::Error> {
        Ok(match state {
            SearchIndexState::Backfilling(state) => {
                // Maintain rollback compatibility with the old format by writing empty
                // backfilling states using the old format. Since we don't
                // currently use the new format, all states should be empty, so
                // we should always write the old format. TODO(CX-6465): Clean
                // this up.
                if state.segments.is_empty() && state.cursor.is_none() {
                    SerializedSearchIndexState::Backfilling
                } else {
                    SerializedSearchIndexState::Backfilling2 {
                        backfill_state: state.try_into()?,
                    }
                }
            },
            SearchIndexState::Backfilled(snapshot) => SerializedSearchIndexState::Backfilled {
                snapshot: snapshot.try_into()?,
            },
            SearchIndexState::SnapshottedAt(snapshot) => SerializedSearchIndexState::Snapshotted {
                snapshot: snapshot.try_into()?,
            },
        })
    }
}

impl TryFrom<SerializedSearchIndexState> for SearchIndexState {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedSearchIndexState) -> Result<Self, Self::Error> {
        Ok(match serialized {
            SerializedSearchIndexState::Backfilling => {
                SearchIndexState::Backfilling(TextIndexBackfillState::new())
            },
            SerializedSearchIndexState::Backfilling2 { backfill_state } => {
                SearchIndexState::Backfilling(backfill_state.try_into()?)
            },
            SerializedSearchIndexState::Backfilled { snapshot } => {
                SearchIndexState::Backfilled(snapshot.try_into()?)
            },
            SerializedSearchIndexState::Snapshotted { snapshot } => {
                SearchIndexState::SnapshottedAt(snapshot.try_into()?)
            },
        })
    }
}

codegen_convex_serialization!(SearchIndexState, SerializedSearchIndexState);
