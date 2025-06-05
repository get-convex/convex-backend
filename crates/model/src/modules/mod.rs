use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::LazyLock,
};

use anyhow::Context;
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        CanonicalizedComponentModulePath,
        ComponentId,
        ResolvedComponentFunctionPath,
    },
    document::{
        ParseDocument,
        ParsedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        IndexName,
        ModuleEnvironment,
    },
    value::{
        ConvexValue,
        ResolvedDocumentId,
    },
};
use database::{
    unauthorized_error,
    BootstrapComponentsModel,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
use metrics::get_module_metadata_timer;
use sync_types::CanonicalizedModulePath;
use value::{
    sha256::{
        Sha256,
        Sha256Digest,
    },
    FieldPath,
    TableName,
};

use self::{
    module_versions::{
        AnalyzedFunction,
        AnalyzedModule,
        ModuleSource,
        SourceMap,
    },
    types::ModuleMetadata,
    user_error::{
        FunctionNotFoundError,
        ModuleNotFoundError,
    },
};
use crate::{
    config::{
        module_loader::ModuleLoader,
        types::{
            ModuleConfig,
            ModuleDiff,
        },
    },
    source_packages::types::SourcePackageId,
    SystemIndex,
    SystemTable,
};

pub mod function_validators;
mod metrics;
pub mod module_versions;
pub mod types;
pub mod user_error;

/// Table name for user modules.
pub static MODULES_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_modules".parse().expect("Invalid built-in module table"));

/// Field for a module's path in `ModuleMetadata`.
static PATH_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "path".parse().expect("Invalid built-in field"));
/// Field for a module's deleted flag in `ModuleMetadata`.
static DELETED_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "deleted".parse().expect("Invalid built-in field"));

pub static MODULE_INDEX_BY_PATH: LazyLock<SystemIndex<ModulesTable>> =
    LazyLock::new(|| SystemIndex::new("by_path", [&PATH_FIELD]).unwrap());
pub static MODULE_INDEX_BY_DELETED: LazyLock<SystemIndex<ModulesTable>> =
    LazyLock::new(|| SystemIndex::new("by_deleted", [&DELETED_FIELD, &PATH_FIELD]).unwrap());

pub static HTTP_MODULE_PATH: LazyLock<CanonicalizedModulePath> =
    LazyLock::new(|| "http.js".parse().unwrap());

pub struct ModulesTable;
impl SystemTable for ModulesTable {
    type Metadata = ModuleMetadata;

    fn table_name() -> &'static TableName {
        &MODULES_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![
            MODULE_INDEX_BY_PATH.clone(),
            MODULE_INDEX_BY_DELETED.clone(),
        ]
    }
}

