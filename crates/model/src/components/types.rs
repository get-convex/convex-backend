use std::collections::{
    BTreeMap,
    BTreeSet,
};

use common::{
    bootstrap_model::components::definition::{
        ComponentDefinitionMetadata,
        SerializedComponentDefinitionMetadata,
    },
    components::ComponentDefinitionPath,
    schemas::DatabaseSchema,
    types::NodeDependency,
};
use semver::Version;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedModulePath;
use value::ConvexObject;

use crate::{
    config::types::{
        ConfigMetadata,
        ModuleConfig,
    },
    modules::module_versions::{
        AnalyzedModule,
        SerializedAnalyzedModule,
    },
    udf_config::types::UdfConfig,
};

#[derive(Debug)]
pub struct ProjectConfig {
    pub config: ConfigMetadata,

    pub app_definition: AppDefinitionConfig,
    pub component_definitions: Vec<ComponentDefinitionConfig>,

    // TODO(CX-6483): Add support for components to declare their own external dependencies.
    pub node_dependencies: Vec<NodeDependency>,
}

#[derive(Debug)]
pub struct AppDefinitionConfig {
    // Bundled `app.config.js` if present, with dependencies on other components marked external
    // and unresolved. Not available at runtime.
    pub definition: Option<ModuleConfig>,
    // Dependencies on other components discovered at bundling time.
    pub dependencies: BTreeSet<ComponentDefinitionPath>,

    // Optional auth.config.js. Not available at runtime.
    pub auth: Option<ModuleConfig>,

    // Optional schema.js. Not available at runtime.
    pub schema: Option<ModuleConfig>,

    // Includes all modules directly available at runtime:
    // - Regular function entry points
    // - http.js
    // - crons.js
    // - Bundler dependency chunks within _deps.
    pub functions: Vec<ModuleConfig>,

    pub udf_server_version: Version,
}

impl AppDefinitionConfig {
    pub fn modules(&self) -> impl Iterator<Item = &ModuleConfig> {
        self.definition
            .iter()
            .chain(self.auth.iter())
            .chain(self.schema.iter())
            .chain(&self.functions)
    }
}

#[derive(Debug)]
pub struct ComponentDefinitionConfig {
    // Relative path from the root `convex/` directory to the component's directory.
    pub definition_path: ComponentDefinitionPath,

    // Bundled component definition at `component.config.js` with dependencies on other components
    // unresolved.
    pub definition: ModuleConfig,
    // Dependencies on other components discovered at bundling time.
    pub dependencies: BTreeSet<ComponentDefinitionPath>,

    // Optional schema.js. Not available at runtime.
    pub schema: Option<ModuleConfig>,

    // Includes all modules directly available at runtime:
    // - Regular function entry points
    // - http.js
    // - crons.js
    // - Bundler dependency chunks within _deps.
    pub functions: Vec<ModuleConfig>,

    pub udf_server_version: Version,
}

impl ComponentDefinitionConfig {
    pub fn modules(&self) -> impl Iterator<Item = &ModuleConfig> {
        std::iter::once(&self.definition)
            .chain(self.schema.iter())
            .chain(&self.functions)
    }
}

#[derive(Clone)]
pub struct EvaluatedComponentDefinition {
    pub definition: ComponentDefinitionMetadata,
    pub schema: Option<DatabaseSchema>,
    pub functions: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    pub udf_config: UdfConfig,
}

impl EvaluatedComponentDefinition {
    pub fn is_app(&self) -> bool {
        self.definition.is_app()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedEvaluatedComponentDefinition {
    definition: SerializedComponentDefinitionMetadata,
    schema: Option<JsonValue>,
    functions: BTreeMap<String, SerializedAnalyzedModule>,
    udf_config: JsonValue,
}

impl TryFrom<EvaluatedComponentDefinition> for SerializedEvaluatedComponentDefinition {
    type Error = anyhow::Error;

    fn try_from(value: EvaluatedComponentDefinition) -> Result<Self, Self::Error> {
        Ok(SerializedEvaluatedComponentDefinition {
            definition: value.definition.try_into()?,
            schema: value.schema.map(|schema| schema.try_into()).transpose()?,
            functions: value
                .functions
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            udf_config: ConvexObject::try_from(value.udf_config)?.into(),
        })
    }
}

impl TryFrom<SerializedEvaluatedComponentDefinition> for EvaluatedComponentDefinition {
    type Error = anyhow::Error;

    fn try_from(value: SerializedEvaluatedComponentDefinition) -> Result<Self, Self::Error> {
        Ok(EvaluatedComponentDefinition {
            definition: value.definition.try_into()?,
            schema: value.schema.map(|schema| schema.try_into()).transpose()?,
            functions: value
                .functions
                .into_iter()
                .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            udf_config: UdfConfig::try_from(ConvexObject::try_from(value.udf_config)?)?,
        })
    }
}
