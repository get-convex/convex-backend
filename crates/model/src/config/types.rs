//! Top-level configuration state registered by the user.

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    path::{
        Component,
        PathBuf,
    },
};

use common::{
    auth::AuthInfo,
    components::CanonicalizedComponentModulePath,
    obj,
    schemas::DatabaseSchema,
    types::ModuleEnvironment,
};
use database::LegacyIndexDiff;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sync_types::{
    module_path::ACTIONS_DIR,
    ModulePath,
};
use value::{
    remove_nullable_object,
    remove_object,
    remove_string,
    remove_vec_of_strings,
    val,
    ConvexArray,
    ConvexObject,
    ConvexValue,
};

use crate::{
    auth::types::{
        AuthDiff,
        AuthInfoPersisted,
    },
    cron_jobs::types::CronIdentifier,
    modules::module_versions::{
        ModuleSource,
        SourceMap,
    },
};

/// User-specified module definition. See [`ModuleMetadata`] and associated
/// structs for the corresponding module metadata used internally by the system.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ModuleConfig {
    /// Relative path to the module.
    pub path: ModulePath,
    /// Module source.
    pub source: ModuleSource,
    /// The module's source map (if available).
    pub source_map: Option<SourceMap>,
    /// The environment is bundled to run in.
    pub environment: ModuleEnvironment,
}

/// This is not safe to use since convex 0.12.0, where we allow defining actions
/// outside of the /actions subfolder. This method should only be used for old
/// cli clients and source packages where environment is not set.
pub fn deprecated_extract_environment_from_path(p: String) -> anyhow::Result<ModuleEnvironment> {
    let path = PathBuf::from(&p);
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(c) => c
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Path {p} contains an invalid Unicode character")),
            Component::RootDir => {
                anyhow::bail!("Module paths must be relative ({p} is absolute)")
            },
            c => anyhow::bail!("Invalid path component {c:?} in {p}"),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    // This is the old way to indicate a module should execute in Node.js.
    let environment = if matches!(&components[..], &[ACTIONS_DIR, ..]) {
        ModuleEnvironment::Node
    } else {
        ModuleEnvironment::Isolate
    };
    Ok(environment)
}

pub const AUTH_CONFIG_FILE_NAME: &str = "auth.config.js";

/// Representation of Convex config metadata deployed by the client. This
/// metadata isn't written to a table but is instead normalized and represented
/// by state in the other metadata tables.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ConfigMetadata {
    /// The local directory on the client containing modules.
    pub functions: String,
    /// Authentication info. Empty if this instance has not set up
    /// authentication.
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "proptest::collection::vec(proptest::prelude::any::<AuthInfo>(), 0..4)"
        )
    )]
    pub auth_info: Vec<AuthInfo>,
}

