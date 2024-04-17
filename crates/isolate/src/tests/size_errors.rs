#![allow(clippy::float_cmp)]

use common::value::ConvexValue;
use runtime::testing::TestRuntime;
use value::{
    array,
    assert_obj,
    assert_val,
};

use crate::{
    test_helpers::UdfTest,
    tests::assert_contains,
};

#[convex_macro::test_runtime]
async fn test_query_throws_nesting_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let js_error = t
        .query_js_error("size_errors:queryThrowsNestingError", assert_obj!())
        .await?;
    assert_contains(&js_error, "Value is too nested");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_does_not_throw_nesting_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.query("size_errors:queryDoesNotThrowNestingError", assert_obj!())
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_argument_does_not_throw_nesting_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;

    let mut deeply_nested = assert_val!(false);
    // 62 levels plus 1 for the whole arguments object, and 1 for the legacy array
    // wrapping the arguments object
    for _ in 0..62 {
        deeply_nested = ConvexValue::Array(array![deeply_nested.clone()]?);
    }

    t.mutation(
        "size_errors:writeToNowhere",
        assert_obj!("x" => deeply_nested),
    )
    .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_argument_throws_nesting_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;

    let mut too_deeply_nested = assert_val!(false);
    // 64 levels plus 1 for the whole arguments object
    for _ in 0..64 {
        too_deeply_nested = ConvexValue::Array(array![too_deeply_nested.clone()]?);
    }
    let argument_error = t
        .action_js_error(
            "size_errors:actionThrowsArgumentNestingError",
            assert_obj!(),
        )
        .await?;
    assert_contains(&argument_error, "Value is too nested");
    Ok(())
}
