use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Clone, Copy, Debug, PartialEq, strum::EnumString, strum::Display, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
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

#[cfg(test)]
mod tests {
    use super::BackendState;

    #[test]
    fn test_backend_state_strum_serde_match() -> anyhow::Result<()> {
        let variants = [
            BackendState::Disabled,
            BackendState::Paused,
            BackendState::Running,
            BackendState::Suspended,
        ];
        for variant in variants {
            let strum_str = variant.to_string();
            let serde_json = serde_json::to_string(&variant)?;
            // serde serializes strings with surrounding quotes
            let serde_str = serde_json.trim_matches('"');
            assert_eq!(strum_str, serde_str, "Mismatch for {variant:?}");
            // Also verify round-trip via strum
            let parsed: BackendState = strum_str.parse()?;
            assert_eq!(parsed, variant);
        }
        Ok(())
    }
}
