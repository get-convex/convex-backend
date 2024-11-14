use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::LazyLock,
};

use common::{
    bootstrap_model::index::{
        DeveloperIndexConfig,
        IndexMetadata,
        SerializedDeveloperIndexConfig,
        SerializedNamedDeveloperIndexConfig,
    },
    components::ComponentPath,
    log_streaming::{
        LogEvent,
        StructuredLogEvent,
    },
    runtime::UnixTimestamp,
    types::{
        GenericIndexName,
        IndexDiff,
        IndexName,
    },
};
use database::LegacyIndexDiff;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    codegen_convex_serialization,
    obj,
    remove_int64,
    remove_nullable_int64,
    remove_nullable_object,
    remove_nullable_vec_of_strings,
    remove_object,
    remove_string,
    remove_vec,
    remove_vec_of_strings,
    ConvexObject,
    ConvexValue,
    TableName,
};

use crate::{
    auth::types::AuthDiff,
    backend_state::types::BackendState,
    components::config::{
        ComponentDiff,
        SerializedComponentDiff,
    },
    config::types::ConfigDiff,
    environment_variables::types::EnvVarName,
    snapshot_imports::types::{
        ImportFormat,
        ImportMode,
        ImportRequestor,
    },
};

pub static DEPLOYMENT_AUDIT_LOG_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_deployment_audit_log"
        .parse()
        .expect("Invalid deployment audit log table")
});

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AuditLogIndexDiff {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::collection::vec(any::<(IndexName, DeveloperIndexConfig)>(), 0..4)"
        )
    )]
    pub added_indexes: Vec<(IndexName, DeveloperIndexConfig)>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::collection::vec(any::<(IndexName, DeveloperIndexConfig)>(), 0..4)"
        )
    )]
    pub removed_indexes: Vec<(IndexName, DeveloperIndexConfig)>,
}

impl From<IndexDiff> for AuditLogIndexDiff {
    fn from(diff: IndexDiff) -> Self {
        let added_indexes = diff
            .added
            .into_iter()
            .map(|IndexMetadata::<TableName> { name, config }| (name, config.into()))
            .collect();
        let removed_indexes = diff
            .dropped
            .into_iter()
            .map(|index| (index.name.clone(), index.into_value().config.into()))
            .collect();
        Self {
            added_indexes,
            removed_indexes,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub enum DeploymentAuditLogEvent {
    CreateEnvironmentVariable {
        name: EnvVarName,
    },
    UpdateEnvironmentVariable {
        name: EnvVarName,
    },
    DeleteEnvironmentVariable {
        name: EnvVarName,
    },
    ReplaceEnvironmentVariable {
        previous_name: EnvVarName,
        name: EnvVarName,
    },
    PushConfig {
        config_diff: ConfigDiff,
    },
    PushConfigWithComponents {
        diffs: PushComponentDiffs,
    },
    BuildIndexes {
        #[cfg_attr(
            any(test, feature = "testing"),
            proptest(strategy = "prop::collection::vec(any::<(IndexName, \
                                 DeveloperIndexConfig)>(), 0..4)")
        )]
        added_indexes: Vec<(IndexName, DeveloperIndexConfig)>,
        #[cfg_attr(
            any(test, feature = "testing"),
            proptest(strategy = "prop::collection::vec(any::<(IndexName, \
                                 DeveloperIndexConfig)>(), 0..4)")
        )]
        removed_indexes: Vec<(IndexName, DeveloperIndexConfig)>,
    },
    ChangeDeploymentState {
        old_state: BackendState,
        new_state: BackendState,
    },
    // TODO: consider adding table names once this is logged for more places
    // and we have a story about limiting size.
    ClearTables,
    SnapshotImport {
        table_names: Vec<TableName>,
        table_count: u64,
        import_mode: ImportMode,
        import_format: ImportFormat,
        requestor: ImportRequestor,
        table_names_deleted: Vec<TableName>,
        table_count_deleted: u64,
    },
}

