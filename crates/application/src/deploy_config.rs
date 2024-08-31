use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    auth::AuthInfo,
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    components::{
        ComponentDefinitionPath,
        ComponentName,
        ComponentPath,
        Resource,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        EnvVarName,
        EnvVarValue,
        NodeDependency,
    },
};
use database::{
    WriteSource,
    SCHEMAS_TABLE,
};
use errors::ErrorMetadata;
use isolate::EvaluateAppDefinitionsResult;
use keybroker::Identity;
use maplit::btreeset;
use model::{
    auth::types::AuthDiff,
    components::{
        config::{
            ComponentConfigModel,
            ComponentDefinitionDiff,
            ComponentDiff,
            SchemaChange,
        },
        file_based_routing::add_file_based_routing,
        type_checking::{
            CheckedComponent,
            InitializerEvaluator,
            TypecheckContext,
        },
        types::{
            AppDefinitionConfig,
            ComponentDefinitionConfig,
            EvaluatedComponentDefinition,
            ProjectConfig,
        },
    },
    config::types::{
        deprecated_extract_environment_from_path,
        ConfigFile,
        ConfigMetadata,
        ModuleConfig,
    },
    environment_variables::EnvironmentVariablesModel,
    external_packages::types::ExternalDepsPackageId,
    modules::module_versions::{
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    source_packages::types::SourcePackage,
    udf_config::types::UdfConfig,
};
use rand::Rng;
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::{
    CanonicalizedModulePath,
    ModulePath,
};
use value::identifier::Identifier;

use crate::Application;

impl<RT: Runtime> Application<RT> {
    pub async fn start_push(&self, request: StartPushRequest) -> anyhow::Result<StartPushResponse> {
        let unix_timestamp = self.runtime.unix_timestamp();
        let dry_run = request.dry_run;
        let config = request
            .into_project_config()
            .map_err(|e| ErrorMetadata::bad_request("InvalidConfig", e.to_string()))?;

        let (external_deps_id, component_definition_packages) =
            self.upload_packages(&config).await?;

        let app_udf_config = UdfConfig {
            server_version: config.app_definition.udf_server_version.clone(),
            import_phase_rng_seed: self.runtime.with_rng(|rng| rng.gen()),
            import_phase_unix_timestamp: unix_timestamp,
        };
        let app_pkg = component_definition_packages
            .get(&ComponentDefinitionPath::root())
            .context("No package for app?")?;

        let identity = Identity::system();
        let mut tx = self.begin(identity.clone()).await?;
        let environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
        tx.into_token()?;
        // TODO(ENG-6500): Fold in our reads here into the hash.
        let (auth_module, app_analysis) = self
            .analyze_modules_with_auth_config(
                app_udf_config.clone(),
                config.app_definition.functions.clone(),
                app_pkg.clone(),
                environment_variables.clone(),
            )
            .await?;

        let auth_info = Application::get_evaluated_auth_config(
            self.runner(),
            environment_variables.clone(),
            auth_module,
            &ConfigFile {
                functions: config.config.functions.clone(),
                auth_info: if config.config.auth_info.is_empty() {
                    None
                } else {
                    Some(config.config.auth_info.clone())
                },
            },
        )
        .await?;

        let evaluated_components = self
            .evaluate_components(
                &config,
                &component_definition_packages,
                app_analysis,
                app_udf_config,
                unix_timestamp,
                environment_variables,
            )
            .await?;
        // Build and typecheck the component tree. We don't strictly need to do this
        // before `/finish_push`, but it's better to fail fast here on errors before
        // waiting for schema backfills to complete.
        let initializer_evaluator = ApplicationInitializerEvaluator::new(
            self,
            &config,
            evaluated_components
                .iter()
                .map(|(k, v)| (k.clone(), v.definition.clone()))
                .collect(),
        )?;
        let ctx = TypecheckContext::new(&evaluated_components, &initializer_evaluator);
        let app = ctx.instantiate_root().await?;

        let schema_change = {
            let mut tx = self.begin(identity.clone()).await?;
            let schema_change = ComponentConfigModel::new(&mut tx)
                .start_component_schema_changes(&app, &evaluated_components)
                .await?;
            if !dry_run {
                self.commit(tx, WriteSource::new("start_push")).await?;
                self.database
                    .load_indexes_into_memory(btreeset! { SCHEMAS_TABLE.clone() })
                    .await?;
            }
            schema_change
        };
        let resp = StartPushResponse {
            external_deps_id,
            component_definition_packages,
            app_auth: auth_info,
            analysis: evaluated_components,
            app,
            schema_change,
        };
        Ok(resp)
    }

    async fn evaluate_components(
        &self,
        config: &ProjectConfig,
        component_definition_packages: &BTreeMap<ComponentDefinitionPath, SourcePackage>,
        app_analysis: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
        app_udf_config: UdfConfig,
        unix_timestamp: UnixTimestamp,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>> {
        let mut app_schema = None;
        if let Some(schema_module) = &config.app_definition.schema {
            app_schema = Some(self.evaluate_schema(schema_module.clone()).await?);
        }

        let mut component_analysis_by_def_path = BTreeMap::new();
        let mut component_schema_by_def_path = BTreeMap::new();
        let mut component_udf_config_by_def_path = BTreeMap::new();

        for component_def in &config.component_definitions {
            let udf_config = UdfConfig {
                server_version: component_def.udf_server_version.clone(),
                import_phase_rng_seed: self.runtime.with_rng(|rng| rng.gen()),
                import_phase_unix_timestamp: unix_timestamp,
            };
            component_udf_config_by_def_path
                .insert(component_def.definition_path.clone(), udf_config.clone());

            let component_pkg = component_definition_packages
                .get(&component_def.definition_path)
                .context("No package for component?")?;
            let component_analysis = self
                .analyze_modules(
                    udf_config.clone(),
                    component_def.functions.clone(),
                    component_pkg.clone(),
                    // Component functions do not have access to environment variables.
                    BTreeMap::new(),
                )
                .await?;
            anyhow::ensure!(component_analysis_by_def_path
                .insert(component_def.definition_path.clone(), component_analysis)
                .is_none());

            if let Some(schema_module) = &component_def.schema {
                let schema = self.evaluate_schema(schema_module.clone()).await?;
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

            evaluated_definitions = self
                .evaluate_app_definitions(
                    app_definition.clone(),
                    component_definitions,
                    dependency_graph,
                    environment_variables,
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
                udf_config: app_udf_config.clone(),
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
                    udf_config: component_udf_config_by_def_path
                        .get(path)
                        .context("Missing UDF config for component?")?
                        .clone(),
                },
            );
        }

        // Add in file-based routing.
        for definition in evaluated_components.values_mut() {
            add_file_based_routing(definition)?;
        }
        Ok(evaluated_components)
    }

    #[minitrace::trace]
    pub async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        self.runner
            .evaluate_app_definitions(
                app_definition,
                component_definitions,
                dependency_graph,
                environment_variables,
            )
            .await
    }
}

struct ApplicationInitializerEvaluator<'a, RT: Runtime> {
    application: &'a Application<RT>,
    component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
    evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
}

