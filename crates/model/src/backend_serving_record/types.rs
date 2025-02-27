use std::collections::BTreeMap;

use value::{
    obj,
    ConvexObject,
    ConvexValue,
};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct BackendServingRecord {
    // We use string instead of Url so it is easier to implement Arbitrary.
    pub preempt_url: String,
}

impl TryFrom<BackendServingRecord> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(backend: BackendServingRecord) -> anyhow::Result<Self> {
        obj!(
            "preemptUrl" => backend.preempt_url,
        )
    }
}

impl TryFrom<ConvexObject> for BackendServingRecord {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();

        let preempt_url = match fields.remove("preemptUrl") {
            Some(ConvexValue::String(s)) => s.into(),
            _ => anyhow::bail!(
                "Missing or invalid `preemptUrl` field for BackendServingRecord: {:?}",
                fields
            ),
        };

        Ok(Self { preempt_url })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::ConvexObject;

    use crate::backend_serving_record::types::BackendServingRecord;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_backend_serving_record_roundtrips(v in any::<BackendServingRecord>()) {
            assert_roundtrips::<BackendServingRecord, ConvexObject>(v);
        }
    }
}
