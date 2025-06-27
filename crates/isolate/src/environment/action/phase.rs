use std::{
    collections::BTreeMap,
    mem,
    sync::Arc,
};

use anyhow::Context;
use common::{
    components::{
        ComponentId,
        Reference,
        Resource,
    },
    http::RequestDestination,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        ConvexOrigin,
        ModuleEnvironment,
    },
};
use database::{
    BootstrapComponentsModel,
    Transaction,
};
use errors::ErrorMetadata;
use model::{
    canonical_urls::CanonicalUrlsModel,
    components::ComponentsModel,
    environment_variables::{
        types::{
            EnvVarName,
            EnvVarValue,
        },
        EnvironmentVariablesModel,
    },
    modules::{
        module_versions::FullModuleSource,
        types::ModuleMetadata,
        ModuleModel,
    },
    source_packages::SourcePackageModel,
    udf_config::UdfConfigModel,
};
use parking_lot::Mutex;
use rand::{
    Rng,
    SeedableRng,
};
use rand_chacha::ChaCha12Rng;
use sync_types::{
    CanonicalizedModulePath,
    ModulePath,
};
use udf::environment::parse_system_env_var_overrides;
use value::{
    identifier::Identifier,
    ConvexValue,
};

use crate::{
    concurrency_limiter::ConcurrencyPermit,
    environment::{
        action::task::TaskRequestEnum,
        helpers::{
            permit::with_release_permit,
            Phase,
        },
        ModuleCodeCacheResult,
    },
    module_cache::ModuleCache,
    timeout::Timeout,
};

/// This struct is similar to UdfPhase. Action execution also has two
/// phases: 1. We start by loading all imported modules, evaluating them, and
/// inserting them into the module map. 2. We find the endpoint and run it.
///
/// Unlike `UdfPhase`, the DB transaction is read-only (used for reading modules
/// and environment variables), and all writes will be handled in their own
/// separate transactions.
pub struct ActionPhase<RT: Runtime> {
    component: ComponentId,
    phase: Phase,
    pub rt: RT,
    preloaded: ActionPreloaded<RT>,
}

enum ActionPreloaded<RT: Runtime> {
    Created {
        tx: Transaction<RT>,
        module_loader: Arc<dyn ModuleCache<RT>>,
        default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        resources: Arc<Mutex<BTreeMap<Reference, Resource>>>,
        convex_origin_override: Arc<Mutex<Option<ConvexOrigin>>>,
    },
    Preloading,
    Ready {
        module_loader: Arc<dyn ModuleCache<RT>>,
        modules: BTreeMap<CanonicalizedModulePath, (ModuleMetadata, Arc<FullModuleSource>)>,
        env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        component_arguments: Option<BTreeMap<Identifier, ConvexValue>>,
        rng: Option<ChaCha12Rng>,
        import_time_unix_timestamp: Option<UnixTimestamp>,
    },
}

impl<RT: Runtime> ActionPhase<RT> {
    pub fn new(
        rt: RT,
        component: ComponentId,
        tx: Transaction<RT>,
        module_loader: Arc<dyn ModuleCache<RT>>,
        default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        resources: Arc<Mutex<BTreeMap<Reference, Resource>>>,
        convex_origin_override: Arc<Mutex<Option<ConvexOrigin>>>,
    ) -> Self {
        Self {
            component,
            phase: Phase::Importing,
            rt,
            preloaded: ActionPreloaded::Created {
                tx,
                module_loader,
                default_system_env_vars,
                resources,
                convex_origin_override,
            },
        }
    }

