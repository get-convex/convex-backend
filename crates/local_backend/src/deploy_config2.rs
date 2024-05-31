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
    components::{
        ComponentDefinitionPath,
        ComponentPath,
    },
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
use database::WriteSource;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use keybroker::Identity;
use model::{
    auth::{
        types::AuthDiff,
        AuthInfoModel,
    },
    components::{
        config::{
            ComponentConfigModel,
            ComponentDefinitionConfigModel,
            ComponentDefinitionDiff,
            ComponentDiff,
            SerializedComponentDefinitionDiff,
            SerializedComponentDiff,
        },
        file_based_routing::add_file_based_routing,
        type_checking::{
            CheckedComponent,
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
        UdfServerVersionDiff,
    },
    external_packages::types::ExternalDepsPackageId,
    modules::module_versions::{
        AnalyzedModule,
        SerializedAnalyzedModule,
    },
    source_packages::{
        types::{
            PackageSize,
            SourcePackage,
        },
        upload_download::download_package,
    },
    udf_config::{
        types::UdfConfig,
        UdfConfigModel,
    },
};
use rand::Rng;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    ConvexObject,
    DeveloperDocumentId,
};

use crate::{
    admin::must_be_admin_from_key,
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

struct StartPushResponse {
    udf_config: UdfConfig,

    external_deps_id: Option<ExternalDepsPackageId>,
    component_definition_packages: BTreeMap<ComponentDefinitionPath, SourcePackage>,

    app_auth: Vec<AuthInfo>,
    analysis: BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>,

    app: CheckedComponent,
}

impl TryFrom<StartPushResponse> for SerializedStartPushResponse {
    type Error = anyhow::Error;

    fn try_from(value: StartPushResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            udf_config: ConvexObject::try_from(value.udf_config)?.try_into()?,
            external_deps_id: value
                .external_deps_id
                .map(|id| String::from(DeveloperDocumentId::from(id))),
            component_definition_packages: value
                .component_definition_packages
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        String::from(k),
                        JsonValue::try_from(ConvexObject::try_from(v)?)?,
                    ))
                })
                .collect::<anyhow::Result<_>>()?,
            app_auth: value.app_auth,
            analysis: value
                .analysis
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            app: value.app.try_into()?,
        })
    }
}

impl TryFrom<SerializedStartPushResponse> for StartPushResponse {
    type Error = anyhow::Error;

    fn try_from(value: SerializedStartPushResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            udf_config: UdfConfig::try_from(ConvexObject::try_from(value.udf_config)?)?,
            external_deps_id: value
                .external_deps_id
                .map(|id| {
                    anyhow::Ok(ExternalDepsPackageId::from(
                        id.parse::<DeveloperDocumentId>()?,
                    ))
                })
                .transpose()?,
            component_definition_packages: value
                .component_definition_packages
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.parse()?,
                        SourcePackage::try_from(ConvexObject::try_from(v)?)?,
                    ))
                })
                .collect::<anyhow::Result<_>>()?,
            app_auth: value.app_auth,
            analysis: value
                .analysis
                .into_iter()
                .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            app: value.app.try_into()?,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedStartPushResponse {
    // Global evaluation
    udf_config: JsonValue,

    // Pointers to uploaded code.
    external_deps_id: Option<String>,
    component_definition_packages: BTreeMap<String, JsonValue>,

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
    Ok(Json(SerializedStartPushResponse::try_from(resp)?))
}