impl ConfigMetadata {
    /// Create new empty config metadata for a new instance.
    pub fn new() -> Self {
        Self {
            functions: "convex/".to_string(),
            auth_info: vec![],
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn test_example() -> Self {
        Self {
            functions: "convex/".to_string(),
            auth_info: vec![AuthInfo::test_example()],
        }
    }

    pub fn from_file(file: ConfigFile, auth_info: Vec<AuthInfo>) -> Self {
        Self {
            functions: file.functions,
            auth_info,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    pub functions: String,
    // Deprecated, moved to AuthConfig.providers
    pub auth_info: Option<Vec<AuthInfo>>,
}

impl TryFrom<ConfigMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(m: ConfigMetadata) -> anyhow::Result<Self> {
        let mut config = BTreeMap::new();
        config.insert("functions".parse()?, ConvexValue::try_from(m.functions)?);

        // The auth config was moved from `authInfo` to `auth.config.js` in modules,
        // do not include it in the config response if it is empty.
        if !m.auth_info.is_empty() {
            let auth_info = m
                .auth_info
                .into_iter()
                .map(|v| Ok(ConvexObject::try_from(AuthInfoPersisted(v))?.into()))
                .collect::<anyhow::Result<Vec<ConvexValue>>>()?
                .try_into()?;
            config.insert("authInfo".parse()?, auth_info);
        }
        config.try_into()
    }
}

impl TryFrom<ConfigMetadata> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: ConfigMetadata) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for ConfigMetadata {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = o.into();
        let functions = match fields.remove("functions") {
            Some(ConvexValue::String(s)) => s.into(),
            _ => anyhow::bail!(
                "Missing or invalid functions field for ConfigMetadata: {:?}",
                fields,
            ),
        };
        let auth_info = match fields.remove("authInfo") {
            Some(v) => ConvexArray::try_from(v)?
                .into_iter()
                .map(|v| {
                    let parsed: AuthInfoPersisted = ConvexObject::try_from(v)?.try_into()?;
                    Ok(parsed.0)
                })
                .collect::<anyhow::Result<Vec<AuthInfo>>>()?,
            _ => vec![],
        };
        Ok(Self {
            functions,
            auth_info,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct ConfigDiff {
    pub auth_diff: AuthDiff,
    pub udf_server_version_diff: Option<UdfServerVersionDiff>,
    pub module_diff: ModuleDiff,
    pub cron_diff: CronDiff,
    pub index_diff: ConfigIndexDiff,
    pub schema_diff: Option<SchemaDiff>,
}

impl TryFrom<ConfigDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: ConfigDiff) -> Result<Self, Self::Error> {
        let server_version_value: ConvexValue = match value.udf_server_version_diff {
            Some(server_version_diff) => {
                ConvexValue::from(ConvexObject::try_from(server_version_diff)?)
            },
            None => ConvexValue::Null,
        };
        obj!(
            "auth" => ConvexObject::try_from(value.auth_diff)?,
            "server_version" => server_version_value,
            "modules" => ConvexObject::try_from(value.module_diff)?,
            "crons" => ConvexObject::try_from(value.cron_diff)?,
            "indexes" => ConvexObject::try_from(value.index_diff)?,
            "schema" => match value.schema_diff {
                Some(schema_diff) => ConvexObject::try_from(schema_diff)?.into(),
                None => ConvexValue::Null,
            },
        )
    }
}

impl TryFrom<ConvexObject> for ConfigDiff {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        Ok(Self {
            auth_diff: remove_object(&mut fields, "auth")?,
            udf_server_version_diff: remove_nullable_object(&mut fields, "server_version")?,
            module_diff: remove_object(&mut fields, "modules")?,
            cron_diff: remove_nullable_object(&mut fields, "crons")?.unwrap_or_default(),
            index_diff: remove_nullable_object(&mut fields, "indexes")?.unwrap_or_default(),
            schema_diff: remove_nullable_object(&mut fields, "schema")?,
        })
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct ConfigIndexDiff {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::vec(
            proptest::prelude::any::<String>(),
            0..4,
        )")
    )]
    pub added: Vec<String>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::collection::vec(
            proptest::prelude::any::<String>(),
            0..4,
        )")
    )]
    pub dropped: Vec<String>,
}

impl From<LegacyIndexDiff> for ConfigIndexDiff {
    fn from(value: LegacyIndexDiff) -> Self {
        Self {
            added: value
                .added
                .into_iter()
                .map(|index_metadata| index_metadata.name.to_string())
                .collect(),
            dropped: value
                .dropped
                .into_iter()
                .map(|index_metadata| index_metadata.name.to_string())
                .collect(),
        }
    }
}

impl TryFrom<ConfigIndexDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: ConfigIndexDiff) -> Result<Self, Self::Error> {
        let added_values: Vec<ConvexValue> = value
            .added
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        let deleted_values: Vec<ConvexValue> = value
            .dropped
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        obj!("added" => added_values, "dropped" => deleted_values)
    }
}

impl TryFrom<ConvexObject> for ConfigIndexDiff {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        Ok(Self {
            added: remove_vec_of_strings(&mut fields, "added")?,
            dropped: remove_vec_of_strings(&mut fields, "dropped")?,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct UdfServerVersionDiff {
    pub previous_version: String,
    pub next_version: String,
}
impl TryFrom<UdfServerVersionDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: UdfServerVersionDiff) -> Result<Self, Self::Error> {
        obj!("previous_version" => value.previous_version, "next_version" => value.next_version)
    }
}

impl TryFrom<ConvexObject> for UdfServerVersionDiff {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        Ok(Self {
            previous_version: remove_string(&mut fields, "previous_version")?,
            next_version: remove_string(&mut fields, "next_version")?,
        })
    }
}

#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Default)
)]
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDiff {
    pub added_functions: Vec<String>,
    pub removed_functions: Vec<String>,
}

impl TryFrom<ModuleDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: ModuleDiff) -> Result<Self, Self::Error> {
        let added_values: Vec<ConvexValue> = value
            .added_functions
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        let removed_values: Vec<ConvexValue> = value
            .removed_functions
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        obj!("added" => added_values, "removed" => removed_values)
    }
}

impl TryFrom<ConvexObject> for ModuleDiff {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        Ok(Self {
            added_functions: remove_vec_of_strings(&mut fields, "added")?,
            removed_functions: remove_vec_of_strings(&mut fields, "removed")?,
        })
    }
}