impl From<LegacyIndexDiff> for DeploymentAuditLogEvent {
    fn from(value: LegacyIndexDiff) -> Self {
        let added_indexes = value
            .added
            .into_iter()
            .map(|m| (m.name, DeveloperIndexConfig::from(m.config)))
            .collect();

        let removed_indexes = value
            .dropped
            .into_iter()
            .map(|d| {
                (
                    d.name.clone(),
                    DeveloperIndexConfig::from(d.into_value().config),
                )
            })
            .collect();

        DeploymentAuditLogEvent::BuildIndexes {
            added_indexes,
            removed_indexes,
        }
    }
}

impl DeploymentAuditLogEvent {
    pub fn action(&self) -> &'static str {
        match self {
            DeploymentAuditLogEvent::CreateEnvironmentVariable { .. } => {
                "create_environment_variable"
            },
            DeploymentAuditLogEvent::UpdateEnvironmentVariable { .. } => {
                "update_environment_variable"
            },
            DeploymentAuditLogEvent::DeleteEnvironmentVariable { .. } => {
                "delete_environment_variable"
            },
            DeploymentAuditLogEvent::ReplaceEnvironmentVariable { .. } => {
                "replace_environment_variable"
            },
            DeploymentAuditLogEvent::PushConfig { .. } => "push_config",
            DeploymentAuditLogEvent::PushConfigWithComponents { .. } => {
                "push_config_with_components"
            },
            DeploymentAuditLogEvent::BuildIndexes { .. } => "build_indexes",
            DeploymentAuditLogEvent::ChangeDeploymentState { .. } => "change_deployment_state",
            DeploymentAuditLogEvent::SnapshotImport { .. } => "snapshot_import",
            DeploymentAuditLogEvent::ClearTables => "clear_tables",
        }
    }

    fn metadata(self) -> anyhow::Result<ConvexObject> {
        match self {
            DeploymentAuditLogEvent::CreateEnvironmentVariable { name }
            | DeploymentAuditLogEvent::UpdateEnvironmentVariable { name }
            | DeploymentAuditLogEvent::DeleteEnvironmentVariable { name } => {
                obj!("variable_name" => name.to_string())
            },
            DeploymentAuditLogEvent::ReplaceEnvironmentVariable {
                previous_name,
                name,
            } => {
                obj!("variable_name" => name.to_string(), "previous_variable_name" => previous_name.to_string())
            },
            DeploymentAuditLogEvent::PushConfig { config_diff } => {
                ConvexObject::try_from(config_diff)
            },
            DeploymentAuditLogEvent::PushConfigWithComponents { diffs } => diffs.try_into(),
            DeploymentAuditLogEvent::BuildIndexes {
                added_indexes,
                removed_indexes,
            } => {
                let added_indexes_metadata: Vec<ConvexValue> = added_indexes
                    .into_iter()
                    .map(|(name, config)| {
                        let config_value = ConvexValue::try_from(config)?;
                        let name_value = ConvexValue::try_from(name.to_string())?;
                        let metadata_value = match config_value {
                            ConvexValue::Object(o) => {
                                ConvexValue::Object(o.shallow_merge(obj!("name" => name_value)?)?)
                            },
                            _ => anyhow::bail!("Expected config value to be an object"),
                        };
                        Ok(metadata_value)
                    })
                    .collect::<anyhow::Result<Vec<ConvexValue>>>()?;

                let removed_indexes_metadata: Vec<ConvexValue> = removed_indexes
                    .into_iter()
                    .map(|(name, config)| {
                        let config_value = ConvexValue::try_from(config)?;
                        let name_value = ConvexValue::try_from(name.to_string())?;
                        let metadata_value = match config_value {
                            ConvexValue::Object(o) => {
                                ConvexValue::Object(o.shallow_merge(obj!("name" => name_value)?)?)
                            },
                            _ => anyhow::bail!("Expected config value to be an object"),
                        };
                        Ok(metadata_value)
                    })
                    .collect::<anyhow::Result<Vec<ConvexValue>>>()?;

                obj!(
                    "added_indexes" => added_indexes_metadata,
                    "removed_indexes" => removed_indexes_metadata
                )
            },
            DeploymentAuditLogEvent::ChangeDeploymentState {
                old_state,
                new_state,
            } => {
                obj!("old_state" => old_state.to_string(), "new_state" => new_state.to_string())
            },
            DeploymentAuditLogEvent::SnapshotImport {
                table_names,
                table_count,
                import_mode,
                import_format,
                requestor,
                table_names_deleted,
                table_count_deleted,
            } => {
                let table_names: Vec<_> = table_names
                    .into_iter()
                    .map(|table_name| {
                        anyhow::Ok(ConvexValue::String(table_name.to_string().try_into()?))
                    })
                    .try_collect()?;
                let table_names_deleted: Vec<_> = table_names_deleted
                    .into_iter()
                    .map(|table_name| {
                        anyhow::Ok(ConvexValue::String(table_name.to_string().try_into()?))
                    })
                    .try_collect()?;
                obj!(
                    "table_names" => table_names,
                    "table_count" => table_count as i64,
                    "import_mode" => import_mode.to_string(),
                    "import_format" => ConvexObject::try_from(import_format)?,
                    "requestor" => ConvexObject::try_from(requestor)?,
                    "table_names_deleted" => table_names_deleted,
                    "table_count_deleted" => table_count_deleted as i64,
                )
            },
            DeploymentAuditLogEvent::ClearTables => obj!(),
        }
    }

    pub fn to_log_event(
        event: DeploymentAuditLogEvent,
        timestamp: UnixTimestamp,
    ) -> anyhow::Result<LogEvent> {
        let action = event.action().to_string();
        let JsonValue::Object(metadata_fields) = event.metadata()?.into() else {
            anyhow::bail!("DeploymentAuditLogEvent metdata was not a JSON object")
        };
        Ok(LogEvent {
            timestamp,
            event: StructuredLogEvent::DeploymentAuditLog {
                action,
                metadata: metadata_fields,
            },
        })
    }
}

