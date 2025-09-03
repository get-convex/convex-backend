use common::db_schema_with_indexes;
use database::{
    test_helpers::{
        index_utils::{
            descriptors,
            new_index_descriptor,
            values,
        },
        new_tx,
        DbFixtures,
    },
    IndexModel,
};
use runtime::testing::TestRuntime;
use value::TableNamespace;

use crate::{
    config::index_test_utils::{
        backfill_indexes,
        deploy_schema,
        expect_diff,
        prepare_schema,
    },
    test_helpers::DbFixturesWithModel,
};

#[convex_macro::test_runtime]
async fn get_index_diff_with_no_indexes_returns_empty_diff(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;
    let schema = db_schema_with_indexes!();

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff);

    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_no_existing_tables_and_one_new_index_returns_added_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let index_name = "index";
    let table_name = "table";
    let schema = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff; added:[(table_name, index_name, vec!["a"])], dropped:[]);

    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_table_but_no_index_and_one_new_index_returns_added_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let index_name = "index";
    let table_name = "table";

    let schema_table_only = db_schema_with_indexes!(table_name => {});
    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema_table_only)
        .await?;

    let schema_with_index = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema_with_index.tables)
        .await?;

    expect_diff!(diff; added:[(table_name, index_name, vec!["a"])], dropped:[]);

    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_one_existing_index_that_is_removed_returns_dropped_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let index_name = "index";
    let table_name = "table";
    let schema_with_index = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });

    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema_with_index)
        .await?;

    let schema_without_index = db_schema_with_indexes!(table_name => {});

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema_without_index.tables)
        .await?;

    expect_diff!(diff; added:[], dropped:[(table_name, index_name, vec!["a"])]);

    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_one_existing_index_when_table_is_removed_returns_dropped_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let index_name = "index";
    let table_name = "table";
    let schema_with_index = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });

    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(TableNamespace::test_user(), &schema_with_index)
        .await?;

    let schema_without_index = db_schema_with_indexes!();

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema_without_index.tables)
        .await?;

    expect_diff!(diff; added:[], dropped:[(table_name, index_name, vec!["a"])]);

    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_one_existing_index_that_is_mutated_returns_mutated_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let index_name = "index";
    let table_name = "table";
    let schema_with_single_field_index = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });

    IndexModel::new(&mut tx)
        .prepare_new_and_mutated_indexes(
            TableNamespace::test_user(),
            &schema_with_single_field_index,
        )
        .await?;

    let schema_with_multi_field_index = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a", "b"])
    });

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(
            TableNamespace::test_user(),
            &schema_with_multi_field_index.tables,
        )
        .await?;

    expect_diff!(diff;
        added:[(table_name, index_name, vec!["a", "b"])],
        dropped:[(table_name, index_name, vec!["a"])]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_new_indexes_from_two_tables_returns_added_indexes_from_both(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let table_name1 = "table1";
    let index_name1 = "index1";

    let table_name2 = "table2";
    let index_name2 = "index2";
    let schema = db_schema_with_indexes!(
        table_name1 => {
            indexes: (index_name1, vec!["a"])
        },
        table_name2 => {
            indexes: (index_name2, vec!["a"])
        }
    );

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ;
        added:[(table_name1, index_name1, vec!["a"]), (table_name2, index_name2, vec!["a"])],
        dropped:[]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_existing_unmodified_enabled_indexes_ignores_them(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

    let table_name1 = "table1";
    let index_name1 = "index1";

    let table_name2 = "table2";
    let index_name2 = "index2";
    let schema = db_schema_with_indexes!(
        table_name1 => {
            indexes: (index_name1, vec!["a"])
        },
        table_name2 => {
            indexes: (index_name2, vec!["a"])
        }
    );
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ;
        added:[],
        dropped:[]);
    Ok(())
}

// Test that `get_index_diff` with existing unmodified indexes returns the
// backfilled index as identical.
#[convex_macro::test_runtime]
async fn test_clean_index_diff_after_backfill(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

    let table_name = "table1";
    let index_name = "index1";

    let schema = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });
    prepare_schema(&db, schema.clone()).await?;
    backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;

    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff);
    assert_eq!(
        descriptors(values(diff.identical)),
        vec![new_index_descriptor(table_name, index_name)?]
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn get_index_diff_with_existing_unmodified_backfilled_indexes_prepare_behavior_ignores_it(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;

    let table_name = "table1";
    let index_name = "index1";

    let schema = db_schema_with_indexes!(table_name => {
        indexes: (index_name, vec!["a"])
    });
    prepare_schema(&db, schema.clone()).await?;
    backfill_indexes(rt.clone(), db.clone(), tp.clone()).await?;

    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff);
    Ok(())
}

