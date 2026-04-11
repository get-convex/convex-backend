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
pub enum BackendState {
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

impl BackendState {
    pub fn is_stopped(&self) -> bool {
        matches!(
            self,
            BackendState::Disabled | BackendState::Paused | BackendState::Suspended
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewBackendState {
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

    /// The backend is stopped because the user went over the free plan limits
    /// spending limits, but the user can now manually resume it.
    Resumable,

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

impl NewBackendState {
    pub fn to_old_lossy(self) -> BackendState {
        match (self.system, self.user) {
            (SystemStopState::Disabled, _) => BackendState::Disabled,
            (SystemStopState::Suspended, _) => BackendState::Suspended,
            (SystemStopState::Resumable, _) => BackendState::Paused,
            (SystemStopState::None, UserStopState::Paused) => BackendState::Paused,
            (SystemStopState::None, UserStopState::None) => BackendState::Running,
        }
    }
}

impl BackendState {
    pub fn user_state(&self) -> UserStopState {
        match self {
            BackendState::Paused => UserStopState::Paused,
            _ => UserStopState::None,
        }
    }

    pub fn system_state(&self) -> SystemStopState {
        match self {
            BackendState::Disabled => SystemStopState::Disabled,
            BackendState::Suspended => SystemStopState::Suspended,
            _ => SystemStopState::None,
        }
    }
}

impl NewBackendState {
    pub fn is_stopped(&self) -> bool {
        self.system != SystemStopState::None || self.user != UserStopState::None
    }
}
