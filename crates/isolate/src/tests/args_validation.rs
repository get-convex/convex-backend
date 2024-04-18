use common::{
    errors::JsError,
    testing::TestPersistence,
};
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    ConvexValue,
};

use super::assert_contains;
use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

#[convex_macro::test_runtime]
async fn test_incorrect_arg(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .query_js_error("args_validation:stringArg", assert_obj!("arg" => 123))
            .await?;
        assert_contains(&e, "ArgumentValidationError");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_missing_arg(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .query_js_error("args_validation:stringArg", assert_obj!())
            .await?;
        assert_contains(&e, "ArgumentValidationError");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_extra_arg(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .query_js_error(
                "args_validation:stringArg",
                assert_obj!(
                    "arg" => "argValue",
                    "extraArg" => "argValue"
                ),
            )
            .await?;
        assert_contains(&e, "ArgumentValidationError");
        Ok(())
    })
    .await
}

async fn query_js_error_args_array(
    t: UdfTest<TestRuntime, TestPersistence>,
    udf_path: &str,
    args: Vec<ConvexValue>,
) -> anyhow::Result<JsError> {
    let outcome = t
        .raw_query(udf_path, args, Identity::system(), None)
        .await?;
    Ok(outcome.result.unwrap_err())
}

#[convex_macro::test_runtime]
async fn test_no_arg(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = query_js_error_args_array(t, "args_validation:stringArg", vec![]).await?;
        assert_contains(&e, "ArgumentValidationError");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_too_many_args(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = query_js_error_args_array(
            t,
            "args_validation:stringArg",
            vec![
                ConvexValue::Object(assert_obj!()),
                ConvexValue::Object(assert_obj!()),
            ],
        )
        .await?;
        assert_contains(&e, "ArgumentValidationError");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_arg_not_an_object(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = query_js_error_args_array(
            t,
            "args_validation:stringArg",
            vec![ConvexValue::String("stringArg".try_into()?)],
        )
        .await?;
        assert_contains(&e, "ArgumentValidationError");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_correct_arg(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(
            let ConvexValue::String(result) = t
                .query(
                    "args_validation:stringArg",
                    assert_obj!("arg" => "argValue"),
                )
                .await?
        );
        assert_eq!(result.to_string(), "argValue");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_record(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let args_obj = assert_obj!(
            "foo" => 0.,
            "bar" => 1.,
            "baz" => 2.,
        );
        must_let!(
            let ConvexValue::Object(result) = t
                .query(
                    "args_validation:recordArg",
                    assert_obj!("arg" => args_obj.clone()),
                )
                .await?
        );
        assert_eq!(result, args_obj);
        Ok(())
    })
    .await
}
