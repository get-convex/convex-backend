#![allow(clippy::float_cmp)]

use common::value::ConvexValue;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::assert_obj;

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error("custom_errors:queryThrows", assert_obj!())
            .await?;
        must_let!(let Some(ConvexValue::Boolean(true)) = js_error.custom_data);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws_async(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error("custom_errors:queryThrowsAsync", assert_obj!())
            .await?;
        must_let!(let Some(ConvexValue::Boolean(true)) = js_error.custom_data);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_mutation_throws(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .mutation_js_error("custom_errors:mutationThrows", assert_obj!())
            .await?;
        must_let!(let Some(ConvexValue::Boolean(true)) = js_error.custom_data);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_mutation_throws_null(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .mutation_js_error("custom_errors:mutationThrowsNull", assert_obj!())
            .await?;
        must_let!(let Some(ConvexValue::Null) = js_error.custom_data);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_mutation_throws_string(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .mutation_js_error("custom_errors:mutationThrowsString", assert_obj!())
            .await?;
        must_let!(let Some(ConvexValue::String(..)) = js_error.custom_data);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_mutation_throws_object(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .mutation_js_error("custom_errors:mutationThrowsObject", assert_obj!())
            .await?;
        must_let!(let Some(ConvexValue::Object(obj)) = js_error.custom_data);
        must_let!(let Some(ConvexValue::String(foo)) = obj.get("foo"));
        assert_eq!(foo.to_string(), "Mike".to_owned());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws_message(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error("custom_errors:queryThrowsMessage", assert_obj!())
            .await?;
        let error_string = format!("{js_error}");
        must_let!(let Some(ConvexValue::String(..)) = js_error.custom_data);
        assert!(error_string.starts_with("Uncaught ConvexError: Hello James"));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws_object_with_message(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error("custom_errors:queryThrowsObjectWithMessage", assert_obj!())
            .await?;
        let error_string = format!("{js_error}");
        must_let!(let Some(ConvexValue::Object(obj)) = js_error.custom_data);
        must_let!(let Some(ConvexValue::String(..)) = obj.get("message"));
        assert!(error_string.starts_with("Uncaught ConvexError: {\"message\":\"Hello James\"}"));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws_custom_subclass(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error("custom_errors:queryThrowsCustomSubclass", assert_obj!())
            .await?;
        let error_string = format!("{js_error}");
        must_let!(let Some(ConvexValue::String(..)) = js_error.custom_data);
        assert!(error_string.starts_with("Uncaught MyFancyError: Hello James"));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws_custom_subclass_with_object(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error(
                "custom_errors:queryThrowsCustomSubclassWithObject",
                assert_obj!(),
            )
            .await?;
        let error_string = format!("{js_error}");
        must_let!(let Some(ConvexValue::Object(obj)) = js_error.custom_data);
        must_let!(let Some(ConvexValue::String(..)) = obj.get("message"));
        must_let!(let Some(ConvexValue::String(..)) = obj.get("code"));
        assert!(error_string.starts_with(
            "Uncaught MyFancyError: {\"message\":\"Hello James\",\"code\":\"bad boy\"}"
        ));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_custom_errors_query_throws_not_custom(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let js_error = t
            .query_js_error("custom_errors:queryThrowsNotCustom", assert_obj!())
            .await?;
        assert_eq!(None, js_error.custom_data);
        Ok(())
    })
    .await
}