pub struct ModuleModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ModuleModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[fastrace::trace]
    pub async fn apply(
        &mut self,
        component: ComponentId,
        modules: Vec<ModuleConfig>,
        source_package_id: Option<SourcePackageId>,
        mut analyze_results: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    ) -> anyhow::Result<ModuleDiff> {
        if modules.iter().any(|c| c.path.is_system()) {
            anyhow::bail!("You cannot push functions under the '_system/' directory.");
        }

        let mut added_modules = BTreeSet::new();

        // Add new modules.
        let mut remaining_modules: BTreeMap<_, _> = self
            .get_application_metadata(component)
            .await?
            .into_iter()
            .map(|module| (module.path.clone(), module.id()))
            .collect();
        for module in modules {
            let path = module.path.canonicalize();
            let existing_module_id = remaining_modules.remove(&path);
            if existing_module_id.is_none() {
                added_modules.insert(path.clone());
            }
            let analyze_result = if !path.is_deps() {
                // We expect AnalyzeResult to always be set for non-dependency modules.
                let analyze_result = analyze_results.remove(&path).context(format!(
                    "Missing analyze result for module {}",
                    path.as_str()
                ))?;
                Some(analyze_result)
            } else {
                // We don't analyze dependencies.
                None
            };
            self.put(
                existing_module_id,
                CanonicalizedComponentModulePath {
                    component,
                    module_path: path.clone(),
                },
                module.source,
                source_package_id.context("missing source_package_id")?,
                module.source_map,
                analyze_result,
                module.environment,
            )
            .await?;
        }

        let mut removed_modules = BTreeSet::new();
        for (path, module_id) in remaining_modules {
            removed_modules.insert(path.clone());
            self.delete(component, module_id).await?;
        }
        ModuleDiff::new(added_modules, removed_modules)
    }

    /// Returns the registered modules metadata, including system modules.
    #[fastrace::trace]
    pub async fn get_all_metadata(
        &mut self,
        component: ComponentId,
    ) -> anyhow::Result<Vec<ParsedDocument<ModuleMetadata>>> {
        // Hacky: Scan the _by_id index instead of the _by_creation_time index
        // (which is used by `Query::full_table_scan`)
        // This prevents creating too many read ranges in the transaction later
        // if we need to replace many documents by-id.
        let index_query = Query::index_range(IndexRange {
            index_name: IndexName::by_id(MODULES_TABLE.clone()),
            range: vec![],
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, component.into(), index_query)?;

        let mut modules = Vec::new();
        while let Some(metadata_document) = query_stream.next(self.tx, None).await? {
            let metadata: ParsedDocument<ModuleMetadata> = metadata_document.parse()?;
            modules.push(metadata);
        }
        Ok(modules)
    }

    pub async fn get_application_metadata(
        &mut self,
        component: ComponentId,
    ) -> anyhow::Result<Vec<ParsedDocument<ModuleMetadata>>> {
        let modules = self
            .get_all_metadata(component)
            .await?
            .into_iter()
            .filter(|metadata| !metadata.path.is_system())
            .collect();
        Ok(modules)
    }

    /// Returns all registered modules that aren't system modules.
    pub async fn get_application_modules(
        &mut self,
        component: ComponentId,
        module_loader: &dyn ModuleLoader<RT>,
    ) -> anyhow::Result<BTreeMap<CanonicalizedModulePath, ModuleConfig>> {
        let mut modules = BTreeMap::new();
        for metadata in self.get_all_metadata(component).await? {
            let path = metadata.path.clone();
            if !path.is_system() {
                let environment = metadata.environment;
                let full_source = module_loader
                    .get_module(
                        self.tx,
                        CanonicalizedComponentModulePath {
                            component,
                            module_path: metadata.path.clone(),
                        },
                    )
                    .await?
                    .context("Module source does not exist")?;
                let module_config = ModuleConfig {
                    path: path.clone().into(),
                    source: full_source.source.clone(),
                    source_map: full_source.source_map.clone(),
                    environment,
                };
                if modules.insert(path.clone(), module_config).is_some() {
                    panic!("Duplicate application module at {:?}", path);
                }
            }
        }
        Ok(modules)
    }

    pub async fn get_metadata_for_function(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let module_path = BootstrapComponentsModel::new(self.tx).function_path_to_module(&path)?;
        let module_metadata = self.get_metadata(module_path).await?;
        Ok(module_metadata)
    }

    pub async fn get_metadata_for_function_by_id(
        &mut self,
        path: &ResolvedComponentFunctionPath,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let module_path = CanonicalizedComponentModulePath {
            component: path.component,
            module_path: path.udf_path.module().clone(),
        };
        let module_metadata = self.get_metadata(module_path).await?;
        Ok(module_metadata)
    }

    /// Helper function to get a module at the latest version.
    pub async fn get_metadata(
        &mut self,
        path: CanonicalizedComponentModulePath,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let timer = get_module_metadata_timer();

        let is_system = path.module_path.is_system();
        if is_system && !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("get_module"))
        }
        let module_metadata = match self.module_metadata(path).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        timer.finish();
        Ok(Some(module_metadata))
    }

    /// Put a module's source at a given path.
    /// `module_id` is the existing module at this `path`.
    pub async fn put(
        &mut self,
        module_id: Option<ResolvedDocumentId>,
        path: CanonicalizedComponentModulePath,
        source: ModuleSource,
        source_package_id: SourcePackageId,
        source_map: Option<SourceMap>,
        analyze_result: Option<AnalyzedModule>,
        environment: ModuleEnvironment,
    ) -> anyhow::Result<()> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("put_module"));
        }
        if path.module_path.is_system() {
            anyhow::bail!("You cannot push a function under '_system/'");
        }
        anyhow::ensure!(
            path.module_path.is_deps() || analyze_result.is_some(),
            "AnalyzedModule is required for non-dependency modules"
        );
        let sha256 = hash_module_source(&source, source_map.as_ref());
        self.put_module_metadata(
            module_id,
            path,
            source_package_id,
            analyze_result,
            environment,
            sha256,
        )
        .await?;
        Ok(())
    }

    async fn put_module_metadata(
        &mut self,
        module_id: Option<ResolvedDocumentId>,
        path: CanonicalizedComponentModulePath,
        source_package_id: SourcePackageId,
        analyze_result: Option<AnalyzedModule>,
        environment: ModuleEnvironment,
        sha256: Sha256Digest,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let new_metadata = ModuleMetadata {
            path: path.module_path,
            source_package_id,
            environment,
            analyze_result: analyze_result.clone(),
            sha256,
        };
        let module_id = match module_id {
            Some(module_id) => {
                SystemMetadataModel::new(self.tx, path.component.into())
                    .replace(module_id, new_metadata.try_into()?)
                    .await?;
                module_id
            },
            None => {
                SystemMetadataModel::new(self.tx, path.component.into())
                    .insert(&MODULES_TABLE, new_metadata.try_into()?)
                    .await?
            },
        };
        Ok(module_id)
    }

    /// Delete a module, making it inaccessible for subsequent transactions.
    pub async fn delete(
        &mut self,
        component: ComponentId,
        module_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("delete_module"));
        }
        let namespace = component.into();
        SystemMetadataModel::new(self.tx, namespace)
            .delete(module_id)
            .await?;
        Ok(())
    }

    #[convex_macro::instrument_future]
    async fn module_metadata(
        &mut self,
        path: CanonicalizedComponentModulePath,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let namespace = path.component.into();
        let module_path = ConvexValue::try_from(path.module_path.as_str())?;
        let index_range = IndexRange {
            index_name: MODULE_INDEX_BY_PATH.name(),
            range: vec![IndexRangeExpression::Eq(
                PATH_FIELD.clone(),
                module_path.into(),
            )],
            order: Order::Asc,
        };
        let module_query = Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, namespace, module_query)?;
        let module_document: ParsedDocument<ModuleMetadata> =
            match query_stream.expect_at_most_one(self.tx).await? {
                Some(v) => v.parse()?,
                None => return Ok(None),
            };
        Ok(Some(module_document))
    }

    // Helper method that returns the AnalyzedFunction for the specified path.
    // It returns a user error if the module or function does not exist.
    // Note that using this method will error if AnalyzedResult is not backfilled,
    pub async fn get_analyzed_function(
        &mut self,
        path: &CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<anyhow::Result<AnalyzedFunction>> {
        let udf_path = &path.udf_path;
        let Some(module) = self.get_metadata_for_function(path.clone()).await? else {
            let err = ModuleNotFoundError::new(udf_path.module().as_str());
            return Ok(Err(ErrorMetadata::bad_request(
                "ModuleNotFound",
                err.to_string(),
            )
            .into()));
        };

        // Dependency modules don't have AnalyzedModule.
        if !udf_path.module().is_deps() {
            let analyzed_module = module
                .analyze_result
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Expected analyze result for {udf_path:?}"))?;

            for function in &analyzed_module.functions {
                if &function.name == udf_path.function_name() {
                    return Ok(Ok(function.clone()));
                }
            }
        }

        Ok(Err(ErrorMetadata::bad_request(
            "FunctionNotFound",
            FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
                .to_string(),
        )
        .into()))
    }

    // Helper method that returns the AnalyzedFunction for the specified path.
    // It returns a user error if the module or function does not exist.
    // Note that using this method will error if AnalyzedResult is not backfilled,
    pub async fn get_analyzed_function_by_id(
        &mut self,
        path: &ResolvedComponentFunctionPath,
    ) -> anyhow::Result<anyhow::Result<AnalyzedFunction>> {
        let udf_path = &path.udf_path;
        let Some(module) = self.get_metadata_for_function_by_id(path).await? else {
            let err = ModuleNotFoundError::new(udf_path.module().as_str());
            return Ok(Err(ErrorMetadata::bad_request(
                "ModuleNotFound",
                err.to_string(),
            )
            .into()));
        };

        // Dependency modules don't have AnalyzedModule.
        if !udf_path.module().is_deps() {
            let analyzed_module = module
                .analyze_result
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Expected analyze result for {udf_path:?}"))?;

            for function in &analyzed_module.functions {
                if &function.name == udf_path.function_name() {
                    return Ok(Ok(function.clone()));
                }
            }
        }

        Ok(Err(ErrorMetadata::bad_request(
            "FunctionNotFound",
            FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
                .to_string(),
        )
        .into()))
    }

    pub async fn get_http(
        &mut self,
        component: ComponentId,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let path = CanonicalizedComponentModulePath {
            component,
            module_path: HTTP_MODULE_PATH.clone(),
        };
        self.get_metadata(path).await
    }

    pub async fn has_http(&mut self, component: ComponentId) -> anyhow::Result<bool> {
        Ok(self.get_http(component).await?.is_some())
    }
}

/// Hash a module's source and source map. This same hash is also computed in
/// the CLI to determine if a module has changed. Therefore this algorithm
/// can never be changed (if you want a new algorithm, we need a new API
/// endpoint and a new CLI version to call it).
pub fn hash_module_source(source: &ModuleSource, source_map: Option<&SourceMap>) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    if let Some(source_map) = source_map {
        hasher.update(source_map.as_bytes());
    }
    hasher.finalize()
}
