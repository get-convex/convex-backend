use std::convert::TryFrom;

use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    index_snapshot::SerializedSearchIndexSnapshot,
    SearchIndexSnapshot,
};

/// The state of a search index.
/// Search indexes begin in `Backfilling`.
/// Once the backfill completes, we'll have a snapshot at a timestamp which
/// continually moves forward.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SearchIndexState {
    Backfilling,
    Backfilled(SearchIndexSnapshot),
    SnapshottedAt(SearchIndexSnapshot),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SerializedSearchIndexState {
    Backfilling,
    Backfilled {
        #[serde(flatten)]
        snapshot: SerializedSearchIndexSnapshot,
    },
    Snapshotted {
        #[serde(flatten)]
        snapshot: SerializedSearchIndexSnapshot,
    },
}

impl TryFrom<SearchIndexState> for SerializedSearchIndexState {
    type Error = anyhow::Error;

    fn try_from(state: SearchIndexState) -> Result<Self, Self::Error> {
        Ok(match state {
            SearchIndexState::Backfilling => SerializedSearchIndexState::Backfilling,
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
            SerializedSearchIndexState::Backfilling => SearchIndexState::Backfilling,
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
