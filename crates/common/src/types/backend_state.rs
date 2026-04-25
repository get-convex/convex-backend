use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Clone, Copy, Debug, PartialEq, strum::EnumString, strum::Display, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
/// Represents the different states a backend can be in.
pub enum OldBackendState {
    /// Disabled - will not serve any requests. Set when exceeds the allowed
    /// usage based on the tier etc. May leave this state after some time.
    Disabled,
    /// Paused by the user. Will not leave this state until the user explicitly
    /// unpauses.
    Paused,
    /// Running - will serve requests.
    Running,
    /// Suspended - will not serve any requests. Set by big brain tool. May
    /// leave this state only by admin command.
    Suspended,
}

impl OldBackendState {
    pub fn is_stopped(&self) -> bool {
        matches!(
            self,
            OldBackendState::Disabled | OldBackendState::Paused | OldBackendState::Suspended
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendState {
    pub system: SystemStopState,
    pub user: UserStopState,
}

/// Indicates whether the backend has been stopped for a system-initiated reason
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum SystemStopState {
    None,

    /// Stopped by going over the free plan limits or spending limits
    Disabled,

    /// Stopped manually by an admin
    Suspended,
}

/// Indicates whether the backend has been paused by the user
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum UserStopState {
    None,

    /// The user paused the backend
    Paused,
}

impl BackendState {
    pub fn to_old_lossy(self) -> OldBackendState {
        match (self.system, self.user) {
            (SystemStopState::Disabled, _) => OldBackendState::Disabled,
            (SystemStopState::Suspended, _) => OldBackendState::Suspended,
            (SystemStopState::None, UserStopState::Paused) => OldBackendState::Paused,
            (SystemStopState::None, UserStopState::None) => OldBackendState::Running,
        }
    }
}

impl OldBackendState {
    pub fn user_state(&self) -> UserStopState {
        match self {
            OldBackendState::Paused => UserStopState::Paused,
            _ => UserStopState::None,
        }
    }

    pub fn system_state(&self) -> SystemStopState {
        match self {
            OldBackendState::Disabled => SystemStopState::Disabled,
            OldBackendState::Suspended => SystemStopState::Suspended,
            _ => SystemStopState::None,
        }
    }
}

impl BackendState {
    pub fn is_stopped(&self) -> bool {
        self.system != SystemStopState::None || self.user != UserStopState::None
    }
}
