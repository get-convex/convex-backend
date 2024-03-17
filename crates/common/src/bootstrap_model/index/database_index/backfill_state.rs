use std::convert::TryFrom;

use serde::{
    Deserialize,
    Serialize,
};

/// Represents state of currently backfilling index.
/// We currently do not checkpoint. Will extend the struct when we do.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DatabaseIndexBackfillState;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDatabaseIndexBackfillState {}

impl TryFrom<DatabaseIndexBackfillState> for SerializedDatabaseIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(_config: DatabaseIndexBackfillState) -> anyhow::Result<Self> {
        Ok(Self {})
    }
}

impl TryFrom<SerializedDatabaseIndexBackfillState> for DatabaseIndexBackfillState {
    type Error = anyhow::Error;

    fn try_from(_config: SerializedDatabaseIndexBackfillState) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
