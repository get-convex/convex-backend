use std::{
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context;
use common::{
    components::{
        CanonicalizedComponentModulePath,
        ComponentId,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::ModuleEnvironment,
};
use database::{
    BiggestDocumentWrites,
    BootstrapComponentsModel,
    FunctionExecutionSize,
    Transaction,
};
use errors::ErrorMetadata;
use model::{
    environment_variables::{
        types::{
            EnvVarName,
            EnvVarValue,
        },
        EnvironmentVariablesModel,
        PreloadedEnvironmentVariables,
    },
    modules::{
        module_versions::FullModuleSource,
        ModuleModel,
    },
    source_packages::SourcePackageModel,
    udf_config::UdfConfigModel,
};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use sync_types::ModulePath;
use udf::environment::system_env_vars;
use value::{
    identifier::Identifier,
    ConvexValue,
};

use crate::{
    concurrency_limiter::ConcurrencyPermit,
    environment::{
        helpers::{
            permit::with_release_permit,
            Phase,
        },
        ModuleCodeCacheResult,
    },
    module_cache::ModuleCache,
    timeout::Timeout,
};

/// UDF execution has two phases:
///
/// 1. We start by loading all imported modules, evaluating them, and inserting
///    them into the module map.
/// 2. We find the query or mutation function in the specified module and run
///    it.
///
/// We shouldn't be looking at the database in the first step (other than to
/// load code), and we shouldn't be performing dynamic imports in the second
/// step. This structure is responsible for enforcing these invariants.
pub struct UdfPhase<RT: Runtime> {
    phase: Phase,

    // We "check out" the transaction when executing a cross-component
    // call. Until we implement subtransactions, we cannot run any
    // user code concurrently with a component call.
    tx: Option<Transaction<RT>>,

    pub rt: RT,
    module_loader: Arc<dyn ModuleCache<RT>>,
    preloaded: UdfPreloaded,
    component: ComponentId,
}

enum UdfPreloaded {
    Created {
        default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    },
    Ready {
        rng: Option<ChaCha12Rng>,
        observed_rng_during_execution: bool,
        unix_timestamp: Option<UnixTimestamp>,
        observed_time_during_execution: bool,
        observed_identity_during_execution: bool,
        env_vars: Option<PreloadedEnvironmentVariables>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        component: ComponentId,
        component_arguments: Option<BTreeMap<Identifier, ConvexValue>>,
    },
}

impl<RT: Runtime> UdfPhase<RT> {
    pub fn new(
        tx: Transaction<RT>,
        rt: RT,
        module_loader: Arc<dyn ModuleCache<RT>>,
        default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        component: ComponentId,
    ) -> Self {
        Self {
            phase: Phase::Importing,
            tx: Some(tx),
            rt,
            module_loader,
            preloaded: UdfPreloaded::Created {
                default_system_env_vars,
            },
            component,
        }
    }

    #[fastrace::trace]
    pub async fn initialize(
        &mut self,
        timeout: &mut Timeout<RT>,
        permit_slot: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(self.phase == Phase::Importing);
        let UdfPreloaded::Created {
            default_system_env_vars,
        } = &self.preloaded
        else {
            anyhow::bail!("UdfPhase initialized twice");
        };
        let default_system_env_vars = default_system_env_vars.clone();

        let component = self.component;

        let component_args = if !component.is_root() {
            Some(
                with_release_permit(
                    timeout,
                    permit_slot,
                    BootstrapComponentsModel::new(self.tx_mut()?).load_component_args(component),
                )
                .await?,
            )
        } else {
            None
        };

        // UdfConfig might not be defined for super old modules or system modules.
        let udf_config = with_release_permit(
            timeout,
            permit_slot,
            UdfConfigModel::new(self.tx_mut()?, component.into()).get(),
        )
        .await?;
        let rng = udf_config
            .as_ref()
            .map(|c| ChaCha12Rng::from_seed(c.import_phase_rng_seed));
        let unix_timestamp = udf_config.as_ref().map(|c| c.import_phase_unix_timestamp);

        let env_vars = if component.is_root() {
            Some(
                with_release_permit(
                    timeout,
                    permit_slot,
                    EnvironmentVariablesModel::new(self.tx_mut()?).preload(),
                )
                .await?,
            )
        } else {
            None
        };

        let system_env_vars = with_release_permit(
            timeout,
            permit_slot,
            system_env_vars(self.tx_mut()?, default_system_env_vars.clone()),
        )
        .await?;

        self.preloaded = UdfPreloaded::Ready {
            rng,
            observed_rng_during_execution: false,
            unix_timestamp,
            observed_time_during_execution: false,
            observed_identity_during_execution: false,
            env_vars,
            system_env_vars,
            component,
            component_arguments: component_args,
        };
        Ok(())
    }

    pub fn component(&self) -> anyhow::Result<ComponentId> {
        let UdfPreloaded::Ready { component, .. } = &self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        Ok(*component)
    }

    pub fn component_arguments(&self) -> anyhow::Result<&BTreeMap<Identifier, ConvexValue>> {
        let UdfPreloaded::Ready {
            component_arguments: component_args,
            ..
        } = &self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        let Some(component_args) = component_args else {
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
        Ok(component_args)
    }

    #[fastrace::trace]
    pub async fn get_module(
        &mut self,
        module_path: &ModulePath,
        timeout: &mut Timeout<RT>,
        permit_slot: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<(Arc<FullModuleSource>, ModuleCodeCacheResult)>> {
        if self.phase != Phase::Importing {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDynamicImport",
                format!("Can't dynamically import {module_path:?} in a query or mutation")
            ));
        }
        let UdfPreloaded::Ready { component, .. } = &self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        let component = *component;
        let path = CanonicalizedComponentModulePath {
            component,
            module_path: module_path.clone().canonicalize(),
        };
        let Some((module_metadata, source_package)) =
            with_release_permit(timeout, permit_slot, async {
                match ModuleModel::new(self.tx_mut()?)
                    .get_metadata(path.clone())
                    .await?
                {
                    None => anyhow::Ok(None),
                    Some(module_metadata) => {
                        let source_package =
                            SourcePackageModel::new(self.tx_mut()?, component.into())
                                .get(module_metadata.source_package_id)
                                .await?;
                        anyhow::Ok(Some((module_metadata, source_package)))
                    },
                }
            })
            .await?
        else {
            return Ok(None);
        };

        anyhow::ensure!(
            module_metadata.environment == ModuleEnvironment::Isolate,
            "Trying to execute {:?} in isolate, but it is bundled for {:?}.",
            module_path,
            module_metadata.environment
        );

        let module_loader = self.module_loader.clone();
        let module_source = with_release_permit(
            timeout,
            permit_slot,
            module_loader.get_module_with_metadata(module_metadata.clone(), source_package),
        )
        .await?;
        let code_cache_result = module_loader.code_cache_result(module_metadata.into_value());
        Ok(Some((module_source, code_cache_result)))
    }

    pub fn tx(&mut self) -> anyhow::Result<&mut Transaction<RT>> {
        if self.phase != Phase::Executing {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDbDuringImport",
                "Can't use database at import time",
            ));
        }
        self.tx_mut()
    }

    pub fn take_tx(&mut self) -> anyhow::Result<Transaction<RT>> {
        self.tx
            .take()
            .context("Transaction missing due to concurrent component call")
    }

    pub fn put_tx(&mut self, tx: Transaction<RT>) -> anyhow::Result<()> {
        anyhow::ensure!(self.tx.is_none());
        self.tx = Some(tx);
        Ok(())
    }

    fn tx_mut(&mut self) -> anyhow::Result<&mut Transaction<RT>> {
        self.tx
            .as_mut()
            .context("Transaction missing due to concurrent component call")
    }

    fn tx_ref(&self) -> anyhow::Result<&Transaction<RT>> {
        self.tx
            .as_ref()
            .context("Transaction missing due to concurrent component call")
    }

    pub fn into_transaction(self) -> anyhow::Result<Transaction<RT>> {
        self.tx
            .context("Transaction missing due to concurrent component call")
    }

    pub fn biggest_document_writes(&self) -> anyhow::Result<Option<BiggestDocumentWrites>> {
        Ok(self.tx_ref()?.biggest_document_writes())
    }

    pub fn execution_size(&self) -> anyhow::Result<FunctionExecutionSize> {
        Ok(self.tx_ref()?.execution_size())
    }

    pub fn begin_execution(
        &mut self,
        rng_seed: [u8; 32],
        execution_unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<()> {
        if self.phase != Phase::Importing {
            anyhow::bail!("Phase was already {:?}", self.phase)
        }
        let UdfPreloaded::Ready {
            ref mut rng,
            ref mut unix_timestamp,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        self.phase = Phase::Executing;
        *rng = Some(ChaCha12Rng::from_seed(rng_seed));
        *unix_timestamp = Some(execution_unix_timestamp);
        Ok(())
    }

    pub fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        let UdfPreloaded::Ready {
            ref env_vars,
            ref system_env_vars,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        let tx = self
            .tx
            .as_mut()
            .context("Transaction missing due to concurrent component call")?;
        let Some(env_vars) = env_vars else {
            return Ok(None);
        };
        if let Some(var) = env_vars.get(tx, &name)? {
            return Ok(Some(var));
        }
        Ok(system_env_vars.get(&name).cloned())
    }

    pub fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        let UdfPreloaded::Ready {
            ref mut rng,
            ref mut observed_rng_during_execution,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        if self.phase == Phase::Executing {
            *observed_rng_during_execution = true;
        }
        let Some(ref mut rng) = rng else {
            // Fail for old module without import time rng populated.
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoRandomDuringImport",
                "Math.random unsupported at import time",
            ));
        };
        Ok(rng)
    }

    pub fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        let UdfPreloaded::Ready {
            unix_timestamp,
            ref mut observed_time_during_execution,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        if self.phase == Phase::Executing {
            *observed_time_during_execution = true;
        }
        let Some(unix_timestamp) = unix_timestamp else {
            // Fail for old modules without import time timestamp populated.
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDateDuringImport",
                "Date unsupported at import time"
            ));
        };
        Ok(unix_timestamp)
    }

    pub fn observe_identity(&mut self) -> anyhow::Result<()> {
        let UdfPreloaded::Ready {
            ref mut observed_identity_during_execution,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        *observed_identity_during_execution = true;
        Ok(())
    }

    pub fn observed_rng(&self) -> bool {
        match self.preloaded {
            UdfPreloaded::Ready {
                observed_rng_during_execution,
                ..
            } => observed_rng_during_execution,
            UdfPreloaded::Created { .. } => false,
        }
    }

    pub fn observed_time(&self) -> bool {
        match self.preloaded {
            UdfPreloaded::Ready {
                observed_time_during_execution,
                ..
            } => observed_time_during_execution,
            UdfPreloaded::Created { .. } => false,
        }
    }

    pub fn observed_identity(&self) -> bool {
        match self.preloaded {
            UdfPreloaded::Ready {
                observed_identity_during_execution,
                ..
            } => observed_identity_during_execution,
            UdfPreloaded::Created { .. } => false,
        }
    }

    pub fn module_loader(&self) -> &Arc<dyn ModuleCache<RT>> {
        &self.module_loader
    }
}
