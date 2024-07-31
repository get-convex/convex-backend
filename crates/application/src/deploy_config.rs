use std::collections::BTreeMap;

use common::{
    auth::AuthInfo,
    components::{
        ComponentDefinitionPath,
        ComponentPath,
    },
    types::NodeDependency,
};
use errors::ErrorMetadata;
use model::{
    auth::types::AuthDiff,
    components::{
        config::{
            ComponentDefinitionDiff,
            ComponentDiff,
            SchemaChange,
        },
        type_checking::CheckedComponent,
        types::{
            AppDefinitionConfig,
            ComponentDefinitionConfig,
            EvaluatedComponentDefinition,
            ProjectConfig,
        },
    },
    config::types::{
        deprecated_extract_environment_from_path,
        ConfigMetadata,
        ModuleConfig,
    },
    external_packages::types::ExternalDepsPackageId,
    modules::module_versions::{
        ModuleSource,
        SourceMap,
    },
    source_packages::types::SourcePackage,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::ModulePath;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StartPushRequest {
    pub admin_key: String,
    pub dry_run: bool,

    pub functions: String,

    pub app_definition: AppDefinitionConfigJson,
    pub component_definitions: Vec<ComponentDefinitionConfigJson>,

    pub node_dependencies: Vec<NodeDependencyJson>,
}

impl StartPushRequest {
    pub fn into_project_config(self) -> anyhow::Result<ProjectConfig> {
        Ok(ProjectConfig {
            config: ConfigMetadata {
                functions: self.functions,
                auth_info: vec![],
            },
            app_definition: self.app_definition.try_into()?,
            component_definitions: self
                .component_definitions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<_>>()?,
            node_dependencies: self
                .node_dependencies
                .into_iter()
                .map(NodeDependency::from)
                .collect(),
        })
    }
}

#[derive(Debug)]
pub struct StartPushResponse {
    pub external_deps_id: Option<ExternalDepsPackageId>,
    pub component_definition_packages: BTreeMap<ComponentDefinitionPath, SourcePackage>,

    pub app_auth: Vec<AuthInfo>,
    pub analysis: BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>,

    pub app: CheckedComponent,

    pub schema_change: SchemaChange,
}

impl From<NodeDependencyJson> for NodeDependency {
    fn from(value: NodeDependencyJson) -> Self {
        Self {
            package: value.name,
            version: value.version,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppDefinitionConfigJson {
    pub definition: Option<ModuleJson>,
    pub dependencies: Vec<String>,
    pub schema: Option<ModuleJson>,
    pub functions: Vec<ModuleJson>,
    pub udf_server_version: String,
}

impl TryFrom<AppDefinitionConfigJson> for AppDefinitionConfig {
    type Error = anyhow::Error;

    fn try_from(value: AppDefinitionConfigJson) -> Result<Self, Self::Error> {
        Ok(Self {
            definition: value.definition.map(TryInto::try_into).transpose()?,
            dependencies: value
                .dependencies
                .into_iter()
                .map(|s| s.parse())
                .collect::<anyhow::Result<_>>()?,
            schema: value.schema.map(TryInto::try_into).transpose()?,
            functions: value
                .functions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<_>>()?,
            udf_server_version: value.udf_server_version.parse()?,
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDefinitionConfigJson {
    pub definition_path: String,
    pub definition: ModuleJson,
    pub dependencies: Vec<String>,
    pub schema: Option<ModuleJson>,
    pub functions: Vec<ModuleJson>,
    pub udf_server_version: String,
}

impl TryFrom<ComponentDefinitionConfigJson> for ComponentDefinitionConfig {
    type Error = anyhow::Error;

    fn try_from(value: ComponentDefinitionConfigJson) -> Result<Self, Self::Error> {
        Ok(Self {
            definition_path: value.definition_path.parse()?,
            definition: value.definition.try_into()?,
            dependencies: value
                .dependencies
                .into_iter()
                .map(|s| s.parse())
                .collect::<anyhow::Result<_>>()?,
            schema: value.schema.map(TryInto::try_into).transpose()?,
            functions: value
                .functions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<_>>()?,
            udf_server_version: value.udf_server_version.parse()?,
        })
    }
}

/// API level structure for representing modules as Json
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModuleJson {
    pub path: String,
    pub source: ModuleSource,
    pub source_map: Option<SourceMap>,
    pub environment: Option<String>,
}

impl From<ModuleConfig> for ModuleJson {
    fn from(
        ModuleConfig {
            path,
            source,
            source_map,
            environment,
        }: ModuleConfig,
    ) -> ModuleJson {
        ModuleJson {
            path: path.into(),
            source,
            source_map,
            environment: Some(environment.to_string()),
        }
    }
}

impl TryFrom<ModuleJson> for ModuleConfig {
    type Error = anyhow::Error;

    fn try_from(
        ModuleJson {
            path,
            source,
            source_map,
            environment,
        }: ModuleJson,
    ) -> anyhow::Result<ModuleConfig> {
        let environment = match environment {
            Some(s) => s.parse()?,
            // Default to using the path for backwards compatibility
            None => deprecated_extract_environment_from_path(path.clone())?,
        };
        Ok(ModuleConfig {
            path: parse_module_path(&path)?,
            source,
            source_map,
            environment,
        })
    }
}

pub fn parse_module_path(path: &str) -> anyhow::Result<ModulePath> {
    path.parse().map_err(|e: anyhow::Error| {
        let msg = format!("{path} is not a valid path to a Convex module. {e}");
        e.context(ErrorMetadata::bad_request("BadConvexModuleIdentifier", msg))
    })
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeDependencyJson {
    name: String,
    version: String,
}

pub struct FinishPushDiff {
    pub auth_diff: AuthDiff,
    pub definition_diffs: BTreeMap<ComponentDefinitionPath, ComponentDefinitionDiff>,
    pub component_diffs: BTreeMap<ComponentPath, ComponentDiff>,
}
