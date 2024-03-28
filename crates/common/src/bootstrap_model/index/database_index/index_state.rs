use std::convert::TryFrom;

use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    DatabaseIndexBackfillState,
    SerializedDatabaseIndexBackfillState,
};

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

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum SerializedDatabaseIndexState {
    #[serde(rename_all = "camelCase")]
    Backfilling {
        backfill_state: SerializedDatabaseIndexBackfillState,
    },
    // Use Backfilled2 to distinguish between records impacted by CX-3897
    Backfilled2,
    Enabled,

    // We have historical records with Disabled state.
    Disabled,
}

impl TryFrom<DatabaseIndexState> for SerializedDatabaseIndexState {
    type Error = anyhow::Error;

    fn try_from(config: DatabaseIndexState) -> anyhow::Result<Self> {
        Ok(match config {
            DatabaseIndexState::Backfilling(st) => SerializedDatabaseIndexState::Backfilling {
                backfill_state: st.try_into()?,
            },
            DatabaseIndexState::Backfilled => SerializedDatabaseIndexState::Backfilled2,
            DatabaseIndexState::Enabled => SerializedDatabaseIndexState::Enabled,
        })
    }
}

impl TryFrom<SerializedDatabaseIndexState> for DatabaseIndexState {
    type Error = anyhow::Error;

    fn try_from(config: SerializedDatabaseIndexState) -> anyhow::Result<Self> {
        Ok(match config {
            SerializedDatabaseIndexState::Backfilling { backfill_state } => {
                DatabaseIndexState::Backfilling(backfill_state.try_into()?)
            },
            SerializedDatabaseIndexState::Backfilled2 => DatabaseIndexState::Backfilled,
            SerializedDatabaseIndexState::Enabled => DatabaseIndexState::Enabled,
            // TODO(Presley): Backfill and delete Disabled state.
            SerializedDatabaseIndexState::Disabled => {
                DatabaseIndexState::Backfilling(DatabaseIndexBackfillState {
                    index_created_lower_bound: None,
                    retention_started: false,
                })
            },
        })
    }
}

codegen_convex_serialization!(DatabaseIndexState, SerializedDatabaseIndexState);