impl ModuleDiff {
    pub fn new(
        added_module_paths: BTreeSet<CanonicalizedComponentModulePath>,
        removed_module_paths: BTreeSet<CanonicalizedComponentModulePath>,
    ) -> anyhow::Result<Self> {
        let mut added_functions = Vec::with_capacity(added_module_paths.len());
        for m in added_module_paths {
            let m = m.into_root_module_path()?;
            if m.is_deps() || m.is_system() {
                continue;
            }
            added_functions.push(m.as_str().to_string());
        }
        let mut removed_functions = Vec::with_capacity(removed_module_paths.len());
        for m in removed_module_paths {
            let m = m.into_root_module_path()?;
            if m.is_deps() || m.is_system() {
                continue;
            }
            removed_functions.push(m.as_str().to_string());
        }
        Ok(Self {
            added_functions,
            removed_functions,
        })
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct CronDiff {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub deleted: Vec<String>,
}

impl TryFrom<CronDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: CronDiff) -> Result<Self, Self::Error> {
        let added_values: Vec<ConvexValue> = value
            .added
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        let updated_values: Vec<ConvexValue> = value
            .updated
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        let deleted_values: Vec<ConvexValue> = value
            .deleted
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<anyhow::Result<Vec<ConvexValue>>>()?;
        obj!("added" => added_values, "updated" => updated_values, "deleted" => deleted_values)
    }
}

impl TryFrom<ConvexObject> for CronDiff {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        Ok(Self {
            added: remove_vec_of_strings(&mut fields, "added")?,
            updated: remove_vec_of_strings(&mut fields, "updated")?,
            deleted: remove_vec_of_strings(&mut fields, "deleted")?,
        })
    }
}

impl CronDiff {
    pub fn new(
        added_crons: Vec<&CronIdentifier>,
        updated_crons: Vec<&CronIdentifier>,
        deleted_crons: Vec<&CronIdentifier>,
    ) -> Self {
        Self {
            added: added_crons.into_iter().map(|c| c.to_string()).collect(),
            updated: updated_crons.into_iter().map(|c| c.to_string()).collect(),
            deleted: deleted_crons.into_iter().map(|c| c.to_string()).collect(),
        }
    }
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDiff {
    pub previous_schema: Option<DatabaseSchema>,
    pub next_schema: Option<DatabaseSchema>,
}
impl TryFrom<SchemaDiff> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(
        SchemaDiff {
            previous_schema,
            next_schema,
        }: SchemaDiff,
    ) -> Result<Self, Self::Error> {
        obj!(
            "previous_schema" => match previous_schema {
            Some(previous_schema) => val!(
                serde_json::to_string(&JsonValue::try_from(previous_schema)?)?
            ),
                None => val!(null),
            },
            "next_schema" => match next_schema {
                Some(next_schema) => val!(
                    serde_json::to_string(&JsonValue::try_from(next_schema)?)?
                ),
                None => val!(null),
            },
        )
    }
}

impl TryFrom<ConvexObject> for SchemaDiff {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = o.into();
        let previous_schema = match fields.remove("previous_schema") {
            Some(ConvexValue::String(s)) => {
                let json_value: JsonValue = serde_json::from_str(&s)?;
                Some(DatabaseSchema::try_from(json_value)?)
            },
            Some(ConvexValue::Null) => None,
            _ => anyhow::bail!("Invalid previous_schema field"),
        };
        let next_schema = match fields.remove("next_schema") {
            Some(ConvexValue::String(s)) => {
                let json_value: JsonValue = serde_json::from_str(&s)?;
                Some(DatabaseSchema::try_from(json_value)?)
            },
            Some(ConvexValue::Null) => None,
            _ => anyhow::bail!("Invalid next_schema field"),
        };
        Ok(Self {
            previous_schema,
            next_schema,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::{
        ConfigDiff,
        ConfigMetadata,
    };
    use crate::config::types::SchemaDiff;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_config_metadata_roundtrips(v in any::<ConfigMetadata>()) {
            assert_roundtrips::<ConfigMetadata, ConvexObject>(v);
        }

        #[test]
        fn test_config_diff_to_object(v in any::<ConfigDiff>()) {
            ConvexObject::try_from(v).unwrap();
        }

        #[test]
        fn test_schema_diff_roundtrip(v in any::<SchemaDiff>()) {
            assert_roundtrips::<SchemaDiff, ConvexObject>(v);
        }
    }
}
