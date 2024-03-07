use std::collections::BTreeMap;

pub use common::types::BackendState;
use value::{
    obj,
    ConvexObject,
    ConvexValue,
};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct PersistedBackendState(pub BackendState);

impl TryFrom<PersistedBackendState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(state: PersistedBackendState) -> anyhow::Result<Self> {
        obj!("state" => state.0.to_string())
    }
}

impl TryFrom<ConvexObject> for PersistedBackendState {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> anyhow::Result<Self> {
        let mut fields: BTreeMap<_, _> = object.into();
        let state = match fields.remove("state") {
            Some(ConvexValue::String(s)) => s.parse()?,
            _ => anyhow::bail!("Missing state field for BackendState: {fields:?}"),
        };
        Ok(Self(state))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::ConvexObject;

    use crate::backend_state::types::PersistedBackendState;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_using_proptest(v in any::<PersistedBackendState>()) {
            assert_roundtrips::<PersistedBackendState, ConvexObject>(v);
        }
    }
}