    #[fastrace::trace]
    pub async fn initialize(
        &mut self,
        timeout: &mut Timeout<RT>,
        permit_slot: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(self.phase == Phase::Importing);

        let preloaded = mem::replace(&mut self.preloaded, ActionPreloaded::Preloading);
        let ActionPreloaded::Created {
            mut tx,
            module_loader,
            default_system_env_vars,
            resources,
            convex_origin_override,
        } = preloaded
        else {
            anyhow::bail!("ActionPhase initialized twice");
        };

        let component_id = self.component;

        let udf_config = with_release_permit(
            timeout,
            permit_slot,
            UdfConfigModel::new(&mut tx, component_id.into()).get(),
        )
        .await?;

        let rng = udf_config
            .as_ref()
            .map(|c| ChaCha12Rng::from_seed(c.import_phase_rng_seed));
        let import_time_unix_timestamp = udf_config.as_ref().map(|c| c.import_phase_unix_timestamp);

        let (module_metadata, source_package) = with_release_permit(timeout, permit_slot, async {
            let module_metadata = ModuleModel::new(&mut tx)
                .get_all_metadata(component_id)
                .await?;
            let source_package = SourcePackageModel::new(&mut tx, component_id.into())
                .get_latest()
                .await?;
            let loaded_resources = ComponentsModel::new(&mut tx)
                .preload_resources(component_id)
                .await?;
            {
                let mut resources = resources.lock();
                *resources = loaded_resources;
            }
            Ok((module_metadata, source_package))
        })
        .await?;

        let modules = with_release_permit(timeout, permit_slot, async {
            let mut modules = BTreeMap::new();
            for metadata in module_metadata {
                if metadata.path.is_system() {
                    continue;
                }
                let path = metadata.path.clone();
                let module = module_loader
                    .get_module_with_metadata(
                        metadata.clone(),
                        source_package.clone().context("source package not found")?,
                    )
                    .await?;
                modules.insert(path, (metadata.into_value(), module));
            }
            Ok(modules)
        })
        .await?;

        let canonical_urls = with_release_permit(
            timeout,
            permit_slot,
            CanonicalUrlsModel::new(&mut tx).get_canonical_urls(),
        )
        .await?;
        if let Some(cloud_url) = canonical_urls.get(&RequestDestination::ConvexCloud) {
            *convex_origin_override.lock() = Some(ConvexOrigin::from(&cloud_url.url));
        }
        // Environment variables are not accessible in component functions.
        let env_vars = if self.component.is_root() {
            let mut env_vars = default_system_env_vars;
            env_vars.extend(parse_system_env_var_overrides(canonical_urls)?);
            let user_env_vars = with_release_permit(
                timeout,
                permit_slot,
                EnvironmentVariablesModel::new(&mut tx).get_all(),
            )
            .await?;
            env_vars.extend(user_env_vars);
            env_vars
        } else {
            BTreeMap::new()
        };

        let component_arguments = if self.component.is_root() {
            None
        } else {
            Some(
                with_release_permit(
                    timeout,
                    permit_slot,
                    BootstrapComponentsModel::new(&mut tx).load_component_args(component_id),
                )
                .await?,
            )
        };

        self.preloaded = ActionPreloaded::Ready {
            module_loader,
            modules,
            env_vars,
            component_arguments,
            rng,
            import_time_unix_timestamp,
        };

        Ok(())
    }

    pub fn component(&self) -> ComponentId {
        self.component
    }

    pub fn get_module(
        &mut self,
        module_path: &ModulePath,
        _timeout: &mut Timeout<RT>,
        _permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<(Arc<FullModuleSource>, ModuleCodeCacheResult)>> {
        let ActionPreloaded::Ready {
            ref module_loader,
            ref modules,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        let module = modules.get(&module_path.clone().canonicalize());

        let Some((module, source)) = module else {
            return Ok(None);
        };

        anyhow::ensure!(
            module.environment == ModuleEnvironment::Isolate,
            "Trying to execute {:?} in isolate, but it is bundled for {:?}.",
            module_path,
            module.environment
        );

        let code_cache_result = module_loader.clone().code_cache_result(module.clone());
        Ok(Some((source.clone(), code_cache_result)))
    }

    pub fn begin_execution(&mut self) -> anyhow::Result<()> {
        if self.phase != Phase::Importing {
            anyhow::bail!("Phase was already {:?}", self.phase)
        }
        let ActionPreloaded::Ready { ref mut rng, .. } = self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        self.phase = Phase::Executing;
        let rng_seed = self.rt.rng().random();
        *rng = Some(ChaCha12Rng::from_seed(rng_seed));
        Ok(())
    }

    pub fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        let ActionPreloaded::Ready { ref env_vars, .. } = self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        Ok(env_vars.get(&name).cloned())
    }

    pub fn component_arguments(&self) -> anyhow::Result<&BTreeMap<Identifier, ConvexValue>> {
        let ActionPreloaded::Ready {
            ref component_arguments,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        let Some(component_arguments) = component_arguments else {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoComponentArgs",
                "Component arguments are not available within the app",
            ));
        };
        if self.phase != Phase::Executing {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoComponentArgsDuringImport",
                "Can't use `componentArg` at import time",
            ));
        }
        Ok(component_arguments)
    }

    pub fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        let ActionPreloaded::Ready { ref mut rng, .. } = self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        let Some(ref mut rng) = rng else {
            // Fail for old module without import time rng populated.
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoRandomDuringImport",
                "Math.random unsupported at import time"
            ));
        };
        Ok(rng)
    }

    pub fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        let ActionPreloaded::Ready {
            import_time_unix_timestamp,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        let timestamp = if self.phase == Phase::Importing {
            let Some(unix_timestamp) = import_time_unix_timestamp else {
                // Fail for old modules without import time timestamp populated.
                anyhow::bail!(ErrorMetadata::bad_request(
                    "NoDateDuringImport",
                    "Date unsupported at import time"
                ));
            };
            unix_timestamp
        } else {
            self.rt.unix_timestamp()
        };
        Ok(timestamp)
    }

    pub fn require_executing(&self, request: &TaskRequestEnum) -> anyhow::Result<()> {
        if self.phase == Phase::Importing {
            anyhow::bail!(ErrorMetadata::bad_request(
                format!("No{}DuringImport", request.name_for_error()),
                format!(
                    "{} unsupported at import time",
                    request.description_for_error()
                ),
            ));
        }
        Ok(())
    }
}