// Test that `get_index_diff` with new indexes from two tables with the same
// index name returns added indexes from both.
#[convex_macro::test_runtime]
async fn test_same_index_name_across_two_tables(rt: TestRuntime) -> anyhow::Result<()> {
    let mut tx = new_tx(rt).await?;

    let index_name = "index";

    let table_name1 = "table1";
    let table_name2 = "table2";
    let schema = db_schema_with_indexes!(
        table_name1 => {
            indexes: (index_name, vec!["a"])
        },
        table_name2 => {
            indexes: (index_name, vec!["a"])
        }
    );

    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ;
        added:[(table_name1, index_name, vec!["a"]), (table_name2, index_name, vec!["a"])],
        dropped:[]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_add_staged_index(rt: TestRuntime) -> anyhow::Result<()> {
    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["a.q", "b.q"])
        staged_text_indexes: ("index2", "b.q")
        staged_vector_indexes: ("index3", "c.q")
    });

    let mut tx = new_tx(rt).await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ; added:[
        ("table", "index1", vec!["a.q", "b.q"]),
        ("table", "index2", vec!["b.q"]),
        ("table", "index3", vec!["c.q"]),
    ]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_enable_staged_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["a.q", "b.q"])
        staged_text_indexes: ("index2", "b.q")
        staged_vector_indexes: ("index3", "c.q")
    });
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let schema = db_schema_with_indexes!("table" => {
        indexes: ("index1", vec!["a.q", "b.q"])
        text_indexes: ("index2", "b.q")
        vector_indexes: ("index3", "c.q")
    });
    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ; enabled:[
        ("table", "index1", vec!["a.q", "b.q"]),
        ("table", "index2", vec!["b.q"]),
        ("table", "index3", vec!["c.q"]),
    ]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_disable_enabled_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!("table" => {
        indexes: ("index1", vec!["a.q", "b.q"])
        text_indexes: ("index2", "b.q")
        vector_indexes: ("index3", "c.q")
    });
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["a.q", "b.q"])
        staged_text_indexes: ("index2", "b.q")
        staged_vector_indexes: ("index3", "c.q")
    });
    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ; disabled:[
        ("table", "index1", vec!["a.q", "b.q"]),
        ("table", "index2", vec!["b.q"]),
        ("table", "index3", vec!["c.q"]),
    ]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_remove_staged_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["a.q", "b.q"])
        staged_text_indexes: ("index2", "b.q")
        staged_vector_indexes: ("index3", "c.q")
    });
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let schema = db_schema_with_indexes!("table" => {});
    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ; dropped:[
        ("table", "index1", vec!["a.q", "b.q"]),
        ("table", "index2", vec!["b.q"]),
        ("table", "index3", vec!["c.q"]),
    ]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_change_staged_index(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["a.q"])
    });
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["b.q"])
    });
    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ;
        added:[("table", "index1", vec!["b.q"])],
        dropped: [("table", "index1", vec!["a.q"])]
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_change_staged_index_while_enabling(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["a.q"])
    });
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let schema = db_schema_with_indexes!("table" => {
        indexes: ("index1", vec!["b.q"])
    });
    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ;
        added:[("table", "index1", vec!["b.q"])],
        dropped: [("table", "index1", vec!["a.q"])]
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_change_index_while_disabling(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { tp, db, .. } = DbFixtures::new_with_model(&rt).await?;
    let schema = db_schema_with_indexes!("table" => {
        indexes: ("index1", vec!["a.q"])
    });
    deploy_schema(&rt, tp.clone(), &db, schema.clone()).await?;

    let schema = db_schema_with_indexes!("table" => {
        staged_db_indexes: ("index1", vec!["b.q"])
    });
    let mut tx = db.begin_system().await?;
    let diff = IndexModel::new(&mut tx)
        .get_index_diff(TableNamespace::test_user(), &schema.tables)
        .await?;

    expect_diff!(diff ;
        added:[("table", "index1", vec!["b.q"])],
        dropped: [("table", "index1", vec!["a.q"])]
    );
    Ok(())
}
