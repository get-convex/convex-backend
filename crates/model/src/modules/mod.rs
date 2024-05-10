use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use anyhow::Context;
use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    interval::{
        BinaryKey,
        Interval,
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
        VALUE_TOO_LARGE_SHORT_MSG,
    },
};
use database::{
    defaults::system_index,
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use metrics::{
    get_module_metadata_timer,
    get_module_version_timer,
};
use sync_types::{
    CanonicalizedModulePath,
    CanonicalizedUdfPath,
};
use value::{
    values_to_bytes,
    FieldPath,
    TableName,
};

use self::{
    module_versions::{
        AnalyzedFunction,
        AnalyzedModule,
        ModuleSource,
        ModuleVersion,
        ModuleVersionMetadata,
        SourceMap,
    },
    types::ModuleMetadata,
    user_error::{
        FunctionNotFoundError,
        ModuleNotFoundError,
    },
};
use crate::{
    config::types::ModuleConfig,
    source_packages::types::SourcePackageId,
    SystemIndex,
    SystemTable,
};

pub mod args_validator;
mod metrics;
pub mod module_versions;
pub mod types;
pub mod user_error;

/// Table name for user modules.
pub static MODULES_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_modules".parse().expect("Invalid built-in module table"));

/// Table name for the versions of a module.
pub static MODULE_VERSIONS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_module_versions"
        .parse()
        .expect("Invalid built-in module table")
});

/// Field pointing to the `ModuleMetadata` document from
/// `ModuleVersionMetadata`.
static MODULE_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "module_id".parse().expect("Invalid built-in field"));
/// Field for a module's version in `ModuleVersionMetadata`.
static VERSION_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "version".parse().expect("Invalid built-in field"));

/// Field for a module's path in `ModuleMetadata`.
static PATH_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "path".parse().expect("Invalid built-in field"));
/// Field for a module's deleted flag in `ModuleMetadata`.
static DELETED_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "deleted".parse().expect("Invalid built-in field"));

pub static MODULE_INDEX_BY_PATH: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&MODULES_TABLE, "by_path"));
pub static MODULE_INDEX_BY_DELETED: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&MODULES_TABLE, "by_deleted"));
pub static MODULE_VERSION_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&MODULE_VERSIONS_TABLE, "by_module_and_version"));

pub struct ModulesTable;
impl SystemTable for ModulesTable {
    fn table_name(&self) -> &'static TableName {
        &MODULES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![
            SystemIndex {
                name: MODULE_INDEX_BY_PATH.clone(),
                fields: vec![PATH_FIELD.clone()].try_into().unwrap(),
            },
            SystemIndex {
                name: MODULE_INDEX_BY_DELETED.clone(),
                fields: vec![DELETED_FIELD.clone(), PATH_FIELD.clone()]
                    .try_into()
                    .unwrap(),
            },
        ]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<ModuleMetadata>::try_from(document).map(|_| ())
    }
}
pub struct ModuleVersionsTable;
impl SystemTable for ModuleVersionsTable {
    fn table_name(&self) -> &'static TableName {
        &MODULE_VERSIONS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: MODULE_VERSION_INDEX.clone(),
            fields: vec![MODULE_ID_FIELD.clone(), VERSION_FIELD.clone()]
                .try_into()
                .unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<ModuleVersionMetadata>::try_from(document).map(|_| ())
    }
}

