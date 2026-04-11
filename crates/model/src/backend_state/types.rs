pub use common::types::{
    BackendState,
    NewBackendState,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

#[derive(Debug, PartialEq, Clone)]
pub enum PersistedBackendState {
    Old(BackendState),
    New(NewBackendState),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SerializedBackendState {
    Old { state: String },
    New { system: String, user: String },
}

impl From<PersistedBackendState> for SerializedBackendState {
    fn from(state: PersistedBackendState) -> Self {
        match state {
            PersistedBackendState::Old(state) => Self::Old {
                state: state.to_string(),
            },
            PersistedBackendState::New(state) => Self::New {
                system: state.system.to_string(),
                user: state.user.to_string(),
            },
        }
    }
}

impl TryFrom<SerializedBackendState> for PersistedBackendState {
    type Error = anyhow::Error;

    fn try_from(object: SerializedBackendState) -> anyhow::Result<Self> {
        Ok(match object {
            SerializedBackendState::Old { state } => Self::Old(state.parse()?),
            SerializedBackendState::New { system, user } => Self::New(NewBackendState {
                system: system.parse()?,
                user: user.parse()?,
            }),
        })
    }
}

impl PersistedBackendState {
    pub fn to_old_lossy(&self) -> BackendState {
        match self {
            PersistedBackendState::Old(old_backend_state) => *old_backend_state,
            PersistedBackendState::New(backend_state) => backend_state.to_old_lossy(),
        }
    }
}

codegen_convex_serialization!(PersistedBackendState, SerializedBackendState);
