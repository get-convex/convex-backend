use std::collections::BTreeSet;

use enum_iterator::Sequence;
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;

#[derive(
    Copy,
    Clone,
    Debug,
    Deserialize,
    PartialEq,
    Eq,
    Serialize,
    PartialOrd,
    Ord,
    Sequence,
    strum::EnumString,
    strum::Display,
    ToSchema,
)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum DeploymentType {
    Dev,
    Prod,
    Preview,
}

impl DeploymentType {
    pub fn all_types() -> BTreeSet<Self> {
        enum_iterator::all().collect()
    }

    pub fn as_sentry_tag(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::DeploymentType;

    #[test]
    fn test_deployment_type_roundtrips() -> anyhow::Result<()> {
        for d in DeploymentType::all_types() {
            assert_eq!(d.to_string().parse::<DeploymentType>()?, d);
        }
        assert_eq!(DeploymentType::Dev.to_string(), "dev");
        assert_eq!(DeploymentType::Dev.as_sentry_tag(), "dev");
        Ok(())
    }
}