impl TryFrom<DeploymentAuditLogEvent> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: DeploymentAuditLogEvent) -> anyhow::Result<Self> {
        obj!("action" => value.action(), "metadata" => value.metadata()?)
    }
}

fn value_to_index_metadata(
    value: ConvexValue,
) -> anyhow::Result<(IndexName, DeveloperIndexConfig)> {
    let obj = ConvexObject::try_from(value)?;
    let mut fields = BTreeMap::from(obj);
    let name = remove_string(&mut fields, "name")?;
    let obj = ConvexObject::try_from(fields)?;
    let developer_index_config = obj.try_into()?;
    Ok((IndexName::from_str(&name)?, developer_index_config))
}

impl TryFrom<ConvexObject> for DeploymentAuditLogEvent {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields = BTreeMap::from(obj);
        let action: String = remove_string(&mut fields, "action")?;
        let metadata: ConvexObject = remove_object(&mut fields, "metadata")?;
        let mut fields = BTreeMap::from(metadata);
        let event = match &*action {
            "create_environment_variable" => DeploymentAuditLogEvent::CreateEnvironmentVariable {
                name: remove_string(&mut fields, "variable_name")?.parse()?,
            },
            "delete_environment_variable" => DeploymentAuditLogEvent::DeleteEnvironmentVariable {
                name: remove_string(&mut fields, "variable_name")?.parse()?,
            },
            "update_environment_variable" => DeploymentAuditLogEvent::UpdateEnvironmentVariable {
                name: remove_string(&mut fields, "variable_name")?.parse()?,
            },
            "replace_environment_variable" => DeploymentAuditLogEvent::ReplaceEnvironmentVariable {
                previous_name: remove_string(&mut fields, "previous_variable_name")?.parse()?,
                name: remove_string(&mut fields, "variable_name")?.parse()?,
            },
            "push_config" => DeploymentAuditLogEvent::PushConfig {
                config_diff: ConvexObject::try_from(fields)?.try_into()?,
            },
            "push_config_with_components" => DeploymentAuditLogEvent::PushConfigWithComponents {
                diffs: ConvexObject::try_from(fields)?.try_into()?,
            },
            "build_indexes" => {
                let added_indexes = remove_vec(&mut fields, "added_indexes")?
                    .into_iter()
                    .map(value_to_index_metadata)
                    .try_collect()?;
                let removed_indexes = remove_vec(&mut fields, "removed_indexes")?
                    .into_iter()
                    .map(value_to_index_metadata)
                    .try_collect()?;
                DeploymentAuditLogEvent::BuildIndexes {
                    added_indexes,
                    removed_indexes,
                }
            },
            "change_deployment_state" => DeploymentAuditLogEvent::ChangeDeploymentState {
                old_state: remove_string(&mut fields, "old_state")?.parse()?,
                new_state: remove_string(&mut fields, "new_state")?.parse()?,
            },
            "clear_tables" => DeploymentAuditLogEvent::ClearTables,
            "snapshot_import" => {
                let table_names = remove_vec_of_strings(&mut fields, "table_names")?
                    .iter()
                    .map(|s| TableName::from_str(s))
                    .try_collect()?;
                let table_names_deleted =
                    remove_nullable_vec_of_strings(&mut fields, "table_names_deleted")?
                        .unwrap_or_default()
                        .iter()
                        .map(|s| TableName::from_str(s))
                        .try_collect()?;
                DeploymentAuditLogEvent::SnapshotImport {
                    table_names,
                    table_count: remove_int64(&mut fields, "table_count")? as u64,
                    import_mode: remove_string(&mut fields, "import_mode")?.parse()?,
                    import_format: remove_object(&mut fields, "import_format")?,
                    requestor: remove_nullable_object(&mut fields, "requestor")?
                        .unwrap_or(ImportRequestor::SnapshotImport),
                    table_names_deleted,
                    table_count_deleted: remove_nullable_int64(&mut fields, "table_count_deleted")?
                        .unwrap_or(0) as u64,
                }
            },
            _ => anyhow::bail!("action {action} unrecognized"),
        };
        Ok(event)
    }
}

