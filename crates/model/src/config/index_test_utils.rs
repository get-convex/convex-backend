use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::Arc,
};

use common::{
    bootstrap_model::schema::SchemaState,
    components::ComponentId,
    persistence::{
        NoopRetentionValidator,
        Persistence,
    },
    schemas::DatabaseSchema,
};
use database::{
    text_index_worker::flusher::backfill_text_indexes,
    vector_index_worker::flusher::backfill_vector_indexes,
    Database,
    IndexModel,
    IndexWorker,
    SchemaModel,
};
use runtime::testing::TestRuntime;
use search::searcher::InProcessSearcher;
use storage::LocalDirStorage;
use value::{
    ResolvedDocumentId,
    TableNamespace,
};

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
        $(added: [$(($at:expr, $ai:expr, $af:expr)),*$(,)?]$(,)?)?
        $(dropped: [$(($dt:expr, $di:expr, $df:expr)),*$(,)?]$(,)?)?
        $(enabled: [$(($et:expr, $ei:expr, $ef:expr)),*$(,)?]$(,)?)?
        $(disabled: [$(($dit:expr, $dii:expr, $dif:expr)),*$(,)?]$(,)?)?
    ) => {
        let added_descriptors = vec![
            $($((
                new_index_descriptor($at, $ai)?,
                $af.into_iter().map(|str| str.to_string()).collect(),
            ),)*)?
        ];
        let dropped_descriptors = vec![
            $($((
                new_index_descriptor($dt, $di)?,
                $df.into_iter().map(|str| str.to_string()).collect()
            ),)*)?
        ];
        let enabled_descriptors = vec![
            $($((
                new_index_descriptor($et, $ei)?,
                $ef.into_iter().map(|str| str.to_string()).collect()
            ),)*)?
        ];
        let disabled_descriptors = vec![
            $($((
                new_index_descriptor($dit, $dii)?,
                $dif.into_iter().map(|str| str.to_string()).collect()
            ),)*)?
        ];
        assert_eq!(
            database::test_helpers::index_utils::index_descriptors_and_fields(&$diff),
            vec![added_descriptors, dropped_descriptors, enabled_descriptors, disabled_descriptors]
        );
    };
    ($diff:expr) => {
        expect_diff!($diff;)
    };
}
pub(crate) use expect_diff;

use super::types::ConfigMetadata;

pub fn assert_root_cause_contains<T: Debug>(result: anyhow::Result<T>, expected: &str) {
    let error = result.unwrap_err();
    let root_cause = error.root_cause();
    assert!(
        format!("{root_cause}").contains(expected),
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
) -> anyhow::Result<ResolvedDocumentId> {
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
    schema_id: Option<ResolvedDocumentId>,
) -> anyhow::Result<()> {
    let config_metadata = ConfigMetadata {
        functions: "convex/".to_string(),
        auth_info: vec![],
    };

    let mut tx = db.begin_system().await?;
    ConfigModel::new(&mut tx, ComponentId::test_user())
        .apply(
            config_metadata,
            vec![],
            UdfConfig::new_for_test(db.runtime(), "1000.0.0".parse()?),
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
    let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
    let segment_term_metadata_fetcher = Arc::new(InProcessSearcher::new(rt.clone()).await?);
    backfill_text_indexes(
        rt.clone(),
        db.clone(),
        tp.reader(),
        storage.clone(),
        segment_term_metadata_fetcher,
    )
    .await?;
    backfill_vector_indexes(rt.clone(), db.clone(), tp.reader(), storage).await?;
    // As long as these tests don't actually have data in the tables, we could
    // probably just mutate the index state. But running the whole IndexWorker
    // is easy and is a bit more robust to changes, so why not...
    let retention_validator = Arc::new(NoopRetentionValidator);
    IndexWorker::new_terminating(rt, tp, retention_validator, db).await?;
    Ok(())
}
