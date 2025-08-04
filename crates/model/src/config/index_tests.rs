use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    future::Future,
    str::FromStr,
};

use common::{
    bootstrap_model::index::{
        database_index::DatabaseIndexState,
        text_index::TextIndexState,
        vector_index::VectorIndexState,
        IndexConfig,
    },
    object_validator,
    schemas::{
        validator::{
            FieldValidator,
            Validator,
        },
        DatabaseSchema,
        DocumentSchema,
        TableDefinition,
        TextIndexSchema,
    },
    types::TableName,
    value::FieldPath,
};
use database::{
    test_helpers::{
        index_utils::{
            assert_backfilled,
            assert_backfilling,
            assert_enabled,
            get_index_fields,
            get_recent_index_metadata,
            new_index_descriptor,
            new_index_name,
        },
        new_test_database,
        new_tx,
        DbFixtures,
    },
    Database,
    IndexModel,
};
use runtime::testing::TestRuntime;
use value::TableNamespace;

use crate::{
    config::index_test_utils::{
        apply_config,
        assert_root_cause_contains,
        backfill_indexes,
        db_schema_with_indexes,
        deploy_schema,
        expect_diff,
        prepare_schema,
    },
    test_helpers::DbFixturesWithModel,
};

macro_rules! db_schema_with_search_indexes {
    ($($table:expr => [$(($index_name:expr, $field:expr)),*]),* $(,)?) => {
        {

            #[allow(unused)]
            let mut tables = BTreeMap::new();
            {
                $(
                    let table_name: TableName = str::parse($table)?;
                    #[allow(unused)]
                    let mut text_indexes = BTreeMap::new();
                    $(
                        let index_name = new_index_name($table, $index_name)?;
                        let field_path: FieldPath = str::parse($field).unwrap();
                        text_indexes.insert(
                            index_name.descriptor().clone(),
                            TextIndexSchema::new(
                                index_name.descriptor().clone(),
                                field_path.try_into()?,
                                BTreeSet::new(),
                            )?,
                        );
                    )*
                    let table_def = TableDefinition {
                        table_name: table_name.clone(),
                        indexes: BTreeMap::new(),
                        staged_db_indexes: Default::default(),
                        text_indexes,
                        staged_text_indexes: Default::default(),
                        vector_indexes: Default::default(),
                        staged_vector_indexes: Default::default(),
                        document_type: None,
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}

type FnGenSchema =
    Box<dyn Fn(&str, &str, &str, Option<DocumentSchema>) -> anyhow::Result<DatabaseSchema>>;

const TABLE_NAME: &str = "table";
const INDEX_NAME: &str = "index";

/// Run the given test function twice, once on a search index and once on a
/// database index.
///
/// This function can only test cases where there is one table with one index
/// with one field. More complex cases may need to be tested in different
/// functions. Cases where behavior differs between types of indexes should be
/// tested in separate functions as well.
async fn test_search_and_db_indexes<T, Fut>(rt: TestRuntime, test: T) -> anyhow::Result<()>
where
    T: Fn(
        TestRuntime,
        Box<dyn Fn(&str, &str, &str, Option<DocumentSchema>) -> anyhow::Result<DatabaseSchema>>,
    ) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    fn with_document_type(
        schema: DatabaseSchema,
        table_name: &str,
        document_type: Option<DocumentSchema>,
    ) -> DatabaseSchema {
        let mut result = schema;
        result
            .tables
            .get_mut(&TableName::from_str(table_name).unwrap())
            .unwrap()
            .document_type = document_type;
        result
    }

    fn generate_db_schema(
        table_name: &str,
        index_name: &str,
        field: &str,
        document_type: Option<DocumentSchema>,
    ) -> anyhow::Result<DatabaseSchema> {
        Ok(with_document_type(
            db_schema_with_indexes!(&table_name => [(index_name, vec![field])]),
            table_name,
            document_type,
        ))
    }

    fn generate_search_schema(
        table_name: &str,
        index_name: &str,
        field: &str,
        document_type: Option<DocumentSchema>,
    ) -> anyhow::Result<DatabaseSchema> {
        Ok(with_document_type(
            db_schema_with_search_indexes!(&table_name => [(index_name, field)]),
            table_name,
            document_type,
        ))
    }

    test(rt.clone(), Box::new(generate_db_schema)).await?;
    test(rt.clone(), Box::new(generate_search_schema)).await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_new_index_marks_index_backfilling_and_returns_it(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let schema: DatabaseSchema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        let mut tx = new_tx(rt).await?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        expect_diff!(result ; added:[(TABLE_NAME, INDEX_NAME, vec!["a"])], dropped:[]);
        assert_backfilling(tx, TABLE_NAME, INDEX_NAME)
    })
    .await
}

// We expect the index to be returned because it's used by the CLI to tell the
// user what indexes will be impacted by their push.
#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_removed_index_does_not_remove_it_but_does_return_it(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;
        let mut tx = db.begin_system().await?;
        let schema = db_schema_with_indexes!(TABLE_NAME =>[]);
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        expect_diff!(result ; added:[], dropped:[(TABLE_NAME, INDEX_NAME, vec!["a"])]);
        assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_mutated_index_not_yet_enabled_removes_the_old_version(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        let mut tx = new_tx(rt).await?;
        IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        let current_index = IndexModel::new(&mut tx)
            .pending_index_metadata(
                TableNamespace::test_user(),
                &new_index_name(TABLE_NAME, INDEX_NAME)?,
            )?
            .unwrap();

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        assert!(!IndexModel::new(&mut tx)
            .get_all_indexes()
            .await?
            .iter()
            .any(|index| index.id().internal_id() == current_index.id().internal_id()));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_mutated_index_not_yet_enabled_stores_new_version(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        let mut tx = new_tx(rt).await?;
        let mut index_model = IndexModel::new(&mut tx);
        index_model
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        index_model
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        let new_index = index_model
            .pending_index_metadata(
                TableNamespace::test_user(),
                &new_index_name(TABLE_NAME, INDEX_NAME)?,
            )?
            .unwrap()
            .into_value();

        let actual_fields = get_index_fields(new_index);
        assert_eq!(actual_fields, vec!["b"]);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_mutated_index_not_yet_enabled_backfills_and_returns_it(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        let mut tx = new_tx(rt).await?;
        IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;

        expect_diff!(result ; added:[(TABLE_NAME, INDEX_NAME, vec!["b"])], dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"])]);
        assert_backfilling(tx, TABLE_NAME, INDEX_NAME)
    })
    .await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_enabled_and_pending_mutated_index_removes_pending_version(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db , ..} = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        let mut tx = db.begin_system().await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;
        expect_diff!(result ; added:[(TABLE_NAME, INDEX_NAME, vec!["b"])], dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"])]);
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "c", None)?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;
        expect_diff!(result ;
            added:[(TABLE_NAME, INDEX_NAME, vec!["c"])],
            dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"]), (TABLE_NAME, INDEX_NAME, vec!["b"])]);
        db.commit(tx).await?;

        let all_index_data: Vec<IndexConfig> =
            get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

        assert_index_data(
            all_index_data,
            vec![
                TestIndexConfig::new("c", TestIndexState::Backfilling),
                TestIndexConfig::new("a", TestIndexState::Enabled),
            ],
        );
        Ok(())
    })
    .await
}

// Check that preparing an edited search index on a table retains the existing
// enabled index during the prepare phase.
#[convex_macro::test_runtime]
async fn test_prepare_editing_enabled_search_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "a")]);
    deploy_schema(&rt, tp.clone(), &db, schema).await?;

    let mut tx = db.begin_system().await?;
    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "b")]);
    let result = IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    expect_diff!(result ;
        added:[(TABLE_NAME, INDEX_NAME, vec!["b"])],
        dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"])]);
    db.commit(tx).await?;

    let all_index_data: Vec<IndexConfig> =
        get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

    assert_index_data(
        all_index_data,
        vec![
            TestIndexConfig::new("b", TestIndexState::Backfilling),
            TestIndexConfig::new("a", TestIndexState::Enabled),
        ],
    );
    Ok(())
}

