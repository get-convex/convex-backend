use uuid::Uuid;
use value::ConvexValue;

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(
    Clone, Debug, Eq, PartialEq, Ord, PartialOrd, derive_more::Display, derive_more::FromStr,
)]
pub struct StorageUuid(
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "Uuid::new_v4 as fn() -> Uuid")
    )]
    Uuid,
);

impl From<Uuid> for StorageUuid {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl TryFrom<StorageUuid> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(s: StorageUuid) -> anyhow::Result<Self> {
        s.to_string().try_into()
    }
}

impl TryFrom<ConvexValue> for StorageUuid {
    type Error = anyhow::Error;

    fn try_from(v: ConvexValue) -> anyhow::Result<Self> {
        match v {
            ConvexValue::String(s) => Ok(StorageUuid(Uuid::try_parse(&s)?)),
            _ => anyhow::bail!("Can only convert Value::String to StorageUuid"),
        }
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::ConvexValue;

    use crate::{
        testing::assert_roundtrips,
        types::StorageUuid,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_storage_roundtrip(v in any::<StorageUuid>()) {
            assert_roundtrips::<StorageUuid, ConvexValue>(v);
        }

    }
}
