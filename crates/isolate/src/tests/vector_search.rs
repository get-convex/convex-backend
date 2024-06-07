use common::{
    assert_obj,
    bootstrap_model::index::{
        vector_index::VectorDimensions,
        IndexMetadata,
    },
    testing::TestPersistence,
};
use database::IndexModel;
use keybroker::Identity;
use maplit::btreeset;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    ConvexValue,
    TableNamespace,
};

use super::assert_contains;
use crate::{
    test_helpers::UdfTest,
    tests::action::action_udf_test,
};

async fn add_vector_index(t: &UdfTest<TestRuntime, TestPersistence>) -> anyhow::Result<()> {
    let mut tx = t.database.begin(Identity::system()).await?;
    let index = IndexMetadata::new_backfilling_vector_index(
        "vectorTable.vector".parse()?,
        "vector".parse()?,
        VectorDimensions::try_from(4)?,
        btreeset! { "filterA".parse()?, "filterB".parse()? },
    );
    IndexModel::new(&mut tx)
        .add_application_index(TableNamespace::test_user(), index)
        .await?;
    t.database.commit(tx).await?;

    Ok(())
}

async fn add_and_backfill_vector_index(
    t: &UdfTest<TestRuntime, TestPersistence>,
) -> anyhow::Result<()> {
    add_vector_index(t).await?;
    t.backfill_vector_indexes().await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multi_field_filter(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = action_udf_test(rt).await?;

    add_and_backfill_vector_index(&t).await?;
    t.mutation("vector_search:populate", assert_obj!()).await?;

    must_let!(let ConvexValue::String(r) =
t.action("vector_search:multiFieldFilter", assert_obj!()).await?);
    assert_eq!(String::from(r), "success".to_string());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multi_value_filter(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = action_udf_test(rt).await?;

    add_and_backfill_vector_index(&t).await?;
    t.mutation("vector_search:populate", assert_obj!()).await?;

    must_let!(let ConvexValue::String(r) = t.action("vector_search:multiValueFilter", assert_obj!()).await?);
    assert_eq!(String::from(r), "success".to_string());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_single_value_filter(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = action_udf_test(rt).await?;

    add_and_backfill_vector_index(&t).await?;
    t.mutation("vector_search:populate", assert_obj!()).await?;

    must_let!(let ConvexValue::String(r) = t.action("vector_search:singleValueFilter", assert_obj!()).await?);
    assert_eq!(String::from(r), "success".to_string());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_invalid_filter(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = action_udf_test(rt).await?;

    add_and_backfill_vector_index(&t).await?;
    t.mutation("vector_search:populate", assert_obj!()).await?;

    let error = t
        .action_js_error("vector_search:invalidFilter", assert_obj!())
        .await?;
    assert_contains(&error, "must take a field path");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_no_filter(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = action_udf_test(rt).await?;

    add_and_backfill_vector_index(&t).await?;
    t.mutation("vector_search:populate", assert_obj!()).await?;

    must_let!(let ConvexValue::String(r) = t.action("vector_search:noFilter", assert_obj!()).await?);
    assert_eq!(String::from(r), "success".to_string());
    Ok(())
}
