use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    time::{
        Duration,
        Instant,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    auth::AuthInfo,
    bootstrap_model::{
        components::definition::ComponentDefinitionMetadata,
        schema::{
            SchemaMetadata,
            SchemaState,
        },
    },
    components::{
        ComponentDefinitionPath,
        ComponentId,
        ComponentName,
        ComponentPath,
        Resource,
    },
    errors::JsError,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    types::{
        EnvVarName,
        EnvVarValue,
        ModuleEnvironment,
        NodeDependency,
    },
    version::Version,
};
use database::{
    BootstrapComponentsModel,
    IndexModel,
    OccRetryStats,
    Token,
    WriteSource,
    SCHEMAS_TABLE,
};
use errors::ErrorMetadata;
use fastrace::{
    future::FutureExt as _,
    Span,
};
use keybroker::Identity;
use maplit::btreeset;
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
            SchemaChange,
        },
        file_based_routing::file_based_exports,
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
    deployment_audit_log::types::{
        DeploymentAuditLogEvent,
        PushComponentDiffs,
    },
    environment_variables::EnvironmentVariablesModel,
    external_packages::types::ExternalDepsPackageId,
    modules::module_versions::{
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    source_packages::{
        types::{
            NodeVersion,
            SourcePackage,
        },
        upload_download::download_package,
    },
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
use udf::{
    environment::system_env_var_overrides,
    EvaluateAppDefinitionsResult,
};
use usage_tracking::FunctionUsageTracker;
use value::{
    identifier::Identifier,
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
};

use crate::{
    Application,
    ApplyConfigArgs,
    ConfigMetadataAndSchema,
};

pub struct PushAnalytics {
    pub config: ConfigMetadata,
    pub modules: Vec<ModuleConfig>,
    pub udf_server_version: Version,
    pub analyze_results: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    pub schema: Option<DatabaseSchema>,
}

pub struct PushMetrics {
    pub build_external_deps_time: Duration,
    pub upload_source_package_time: Duration,
    pub analyze_time: Duration,
    pub occ_stats: OccRetryStats,
}

impl<RT: Runtime> Application<RT> {
    #[fastrace::trace]
    pub async fn start_push(&self, config: &ProjectConfig) -> anyhow::Result<StartPushResponse> {
        let unix_timestamp = self.runtime.unix_timestamp();
        let (external_deps_id, component_definition_packages) =
            self.upload_packages(config).await?;

        let app_udf_config = UdfConfig {
            server_version: config.app_definition.udf_server_version.clone(),
            import_phase_rng_seed: self.runtime.rng().random(),
            import_phase_unix_timestamp: unix_timestamp,
        };
        let app_pkg = component_definition_packages
            .get(&ComponentDefinitionPath::root())
            .context("No package for app?")?;

        let (user_environment_variables, system_env_var_overrides) = {
            let mut tx = self.begin(Identity::system()).await?;
            let vars = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
            let system_env_var_overrides = system_env_var_overrides(&mut tx).await?;
            tx.into_token()?;
            (vars, system_env_var_overrides)
        };
        let (auth_module, app_analysis) = self
            .analyze_modules_with_auth_config(
                app_udf_config.clone(),
                config.app_definition.functions.clone(),
                app_pkg.clone(),
                user_environment_variables.clone(),
                system_env_var_overrides.clone(),
            )
            .await?;

        let auth_info = Application::get_evaluated_auth_config(
            self.runner(),
            user_environment_variables.clone(),
            system_env_var_overrides.clone(),
            auth_module,
            &ConfigFile {
                functions: config.config.functions.clone(),
                auth_info: if config.config.auth_info.is_empty() {
                    None
                } else {
                    let auth_info = config
                        .config
                        .auth_info
                        .clone()
                        .into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<Vec<_>, _>>()?;
                    Some(auth_info)
                },
            },
        )
        .await?;

        let mut evaluated_components = self
            .evaluate_components(
                config,
                &component_definition_packages,
                app_analysis,
                app_udf_config,
                unix_timestamp,
                user_environment_variables.clone(),
                system_env_var_overrides,
            )
            .await?;
        // Build and typecheck the component tree. We don't strictly need to do this
        // before `/finish_push`, but it's better to fail fast here on errors before
        // waiting for schema backfills to complete.
        let initializer_evaluator = ApplicationInitializerEvaluator::new(
            self,
            config,
            evaluated_components
                .iter()
                .map(|(k, v)| (k.clone(), v.definition.clone()))
                .collect(),
        )?;
        let ctx = TypecheckContext::new(&evaluated_components, &initializer_evaluator);
        let app = ctx.instantiate_root().await?;

        let schema_change = self
            ._handle_schema_change_in_start_push(&app, &evaluated_components)
            .await?;
        self.database
            .load_indexes_into_memory(btreeset! { SCHEMAS_TABLE.clone() })
            .await?;

        // TODO(ENG-7533): Clean up exports from the start push response when we've
        // updated clients to use `functions` directly.
        for (path, definition) in evaluated_components.iter_mut() {
            // We don't need to include exports for the root since we don't use codegen
            // for the app's `api` object.
            if path.is_root() {
                continue;
            }
            anyhow::ensure!(definition.definition.exports.is_empty());
            definition.definition.exports = file_based_exports(&definition.functions)?;
        }

        let resp = StartPushResponse {
            environment_variables: user_environment_variables,
            external_deps_id,
            component_definition_packages,
            app_auth: auth_info,
            analysis: evaluated_components,
            app,
            schema_change,
        };
        Ok(resp)
    }

    #[fastrace::trace]
    async fn _handle_schema_change_in_start_push(
        &self,
        app: &CheckedComponent,
        evaluated_components: &BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>,
    ) -> anyhow::Result<SchemaChange> {
        // Even in dry run mode, we need to commit the schema changes so that
        // wait_for_schema can validate the schema against existing data.
        let (_ts, schema_change) = self
            .execute_with_occ_retries(
                Identity::system(),
                FunctionUsageTracker::new(),
                WriteSource::new("start_push"),
                |tx| {
                    async move {
                        let schema_change = ComponentConfigModel::new(tx)
                            .start_component_schema_changes(app, evaluated_components)
                            .await?;
                        Ok(schema_change)
                    }
                    .into()
                },
            )
            .await?;
        Ok(schema_change)
    }

    #[fastrace::trace]
    async fn evaluate_components(
        &self,
        config: &ProjectConfig,
        component_definition_packages: &BTreeMap<ComponentDefinitionPath, SourcePackage>,
        app_analysis: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
        app_udf_config: UdfConfig,
        unix_timestamp: UnixTimestamp,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
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
                import_phase_rng_seed: self.runtime.rng().random(),
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
                    BTreeMap::new(),
                )
                .await?;
            anyhow::ensure!(component_analysis_by_def_path
                .insert(component_def.definition_path.clone(), component_analysis)
                .is_none());

            if let Some(schema_module) = &component_def.schema {
                let schema = match self.evaluate_schema(schema_module.clone()).await {
                    Ok(schema) => schema,
                    Err(e) => {
                        // Try to downcast to a JsError and turn that into a user-visible error if
                        // so.
                        let e = e.downcast::<JsError>()?;
                        anyhow::bail!(ErrorMetadata::bad_request("InvalidSchema", e.to_string()));
                    },
                };
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

            let definition_result = self
                .evaluate_app_definitions(
                    app_definition.clone(),
                    component_definitions,
                    dependency_graph,
                    user_environment_variables,
                    system_env_var_overrides,
                )
                .await;
            evaluated_definitions = match definition_result {
                Ok(r) => r,
                Err(e) => {
                    let e = e.downcast::<JsError>()?;
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidConvexConfig",
                        e.to_string()
                    ));
                },
            };
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
        Ok(evaluated_components)
    }

    #[fastrace::trace]
    pub async fn evaluate_app_definitions(
        &self,
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        self.runner
            .evaluate_app_definitions(
                app_definition,
                component_definitions,
                dependency_graph,
                user_environment_variables,
                system_env_var_overrides,
            )
            .await
    }

    #[fastrace::trace]
    pub async fn wait_for_schema(
        &self,
        identity: Identity,
        schema_change: SchemaChange,
        timeout: Duration,
    ) -> anyhow::Result<SchemaStatus> {
        let deadline = self.runtime().monotonic_now() + timeout;
        loop {
            let (status, token) = self
                .load_component_schema_status(&identity, &schema_change)
                .await?;
            let now = self.runtime().monotonic_now();
            let in_progress = matches!(status, SchemaStatus::InProgress { .. });
            if !in_progress || now > deadline {
                return Ok(status);
            }
            let subscription = self.subscribe(token).await?;
            tokio::select! {
                _ = subscription.wait_for_invalidation() => {},
                _ = self.runtime.wait(deadline - now)
                    .in_span(fastrace::Span::enter_with_local_parent("wait_for_deadline"))
                 => {},
            }
        }
    }

    #[fastrace::trace]
    pub(crate) async fn load_component_schema_status(
        &self,
        identity: &Identity,
        schema_change: &SchemaChange,
    ) -> anyhow::Result<(SchemaStatus, Token)> {
        let mut tx = self.begin(identity.clone()).await?;
        let mut components_status = BTreeMap::new();
        for (component_path, schema_id) in &schema_change.schema_ids {
            let Some(schema_id) = schema_id else {
                continue;
            };
            let schema_table_number = tx.table_mapping().tablet_number(schema_id.table())?;
            let schema_id = ResolvedDocumentId::new(
                schema_id.table(),
                DeveloperDocumentId::new(schema_table_number, schema_id.internal_id()),
            );
            let document = tx
                .get(schema_id)
                .await?
                .context("Missing schema document")?;
            let SchemaMetadata { state, .. } = document.into_value().0.try_into()?;
            let schema_validation_complete = match state {
                SchemaState::Pending => false,
                SchemaState::Active | SchemaState::Validated => true,
                SchemaState::Failed { error, table_name } => {
                    let status = SchemaStatus::Failed {
                        error,
                        component_path: component_path.clone(),
                        table_name,
                    };
                    return Ok((status, tx.into_token()?));
                },
                SchemaState::Overwritten => {
                    return Ok((SchemaStatus::RaceDetected, tx.into_token()?))
                },
            };

            let component_id = if component_path.is_root() {
                ComponentId::Root
            } else {
                let existing =
                    BootstrapComponentsModel::new(&mut tx).resolve_path(component_path)?;
                let allocated = schema_change.allocated_component_ids.get(component_path);
                let internal_id = match (existing, allocated) {
                    (None, Some(id)) => *id,
                    (Some(doc), None) => doc.id().into(),
                    r => anyhow::bail!("Invalid existing component state: {r:?}"),
                };
                ComponentId::Child(internal_id)
            };
            let namespace = TableNamespace::from(component_id);
            let mut indexes_complete = 0;
            let mut indexes_total = 0;
            for index in IndexModel::new(&mut tx)
                .get_application_indexes(namespace)
                .await?
            {
                // Skip counting indexes that are staged
                if index.config.is_staged() {
                    continue;
                }
                if !index.config.is_backfilling() {
                    indexes_complete += 1;
                }
                indexes_total += 1;
            }
            components_status.insert(
                component_path.clone(),
                ComponentSchemaStatus {
                    schema_validation_complete,
                    indexes_complete,
                    indexes_total,
                },
            );
        }
        let status = if components_status.values().all(|c| c.is_complete()) {
            SchemaStatus::Complete
        } else {
            SchemaStatus::InProgress {
                components: components_status,
            }
        };
        let token = tx.into_token()?;
        Ok((status, token))
    }

    #[fastrace::trace]
    pub async fn finish_push(
        &self,
        identity: Identity,
        mut start_push: StartPushResponse,
    ) -> anyhow::Result<FinishPushDiff> {
        // Download all source packages. We can remove this once we don't store source
        // in the database.
        let mut downloaded_source_packages = BTreeMap::new();
        for (definition_path, source_package) in &start_push.component_definition_packages {
            let package = download_package(
                self.modules_storage().clone(),
                source_package.storage_key.clone(),
                source_package.sha256.clone(),
            )
            .await?;
            downloaded_source_packages.insert(definition_path.clone(), package);
        }

        // TODO(ENG-7533): Strip out exports from the `StartPushResponse` since we don't
        // want to actually store it in the database. Remove this path once
        // we've stopped sending exports down to the client.
        for definition in start_push.analysis.values_mut() {
            definition.definition.exports = BTreeMap::new();
        }

        let diff = self
            .execute_with_audit_log_events_and_occ_retries(identity.clone(), "finish_push", |tx| {
                let start_push = &start_push;
                let downloaded_source_packages = &downloaded_source_packages;
                async move {
                    // Validate that environment variables haven't changed since `start_push`.
                    let environment_variables =
                        EnvironmentVariablesModel::new(tx).get_all().await?;
                    if environment_variables != start_push.environment_variables {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "RaceDetected",
                            "Environment variables have changed during push"
                        ));
                    }

                    // Update app state: auth info and UDF server version.
                    let auth_diff = AuthInfoModel::new(tx)
                        .put(start_push.app_auth.clone())
                        .await?;

                    // Diff the component definitions.
                    let (definition_diffs, modules_by_definition, udf_config_by_definition) =
                        ComponentDefinitionConfigModel::new(tx)
                            .apply_component_definitions_diff(
                                &start_push.analysis,
                                &start_push.component_definition_packages,
                                downloaded_source_packages,
                            )
                            .await?;

                    // Diff component tree.
                    let component_diffs = ComponentConfigModel::new(tx)
                        .apply_component_tree_diff(
                            &start_push.app,
                            udf_config_by_definition,
                            &start_push.schema_change,
                            modules_by_definition,
                        )
                        .await?;

                    let diffs = PushComponentDiffs {
                        auth_diff: auth_diff.clone(),
                        component_diffs: component_diffs.clone(),
                    };
                    let audit_log_events =
                        vec![DeploymentAuditLogEvent::PushConfigWithComponents { diffs }];
                    let diff = FinishPushDiff {
                        auth_diff,
                        definition_diffs,
                        component_diffs,
                    };
                    Ok((diff, audit_log_events))
                }
                .in_span(Span::enter_with_local_parent("finish_push_tx"))
                .into()
            })
            .await?;

        Ok(diff)
    }

    /// N.B.: does not check auth
    pub async fn push_config_no_components(
        &self,
        identity: Identity,
        config_file: ConfigFile,
        modules: Vec<ModuleConfig>,
        udf_server_version: Version,
        schema_id: Option<String>,
        node_dependencies: Option<Vec<NodeDependencyJson>>,
        node_version: Option<NodeVersion>,
    ) -> anyhow::Result<(PushAnalytics, PushMetrics)> {
        let begin_build_external_deps = Instant::now();
        // Upload external node dependencies separately
        let external_deps_id_and_pkg = if let Some(deps) = node_dependencies
            && !deps.is_empty()
        {
            let deps: Vec<_> = deps.into_iter().map(NodeDependency::from).collect();
            Some(self.build_external_node_deps(deps).await?)
        } else {
            None
        };
        let end_build_external_deps = Instant::now();
        let external_deps_pkg_size = external_deps_id_and_pkg
            .as_ref()
            .map(|(_, pkg)| pkg.package_size)
            .unwrap_or_default();

        let source_package = self
            .upload_package(&modules, external_deps_id_and_pkg, node_version)
            .await?;
        let end_upload_source_package = Instant::now();
        // Verify that we have not exceeded the max zipped or unzipped file size
        let combined_pkg_size = source_package.package_size + external_deps_pkg_size;
        combined_pkg_size.verify_size()?;

        let udf_config = UdfConfig {
            server_version: udf_server_version,
            // Generate a new seed and timestamp to be used at import time.
            import_phase_rng_seed: self.runtime.rng().random(),
            import_phase_unix_timestamp: self.runtime.unix_timestamp(),
        };
        let begin_analyze = Instant::now();
        // Note: This is not transactional with the rest of the deploy to avoid keeping
        // a transaction open for a long time.
        let mut tx = self.begin(Identity::system()).await?;
        let user_environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
        let system_env_var_overrides = system_env_var_overrides(&mut tx).await?;
        drop(tx);
        // Run analyze to make sure the new modules are valid.
        let (auth_module, analyze_results) = self
            .analyze_modules_with_auth_config(
                udf_config.clone(),
                modules.clone(),
                source_package.clone(),
                user_environment_variables,
                system_env_var_overrides,
            )
            .await?;
        let end_analyze = Instant::now();
        let (
            ConfigMetadataAndSchema {
                config_metadata,
                schema,
            },
            occ_stats,
        ) = self
            .apply_config_with_retries(
                identity.clone(),
                ApplyConfigArgs {
                    auth_module,
                    config_file,
                    schema_id,
                    modules: modules.clone(),
                    udf_config: udf_config.clone(),
                    source_package,
                    analyze_results: analyze_results.clone(),
                },
            )
            .await?;

        Ok((
            PushAnalytics {
                config: config_metadata,
                modules,
                udf_server_version: udf_config.server_version,
                analyze_results,
                schema,
            },
            PushMetrics {
                build_external_deps_time: end_build_external_deps - begin_build_external_deps,
                upload_source_package_time: end_upload_source_package - end_build_external_deps,
                analyze_time: end_analyze - begin_analyze,
                occ_stats,
            },
        ))
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
impl<RT: Runtime> InitializerEvaluator for ApplicationInitializerEvaluator<'_, RT> {
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

    pub functions: String,

    pub app_definition: AppDefinitionConfigJson,
    pub component_definitions: Vec<ComponentDefinitionConfigJson>,

    pub node_dependencies: Vec<NodeDependencyJson>,

    pub node_version: Option<String>,
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
            node_version: self.node_version.map(|v| v.parse()).transpose()?,
        })
    }
}

