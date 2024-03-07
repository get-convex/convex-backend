use std::collections::BTreeMap;

use value::{
    obj,
    ConvexObject,
    ConvexValue,
    TableName,
    TableNumber,
};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VirtualTableMetadata {
    pub name: TableName,
    pub number: TableNumber,
}

impl VirtualTableMetadata {
    pub fn new(name: TableName, number: TableNumber) -> Self {
        Self { name, number }
    }
}

impl TryFrom<VirtualTableMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: VirtualTableMetadata) -> Result<Self, Self::Error> {
        obj!("name" => String::from(value.name), "number" => (u32::from(value.number) as i64))
    }
}

impl TryFrom<ConvexObject> for VirtualTableMetadata {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = object.into();
        let name = match fields.remove("name") {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!("Invalid name field for VirtualTableMetadata: {:?}", v),
        };

        let number = match fields.remove("number") {
            Some(ConvexValue::Int64(v)) => u32::try_from(v)?.try_into()?,
            v => anyhow::bail!("Invalid number field for VirtualTableMetadata: {:?}", v),
        };

        Ok(Self { name, number })
    }
}

impl TryFrom<ConvexValue> for VirtualTableMetadata {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        match value {
            ConvexValue::Object(o) => o.try_into(),
            _ => anyhow::bail!("Invalid table metadata value"),
        }
    }
}

#[cfg(test)]
mod tests {
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::VirtualTableMetadata;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_table_roundtrips(v in any::<VirtualTableMetadata>()) {
            assert_roundtrips::<VirtualTableMetadata, ConvexObject>(v);
        }
    }
}
