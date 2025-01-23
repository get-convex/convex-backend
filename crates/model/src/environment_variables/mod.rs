use std::{
    collections::{
        BTreeMap,
        HashMap,
        HashSet,
    },
    sync::LazyLock,
};

use anyhow::Context;
use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    interval::Interval,
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        env_var_name_forbidden,
        env_var_name_not_unique,
        IndexName,
    },
};
use database::{
    defaults::system_index,
    PreloadedIndexRange,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
use value::{
    ConvexValue,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    deployment_audit_log::types::DeploymentAuditLogEvent,
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
        EnvironmentVariable,
        PersistedEnvironmentVariable,
    },
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static ENVIRONMENT_VARIABLES_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_environment_variables"
        .parse()
        .expect("Invalid built-in environment variables table")
});

pub static ENVIRONMENT_VARIABLES_INDEX_BY_NAME: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&ENVIRONMENT_VARIABLES_TABLE, "by_name"));
static NAME_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "name".parse().expect("invalid name field"));

pub struct EnvironmentVariablesTable;
impl SystemTable for EnvironmentVariablesTable {
    fn table_name(&self) -> &'static TableName {
        &ENVIRONMENT_VARIABLES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: ENVIRONMENT_VARIABLES_INDEX_BY_NAME.clone(),
            fields: vec![NAME_FIELD.clone()].try_into().unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<PersistedEnvironmentVariable>::try_from(document).map(|_| ())
    }
}

pub struct EnvironmentVariablesModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

pub struct PreloadedEnvironmentVariables {
    range: PreloadedIndexRange,
}

impl PreloadedEnvironmentVariables {
    pub fn get<RT: Runtime>(
        &self,
        tx: &mut Transaction<RT>,
        name: &EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        let key = Some(ConvexValue::try_from(String::from(name.clone()))?);
        let Some(doc) = self.range.get(tx, &key)? else {
            return Ok(None);
        };
        let doc: ParsedDocument<PersistedEnvironmentVariable> = doc.clone().try_into()?;
        let var = doc.into_value().0;
        anyhow::ensure!(var.name() == name, "Invalid environment variable");
        Ok(Some(var.into_value()))
    }
}

impl<'a, RT: Runtime> EnvironmentVariablesModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn preload(&mut self) -> anyhow::Result<PreloadedEnvironmentVariables> {
        let range = self
            .tx
            .preload_index_range(
                TableNamespace::Global,
                &ENVIRONMENT_VARIABLES_INDEX_BY_NAME,
                &Interval::all(),
            )
            .await?;
        Ok(PreloadedEnvironmentVariables { range })
    }

    pub async fn get(
        &mut self,
        name: &EnvVarName,
    ) -> anyhow::Result<Option<ParsedDocument<EnvironmentVariable>>> {
        let query = value_query_from_env_var(name)?;
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        query_stream
            .expect_at_most_one(self.tx)
            .await?
            .map(|doc| {
                let doc: ParsedDocument<PersistedEnvironmentVariable> = doc.try_into()?;
                doc.map(|doc| Ok(doc.0))
            })
            .transpose()
    }

    pub async fn get_by_id_legacy(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<EnvironmentVariable>> {
        let Some(doc) = self.tx.get(id).await? else {
            return Ok(None);
        };
        let persisted: ParsedDocument<PersistedEnvironmentVariable> = doc.try_into()?;
        Ok(Some(persisted.into_value().0))
    }

    #[fastrace::trace]
    pub async fn get_all(&mut self) -> anyhow::Result<BTreeMap<EnvVarName, EnvVarValue>> {
        let query = Query::full_table_scan(ENVIRONMENT_VARIABLES_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let mut environment_variables = BTreeMap::new();
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            let env_var: ParsedDocument<PersistedEnvironmentVariable> = doc.try_into()?;
            let old_value = environment_variables
                .insert(env_var.0.name().to_owned(), env_var.0.value().to_owned());
            anyhow::ensure!(old_value.is_none(), "Duplicate environment variable");
        }
        Ok(environment_variables)
    }

    pub async fn create(
        &mut self,
        env_var: EnvironmentVariable,
        forbidden_names: &HashSet<EnvVarName>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        if forbidden_names.contains(env_var.name()) {
            anyhow::bail!(env_var_name_forbidden(env_var.name()));
        }
        SystemMetadataModel::new_global(self.tx)
            .insert(
                &ENVIRONMENT_VARIABLES_TABLE,
                PersistedEnvironmentVariable(env_var).try_into()?,
            )
            .await
    }

    pub async fn delete(
        &mut self,
        name: &EnvVarName,
    ) -> anyhow::Result<Option<EnvironmentVariable>> {
        let Some(doc) = self.get(name).await? else {
            return Ok(None);
        };
        let document = SystemMetadataModel::new_global(self.tx)
            .delete(doc.id())
            .await?;
        let env_var: ParsedDocument<PersistedEnvironmentVariable> = document.try_into()?;
        Ok(Some(env_var.into_value().0))
    }

    pub async fn edit(
        &mut self,
        changes: HashMap<ResolvedDocumentId, EnvironmentVariable>,
    ) -> anyhow::Result<Vec<DeploymentAuditLogEvent>> {
        let mut audit_events = vec![];

        // Ensure that there are no conflict between new variable names
        let new_names: HashSet<EnvVarName> = changes
            .values()
            .map(|env_var| env_var.name().clone())
            .collect();
        if new_names.len() != changes.len() {
            anyhow::bail!(env_var_name_not_unique(None));
        }

        let changed_env_vars_ids: HashSet<ResolvedDocumentId> = changes.keys().cloned().collect();

        let mut previous_env_vars: HashMap<ResolvedDocumentId, EnvironmentVariable> =
            HashMap::new();
        for (id, environment_variable) in changes.clone() {
            let new_env_var_name = environment_variable.name().to_owned();
            let document = self.tx.get(id).await?.ok_or_else(|| {
                ErrorMetadata::not_found(
                    "ModifiedEnvVarNotFound",
                    "The modified environment variable couldn’t be found.",
                )
            })?;

            // Ensure there is no conflict with an environment variable not in this change
            let maybe_env_var_with_name = self.get(&new_env_var_name).await?;
            if let Some(env_var_with_name) = maybe_env_var_with_name
                && !changed_env_vars_ids.contains(&env_var_with_name.id())
            {
                anyhow::bail!(env_var_name_not_unique(Some(&new_env_var_name)));
            }

            SystemMetadataModel::new_global(self.tx)
                .replace(
                    id,
                    PersistedEnvironmentVariable(environment_variable).try_into()?,
                )
                .await?;

            let previous_env_var: ParsedDocument<PersistedEnvironmentVariable> =
                document.try_into()?;
            previous_env_vars.insert(id, previous_env_var.into_value().0);
        }

        for (id, previous_env_var) in previous_env_vars {
            let new_env_var = changes
                .get(&id)
                .context("can’t find the matching new environment variable in changes")?;

            // Log up to two events for each env variable:
            // - ReplaceEnvironmentVariable if the name changed
            // - UpdateEnvironmentVariable if the value changed
            if new_env_var.name() != previous_env_var.name() {
                audit_events.push(DeploymentAuditLogEvent::ReplaceEnvironmentVariable {
                    previous_name: previous_env_var.name().clone(),
                    name: new_env_var.name().clone(),
                });
            }
            if previous_env_var.value() != new_env_var.value() {
                audit_events.push(DeploymentAuditLogEvent::UpdateEnvironmentVariable {
                    name: new_env_var.name().clone(),
                });
            };
        }

        Ok(audit_events)
    }
}

