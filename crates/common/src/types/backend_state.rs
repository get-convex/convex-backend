#[derive(Clone, Copy, Debug, PartialEq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
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
