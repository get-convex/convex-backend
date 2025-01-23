use std::{
    collections::BTreeMap,
    time::{
        Duration,
        Instant,
    },
};

use anyhow::Context;
use application::{
    deploy_config::{
        ModuleJson,
        NodeDependencyJson,
    },
    Application,
    ApplyConfigArgs,
    ConfigMetadataAndSchema,
};
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    components::ComponentId,
    http::{
        extract::Json,
        HttpResponseError,
    },
    runtime::Runtime,
    schemas::DatabaseSchema,
    types::NodeDependency,
    version::Version,
};
use database::OccRetryStats;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use keybroker::Identity;
use model::{
    config::{
        types::{
            ConfigFile,
            ConfigMetadata,
            ModuleConfig,
        },
        ConfigModel,
    },
    environment_variables::EnvironmentVariablesModel,
    modules::module_versions::AnalyzedModule,
    source_packages::types::PackageSize,
    udf_config::types::UdfConfig,
};
use rand::Rng;
use runtime::prod::ProdRuntime;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedModulePath;
use value::ConvexObject;

use crate::{
    admin::{
        must_be_admin_from_key,
        must_be_admin_with_write_access,
    },
    EmptyResponse,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConfigRequest {
    pub admin_key: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConfigResponse {
    pub config: JsonValue,
    pub modules: Vec<ModuleJson>,
    pub udf_server_version: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConfigHashesResponse {
    pub config: JsonValue,
    pub module_hashes: Vec<ModuleHashJson>,
    pub udf_server_version: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClientPushMetrics {
    pub typecheck: f64,
    pub bundle: f64,
    pub schema_push: f64,
    pub code_pull: f64,
    pub total_before_push: f64,
    pub module_diff_stats: Option<ModuleDiffStats>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDiffStats {
    updated: ModuleDiffStat,
    added: ModuleDiffStat,
    identical: ModuleDiffStat,
    num_dropped: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDiffStat {
    count: usize,
    size: usize,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code)]
pub struct BundledModuleInfoJson {
    name: String,
    platform: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigJson {
    pub config: ConfigFile,
    pub modules: Vec<ModuleJson>,
    pub admin_key: String,
    pub udf_server_version: String,
    // Used in CLI >= 0.14.0, None when there is no schema file.
    pub schema_id: Option<String>,
    // Used in CLI >= future
    pub push_metrics: Option<ClientPushMetrics>,
    // Use for external node dependencies
    // TODO: add what version of CLI this is used for
    pub node_dependencies: Option<Vec<NodeDependencyJson>>,
    // Additional information about the names of the bundled modules.
    // We can use that for stats as well provide better debug messages.
    // Used in CLI >= future
    pub bundled_module_infos: Option<Vec<BundledModuleInfoJson>>,
}

pub struct ConfigStats {
    pub num_node_modules: usize,
    pub size_node_modules: usize,
    pub num_v8_modules: usize,
    pub size_v8_modules: usize,
}

static NODE_ENVIRONMENT: &str = "node";
impl ConfigJson {
    pub fn stats(&self) -> ConfigStats {
        let num_node_modules = self
            .modules
            .iter()
            .filter(|module| module.environment.as_deref() == Some(NODE_ENVIRONMENT))
            .count();
        let size_node_modules = self
            .modules
            .iter()
            .filter(|module| module.environment.as_deref() == Some(NODE_ENVIRONMENT))
            .fold(0, |acc, e| {
                acc + e.source.len() + e.source_map.as_ref().map_or(0, |sm| sm.len())
            });
        let size_v8_modules = self
            .modules
            .iter()
            .filter(|module| module.environment.as_deref() != Some(NODE_ENVIRONMENT))
            .fold(0, |acc, e| {
                acc + e.source.len() + e.source_map.as_ref().map_or(0, |sm| sm.len())
            });
        let num_v8_modules = self
            .modules
            .iter()
            .filter(|module| module.environment.as_deref() != Some(NODE_ENVIRONMENT))
            .count();
        ConfigStats {
            num_v8_modules,
            num_node_modules,
            size_v8_modules,
            size_node_modules,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleHashJson {
    path: String,
    hash: String,
    environment: Option<String>,
}

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

#[debug_handler]
pub async fn get_config(
    State(st): State<LocalAppState>,
    Json(req): Json<GetConfigRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let identity = must_be_admin_from_key(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key,
    )
    .await?;

    let mut tx = st.application.begin(identity).await?;
    let component = ComponentId::Root; // This endpoint is only used pre-components.
    let (config, modules, udf_config) = ConfigModel::new(&mut tx, component)
        .get_with_module_source(st.application.modules_cache())
        .await?;
    let config = ConvexObject::try_from(config)?;
    let config: JsonValue = config.into();

    let modules = modules.into_iter().map(|m| m.into()).collect();
    let udf_server_version = udf_config.map(|config| format!("{}", config.server_version));
    // Should this be committed?
    st.application.commit(tx, "get_config").await?;
    Ok(Json(GetConfigResponse {
        config,
        modules,
        udf_server_version,
    }))
}

#[debug_handler]
pub async fn get_config_hashes(
    State(st): State<LocalAppState>,
    Json(req): Json<GetConfigRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let identity = must_be_admin_from_key(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key,
    )
    .await?;

    let mut tx = st.application.begin(identity).await?;
    let component = ComponentId::Root; // This endpoint is not used in components push.
    let (config, modules, udf_config) = ConfigModel::new(&mut tx, component)
        .get_with_module_metadata()
        .await?;
    let module_hashes: Vec<_> = modules
        .into_iter()
        .map(|m| ModuleHashJson {
            path: m.path.clone().into(),
            hash: m.sha256.as_hex(),
            environment: Some(m.environment.to_string()),
        })
        .collect();
    let config = ConvexObject::try_from(config)?;
    let config: JsonValue = config.into();

    let udf_server_version = udf_config.map(|config| format!("{}", config.server_version));
    Ok(Json(GetConfigHashesResponse {
        config,
        module_hashes,
        udf_server_version,
    }))
}

#[debug_handler]
pub async fn push_config(
    State(st): State<LocalAppState>,
    Json(req): Json<ConfigJson>,
) -> Result<impl IntoResponse, HttpResponseError> {
    push_config_handler(&st.application, req)
        .await
        .map_err(|e| e.wrap_error_message(|msg| format!("Hit an error while pushing:\n{msg}")))?;

    Ok(Json(EmptyResponse {}))
}

#[fastrace::trace]
pub async fn push_config_handler(
    application: &Application<ProdRuntime>,
    config: ConfigJson,
) -> anyhow::Result<(Identity, PushAnalytics, PushMetrics)> {
    let modules: Vec<ModuleConfig> = config
        .modules
        .into_iter()
        .map(|m| m.try_into())
        .collect::<anyhow::Result<Vec<_>>>()?;

    let identity = application
        .app_auth()
        .check_key(config.admin_key, application.instance_name())
        .await
        .context("bad admin key error")?;

    must_be_admin_with_write_access(&identity)?;

    let udf_server_version = Version::parse(&config.udf_server_version).context(
        ErrorMetadata::bad_request("InvalidVersion", "The function version is invalid"),
    )?;

    let begin_build_external_deps = Instant::now();
    // Upload external node dependencies separately
    let external_deps_id_and_pkg = if let Some(deps) = config.node_dependencies
        && !deps.is_empty()
    {
        let deps: Vec<_> = deps.into_iter().map(NodeDependency::from).collect();
        Some(application.build_external_node_deps(deps).await?)
    } else {
        None
    };
    let end_build_external_deps = Instant::now();
    let external_deps_pkg_size = external_deps_id_and_pkg
        .as_ref()
        .map(|(_, pkg)| pkg.package_size)
        .unwrap_or(PackageSize::default());

    let source_package = application
        .upload_package(&modules, external_deps_id_and_pkg)
        .await?;
    let end_upload_source_package = Instant::now();
    // Verify that we have not exceeded the max zipped or unzipped file size
    let combined_pkg_size = source_package.package_size + external_deps_pkg_size;
    combined_pkg_size.verify_size()?;

    let udf_config = UdfConfig {
        server_version: udf_server_version,
        // Generate a new seed and timestamp to be used at import time.
        import_phase_rng_seed: application.runtime().rng().gen(),
        import_phase_unix_timestamp: application.runtime().unix_timestamp(),
    };
    let begin_analyze = Instant::now();
    // Note: This is not transactional with the rest of the deploy to avoid keeping
    // a transaction open for a long time.
    let mut tx = application.begin(Identity::system()).await?;
    let environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
    drop(tx);
    // Run analyze to make sure the new modules are valid.
    let (auth_module, analyze_results) = application
        .analyze_modules_with_auth_config(
            udf_config.clone(),
            modules.clone(),
            source_package.clone(),
            environment_variables,
        )
        .await?;
    let end_analyze = Instant::now();
    let (
        ConfigMetadataAndSchema {
            config_metadata,
            schema,
        },
        occ_stats,
    ) = application
        .apply_config_with_retries(
            identity.clone(),
            ApplyConfigArgs {
                auth_module,
                config_file: config.config,
                schema_id: config.schema_id,
                modules: modules.clone(),
                udf_config: udf_config.clone(),
                source_package,
                analyze_results: analyze_results.clone(),
            },
        )
        .await?;

    Ok((
        identity,
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
