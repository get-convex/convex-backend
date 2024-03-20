use std::collections::BTreeMap;

use anyhow::{
    anyhow,
    Context,
};
use application::{
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
    http::{
        extract::Json,
        HttpResponseError,
    },
    runtime::Runtime,
    schemas::DatabaseSchema,
    sha256::Sha256,
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
            deprecated_extract_environment_from_path,
            ConfigFile,
            ConfigMetadata,
            ModuleConfig,
            AUTH_CONFIG_FILE_NAME,
        },
        ConfigModel,
    },
    modules::module_versions::{
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    source_packages::types::{
        PackageSize,
        SourcePackage,
    },
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
    admin::must_be_admin_from_keybroker,
    parse::parse_module_path,
    EmptyResponse,
    LocalAppState,
};

// The maximum number of user defined modules
pub const MAX_USER_MODULES: usize = 10000;

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

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PushMetrics {
    pub typecheck: f64,
    pub bundle: f64,
    pub schema_push: f64,
    pub code_pull: f64,
    pub total_before_push: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDependencyJson {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BundledModuleInfoJson {
    name: String,
    platform: String,
}

impl From<NodeDependencyJson> for NodeDependency {
    fn from(value: NodeDependencyJson) -> Self {
        Self {
            package: value.name,
            version: value.version,
        }
    }
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
    pub push_metrics: Option<PushMetrics>,
    // Use for external node dependencies
    // TODO: add what version of CLI this is used for
    pub node_dependencies: Option<Vec<NodeDependencyJson>>,
    // Additional information about the names of the bundled modules.
    // We can use that for stats as well provide better debug messages.
    // Used in CLI >= future
    #[allow(dead_code)]
    pub bundled_module_infos: Option<Vec<BundledModuleInfoJson>>,
}

static NODE_ENVIRONMENT: &str = "node";
impl ConfigJson {
    pub fn stats(&self) -> (usize, usize, usize, usize) {
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
        (
            num_v8_modules,
            num_node_modules,
            size_v8_modules,
            size_node_modules,
        )
    }
}

/// API level structure for representing modules as Json
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleJson {
    path: String,
    source: ModuleSource,
    source_map: Option<SourceMap>,
    environment: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleHashJson {
    path: String,
    hash: String,
    environment: Option<String>,
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

impl ModuleHashJson {
    fn hash(
        ModuleConfig {
            path,
            source,
            source_map,
            environment,
        }: ModuleConfig,
    ) -> ModuleHashJson {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        if let Some(source_map) = source_map {
            hasher.update(source_map.as_bytes());
        }
        let hash = hasher.finalize();
        ModuleHashJson {
            path: path.into(),
            hash: hex::encode(hash),
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

pub struct PushAnalytics {
    pub config: ConfigMetadata,
    pub modules: Vec<ModuleConfig>,
    pub udf_server_version: Version,
    pub analyze_results: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    pub schema: Option<DatabaseSchema>,
    pub occ_stats: OccRetryStats,
}

#[debug_handler]
pub async fn get_config(
    State(st): State<LocalAppState>,
    Json(req): Json<GetConfigRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let identity = must_be_admin_from_keybroker(
        st.application.key_broker(),
        Some(st.instance_name.clone()),
        req.admin_key,
    )?;

    let mut tx = st.application.begin(identity).await?;
    let (config, modules, udf_config) = ConfigModel::new(&mut tx).get().await?;
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
    let identity = must_be_admin_from_keybroker(
        st.application.key_broker(),
        Some(st.instance_name.clone()),
        req.admin_key,
    )?;

    let mut tx = st.application.begin(identity).await?;
    let (config, modules, udf_config) = ConfigModel::new(&mut tx).get().await?;
    let config = ConvexObject::try_from(config)?;
    let config: JsonValue = config.into();

    let module_hashes = modules.into_iter().map(ModuleHashJson::hash).collect();
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

pub async fn push_config_handler(
    application: &Application<ProdRuntime>,
    config: ConfigJson,
) -> anyhow::Result<(Identity, PushAnalytics)> {
    let modules: Vec<ModuleConfig> = config
        .modules
        .into_iter()
        .map(|m| m.try_into())
        .collect::<anyhow::Result<Vec<_>>>()?;
    let identity = application
        .key_broker()
        .check_admin_key(&config.admin_key)
        .context("bad admin key error")?;

    let udf_server_version = Version::parse(&config.udf_server_version).context(
        ErrorMetadata::bad_request("InvalidVersion", "The function version is invalid"),
    )?;

    // Upload external node dependencies separately
    let external_deps_id_and_pkg = if let Some(deps) = config.node_dependencies
        && !deps.is_empty()
    {
        let deps: Vec<_> = deps.into_iter().map(NodeDependency::from).collect();
        Some(application.build_external_node_deps(deps).await?)
    } else {
        None
    };
    let external_deps_pkg_size = external_deps_id_and_pkg
        .as_ref()
        .map(|(_, pkg)| pkg.package_size)
        .unwrap_or(PackageSize::default());

    let source_package = application
        .upload_package(&modules, external_deps_id_and_pkg)
        .await?;

    // Verify that we have not exceeded the max zipped or unzipped file size
    let combined_pkg_size = source_package
        .as_ref()
        .map(|pkg| pkg.package_size)
        .unwrap_or(PackageSize::default())
        + external_deps_pkg_size;
    combined_pkg_size.verify_size()?;

    let udf_config = UdfConfig {
        server_version: udf_server_version,
        // Generate a new seed and timestamp to be used at import time.
        import_phase_rng_seed: application.runtime().with_rng(|rng| rng.gen()),
        import_phase_unix_timestamp: application.runtime().unix_timestamp(),
    };

    // Run analyze to make sure the new modules are valid.
    let (auth_module, analyze_result) = analyze_modules_with_auth_config(
        application,
        udf_config.clone(),
        modules.clone(),
        source_package.clone(),
    )
    .await?;
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
                analyze_results: analyze_result.clone(),
            },
        )
        .await?;

    Ok((
        identity,
        PushAnalytics {
            config: config_metadata,
            modules,
            udf_server_version: udf_config.server_version,
            analyze_results: analyze_result,
            schema,
            occ_stats,
        },
    ))
}

async fn analyze_modules_with_auth_config(
    application: &Application<ProdRuntime>,
    udf_config: UdfConfig,
    modules: Vec<ModuleConfig>,
    source_package: Option<SourcePackage>,
) -> anyhow::Result<(
    Option<ModuleConfig>,
    BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
)> {
    // Don't analyze the auth config module
    let (auth_modules, analyzed_modules): (Vec<_>, Vec<_>) =
        modules.into_iter().partition(|module| {
            module.path.clone().canonicalize() == AUTH_CONFIG_FILE_NAME.parse().unwrap()
        });
    let auth_module = auth_modules.first();

    let mut analyze_result =
        analyze_modules(application, udf_config, analyzed_modules, source_package).await?;

    // Add an empty analyzed result for the auth config module
    if let Some(auth_module) = auth_module {
        analyze_result.insert(
            auth_module.path.clone().canonicalize(),
            AnalyzedModule::default(),
        );
    }
    Ok((auth_module.cloned(), analyze_result))
}

// Helper method to call analyze and throw appropriate HttpError.
pub async fn analyze_modules(
    application: &Application<ProdRuntime>,
    udf_config: UdfConfig,
    modules: Vec<ModuleConfig>,
    source_package: Option<SourcePackage>,
) -> anyhow::Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>> {
    let num_dep_modules = modules.iter().filter(|m| m.path.is_deps()).count();
    anyhow::ensure!(
        modules.len() - num_dep_modules <= MAX_USER_MODULES,
        ErrorMetadata::bad_request(
            "InvalidModules",
            format!(
                r#"Too many function files ({} > maximum {}) in "convex/". See our docs (https://docs.convex.dev/using/writing-convex-functions#using-libraries) for more details."#,
                modules.len() - num_dep_modules,
                MAX_USER_MODULES
            ),
        )
    );
    // We exclude dependency modules from the user limit since they don't depend on
    // the developer. We don't expect dependencies to be more than the user defined
    // modules though. If we ever have crazy amount of dependency modules,
    // throw a system errors so we can debug.
    anyhow::ensure!(
        modules.len() <= 2 * MAX_USER_MODULES,
        "Too many dependencies modules! Dependencies: {}, Total modules: {}",
        num_dep_modules,
        modules.len()
    );

    // Run analyze the modules to make sure they are valid.
    match application
        .analyze(udf_config, modules, source_package)
        .await?
    {
        Ok(m) => Ok(m),
        Err(js_error) => {
            let e = ErrorMetadata::bad_request(
                "InvalidModules",
                format!(
                    "Loading the pushed modules encountered the following
    error:\n{js_error}"
                ),
            );
            Err(anyhow!(js_error).context(e))
        },
    }
}
