pub use common::types::BackendState;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

#[derive(Debug, PartialEq, Clone)]
pub struct PersistedBackendState(pub BackendState);

#[derive(Serialize, Deserialize)]
pub struct SerializedBackendState {
    pub state: String,
}

impl From<PersistedBackendState> for SerializedBackendState {
    fn from(state: PersistedBackendState) -> Self {
        SerializedBackendState {
            state: state.0.to_string(),
        }
    }
}

impl TryFrom<SerializedBackendState> for PersistedBackendState {
    type Error = anyhow::Error;

    fn try_from(object: SerializedBackendState) -> anyhow::Result<Self> {
        let state = object.state.parse()?;
        Ok(Self(state))
    }
}

codegen_convex_serialization!(PersistedBackendState, SerializedBackendState);
