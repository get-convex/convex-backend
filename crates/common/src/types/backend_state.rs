use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Clone, Copy, Debug, PartialEq, strum::EnumString, strum::Display, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
// TODO(nicolas) remove once /change_deployment_state_system is removed
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendState {
    pub system: SystemStopState,
    pub usage_limit: UsageLimitStopState,
    pub user: UserStopState,
}

/// Indicates whether the backend has been stopped for a system-initiated reason
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::EnumString, strum::Display, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
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

/// Indicates whether the backend has been stopped by a configured usage limit.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::EnumString, strum::Display, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UsageLimitStopState {
    None,

    /// Stopped by exceeding a deployment usage limit.
    Disabled,
}

impl BackendState {
    // TODO(nicolas) Remove once consistency-check is updated to stop using
    // /get_deployment_state
    pub fn to_old_lossy(self) -> OldBackendState {
        match (self.system, self.user) {
            (SystemStopState::Disabled, _) => OldBackendState::Disabled,
            (SystemStopState::Suspended, _) => OldBackendState::Suspended,
            (SystemStopState::None, UserStopState::Paused) => OldBackendState::Paused,
            (SystemStopState::None, UserStopState::None) => OldBackendState::Running,
        }
    }
}

impl BackendState {
    pub fn is_stopped(&self) -> bool {
        self.system != SystemStopState::None
            || self.usage_limit != UsageLimitStopState::None
            || self.user != UserStopState::None
    }
}
