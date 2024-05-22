use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use application::Application;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    auth::AuthInfo,
    bootstrap_model::components::definition::{
        ComponentDefinitionMetadata,
        SerializedComponentDefinitionMetadata,
    },
    components::ComponentDefinitionPath,
    http::{
        extract::Json,
        HttpResponseError,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::NodeDependency,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use model::{
    components::{
        file_based_routing::add_file_based_routing,
        type_checking::{
            SerializedCheckedComponent,
            TypecheckContext,
        },
        types::{
            AppDefinitionConfig,
            ComponentDefinitionConfig,
            EvaluatedComponentDefinition,
            ProjectConfig,
            SerializedEvaluatedComponentDefinition,
        },
    },
    config::types::{
        ConfigFile,
        ConfigMetadata,
    },
    modules::module_versions::{
        AnalyzedModule,
        SerializedAnalyzedModule,
    },
    source_packages::types::PackageSize,
    udf_config::types::UdfConfig,
};
use rand::Rng;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::DeveloperDocumentId;

use crate::{
    admin::must_be_admin_from_keybroker,
    deploy_config::{
        analyze_modules,
        ModuleJson,
        NodeDependencyJson,
    },
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigJson {
    pub admin_key: String,

    pub functions: String,
    pub udf_server_version: String,

    pub app_definition: AppDefinitionConfigJson,
    pub component_definitions: Vec<ComponentDefinitionConfigJson>,

    pub node_dependencies: Vec<NodeDependencyJson>,
}

impl ProjectConfigJson {
    pub fn into_project_config(
        self,
        import_phase_rng_seed: [u8; 32],
        import_phase_unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<ProjectConfig> {
        Ok(ProjectConfig {
            config: ConfigMetadata {
                functions: self.functions,
                auth_info: vec![],
            },
            udf_config: UdfConfig {
                server_version: self.udf_server_version.parse()?,
                import_phase_rng_seed,
                import_phase_unix_timestamp,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDefinitionConfigJson {
    pub definition: Option<ModuleJson>,
    pub dependencies: Vec<String>,
    pub auth: Option<ModuleJson>,
    pub schema: Option<ModuleJson>,
    pub functions: Vec<ModuleJson>,
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
            auth: value.auth.map(TryInto::try_into).transpose()?,
            schema: value.schema.map(TryInto::try_into).transpose()?,
            functions: value
                .functions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDefinitionConfigJson {
    pub definition_path: String,
    pub definition: ModuleJson,
    pub dependencies: Vec<String>,
    pub schema: Option<ModuleJson>,
    pub functions: Vec<ModuleJson>,
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
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPushResponse {
    // Pointers to uploaded code.
    external_deps_id: Option<String>,
    app_package: String,
    component_packages: BTreeMap<String, String>,

    // Analysis results.
    app_auth: Vec<AuthInfo>,
    analysis: BTreeMap<String, SerializedEvaluatedComponentDefinition>,

    // Typechecking results.
    app: SerializedCheckedComponent,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedComponent {
    definition: SerializedComponentDefinitionMetadata,
    schema: Option<JsonValue>,
    modules: BTreeMap<String, SerializedAnalyzedModule>,
}

#[debug_handler]
pub async fn start_push(
    State(st): State<LocalAppState>,
    Json(req): Json<ProjectConfigJson>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let resp = start_push_handler(&st, req)
        .await
        .map_err(|e| e.wrap_error_message(|msg| format!("Hit an error while pushing:\n{msg}")))?;
    Ok(Json(resp))
}

#[minitrace::trace]
pub async fn start_push_handler(
    st: &LocalAppState,
    config_json: ProjectConfigJson,
) -> anyhow::Result<StartPushResponse> {
    let identity = must_be_admin_from_keybroker(
        st.application.key_broker(),
        Some(st.instance_name.clone()),
        config_json.admin_key.clone(),
    )?;

    let rt = st.application.runtime();
    let rng_seed = rt.with_rng(|rng| rng.gen());
    let unix_timestamp = rt.unix_timestamp();
    let config = config_json
        .into_project_config(rng_seed, unix_timestamp)
        .map_err(|e| ErrorMetadata::bad_request("InvalidConfig", e.to_string()))?;

    let external_deps_id_and_pkg = if !config.node_dependencies.is_empty() {
        let deps = st
            .application
            .build_external_node_deps(config.node_dependencies)
            .await?;
        Some(deps)
    } else {
        None
    };

    let mut total_size = external_deps_id_and_pkg
        .as_ref()
        .map(|(_, pkg)| pkg.package_size)
        .unwrap_or(PackageSize::default());

    let app_modules = config.app_definition.modules().cloned().collect();
    let app_pkg = st
        .application
        .upload_package(&app_modules, external_deps_id_and_pkg.clone())
        .await?
        .context("No package for app?")?;
    total_size += app_pkg.package_size;

    let mut component_pkg_by_def_path = BTreeMap::new();
    for component_def in &config.component_definitions {
        let component_modules = component_def.modules().cloned().collect();
        let component_pkg = st
            .application
            .upload_package(&component_modules, None)
            .await?
            .context("No package for component?")?;
        total_size += component_pkg.package_size;
        anyhow::ensure!(component_pkg_by_def_path
            .insert(component_def.definition_path.clone(), component_pkg)
            .is_none());
    }

    total_size.verify_size()?;

    let mut app_analysis = analyze_modules(
        &st.application,
        config.udf_config.clone(),
        config.app_definition.functions.clone(),
        Some(app_pkg.clone()),
    )
    .await?;

    // Evaluate auth and add in an empty `auth.config.js` to the analysis.
    let auth_info = {
        let mut tx = st.application.begin(identity.clone()).await?;
        Application::get_evaluated_auth_config(
            st.application.runner(),
            &mut tx,
            config.app_definition.auth.clone(),
            &ConfigFile {
                functions: config.config.functions.clone(),
                auth_info: if config.config.auth_info.is_empty() {
                    None
                } else {
                    Some(config.config.auth_info.clone())
                },
            },
        )
        .await?
    };
    if let Some(auth_module) = &config.app_definition.auth {
        app_analysis.insert(
            auth_module.path.clone().canonicalize(),
            AnalyzedModule::default(),
        );
    }

    let mut app_schema = None;
    if let Some(schema_module) = &config.app_definition.schema {
        app_schema = Some(
            st.application
                .evaluate_schema(schema_module.clone())
                .await?,
        );
    }

    let mut component_analysis_by_def_path = BTreeMap::new();
    let mut component_schema_by_def_path = BTreeMap::new();

    for component_def in &config.component_definitions {
        let component_pkg = component_pkg_by_def_path
            .get(&component_def.definition_path)
            .context("No package for component?")?;
        let component_analysis = analyze_modules(
            &st.application,
            config.udf_config.clone(),
            component_def.functions.clone(),
            Some(component_pkg.clone()),
        )
        .await?;
        anyhow::ensure!(component_analysis_by_def_path
            .insert(component_def.definition_path.clone(), component_analysis)
            .is_none());

        if let Some(schema_module) = &component_def.schema {
            let schema = st
                .application
                .evaluate_schema(schema_module.clone())
                .await?;
            anyhow::ensure!(component_schema_by_def_path
                .insert(component_def.definition_path.clone(), schema)
                .is_none());
        }
    }

    let mut evaluated_definitions = BTreeMap::new();

    if let Some(ref app_definition) = config.app_definition.definition {
        let mut dependency_graph = BTreeSet::new();
        let mut component_definitions = BTreeMap::new();

        for dep in &config.app_definition.dependencies {
            dependency_graph.insert((ComponentDefinitionPath::root(), dep.clone()));
        }

        for component_def in &config.component_definitions {
            anyhow::ensure!(!component_def.definition_path.is_root());
            component_definitions.insert(
                component_def.definition_path.clone(),
                component_def.definition.clone(),
            );
            for dep in &component_def.dependencies {
                dependency_graph.insert((component_def.definition_path.clone(), dep.clone()));
            }
        }

        evaluated_definitions = st
            .application
            .evaluate_app_definitions(
                app_definition.clone(),
                component_definitions,
                dependency_graph,
            )
            .await?;
    } else {
        evaluated_definitions.insert(
            ComponentDefinitionPath::root(),
            ComponentDefinitionMetadata::default_root(),
        );
    }

    let mut evaluated_components = BTreeMap::new();
    evaluated_components.insert(
        ComponentDefinitionPath::root(),
        EvaluatedComponentDefinition {
            definition: evaluated_definitions[&ComponentDefinitionPath::root()].clone(),
            schema: app_schema.clone(),
            functions: app_analysis.clone(),
        },
    );
    for (path, definition) in &evaluated_definitions {
        if path.is_root() {
            continue;
        }
        evaluated_components.insert(
            path.clone(),
            EvaluatedComponentDefinition {
                definition: definition.clone(),
                schema: component_schema_by_def_path.get(path).cloned(),
                functions: component_analysis_by_def_path
                    .get(path)
                    .context("Missing analysis for component?")?
                    .clone(),
            },
        );
    }

    // Add in file-based routing.
    for definition in evaluated_components.values_mut() {
        add_file_based_routing(definition)?;
    }

    // Build and typecheck the component tree.
    let ctx = TypecheckContext::new(&evaluated_components);
    let app = ctx
        .instantiate_root()
        .map_err(|e| ErrorMetadata::bad_request("TypecheckError", e.to_string()))?;

    tracing::info!("Instantiated root: {app:#?}");

    let resp = StartPushResponse {
        external_deps_id: external_deps_id_and_pkg
            .map(|(id, _)| String::from(DeveloperDocumentId::from(id))),
        app_package: String::from(app_pkg.storage_key),
        component_packages: component_pkg_by_def_path
            .into_iter()
            .map(|(def_path, pkg)| (String::from(def_path), String::from(pkg.storage_key)))
            .collect(),
        app_auth: auth_info,
        analysis: evaluated_components
            .into_iter()
            .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
            .collect::<anyhow::Result<_>>()?,
        app: app.try_into()?,
    };
    Ok(resp)
}
