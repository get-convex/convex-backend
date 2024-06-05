use std::{
    collections::BTreeMap,
    sync::{
        atomic::{
            AtomicBool,
            Ordering,
        },
        Arc,
    },
};

use common::{
    components::{
        CanonicalizedComponentModulePath,
        ComponentDefinitionId,
        ComponentId,
        ComponentPath,
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
    config::module_loader::ModuleLoader,
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
    udf_config::UdfConfigModel,
};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use sync_types::ModulePath;

use crate::{
    concurrency_limiter::ConcurrencyPermit,
    environment::helpers::{
        permit::with_release_permit,
        Phase,
    },
    timeout::Timeout,
};

/// UDF execution has two phases:
///
/// 1. We start by loading all imported modules, evaluating them, and inserting
/// them into the module map. 2. We find the query or mutation function in the
/// specified module and run it.
///
/// We shouldn't be looking at the database in the first step (other than to
/// load code), and we shouldn't be performing dynamic imports in the second
/// step. This structure is responsible for enforcing these invariants.
pub struct UdfPhase<RT: Runtime> {
    phase: Phase,
    tx: Transaction<RT>,
    pub rt: RT,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    preloaded: UdfPreloaded,
    component_path: ComponentPath,
}

enum UdfPreloaded {
    Created,
    Ready {
        rng: Option<ChaCha12Rng>,
        observed_rng_during_execution: bool,
        unix_timestamp: Option<UnixTimestamp>,
        observed_time_during_execution: AtomicBool,
        env_vars: PreloadedEnvironmentVariables,
        component: ComponentId,
        component_definition: ComponentDefinitionId,
    },
}

impl<RT: Runtime> UdfPhase<RT> {
    pub fn new(
        tx: Transaction<RT>,
        rt: RT,
        module_loader: Arc<dyn ModuleLoader<RT>>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
        component_path: ComponentPath,
    ) -> Self {
        Self {
            phase: Phase::Importing,
            tx,
            rt,
            module_loader,
            system_env_vars,
            preloaded: UdfPreloaded::Created,
            component_path,
        }
    }

    pub async fn initialize(
        &mut self,
        timeout: &mut Timeout<RT>,
        permit_slot: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(self.phase == Phase::Importing);
        let UdfPreloaded::Created = self.preloaded else {
            anyhow::bail!("UdfPhase initialized twice");
        };

        // UdfConfig might not be defined for super old modules or system modules.
        let udf_config = with_release_permit(
            timeout,
            permit_slot,
            UdfConfigModel::new(&mut self.tx).get(),
        )
        .await?;
        let rng = udf_config
            .as_ref()
            .map(|c| ChaCha12Rng::from_seed(c.import_phase_rng_seed));
        let unix_timestamp = udf_config.as_ref().map(|c| c.import_phase_unix_timestamp);

        let env_vars = with_release_permit(
            timeout,
            permit_slot,
            EnvironmentVariablesModel::new(&mut self.tx).preload(),
        )
        .await?;

        let (component_definition, component) = with_release_permit(
            timeout,
            permit_slot,
            BootstrapComponentsModel::new(&mut self.tx)
                .component_path_to_ids(self.component_path.clone()),
        )
        .await?;

        self.preloaded = UdfPreloaded::Ready {
            rng,
            observed_rng_during_execution: false,
            unix_timestamp,
            observed_time_during_execution: AtomicBool::new(false),
            env_vars,
            component,
            component_definition,
        };
        Ok(())
    }

    pub fn component(&self) -> anyhow::Result<ComponentId> {
        let UdfPreloaded::Ready { component, .. } = &self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        Ok(*component)
    }

    pub async fn get_module(
        &mut self,
        module_path: &ModulePath,
        timeout: &mut Timeout<RT>,
        permit_slot: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>> {
        if self.phase != Phase::Importing {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDynamicImport",
                format!("Can't dynamically import {module_path:?} in a query or mutation")
            ));
        }
        let UdfPreloaded::Ready {
            component_definition,
            ..
        } = &self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        let path = CanonicalizedComponentModulePath {
            component: *component_definition,
            module_path: module_path.clone().canonicalize(),
        };
        let module = with_release_permit(
            timeout,
            permit_slot,
            ModuleModel::new(&mut self.tx).get_metadata(path.clone()),
        )
        .await?;
        let module_version = with_release_permit(
            timeout,
            permit_slot,
            self.module_loader.get_module(&mut self.tx, path),
        )
        .await?;

        if let Some(module) = module.as_ref() {
            anyhow::ensure!(
                module.environment == ModuleEnvironment::Isolate,
                "Trying to execute {:?} in isolate, but it is bundled for {:?}.",
                module_path,
                module.environment
            );
        };

        Ok(module_version.map(|m| (*m).clone()))
    }

    pub fn tx(&mut self) -> Result<&mut Transaction<RT>, ErrorMetadata> {
        if self.phase != Phase::Executing {
            return Err(ErrorMetadata::bad_request(
                "NoDbDuringImport",
                "Can't use database at import time",
            ));
        }
        Ok(&mut self.tx)
    }

    pub fn into_transaction(self) -> Transaction<RT> {
        self.tx
    }

    pub fn biggest_document_writes(&self) -> Option<BiggestDocumentWrites> {
        self.tx.biggest_document_writes()
    }

    pub fn execution_size(&self) -> FunctionExecutionSize {
        self.tx.execution_size()
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
        let UdfPreloaded::Ready { ref env_vars, .. } = self.preloaded else {
            anyhow::bail!("Phase not initialized");
        };
        if let Some(var) = env_vars.get(&mut self.tx, &name)? {
            return Ok(Some(var.clone()));
        }
        Ok(self.system_env_vars.get(&name).cloned())
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

    pub fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        let UdfPreloaded::Ready {
            unix_timestamp,
            ref observed_time_during_execution,
            ..
        } = self.preloaded
        else {
            anyhow::bail!("Phase not initialized");
        };
        if self.phase == Phase::Executing {
            observed_time_during_execution.store(true, Ordering::SeqCst);
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

    pub fn observed_rng(&self) -> bool {
        match self.preloaded {
            UdfPreloaded::Ready {
                observed_rng_during_execution,
                ..
            } => observed_rng_during_execution,
            UdfPreloaded::Created => false,
        }
    }

    pub fn observed_time(&self) -> bool {
        match self.preloaded {
            UdfPreloaded::Ready {
                ref observed_time_during_execution,
                ..
            } => observed_time_during_execution.load(Ordering::SeqCst),
            UdfPreloaded::Created => false,
        }
    }
}
