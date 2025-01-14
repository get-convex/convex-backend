use std::{
    collections::BTreeMap,
    path::PathBuf,
};

use common::obj;
use serde::Serialize;
use value::{
    ConvexObject,
    ConvexValue,
};

pub type DatabaseVersion = i64;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DatabaseGlobals {
    pub version: DatabaseVersion,

    /// Prefix to put on the aws bucket/lambda keys to make it unguessable
    pub aws_prefix_secret: String,

    /// Storage used by this backend. Cannot be changed once initialized
    /// None - means that no storage type was specified yet
    pub storage_type: Option<StorageType>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum CreationTimeBackfillState {
    NotStarted = 0,
    IncorrectCreationTime = 1,
    Complete = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum BackfillState {
    NotStarted = 0,
    Complete = 1,
}

struct PersistedStorageType(StorageType);

impl TryFrom<PersistedStorageType> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: PersistedStorageType) -> Result<Self, Self::Error> {
        match value.0 {
            StorageType::S3 { s3_prefix } => obj!(
                "tag" => "s3",
                "s3Prefix" => s3_prefix,
            ),
            StorageType::Local { dir } => obj!(
                "tag" => "local",
                "dir" => dir,
            ),
        }
    }
}

impl TryFrom<ConvexObject> for PersistedStorageType {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.into();
        let tag: String = match object_fields.remove("tag") {
            Some(ConvexValue::String(s)) => s.into(),
            v => anyhow::bail!("Invalid tag field: {v:?}"),
        };
        Ok(PersistedStorageType(match tag.as_str() {
            "s3" => {
                let s3_prefix: String = match object_fields.remove("s3Prefix") {
                    Some(ConvexValue::String(s)) => s.into(),
                    v => anyhow::bail!("Invalid s3Prefix field: {v:?}"),
                };
                StorageType::S3 { s3_prefix }
            },
            "local" => {
                let dir: String = match object_fields.remove("dir") {
                    Some(ConvexValue::String(s)) => s.into(),
                    v => anyhow::bail!("Invalid dir field: {v:?}"),
                };
                StorageType::Local { dir }
            },
            v => anyhow::bail!("Invalid tag field: {v:?}"),
        }))
    }
}

impl TryFrom<DatabaseGlobals> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: DatabaseGlobals) -> Result<Self, Self::Error> {
        obj!(
            "version" => value.version,
            "awsPrefixSecret" => value.aws_prefix_secret,
            "storageType" => match value.storage_type {
                Some(storage_type) => ConvexValue::Object(
                    PersistedStorageType(storage_type).try_into()?
                ),
                None => ConvexValue::Null,
            },
        )
    }
}

impl TryFrom<ConvexObject> for DatabaseGlobals {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.into();
        let version = match object_fields.remove("version") {
            Some(ConvexValue::Int64(i)) => i,
            _ => anyhow::bail!("Missing 'version' in {object_fields:?}"),
        };
        let aws_prefix_secret = match object_fields.remove("awsPrefixSecret") {
            Some(ConvexValue::String(s)) => s.into(),
            v => anyhow::bail!("Invalid awsPrefixSecret field: {v:?}"),
        };
        let storage_type = match object_fields.remove("storageType") {
            Some(ConvexValue::Object(o)) => Some(PersistedStorageType::try_from(o)?.0),
            Some(ConvexValue::Null) => None,
            // TODO - remove this None handling once we reach v36
            None => None,
            v => anyhow::bail!("Invalid storageTag field: {v:?}"),
        };

        Ok(Self {
            version,
            aws_prefix_secret,
            storage_type,
        })
    }
}

#[derive(Clone, Debug)]
pub enum StorageTagInitializer {
    S3,
    Local { dir: PathBuf },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum StorageType {
    S3 { s3_prefix: String },
    Local { dir: String },
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::DatabaseGlobals;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_database_globals_roundtrip(v in any::<DatabaseGlobals>()) {
            assert_roundtrips::<DatabaseGlobals, ConvexObject>(v);
        }
    }
}