impl TryFrom<DeploymentAuditLogEvent> for serde_json::Map<String, JsonValue> {
    type Error = anyhow::Error;

    fn try_from(value: DeploymentAuditLogEvent) -> anyhow::Result<Self> {
        let mut map = serde_json::Map::new();
        let action = value.action();
        map.insert("action".to_string(), action.into());
        map.insert(
            "actionMetadata".to_string(),
            // Note that this serialization format might be ugly for certain types until the clean
            // export serialization project is complete.
            value.metadata()?.into(),
        );
        Ok(map)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SerializedIndexDiff {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::vec(any::<
                             SerializedNamedDeveloperIndexConfig>(), 0..4)")
    )]
    pub added_indexes: Vec<SerializedNamedDeveloperIndexConfig>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::vec(any::<
                             SerializedNamedDeveloperIndexConfig>(), 0..4)")
    )]
    pub removed_indexes: Vec<SerializedNamedDeveloperIndexConfig>,
}

impl TryFrom<AuditLogIndexDiff> for SerializedIndexDiff {
    type Error = anyhow::Error;

    fn try_from(diff: AuditLogIndexDiff) -> anyhow::Result<Self> {
        let convert_to_serialized =
            |indexes: Vec<(GenericIndexName<TableName>, DeveloperIndexConfig)>| {
                indexes
                    .into_iter()
                    .map(|(name, config)| {
                        let name = name.to_string();
                        let index_config = SerializedDeveloperIndexConfig::try_from(config)?;
                        anyhow::Ok(SerializedNamedDeveloperIndexConfig { name, index_config })
                    })
                    .try_collect()
            };
        let added_indexes = convert_to_serialized(diff.added_indexes)?;
        let removed_indexes = convert_to_serialized(diff.removed_indexes)?;
        Ok(Self {
            added_indexes,
            removed_indexes,
        })
    }
}

impl TryFrom<SerializedIndexDiff> for AuditLogIndexDiff {
    type Error = anyhow::Error;

    fn try_from(diff: SerializedIndexDiff) -> anyhow::Result<Self> {
        let convert_to_index_metadata = |indexes: Vec<SerializedNamedDeveloperIndexConfig>| {
            indexes
                .into_iter()
                .map(
                    |SerializedNamedDeveloperIndexConfig { name, index_config }| {
                        anyhow::Ok((name.parse()?, index_config.try_into()?))
                    },
                )
                .try_collect()
        };
        let added_indexes = convert_to_index_metadata(diff.added_indexes)?;
        let removed_indexes = convert_to_index_metadata(diff.removed_indexes)?;
        Ok(Self {
            added_indexes,
            removed_indexes,
        })
    }
}
#[derive(Clone, Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct PushComponentDiffs {
    pub auth_diff: AuthDiff,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::btree_map(any::<ComponentPath>(), \
                             any::<ComponentDiff>(), 0..4)")
    )]
    pub component_diffs: BTreeMap<ComponentPath, ComponentDiff>,
}

