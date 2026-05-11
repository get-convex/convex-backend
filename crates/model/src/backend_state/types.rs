pub use common::types::{
    BackendState,
    OldBackendState,
    SystemStopState,
    UserStopState,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

#[derive(Debug, PartialEq, Clone)]
pub struct PersistedBackendState(pub BackendState);

#[derive(Serialize, Deserialize)]
pub struct SerializedBackendState {
    system: String,
    user: String,
}

impl From<PersistedBackendState> for SerializedBackendState {
    fn from(state: PersistedBackendState) -> Self {
        Self {
            system: state.0.system.to_string(),
            user: state.0.user.to_string(),
        }
    }
}

impl TryFrom<SerializedBackendState> for PersistedBackendState {
    type Error = anyhow::Error;

    fn try_from(object: SerializedBackendState) -> anyhow::Result<Self> {
        Ok(Self(BackendState {
            system: object.system.parse()?,
            user: object.user.parse()?,
        }))
    }
}

codegen_convex_serialization!(PersistedBackendState, SerializedBackendState);