// Check that preparing an edited search index while another in progress index
// exists removes the first one.
#[convex_macro::test_runtime]
async fn test_prepare_stacked_search_index_edits(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
    let mut tx = db.begin_system().await?;
    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "a")]);
    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    db.commit(tx).await?;

    let mut tx = db.begin_system().await?;
    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "b")]);
    let result = IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    expect_diff!(result ;
        added: [(TABLE_NAME, INDEX_NAME, vec!["b"])],
        dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"])]);
    db.commit(tx).await?;

    let all_index_data: Vec<IndexConfig> =
        get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

    assert_index_data(
        all_index_data,
        vec![TestIndexConfig::new("b", TestIndexState::Backfilling)],
    );
    Ok(())
}

// Check that that preparing a new mutated sarch index with an existing
// backfilled mutated search index removes and returns it.
#[convex_macro::test_runtime]
async fn test_editing_backfilled_mutated_search_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let mut tx = db.begin_system().await?;
    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "a")]);
    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    db.commit(tx).await?;
    backfill_indexes(rt, db.clone(), tp).await?;

    let mut tx = db.begin_system().await?;
    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "b")]);
    let result = IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    expect_diff!(result ;
        added:[(TABLE_NAME, INDEX_NAME, vec!["b"])],
        dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"])]);
    db.commit(tx).await?;

    let all_index_data: Vec<IndexConfig> =
        get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

    assert_index_data(
        all_index_data,
        vec![TestIndexConfig::new("b", TestIndexState::Backfilling)],
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_enabled_mutated_index_does_not_remove_or_return_it(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        let mut tx = db.begin_system().await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;
        expect_diff!(result ;
        added:[(TABLE_NAME, INDEX_NAME, vec!["b"])],
        dropped: [(TABLE_NAME, INDEX_NAME, vec!["a"])]);
        db.commit(tx).await?;

        let all_index_data: Vec<IndexConfig> =
            get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

        assert_index_data(
            all_index_data,
            vec![
                TestIndexConfig::new("b", TestIndexState::Backfilling),
                TestIndexConfig::new("a", TestIndexState::Enabled),
            ],
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn backfill_indexes_with_pending_and_enabled_mutated_indexes_does_not_modify_enabled_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        let mut tx = db.begin_system().await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;
        db.commit(tx).await?;
        backfill_indexes(rt, db.clone(), tp).await?;

        let all_index_data: Vec<IndexConfig> =
            get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

        assert_index_data(
            all_index_data,
            vec![
                TestIndexConfig::new("b", TestIndexState::Backfilled),
                TestIndexConfig::new("a", TestIndexState::Enabled),
            ],
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_pending_and_enabled_mutated_indexes_removes_enabled_and_enables_pending(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        deploy_schema(&rt, tp, &db, schema).await?;

        let all_index_data: Vec<IndexConfig> =
            get_all_index_configs(&db, TABLE_NAME, INDEX_NAME).await?;

        assert_index_data(
            all_index_data,
            vec![TestIndexConfig::new("b", TestIndexState::Enabled)],
        );

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_enabled_identical_index_does_not_backfill_a_second_copy(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

        let mut tx = db.begin_system().await?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;
        expect_diff!(result ; added:[], dropped: []);
        db.commit(tx).await?;

        assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

// Test that preparning new and mutated indexes with an enabled index and a new
// index doesn't backfill the identical index.
#[convex_macro::test_runtime]
async fn test_two_indexes_on_one_table(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!(TABLE_NAME =>[(INDEX_NAME, vec!["a"])]);
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let other_index = "other";
    let schema =
        db_schema_with_indexes!(TABLE_NAME =>[(INDEX_NAME, vec!["a"]), (other_index, vec!["b"])]);
    let mut tx = db.begin_system().await?;
    let result = IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
        .await?;
    expect_diff!(result ; added:[(TABLE_NAME, other_index, vec!["b"])], dropped: []);
    db.commit(tx).await?;

    assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
}

#[convex_macro::test_runtime]
async fn prepare_new_mutated_indexes_with_backfilled_identical_index_does_not_backfill_it_again(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        prepare_schema(&db, schema.clone()).await?;
        backfill_indexes(rt, db.clone(), tp).await?;
        let mut tx = db.begin_system().await?;
        let result = IndexModel::new(&mut tx)
            .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema)
            .await?;
        expect_diff!(result ; added:[], dropped: []);
        db.commit(tx).await?;

        assert_backfilled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

// The index will instead be removed in the apply_config call that should follow
// prepare_schema.
#[convex_macro::test_runtime]
async fn prepare_schema_with_dropped_index_does_not_remove_it(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // This is an empty schema regardless of whether we're currently testing a
        // search or a db index.
        let schema = db_schema_with_indexes!(TABLE_NAME =>[]);
        prepare_schema(&db, schema).await?;

        assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

// The index will instead be enabled when push_config is eventually called by
// the CLI.
#[convex_macro::test_runtime]
async fn prepare_schema_with_added_index_does_not_enable_it_after_backfill(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        prepare_schema(&db, schema).await?;
        backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;

        assert_backfilled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_backfilling_database_index_throws(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;

    let schema = db_schema_with_indexes!("table" => [("index", vec!["a"])]);

    let schema_id = prepare_schema(&db, schema).await?;

    let result = apply_config(db, Some(schema_id)).await;

    // The CLI should have waited until the index was backfilled before trying to
    // commit the schema.
    assert_root_cause_contains(
        result,
        "Expected backfilled index, but found: Backfilling(DatabaseIndexBackfillState",
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn apply_config_with_backfilling_search_index_throws(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;

    let schema = db_schema_with_search_indexes!("table" => [("index", "a")]);

    let schema_id = prepare_schema(&db, schema).await?;

    let result = apply_config(db, Some(schema_id)).await;

    // The CLI should have waited until the index was backfilled before trying to
    // commit the schema.
    assert_root_cause_contains(
        result,
        "Expected backfilled index, but found: Backfilling(TextIndexBackfillState { segments: [], \
         cursor: None }) for \"index\"",
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn apply_config_with_backfilled_index_sets_it_to_enabled(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        deploy_schema(&rt, tp, &db, schema).await?;

        assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_index_not_present_in_schema_drops_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // Empty schema regardless of whether we're testing search indexes or database.
        let new_schema = db_schema_with_indexes!(TABLE_NAME =>[]);
        deploy_schema(&rt, tp.clone(), &db, new_schema).await?;

        let mut tx = db.begin_system().await?;
        let index_result = get_recent_index_metadata(&mut tx, TABLE_NAME, INDEX_NAME);
        assert_root_cause_contains(
            index_result,
            &format!("Missing index: {TABLE_NAME}.{INDEX_NAME}"),
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_partially_committed_index_not_present_in_schema_drops_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        prepare_schema(&db, schema).await?;
        backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;

        // CLI deploys a different schema without committing the first one
        // Empty schema is the same regardless of which type of index we're testing.
        let new_schema = db_schema_with_indexes!(TABLE_NAME =>[]);
        deploy_schema(&rt, tp.clone(), &db, new_schema).await?;

        let mut tx = db.begin_system().await?;
        let index_result = get_recent_index_metadata(&mut tx, TABLE_NAME, INDEX_NAME);
        assert_root_cause_contains(
            index_result,
            &format!("Missing index: {TABLE_NAME}.{INDEX_NAME}"),
        );
        Ok(())
    })
    .await
}

// Test that deploying a schema with partially committed mutated backfilled
// indexes that are mutated again commits the final version.
#[convex_macro::test_runtime]
async fn test_mutating_partially_committed_mutated_index(rt: TestRuntime) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        // CLI pushes a new schema with an index
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content. Probably due to schema validation, this push fails and is
        // not committed.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        prepare_schema(&db, schema).await?;
        backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;

        // User then tries to commit a third version of the index, which should work.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "c", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        assert_enabled_with_fields(&db, TABLE_NAME, INDEX_NAME, vec!["c"]).await?;
        Ok(())
    })
    .await
}

// Test that deploying a schema with partially committed mutated backfilled
// that's been mutated again commits the final version.
#[convex_macro::test_runtime]
async fn test_stacked_schema_edits(rt: TestRuntime) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        // CLI pushes a new schema with an index
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content. Probably due to schema validation, this push fails and is
        // not committed.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        prepare_schema(&db, schema).await?;

        // User then tries to commit a third version of the index, which should work.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "c", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        assert_enabled_with_fields(&db, TABLE_NAME, INDEX_NAME, vec!["c"]).await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn deploy_schema_with_partially_committed_mutated_index_in_backfilled_state_commits_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        // CLI pushes a new schema with an index
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content. Probably due to schema validation, this push fails and is
        // not committed.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        prepare_schema(&db, schema).await?;
        backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;

        // User then tries to commit the new schema a second time, which should work.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        assert_enabled_with_fields(&db, TABLE_NAME, INDEX_NAME, vec!["b"]).await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn deploy_schema_with_partially_committed_mutated_index_in_backfilling_state_commits_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        // CLI pushes a new schema with an index
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content. Probably due to schema validation, this push fails and is
        // not committed.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        prepare_schema(&db, schema).await?;

        // User then tries to commit the new schema a second time, which should work.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        assert_enabled_with_fields(&db, TABLE_NAME, INDEX_NAME, vec!["b"]).await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_existing_index_and_removed_schema_drops_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        deploy_schema(&rt, tp, &db, schema).await?;

        // CLI sends apply_config without a schema (ie the user deletes the schema
        // file).
        apply_config(db.clone(), None).await?;

        let mut tx = db.begin_system().await?;
        let index_result = get_recent_index_metadata(&mut tx, TABLE_NAME, INDEX_NAME);
        assert_root_cause_contains(
            index_result,
            &format!("Missing index: {TABLE_NAME}.{INDEX_NAME}"),
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_enabled_index_ignores_it(rt: TestRuntime) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { db, tp, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        // CLI pushes a new schema with an index
        let schema_id = prepare_schema(&db, schema).await?;
        backfill_indexes(rt, db.clone(), tp).await?;

        // Probably some other call to apply_config races and enables the index first.
        let mut tx = db.begin_system().await?;
        let generic_index_name = new_index_name(TABLE_NAME, INDEX_NAME)?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(TableNamespace::test_user(), &generic_index_name)
            .await?;
        db.commit(tx).await?;

        // CLI now commits the index
        apply_config(db.clone(), Some(schema_id)).await?;

        // We don't do anything untoward, like crash or remove it
        assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_backfilled_mutated_index_sets_it_to_enabled(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        // CLI pushes a new schema with an index
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "b", None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_backfilled_mutated_index_stores_updated_definition(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;

        // CLI pushes a new schema with an index
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content.
        let new_field_name = "b";
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, new_field_name, None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // We use the new version of the index with the updated fields.
        assert_enabled_with_fields(&db, TABLE_NAME, INDEX_NAME, vec![new_field_name]).await?;

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn apply_config_with_enabled_mutated_index_does_not_fail(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
        // CLI pushes a new schema with an index
        let original_field_name = "a";
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, original_field_name, None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI pushes an updated version of the schema with the same index name, but
        // different content.
        let new_field_name = "b";
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, new_field_name, None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await?;

        // CLI re-pushes the original definition. If we haven't properly updated
        // storage, then this will fail when we try to backfill the index.
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, original_field_name, None)?;
        deploy_schema(&rt, tp.clone(), &db, schema).await
    })
    .await
}

#[convex_macro::test_runtime]
async fn build_indexes_with_backfilled_but_not_enabled_index_does_not_fail(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_search_and_db_indexes(rt, async move |rt, new_schema_with_index: FnGenSchema| {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "a", None)?;
        let mut tx = db.begin_system().await?;
        IndexModel::new(&mut tx)
            .build_indexes(TableNamespace::test_user(), &schema)
            .await?;
        db.commit(tx).await?;

        // If we treat the backfilled but not enabled index as added here, we'll try and
        // add it without removing it, which will trigger a failure.
        let mut tx = db.begin_system().await?;
        IndexModel::new(&mut tx)
            .build_indexes(TableNamespace::test_user(), &schema)
            .await?;
        db.commit(tx).await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn deploy_schema_with_search_config_enables_search_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

    let schema = db_schema_with_search_indexes!(TABLE_NAME =>[(INDEX_NAME, "a")]);

    // CLI pushes a new schema with an index
    deploy_schema(&rt, tp.clone(), &db, schema).await?;

    assert_enabled(&db, TABLE_NAME, INDEX_NAME).await
}

#[convex_macro::test_runtime]
async fn deploy_schema_with_multi_field_database_index_enables_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

    let schema = db_schema_with_indexes!(TABLE_NAME =>[(INDEX_NAME, vec!["a", "b", "c"])]);

    // CLI pushes a new schema with an index
    deploy_schema(&rt, tp.clone(), &db, schema).await?;

    assert_enabled(&db, TABLE_NAME, INDEX_NAME).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn deploy_schema_with_multi_field_database_index_stores_field_names(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

    let field_names = vec!["a", "b", "c"];
    let schema = db_schema_with_indexes!(TABLE_NAME =>[(INDEX_NAME, field_names)]);

    deploy_schema(&rt, tp.clone(), &db, schema).await?;
    assert_enabled_with_fields(&db, TABLE_NAME, INDEX_NAME, field_names).await?;

    Ok(())
}

async fn assert_enabled_with_fields(
    db: &Database<TestRuntime>,
    table: &str,
    index: &str,
    expected_field_names: Vec<&str>,
) -> anyhow::Result<()> {
    let mut tx = db.begin_system().await?;
    let index = IndexModel::new(&mut tx)
        .enabled_index_metadata(TableNamespace::test_user(), &new_index_name(table, index)?)?
        .unwrap();
    let actual_field_names = get_index_fields(index.into_value());

    assert_eq!(actual_field_names, expected_field_names);
    Ok(())
}

/// Returns all (at most 2) index definitions for the given table and index name
async fn get_all_index_configs(
    db: &Database<TestRuntime>,
    table_name: &str,
    index_name: &str,
) -> anyhow::Result<Vec<IndexConfig>> {
    let mut tx = db.begin_system().await?;
    let mut index_model = IndexModel::new(&mut tx);
    let index_name = new_index_name(table_name, index_name)?;
    let pending = index_model.pending_index_metadata(TableNamespace::test_user(), &index_name)?;
    let enabled = index_model.enabled_index_metadata(TableNamespace::test_user(), &index_name)?;
    Ok(vec![pending, enabled]
        .into_iter()
        .flatten()
        .map(|metadata| metadata.into_value().config)
        .collect())
}

#[derive(PartialEq, Debug)]
struct TestIndexConfig(String, TestIndexState);
impl TestIndexConfig {
    fn new(field: &str, state: TestIndexState) -> Self {
        Self(format!("\"{field}\""), state)
    }
}

#[derive(PartialEq, Debug)]
enum TestIndexState {
    Backfilling,
    Backfilled,
    Enabled,
}

fn assert_index_data(actual: Vec<IndexConfig>, expected: Vec<TestIndexConfig>) {
    let actual: Vec<TestIndexConfig> = actual
        .into_iter()
        .map(|config| match config {
            IndexConfig::Database {
                developer_config,
                on_disk_state,
            } => {
                let db_state = match on_disk_state {
                    DatabaseIndexState::Backfilling(_) => TestIndexState::Backfilling,
                    DatabaseIndexState::Backfilled { .. } => TestIndexState::Backfilled,
                    DatabaseIndexState::Enabled => TestIndexState::Enabled,
                };
                assert_eq!(developer_config.fields.len(), 1);
                let field_name = &developer_config.fields[0];
                TestIndexConfig(field_name.to_string(), db_state)
            },
            IndexConfig::Text {
                developer_config,
                on_disk_state,
            } => {
                let search_state = match on_disk_state {
                    TextIndexState::Backfilling(_) => TestIndexState::Backfilling,
                    TextIndexState::Backfilled(_) => TestIndexState::Backfilled,
                    TextIndexState::SnapshottedAt(_) => TestIndexState::Enabled,
                };
                TestIndexConfig(developer_config.search_field.to_string(), search_state)
            },
            IndexConfig::Vector {
                developer_config,
                on_disk_state,
            } => {
                let vector_state = match on_disk_state {
                    VectorIndexState::Backfilling(_) => TestIndexState::Backfilling,
                    VectorIndexState::Backfilled(_) => TestIndexState::Backfilled,
                    VectorIndexState::SnapshottedAt(_) => TestIndexState::Enabled,
                };
                TestIndexConfig(developer_config.vector_field.to_string(), vector_state)
            },
        })
        .collect();

    assert_eq!(actual, expected);
}

mod check_index_references {
    use errors::ErrorMetadata;

    use super::*;

    #[convex_macro::test_runtime]
    async fn returns_an_error_if_index_references_an_non_existing_column_on_an_enforced_schema(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        test_search_and_db_indexes(rt, async move |_, new_schema_with_index: FnGenSchema| {
            let schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "non_existent_field", Some(DocumentSchema::Union(vec![
                object_validator!("field" => FieldValidator::required_field_type(Validator::String)),
            ])))?;

            let result = schema.check_index_references();
            let error = result.unwrap_err().downcast::<ErrorMetadata>().unwrap();
            assert_eq!(error.short_msg, "SchemaDefinitionError");
            Ok(())
        })
        .await
    }

    #[convex_macro::test_runtime]
    async fn does_not_return_an_error_when_referencing_a_non_existing_field_in_an_unenforced_schema(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        test_search_and_db_indexes(rt, async move |_, new_schema_with_index: FnGenSchema | {
            let mut schema = new_schema_with_index(TABLE_NAME, INDEX_NAME, "non_existent_field", Some(DocumentSchema::Union(vec![
                object_validator!("field" => FieldValidator::required_field_type(Validator::String)),
            ])))?;
            schema.schema_validation = false;

            let result = schema.check_index_references();
            result.unwrap();

            Ok(())
        })
        .await
    }

    #[convex_macro::test_runtime]
    async fn does_not_return_an_error_when_referencing_a_field_on_a_nested_path(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        test_search_and_db_indexes(rt, async move |_, new_schema_with_index: FnGenSchema| {
            let schema: DatabaseSchema = new_schema_with_index(
                TABLE_NAME,
                INDEX_NAME,
                "field.subfield",
                Some(DocumentSchema::Union(vec![object_validator!(
                    "field" => FieldValidator::required_field_type(
                        Validator::Object(
                            object_validator!(
                                "subfield" =>
                                FieldValidator::required_field_type(Validator::String)
                            ),
                        )
                    )
                )])),
            )?;

            let result = schema.check_index_references();
            result.unwrap();

            Ok(())
        })
        .await
    }
}
