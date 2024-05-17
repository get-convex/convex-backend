use std::collections::BTreeSet;

use common::components::ComponentDefinitionPath;

use crate::{
    config::types::{
        ConfigMetadata,
        ModuleConfig,
    },
    udf_config::types::UdfConfig,
};

pub struct ProjectConfig {
    pub config: ConfigMetadata,
    pub udf_config: UdfConfig,
    pub component_definitions: Vec<ComponentDefinitionConfig>,
}

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
}

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
}