impl<'a, RT: Runtime> ApplicationInitializerEvaluator<'a, RT> {
    fn new(
        application: &'a Application<RT>,
        config: &'a ProjectConfig,
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
    ) -> anyhow::Result<Self> {
        let mut component_definitions = BTreeMap::new();
        for component_definition in &config.component_definitions {
            anyhow::ensure!(component_definitions
                .insert(
                    component_definition.definition_path.clone(),
                    component_definition.definition.clone(),
                )
                .is_none());
        }
        Ok(Self {
            application,
            component_definitions,
            evaluated_definitions,
        })
    }
}

#[async_trait]
impl<'a, RT: Runtime> InitializerEvaluator for ApplicationInitializerEvaluator<'a, RT> {
    async fn evaluate(
        &self,
        path: ComponentDefinitionPath,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>> {
        let component_definition = self
            .component_definitions
            .get(&path)
            .context(format!("Missing component definition for {path:?}"))?
            .clone();
        self.application
            .runner
            .evaluate_component_initializer(
                self.evaluated_definitions.clone(),
                path,
                component_definition,
                args,
                name,
            )
            .await
    }
}

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

#[derive(Debug)]
pub enum SchemaStatus {
    InProgress {
        components: BTreeMap<ComponentPath, ComponentSchemaStatus>,
    },
    Failed {
        error: String,
        component_path: ComponentPath,
        table_name: Option<String>,
    },
    RaceDetected,
    Complete,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum SchemaStatusJson {
    #[serde(rename_all = "camelCase")]
    InProgress {
        components: BTreeMap<String, ComponentSchemaStatusJson>,
    },
    #[serde(rename_all = "camelCase")]
    Failed {
        error: String,
        component_path: String,
        table_name: Option<String>,
    },
    RaceDetected,
    Complete,
}

impl From<SchemaStatus> for SchemaStatusJson {
    fn from(value: SchemaStatus) -> Self {
        match value {
            SchemaStatus::InProgress { components } => SchemaStatusJson::InProgress {
                components: components
                    .into_iter()
                    .map(|(k, v)| (String::from(k), v.into()))
                    .collect(),
            },
            SchemaStatus::Failed {
                error,
                component_path,
                table_name,
            } => SchemaStatusJson::Failed {
                error,
                component_path: String::from(component_path),
                table_name,
            },
            SchemaStatus::RaceDetected => SchemaStatusJson::RaceDetected,
            SchemaStatus::Complete => SchemaStatusJson::Complete,
        }
    }
}

#[derive(Debug)]
pub struct ComponentSchemaStatus {
    pub schema_validation_complete: bool,
    pub indexes_complete: usize,
    pub indexes_total: usize,
}

impl ComponentSchemaStatus {
    pub fn is_complete(&self) -> bool {
        self.schema_validation_complete && self.indexes_complete == self.indexes_total
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSchemaStatusJson {
    pub schema_validation_complete: bool,
    pub indexes_complete: usize,
    pub indexes_total: usize,
}

impl From<ComponentSchemaStatus> for ComponentSchemaStatusJson {
    fn from(value: ComponentSchemaStatus) -> Self {
        Self {
            schema_validation_complete: value.schema_validation_complete,
            indexes_complete: value.indexes_complete,
            indexes_total: value.indexes_total,
        }
    }
}