pub struct ModuleModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ModuleModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    /// Returns the registered modules metadata, including system modules.
    pub async fn get_all_metadata(
        &mut self,
    ) -> anyhow::Result<Vec<ParsedDocument<ModuleMetadata>>> {
        let index_query = Query::full_table_scan(MODULES_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, index_query)?;

        let mut modules = Vec::new();
        while let Some(metadata_document) = query_stream.next(self.tx, None).await? {
            let metadata: ParsedDocument<ModuleMetadata> = metadata_document.try_into()?;
            modules.push(metadata);
        }
        Ok(modules)
    }

    /// Returns all registered modules that aren't system modules.
    pub async fn get_application_modules(
        &mut self,
    ) -> anyhow::Result<BTreeMap<CanonicalizedModulePath, ModuleConfig>> {
        let mut modules = BTreeMap::new();
        for metadata in self.get_all_metadata().await? {
            let path = metadata.path.clone();
            if !path.is_system() {
                let module_version = self
                    .get_version(metadata.id(), metadata.latest_version)
                    .await?
                    .into_value();
                let module_config = ModuleConfig {
                    path: path.clone().into(),
                    source: module_version.source,
                    source_map: module_version.source_map,
                    environment: metadata.environment,
                };
                if modules.insert(path.clone(), module_config).is_some() {
                    panic!("Duplicate application module at {:?}", path);
                }
            }
        }
        Ok(modules)
    }

    pub async fn get_version(
        &mut self,
        module_id: ResolvedDocumentId,
        version: ModuleVersion,
    ) -> anyhow::Result<ParsedDocument<ModuleVersionMetadata>> {
        let timer = get_module_version_timer();
        let module_id_value: ConvexValue = module_id.into();
        let index_range = IndexRange {
            index_name: MODULE_VERSION_INDEX.clone(),
            range: vec![IndexRangeExpression::Eq(
                MODULE_ID_FIELD.clone(),
                module_id_value.into(),
            )],
            order: Order::Asc,
        };
        let module_query = Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, module_query)?;
        let module_version: ParsedDocument<ModuleVersionMetadata> = query_stream
            .expect_at_most_one(self.tx)
            .await?
            .context(format!(
                "Dangling module version reference: {module_id}@{version}"
            ))?
            .try_into()?;
        anyhow::ensure!(module_version.version == Some(version));
        timer.finish();
        Ok(module_version)
    }

    /// Helper function to get a module at the latest version.
    pub async fn get_metadata(
        &mut self,
        path: CanonicalizedModulePath,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let timer = get_module_metadata_timer();
        if path.is_system() && !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
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
    pub async fn put(
        &mut self,
        path: CanonicalizedModulePath,
        source: ModuleSource,
        source_package_id: Option<SourcePackageId>,
        source_map: Option<SourceMap>,
        analyze_result: Option<AnalyzedModule>,
        environment: ModuleEnvironment,
    ) -> anyhow::Result<()> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("put_module"));
        }
        if path.is_system() {
            anyhow::bail!("You cannot push a function under '_system/'");
        }
        anyhow::ensure!(
            path.is_deps() || analyze_result.is_some(),
            "AnalyzedModule is required for non-dependency modules"
        );
        let (module_id, version) = match self.module_metadata(path.clone()).await? {
            Some(module_metadata) => {
                let previous_version = module_metadata.latest_version;

                // Delete the old module version since it has no more references.
                let previous_version_id = self
                    .get_version(module_metadata.id(), previous_version)
                    .await?
                    .id();

                let latest_version = previous_version + 1;
                let new_metadata = ModuleMetadata {
                    path,
                    latest_version,
                    source_package_id,
                    environment,
                    analyze_result: analyze_result.clone(),
                };
                SystemMetadataModel::new(self.tx)
                    .replace(module_metadata.id(), new_metadata.try_into()?)
                    .await?;

                SystemMetadataModel::new(self.tx)
                    .delete(previous_version_id)
                    .await?;

                (module_metadata.id(), latest_version)
            },
            None => {
                let version = 0;
                let new_metadata = ModuleMetadata {
                    path,
                    latest_version: version,
                    source_package_id,
                    environment,
                    analyze_result: analyze_result.clone(),
                };

                let document_id = SystemMetadataModel::new(self.tx)
                    .insert(&MODULES_TABLE, new_metadata.try_into()?)
                    .await?;
                (document_id, version)
            },
        };
        let new_version = ModuleVersionMetadata {
            module_id: module_id.into(),
            source,
            source_map,
            version: Some(version),
            environment: Some(environment),
        }.try_into()
        .map_err(|e: anyhow::Error| e.map_error_metadata(|em| {
            if em.short_msg == VALUE_TOO_LARGE_SHORT_MSG {
                // Remap the ValueTooLargeError message to something more specific
                // to the modules use case.
                let message = format!(
                    "The functions, source maps, and their dependencies in \"convex/\" are too large. See our docs (https://docs.convex.dev/using/writing-convex-functions#using-libraries) for more details. You can also run `npx convex deploy -v` to print out each source file's bundled size.\n{}", em.msg
                );
                ErrorMetadata::bad_request(
                    "ModulesTooLarge",
                    message,
                )
            } else {
                em
            }
        }))?;
        SystemMetadataModel::new(self.tx)
            .insert(&MODULE_VERSIONS_TABLE, new_version)
            .await?;
        Ok(())
    }

    /// Delete a module, making it inaccessible for subsequent transactions.
    pub async fn delete(&mut self, path: CanonicalizedModulePath) -> anyhow::Result<()> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("delete_module"));
        }
        if let Some(module_metadata) = self.module_metadata(path).await? {
            let module_id = module_metadata.id();
            SystemMetadataModel::new(self.tx).delete(module_id).await?;

            // Delete the module version since it has no more references.
            let module_version = self
                .get_version(module_id, module_metadata.latest_version)
                .await?;
            SystemMetadataModel::new(self.tx)
                .delete(module_version.id())
                .await?;
        }
        Ok(())
    }

    #[convex_macro::instrument_future]
    async fn module_metadata(
        &mut self,
        path: CanonicalizedModulePath,
    ) -> anyhow::Result<Option<ParsedDocument<ModuleMetadata>>> {
        let index_range = IndexRange {
            index_name: MODULE_INDEX_BY_PATH.clone(),
            range: vec![IndexRangeExpression::Eq(
                PATH_FIELD.clone(),
                ConvexValue::try_from(String::from(path))?.into(),
            )],
            order: Order::Asc,
        };
        let module_query = Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, module_query)?;
        let module_document: ParsedDocument<ModuleMetadata> =
            match query_stream.expect_at_most_one(self.tx).await? {
                Some(v) => v.try_into()?,
                None => return Ok(None),
            };
        Ok(Some(module_document))
    }

    // Helper method that returns the AnalyzedFunction for the specified path.
    // It returns a user error if the module or function does not exist.
    // Note that using this method will error if AnalyzedResult is not backfilled,
    pub async fn get_analyzed_function(
        &mut self,
        udf_path: &CanonicalizedUdfPath,
    ) -> anyhow::Result<anyhow::Result<AnalyzedFunction>> {
        let Some(module) = self.get_metadata(udf_path.module().clone()).await? else {
            return Ok(Err(ErrorMetadata::bad_request(
                "ModuleNotFound",
                ModuleNotFoundError::new(udf_path.module().as_str()).to_string(),
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

    pub fn record_module_version_read_dependency(
        &mut self,
        module_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        let fields = vec![MODULE_ID_FIELD.clone()];
        let values = vec![Some(ConvexValue::from(module_id))];
        let module_index_name = MODULE_VERSION_INDEX
            .clone()
            .map_table(&self.tx.table_mapping().name_to_id())?
            .into();
        self.tx.record_system_table_cache_hit(
            module_index_name,
            fields.try_into().expect("Must be valid"),
            Interval::prefix(BinaryKey::from(values_to_bytes(&values[..]))),
        );
        Ok(())
    }

    pub async fn has_http(&mut self) -> anyhow::Result<bool> {
        Ok(self.get_metadata("http.js".parse()?).await?.is_some())
    }
}
