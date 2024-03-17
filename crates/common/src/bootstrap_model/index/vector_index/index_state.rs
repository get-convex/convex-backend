use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    backfill_state::{
        SerializedVectorIndexBackfillState,
        VectorIndexBackfillState,
    },
    index_snapshot::{
        SerializedVectorIndexSnapshot,
        VectorIndexSnapshot,
        VectorIndexSnapshotData,
    },
    segment::FragmentedVectorSegment,
};

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
                    VectorIndexSnapshotData::MultiSegment(ref segments) => Ok(segments),
                    VectorIndexSnapshotData::Unknown(_) => anyhow::bail!("Unknown snapshot data!"),
                }
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SerializedVectorIndexState {
    Backfilling {
        #[serde(flatten)]
        backfill_state: SerializedVectorIndexBackfillState,
    },
    Backfilled {
        #[serde(flatten)]
        snapshot: SerializedVectorIndexSnapshot,
    },
    Snapshotted {
        #[serde(flatten)]
        snapshot: SerializedVectorIndexSnapshot,
    },
}

impl TryFrom<VectorIndexState> for SerializedVectorIndexState {
    type Error = anyhow::Error;

    fn try_from(state: VectorIndexState) -> Result<Self, Self::Error> {
        Ok(match state {
            VectorIndexState::Backfilling(backfill_state) => {
                SerializedVectorIndexState::Backfilling {
                    backfill_state: backfill_state.try_into()?,
                }
            },
            VectorIndexState::Backfilled(snapshot) => SerializedVectorIndexState::Backfilled {
                snapshot: snapshot.try_into()?,
            },
            VectorIndexState::SnapshottedAt(snapshot) => SerializedVectorIndexState::Snapshotted {
                snapshot: snapshot.try_into()?,
            },
        })
    }
}

impl TryFrom<SerializedVectorIndexState> for VectorIndexState {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedVectorIndexState) -> Result<Self, Self::Error> {
        Ok(match serialized {
            SerializedVectorIndexState::Backfilling { backfill_state } => {
                VectorIndexState::Backfilling(backfill_state.try_into()?)
            },
            SerializedVectorIndexState::Backfilled { snapshot } => {
                VectorIndexState::Backfilled(snapshot.try_into()?)
            },
            SerializedVectorIndexState::Snapshotted { snapshot } => {
                VectorIndexState::SnapshottedAt(snapshot.try_into()?)
            },
        })
    }
}

codegen_convex_serialization!(
    VectorIndexState,
    SerializedVectorIndexState,
    test_cases = 64
);