#[minitrace::trace]
pub async fn start_push_handler(
    st: &LocalAppState,
    config_json: ProjectConfigJson,
) -> anyhow::Result<StartPushResponse> {
    let identity = must_be_admin_from_key(
        st.application.app_auth(),
        st.instance_name.clone(),
        config_json.admin_key.clone(),
    )
    .await?;

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

    let mut component_definition_packages = BTreeMap::new();

    let app_modules = config.app_definition.modules().cloned().collect();
    let app_pkg = st
        .application
        .upload_package(&app_modules, external_deps_id_and_pkg.clone())
        .await?
        .context("No package for app?")?;
    total_size += app_pkg.package_size;
    component_definition_packages.insert(ComponentDefinitionPath::root(), app_pkg);

    for component_def in &config.component_definitions {
        let component_modules = component_def.modules().cloned().collect();
        let component_pkg = st
            .application
            .upload_package(&component_modules, None)
            .await?
            .context("No package for component?")?;
        total_size += component_pkg.package_size;
        anyhow::ensure!(component_definition_packages
            .insert(component_def.definition_path.clone(), component_pkg)
            .is_none());
    }

    total_size.verify_size()?;

    let app_pkg = component_definition_packages
        .get(&ComponentDefinitionPath::root())
        .context("No package for app?")?;
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
        let component_pkg = component_definition_packages
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

    // Build and typecheck the component tree. We don't strictly need to do this
    // before `/finish_push`, but it's better to fail fast here on errors before
    // waiting for schema backfills to complete.
    let ctx = TypecheckContext::new(&evaluated_components);
    let app = ctx
        .instantiate_root()
        .map_err(|e| ErrorMetadata::bad_request("TypecheckError", e.to_string()))?;

    let resp = StartPushResponse {
        udf_config: config.udf_config,
        external_deps_id: external_deps_id_and_pkg.map(|(id, _)| id),
        component_definition_packages,
        app_auth: auth_info,
        analysis: evaluated_components,
        app,
    };
    Ok(resp)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishPushRequest {
    admin_key: String,
    start_push: SerializedStartPushResponse,
    dry_run: bool,
}

#[debug_handler]
pub async fn finish_push(
    State(st): State<LocalAppState>,
    Json(req): Json<FinishPushRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let resp = finish_push_handler(&st, req)
        .await
        .map_err(|e| e.wrap_error_message(|msg| format!("Hit an error while pushing:\n{msg}")))?;
    Ok(Json(SerializedFinishPushDiff::try_from(resp)?))
}

async fn finish_push_handler(
    st: &LocalAppState,
    req: FinishPushRequest,
) -> anyhow::Result<FinishPushDiff> {
    let start_push = StartPushResponse::try_from(req.start_push)?;
    let _identity = must_be_admin_from_key(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key.clone(),
    )
    .await?;

    // TODO: Verify that hash matches (env variables, schema, component tree).

    // Download all source packages. We can remove this once we don't store source
    // in the database.
    let mut downloaded_source_packages = BTreeMap::new();
    for (definition_path, source_package) in &start_push.component_definition_packages {
        let package = download_package(
            st.application.modules_storage().clone(),
            source_package.storage_key.clone(),
            source_package.sha256.clone(),
        )
        .await?;
        downloaded_source_packages.insert(definition_path.clone(), package);
    }

    // TODO: We require system identity for creating system tables.
    let mut tx = st.application.begin(Identity::system()).await?;

    // Update app state: auth info and UDF server version.
    let auth_diff = AuthInfoModel::new(&mut tx).put(start_push.app_auth).await?;
    let udf_config_diff = UdfConfigModel::new(&mut tx)
        .set(start_push.udf_config)
        .await?;

    // Diff the component definitions.
    let definition_diffs = ComponentDefinitionConfigModel::new(&mut tx)
        .diff_component_definitions(
            &start_push.analysis,
            &start_push.component_definition_packages,
            &downloaded_source_packages,
        )
        .await?;

    // Diff component tree.
    let component_diffs = ComponentConfigModel::new(&mut tx)
        .diff_component_tree(&start_push.app)
        .await?;

    if !req.dry_run {
        st.application
            .commit(tx, WriteSource::new("finish_push"))
            .await?;
    } else {
        drop(tx);
    }

    let diff = FinishPushDiff {
        auth_diff,
        udf_config_diff,
        definition_diffs,
        component_diffs,
    };
    Ok(diff)
}

struct FinishPushDiff {
    auth_diff: AuthDiff,
    udf_config_diff: Option<UdfServerVersionDiff>,
    definition_diffs: BTreeMap<ComponentDefinitionPath, ComponentDefinitionDiff>,
    component_diffs: BTreeMap<ComponentPath, ComponentDiff>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedFinishPushDiff {
    auth_diff: JsonValue,
    udf_config_diff: Option<JsonValue>,
    definition_diffs: BTreeMap<String, SerializedComponentDefinitionDiff>,
    component_diffs: BTreeMap<String, SerializedComponentDiff>,
}

impl TryFrom<FinishPushDiff> for SerializedFinishPushDiff {
    type Error = anyhow::Error;

    fn try_from(value: FinishPushDiff) -> Result<Self, Self::Error> {
        Ok(Self {
            auth_diff: JsonValue::try_from(ConvexObject::try_from(value.auth_diff)?)?,
            udf_config_diff: value
                .udf_config_diff
                .map(|diff| anyhow::Ok(JsonValue::try_from(ConvexObject::try_from(diff)?)?))
                .transpose()?,
            definition_diffs: value
                .definition_diffs
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
            component_diffs: value
                .component_diffs
                .into_iter()
                .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                .collect::<anyhow::Result<_>>()?,
        })
    }
}
