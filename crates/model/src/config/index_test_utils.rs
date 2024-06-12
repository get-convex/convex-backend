use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::Arc,
};

use common::{
    bootstrap_model::schema::SchemaState,
    persistence::{
        NoopRetentionValidator,
        Persistence,
    },
    schemas::DatabaseSchema,
    value::{
        GenericDocumentId,
        TabletIdAndTableNumber,
    },
    version::Version,
};
use database::{
    vector_index_worker::flusher::backfill_vector_indexes,
    Database,
    IndexModel,
    IndexWorker,
    SchemaModel,
    TextIndexFlusher,
};
use runtime::testing::TestRuntime;
use storage::LocalDirStorage;
use value::TableNamespace;

use crate::{
    config::ConfigModel,
    udf_config::types::UdfConfig,
};

/// Asserts that the index's descriptors match the provided descriptors
///
/// mutated is a conglomeration of `IndexDiff.replacements` and
/// `IndexDiff.replaced`. It asserts that the descriptors in both lists are
/// equivalent. Differences can be checked outside this macro.
///
/// Similarly `IndexDiff.identical` is not checked by this macro.
macro_rules! expect_diff {
    (
        $diff:expr;
        added: [$(($at:expr, $ai:expr, $af:expr)),*],
        dropped: [$(($dt:expr, $di:expr, $df:expr)),*]
    ) => {
        let added_descriptors = vec![
            $((
                new_index_descriptor($at, $ai)?,
                $af.into_iter().map(|str| str.to_string()).collect(),
            ),)*
        ];
        let dropped_descriptors = vec![
            $((
                new_index_descriptor($dt, $di)?,
                $df.into_iter().map(|str| str.to_string()).collect()
            ),)*
        ];
        assert_eq!(
            database::test_helpers::index_utils::index_descriptors_and_fields(&$diff),
            vec![added_descriptors, dropped_descriptors]
        );
    };
}
pub(crate) use expect_diff;

// Turns a mapping of tableName => (index_name, vec![index_fields]) into a
// DatabaseSchema struct.
macro_rules! db_schema_with_indexes {
    ($($table:expr => [$(($index_name:expr, $fields:expr)),*]),* $(,)?) => {
        {
            #[allow(unused)]
            let mut tables = std::collections::BTreeMap::new();
            {
                $(
                    let table_name: common::types::TableName = str::parse($table)?;
                    #[allow(unused)]
                    let mut indexes = std::collections::BTreeMap::new();
                    $(
                        let index_name = database::test_helpers::index_utils::new_index_name(
                            $table,
                            $index_name,
                        )?;
                        let field_paths: Vec<common::paths::FieldPath> = $fields
                            .iter()
                            .map(|s| str::parse(s).unwrap())
                            .collect();
                        indexes.insert(
                            index_name.descriptor().clone(),
                            common::schemas::IndexSchema {
                                index_descriptor: index_name.descriptor().clone(),
                                fields: field_paths.try_into()?,
                            },
                        );
                    )*
                    let table_def = common::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes,
                        search_indexes: Default::default(),
                        vector_indexes: Default::default(),
                        document_type: None,
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            common::schemas::DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}
pub(crate) use db_schema_with_indexes;

use super::types::ConfigMetadata;

pub fn assert_root_cause_contains<T: Debug>(result: anyhow::Result<T>, expected: &str) {
    let error = result.unwrap_err();
    let root_cause = error.root_cause();
    assert!(
        format!("{}", root_cause).contains(expected),
        "Root cause \"{root_cause}\" does not contain expected string:\n\"{expected}\""
    );
}

/// Simulate a CLI pushing a schema, waiting for backfill, then committing the
/// schema.
pub async fn deploy_schema(
    rt: &TestRuntime,
    tp: Arc<dyn Persistence>,
    db: &Database<TestRuntime>,
    schema: DatabaseSchema,
) -> anyhow::Result<()> {
    let schema_id = prepare_schema(db, schema).await?;
    backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;
    apply_config(db.clone(), Some(schema_id)).await?;
    Ok(())
}

// Ideally we'd call the actual prepare_schema method, but it's not currently
// very well factored. So instead we mimic it here.
pub async fn prepare_schema(
    db: &Database<TestRuntime>,
    schema: DatabaseSchema,
) -> anyhow::Result<GenericDocumentId<TabletIdAndTableNumber>> {
    let mut tx = db.begin_system().await?;
    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    let mut schema_model = SchemaModel::new_root_for_test(&mut tx);
    let (schema_id, state) = schema_model.submit_pending(schema).await?;
    // Mimic schema_worker running, without actually running it.
    if state != SchemaState::Validated {
        schema_model.mark_validated(schema_id).await?;
    }
    db.commit(tx).await?;
    Ok(schema_id)
}

pub async fn apply_config(
    db: Database<TestRuntime>,
    schema_id: Option<GenericDocumentId<TabletIdAndTableNumber>>,
) -> anyhow::Result<()> {
    // This is a kind of arbitrary version that supports schema validation. I'm not
    // even sure that the value here matters at all.
    let udf_server_version: Version = Version::parse("0.14.0")?;
    let config_metadata = ConfigMetadata {
        functions: "convex/".to_string(),
        auth_info: vec![],
    };

    let mut tx = db.begin_system().await?;
    ConfigModel::new(&mut tx)
        .apply(
            config_metadata,
            vec![],
            UdfConfig::new_for_test(db.runtime(), udf_server_version),
            None,
            BTreeMap::new(),
            schema_id,
        )
        .await?;
    db.commit(tx).await?;
    Ok(())
}

pub async fn backfill_indexes(
    rt: TestRuntime,
    db: Database<TestRuntime>,
    tp: Arc<dyn Persistence>,
) -> anyhow::Result<()> {
    let storage = LocalDirStorage::new(rt.clone())?;
    TextIndexFlusher::backfill_all_in_test(rt.clone(), db.clone(), Arc::new(storage.clone()))
        .await?;
    backfill_vector_indexes(
        rt.clone(),
        db.clone(),
        tp.reader(),
        Arc::new(storage.clone()),
    )
    .await?;
    // As long as these tests don't actually have data in the tables, we could
    // probably just mutate the index state. But running the whole IndexWorker
    // is easy and is a bit more robust to changes, so why not...
    let retention_validator = Arc::new(NoopRetentionValidator);
    IndexWorker::new_terminating(rt, tp, retention_validator, db).await?;
    Ok(())
}