#[derive(Debug)]
pub struct StartPushResponse {
    // We read the current environment variables when evaluating the definitions, so we need to
    // cancel the push if they change before the commit point.
    pub environment_variables: BTreeMap<EnvVarName, EnvVarValue>,

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
        let functions: Vec<ModuleConfig> = value
            .functions
            .into_iter()
            .map(TryInto::try_into)
            .collect::<anyhow::Result<_>>()?;
        for module in &functions {
            match module.environment {
                ModuleEnvironment::Node => {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "NodeActionsNotSupported",
                        format!(
                            "Node actions are not supported in components. Remove `\"use node;\" \
                             from {}",
                            module.path.as_str()
                        )
                    ));
                },
                ModuleEnvironment::Invalid | ModuleEnvironment::Isolate => {},
            }
        }
        Ok(Self {
            definition_path: value.definition_path.parse()?,
            definition: value.definition.try_into()?,
            dependencies: value
                .dependencies
                .into_iter()
                .map(|s| s.parse())
                .collect::<anyhow::Result<_>>()?,
            schema: value.schema.map(TryInto::try_into).transpose()?,
            functions,
            udf_server_version: value.udf_server_version.parse()?,
        })
    }
}

/// API level structure for representing modules as Json
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModuleJson {
    pub path: String,
    pub source: String,
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
            source: source.to_string(),
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
            source: ModuleSource::new(&source),
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

#[derive(Debug, Default)]
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