fn value_query_from_env_var(env_var: &EnvVarName) -> anyhow::Result<Query> {
    let range = vec![IndexRangeExpression::Eq(
        NAME_FIELD.clone(),
        ConvexValue::try_from(String::from(env_var.clone()))?.into(),
    )];
    Ok(Query::index_range(IndexRange {
        index_name: ENVIRONMENT_VARIABLES_INDEX_BY_NAME.clone(),
        range,
        order: Order::Asc,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::{
        BTreeMap,
        HashSet,
    };

    use common::types::{
        EnvVarName,
        EnvVarValue,
        EnvironmentVariable,
    };
    use database::test_helpers::DbFixtures;
    use maplit::btreemap;
    use runtime::testing::TestRuntime;

    use crate::{
        environment_variables::EnvironmentVariablesModel,
        test_helpers::DbFixturesWithModel,
    };

    #[convex_macro::test_runtime]
    async fn test_create_get(rt: TestRuntime) -> anyhow::Result<()> {
        let database = DbFixtures::new_with_model(&rt).await?.db;
        let mut tx = database.begin_system().await?;
        let mut env_model = EnvironmentVariablesModel::new(&mut tx);
        let name: EnvVarName = "hello".parse()?;
        let value: EnvVarValue = "world".parse()?;
        let env_var = EnvironmentVariable::new(name.clone(), value.clone());
        env_model.create(env_var.clone(), &HashSet::new()).await?;
        assert_eq!(env_model.get(&name).await?.unwrap().into_value(), env_var);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_preload(rt: TestRuntime) -> anyhow::Result<()> {
        let database = DbFixtures::new_with_model(&rt).await?.db;

        let env_vars: BTreeMap<EnvVarName, EnvVarValue> = btreemap! {
            "hello".parse()? => "world".parse()?,
            "goodbye".parse()? => "blue sky".parse()?,
        };
        {
            let mut create_tx = database.begin_system().await?;
            for (name, value) in &env_vars {
                let env_var = EnvironmentVariable::new(name.clone(), value.clone());
                EnvironmentVariablesModel::new(&mut create_tx)
                    .create(env_var.clone(), &HashSet::new())
                    .await?;
            }
            database.commit(create_tx).await?;
        }

        // NB: The tokens don't line up for an empty query (i.e. `names = &[]`) since
        // the preloaded path runs a query, loading a read dependency on the
        // `_index` table, while the regular path doesn't execute anything.
        let test_cases: &[&[&str]] =
            &[&["hello"], &["hello", "goodbye"], &["hello", "nonexistent"]];
        for &names in test_cases {
            let preload_token = {
                let mut preload_tx = database.begin_system().await?;
                let preloaded = EnvironmentVariablesModel::new(&mut preload_tx)
                    .preload()
                    .await?;
                for name in names {
                    let name = name.parse()?;
                    assert_eq!(
                        preloaded.get(&mut preload_tx, &name)?,
                        env_vars.get(&name).cloned()
                    );
                }
                preload_tx.into_token()?
            };
            let regular_token = {
                let mut regular_tx = database.begin_system().await?;
                for name in names {
                    let name = name.parse()?;
                    assert_eq!(
                        EnvironmentVariablesModel::new(&mut regular_tx)
                            .get(&name)
                            .await?
                            .map(|doc| doc.into_value().value),
                        env_vars.get(&name).cloned()
                    );
                }
                regular_tx.into_token()?
            };
            assert_eq!(
                preload_token.reads(),
                regular_token.reads(),
                "Mismatch for {names:?}"
            );
        }

        Ok(())
    }
}
