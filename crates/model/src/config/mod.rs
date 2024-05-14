#[cfg(test)]
mod tests;

#[cfg(test)]
mod index_diff_tests;
#[cfg(test)]
mod index_limits_tests;
#[cfg(test)]
pub mod index_test_utils;
#[cfg(test)]
mod index_tests;
pub mod types;

use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use common::{
    bootstrap_model::schema::SchemaState,
    components::{
        CanonicalizedComponentModulePath,
        ComponentId,
    },
    runtime::Runtime,
    schemas::DatabaseSchema,
};
use database::{
    unauthorized_error,
    IndexModel,
    LegacyIndexDiff,
    SchemaModel,
    Transaction,
};
use value::{
    heap_size::WithHeapSize,
    ResolvedDocumentId,
};

use crate::{
    auth::AuthInfoModel,
    config::types::{
        ConfigDiff,
        ConfigMetadata,
        CronDiff,
        ModuleConfig,
        ModuleDiff,
        SchemaDiff,
        AUTH_CONFIG_FILE_NAME,
    },
    cron_jobs::{
        types::{
            CronIdentifier,
            CronSpec,
        },
        CronModel,
    },
    modules::{
        module_versions::AnalyzedModule,
        ModuleModel,
    },
    source_packages::{
        types::SourcePackage,
        SourcePackageModel,
    },
    udf_config::{
        types::UdfConfig,
        UdfConfigModel,
    },
};