impl TryFrom<SerializedPushComponentDiffs> for PushComponentDiffs {
    type Error = anyhow::Error;

    fn try_from(value: SerializedPushComponentDiffs) -> anyhow::Result<Self> {
        let component_diffs = value
            .component_diffs
            .into_iter()
            .map(
                |ComponentPathAndDiff {
                     component_path,
                     component_diff,
                 }| {
                    let path = ComponentPath::deserialize(component_path.as_deref())?;
                    let diff = ComponentDiff::try_from(component_diff)?;
                    Ok((path, diff))
                },
            )
            .collect::<anyhow::Result<BTreeMap<ComponentPath, ComponentDiff>>>()?;
        Ok(PushComponentDiffs {
            auth_diff: value.auth_diff.unwrap_or_default(),
            component_diffs,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct ComponentPathAndDiff {
    component_path: Option<String>,
    component_diff: SerializedComponentDiff,
}
#[derive(Serialize, Deserialize)]
struct SerializedPushComponentDiffs {
    auth_diff: Option<AuthDiff>,
    component_diffs: Vec<ComponentPathAndDiff>,
}

impl TryFrom<PushComponentDiffs> for SerializedPushComponentDiffs {
    type Error = anyhow::Error;

    fn try_from(value: PushComponentDiffs) -> anyhow::Result<Self> {
        let auth_diff = value.auth_diff;
        let component_diffs = value
            .component_diffs
            .into_iter()
            .map(|(path, diff)| {
                let component_path = path.serialize();
                let component_diff = SerializedComponentDiff::try_from(diff)?;
                Ok(ComponentPathAndDiff {
                    component_path,
                    component_diff,
                })
            })
            .collect::<anyhow::Result<Vec<ComponentPathAndDiff>>>()?;
        Ok(SerializedPushComponentDiffs {
            auth_diff: Some(auth_diff),
            component_diffs,
        })
    }
}

codegen_convex_serialization!(PushComponentDiffs, SerializedPushComponentDiffs);

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::{
        log_streaming::LogEventFormatVersion,
        runtime::UnixTimestamp,
    };
    use proptest::prelude::*;
    use serde_json::json;
    use value::ConvexObject;

    use super::DeploymentAuditLogEvent;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_try_from(e in any::<DeploymentAuditLogEvent>()) {
            ConvexObject::try_from(e).unwrap();
        }

        #[test]
        fn test_json(e in any::<DeploymentAuditLogEvent>()) {
            serde_json::Map::try_from(e).unwrap();
        }
    }

    #[test]
    fn test_serialization_of_audit_log_event() -> anyhow::Result<()> {
        let event = DeploymentAuditLogEvent::to_log_event(
            DeploymentAuditLogEvent::CreateEnvironmentVariable {
                name: "test_env_variable".parse()?,
            },
            UnixTimestamp::from_millis(0),
        )?;
        let event_json = event.to_json_map(LogEventFormatVersion::default())?;
        let value = serde_json::to_value(&event_json)?;
        assert_eq!(
            value,
            json!({
                "topic": "audit_log",
                "timestamp": 0,
                "audit_log_action": "create_environment_variable",
                "audit_log_metadata": "{\"variable_name\":\"test_env_variable\"}",
            })
        );
        Ok(())
    }
}

#[cfg(test)]
mod proptests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::{
        testing::assert_roundtrips,
        ConvexObject,
    };

    use crate::deployment_audit_log::types::DeploymentAuditLogEvent;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_deployment_audit_log_roundtrip(v in any::<DeploymentAuditLogEvent>()) {
            assert_roundtrips::<DeploymentAuditLogEvent, ConvexObject>(v);
        }
    }
}
