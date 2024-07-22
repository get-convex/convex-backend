use std::{
    ops::Deref,
    str::FromStr,
};

use chrono::{
    DateTime,
    Utc,
};
use common::value::{
    convex_object_json_serializer,
    ConvexObject,
    FieldPath,
    IdentifierFieldName,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::constants::{
    ID_FIELD_PATH,
    ID_FIVETRAN_FIELD_NAME,
    METADATA_CONVEX_FIELD_NAME,
    SOFT_DELETE_FIELD_PATH,
    SOFT_DELETE_FIVETRAN_FIELD_NAME,
    SYNCED_FIELD_PATH,
    SYNCED_FIVETRAN_FIELD_NAME,
    UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME,
};

#[derive(
    Hash, Eq, PartialEq, derive_more::Display, Debug, serde::Deserialize, Clone, PartialOrd, Ord,
)]
#[serde(transparent)]
pub struct FivetranTableName(String);

impl FromStr for FivetranTableName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl Deref for FivetranTableName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0[..]
    }
}

#[derive(
    Hash, Eq, PartialEq, derive_more::Display, Debug, serde::Deserialize, Clone, PartialOrd, Ord,
)]
#[serde(transparent)]
pub struct FivetranFieldName(String);

impl FivetranFieldName {
    pub fn is_fivetran_system_field(&self) -> bool {
        self == SYNCED_FIVETRAN_FIELD_NAME.deref()
            || self == SOFT_DELETE_FIVETRAN_FIELD_NAME.deref()
            || self == ID_FIVETRAN_FIELD_NAME.deref()
    }

    /// Returns whether the field is a field starting by `_` which is not
    /// a Fivetran system field. These fields will be stored in
    /// `fivetran.columns` in Convex.
    pub fn is_underscored_field(&self) -> bool {
        self.starts_with('_') && !self.is_fivetran_system_field()
    }
}

impl FromStr for FivetranFieldName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl Deref for FivetranFieldName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0[..]
    }
}

impl TryInto<FieldPath> for FivetranFieldName {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<FieldPath, Self::Error> {
        Ok(if &self == SYNCED_FIVETRAN_FIELD_NAME.deref() {
            SYNCED_FIELD_PATH.clone()
        } else if &self == SOFT_DELETE_FIVETRAN_FIELD_NAME.deref() {
            SOFT_DELETE_FIELD_PATH.clone()
        } else if &self == ID_FIVETRAN_FIELD_NAME.deref() {
            ID_FIELD_PATH.clone()
        } else if let Some(field_name) = self.strip_prefix('_') {
            let field = IdentifierFieldName::from_str(field_name)?;
            FieldPath::new(vec![
                METADATA_CONVEX_FIELD_NAME.clone(),
                UNDERSCORED_COLUMNS_CONVEX_FIELD_NAME.clone(),
                field,
            ])?
        } else {
            let field = IdentifierFieldName::from_str(&self)?;
            FieldPath::for_root_field(field)
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum BatchWriteOperation {
    Upsert,
    Update,
    HardDelete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct BatchWriteRow {
    pub table: String,
    pub operation: BatchWriteOperation,
    #[serde(with = "convex_object_json_serializer")]
    pub row: ConvexObject,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum DeleteType {
    SoftDelete,
    HardDelete,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TruncateTableArgs {
    pub table_name: String,
    pub delete_before: Option<DateTime<Utc>>,
    pub delete_type: DeleteType,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cmd_util::env::env_config;
    use common::value::FieldPath;
    use proptest::prelude::*;

    use crate::api_types::{
        BatchWriteRow,
        FivetranFieldName,
    };

    #[test]
    fn convert_fivetran_user_fields_to_field_path() {
        let expected: FieldPath = FivetranFieldName::from_str("name")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(expected, FieldPath::from_str("name").unwrap());
    }

    #[test]
    fn convert_fivetran_metadata_fields_to_field_path() {
        let expected: FieldPath = FivetranFieldName::from_str("_fivetran_synced")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(expected, FieldPath::from_str("fivetran.synced").unwrap());

        let expected: FieldPath = FivetranFieldName::from_str("_fivetran_id")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(expected, FieldPath::from_str("fivetran.id").unwrap());

        let expected: FieldPath = FivetranFieldName::from_str("_fivetran_deleted")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(expected, FieldPath::from_str("fivetran.deleted").unwrap());
    }

    #[test]
    fn convert_fivetran_fields_starting_with_underscore() {
        let expected: FieldPath = FivetranFieldName::from_str("_file")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(
            expected,
            FieldPath::from_str("fivetran.columns.file").unwrap()
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_object_roundtrips(v in any::<BatchWriteRow>()) {
            let serialized = serde_json::to_string(&v).unwrap();
            let deserialized: BatchWriteRow = serde_json::from_str(&serialized).unwrap();
            assert_eq!(v, deserialized);
        }
    }
}
