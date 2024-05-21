use value::FieldPath;

use crate::common::FieldPath as FieldPathProto;

impl From<FieldPath> for FieldPathProto {
    fn from(n: FieldPath) -> Self {
        FieldPathProto {
            fields: Vec::from(n).into_iter().map(|f| f.into()).collect(),
        }
    }
}

impl TryFrom<FieldPathProto> for FieldPath {
    type Error = anyhow::Error;

    fn try_from(value: FieldPathProto) -> Result<Self, Self::Error> {
        FieldPath::new(
            value
                .fields
                .into_iter()
                .map(|f| f.parse())
                .collect::<anyhow::Result<Vec<_>>>()?,
        )
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::FieldPath;
    use crate::common::FieldPath as FieldPathProto;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_field_path_roundtrips(left in any::<FieldPath>()) {
            assert_roundtrips::<FieldPath, FieldPathProto>(left);
        }
    }
}