pub struct ConfigModel<'a, RT: Runtime> {
    pub tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ConfigModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[minitrace::trace]
    pub async fn apply(
        &mut self,
        config: ConfigMetadata,
        modules: Vec<ModuleConfig>,
        new_config: UdfConfig,
        source_package: Option<SourcePackage>,
        mut analyze_results: BTreeMap<CanonicalizedComponentModulePath, AnalyzedModule>,
        schema_id: Option<ResolvedDocumentId>,
    ) -> anyhow::Result<(ConfigDiff, Option<DatabaseSchema>)> {
        // TODO: Move this check up to `Application`.
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("apply_config"));
        }
        if modules.iter().any(|c| c.path.is_system()) {
            anyhow::bail!("You cannot push functions under the '_system/' directory.");
        }
        let source_package_id = match source_package {
            Some(source_package) => {
                Some(SourcePackageModel::new(self.tx).put(source_package).await?)
            },
            None => None,
        };

        let crons_js = CanonicalizedComponentModulePath {
            component: ComponentId::Root,
            module_path: "crons.js".parse()?,
        };
        let new_crons: WithHeapSize<BTreeMap<CronIdentifier, CronSpec>> =
            if let Some(module) = analyze_results.get(&crons_js) {
                module.cron_specs.clone().unwrap_or_default()
            } else {
                WithHeapSize::default()
            };

        // TODO: Push some of this logic down into `cron_jobs/`.
        let mut cron_model = CronModel::new(self.tx);
        let old_crons = cron_model.list().await?;
        let mut added_crons: Vec<&CronIdentifier> = vec![];
        let mut updated_crons: Vec<&CronIdentifier> = vec![];
        let mut deleted_crons: Vec<&CronIdentifier> = vec![];
        for (name, cron_spec) in &new_crons {
            match old_crons.get(&name.clone()) {
                Some(cron_job) => {
                    if cron_job.cron_spec != cron_spec.clone() {
                        cron_model
                            .update(cron_job.clone(), cron_spec.clone())
                            .await?;
                        updated_crons.push(name);
                    }
                },
                None => {
                    cron_model.create(name.clone(), cron_spec.clone()).await?;
                    added_crons.push(name);
                },
            }
        }
        for (name, cron_job) in &old_crons {
            match new_crons.get(&name.clone()) {
                Some(_) => {},
                None => {
                    cron_model.delete(cron_job.clone()).await?;
                    deleted_crons.push(name);
                },
            }
        }
        tracing::info!(
            "Crons Added: {added_crons:?}, Updated: {updated_crons:?}, Deleted: {deleted_crons:?}"
        );
        let cron_diff = CronDiff::new(added_crons, updated_crons, deleted_crons);

        // TODO: Extract this logic into `schema/`.
        let mut schema_model = SchemaModel::new(self.tx);
        let previous_schema = schema_model
            .get_by_state(SchemaState::Active)
            .await?
            .map(|(_id, schema)| schema);
        let next_schema = if let Some(schema_id) = schema_id {
            Some(schema_model.get_validated_or_active(schema_id).await?.1)
        } else {
            None
        };
        let schema_diff: Option<SchemaDiff> =
            (previous_schema != next_schema).then_some(SchemaDiff {
                previous_schema,
                next_schema: next_schema.clone(),
            });
        if let Some(schema_id) = schema_id {
            schema_model.mark_active(schema_id).await?;
        } else {
            schema_model.clear_active().await?;
        }

        let empty = BTreeMap::new();
        let tables_in_schema = next_schema
            .as_ref()
            .map(|schema| &schema.tables)
            .unwrap_or(&empty);

        // Without a schema id, we cannot accurately determine the status of
        // indexes. So for legacy CLIs, we do nothing here and instead rely
        // on build_indexes / legacy_get_indexes to commit index changes.
        let index_diff = IndexModel::new(self.tx)
            .commit_indexes_for_schema(tables_in_schema)
            .await?;

        tracing::info!(
            "Committed indexes: (added {}. dropped {}) for schema: {schema_id:?}",
            index_diff.added.len(),
            index_diff.dropped.len(),
        );

        // TODO: Extract this logic into `modules/`.
        let mut added_modules = BTreeSet::new();

        // Add new modules.
        let mut remaining_modules = ModuleModel::new(self.tx).get_application_modules().await?;
        for module in modules {
            let path = CanonicalizedComponentModulePath {
                component: ComponentId::Root,
                module_path: module.path.canonicalize(),
            };
            if remaining_modules.remove(&path).is_none() {
                added_modules.insert(path.clone());
            }
            let analyze_result = if !path.module_path.is_deps() {
                // We expect AnalyzeResult to always be set for non-dependency modules.
                let analyze_result = analyze_results.remove(&path).context(format!(
                    "Missing analyze result for module {}",
                    path.module_path.as_str()
                ))?;
                Some(analyze_result)
            } else {
                // We don't analyze dependencies.
                None
            };
            ModuleModel::new(self.tx)
                .put(
                    path,
                    module.source,
                    source_package_id,
                    module.source_map,
                    analyze_result,
                    module.environment,
                )
                .await?;
        }

        let mut removed_modules = BTreeSet::new();
        for (path, _) in remaining_modules {
            removed_modules.insert(path.clone());
            ModuleModel::new(self.tx).delete(path).await?;
        }
        let module_diff = ModuleDiff::new(added_modules, removed_modules)?;

        // Update auth info.
        let auth_diff = AuthInfoModel::new(self.tx).put(config.auth_info).await?;
        let udf_server_version_diff = UdfConfigModel::new(self.tx).set(new_config).await?;
        let config_diff = ConfigDiff {
            module_diff,
            auth_diff,
            udf_server_version_diff,
            cron_diff,
            // TODO(CX-3851): Consider logging the mutated indexes separately.
            // This now includes added, mutated and dropped indexes. Mutated
            // indexes are shown both in 'added' and in 'dropped'
            index_diff: LegacyIndexDiff::from(index_diff).into(),
            schema_diff,
        };

        Ok((config_diff, next_schema))
    }

    /// Return the latest database configuration. This includes only the
    /// user-configurable state and not internal derived state like shapes. We
    /// might want to store this config in memory but for now just reading it
    /// out of the metadata tables to avoid keeping too many sources of truth.
    pub async fn get(
        &mut self,
    ) -> anyhow::Result<(ConfigMetadata, Vec<ModuleConfig>, Option<UdfConfig>)> {
        // TODO: Move to `application/`.
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("get_config"));
        }
        let mut config = ConfigMetadata::new();
        let modules: Vec<_> = ModuleModel::new(self.tx)
            .get_application_modules()
            .await?
            .into_values()
            .collect();

        // If we have an auth config module do not include auth_info in the config
        if !modules
            .iter()
            .any(|module| module.path == AUTH_CONFIG_FILE_NAME.parse().unwrap())
        {
            let auth_info = AuthInfoModel::new(self.tx).get().await?;
            config.auth_info = auth_info.into_iter().map(|doc| doc.into_value()).collect();
        }

        let udf_config = UdfConfigModel::new(self.tx)
            .get()
            .await?
            .map(|u| u.into_value());
        Ok((config, modules, udf_config))
    }
}
