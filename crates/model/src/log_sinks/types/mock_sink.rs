use value::{
    obj,
    ConvexObject,
};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct MockSinkConfig {}

impl TryFrom<ConvexObject> for MockSinkConfig {
    type Error = anyhow::Error;

    fn try_from(_value: ConvexObject) -> Result<Self, Self::Error> {
        Ok(MockSinkConfig {})
    }
}

impl TryFrom<MockSinkConfig> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(_value: MockSinkConfig) -> Result<Self, Self::Error> {
        obj!(
            "type" => "mock",
        )
    }
}
