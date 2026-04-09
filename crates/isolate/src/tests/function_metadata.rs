use common::assert_obj;
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use value::assert_val;

use crate::{
    test_helpers::{
        UdfTest,
        UdfTestType,
    },
    tests::http_action::{
        http_action_udf_test,
        http_request,
    },
};

#[convex_macro::test_runtime]
async fn test_function_metadata_from_query(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        let meta = t
            .query("functionMetadata:metadataFromQuery", assert_obj!())
            .await?;
        assert_eq!(
            meta,
            assert_val!({
                "name" => "functionMetadata:metadataFromQuery",
                "componentPath" => "",
                "type" => "query",
                "visibility" => "public",
            })
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_function_metadata_from_action(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        let meta = t
            .action("functionMetadata:metadataFromAction", assert_obj!())
            .await?;
        assert_eq!(
            meta,
            assert_val!({
                "name" => "functionMetadata:metadataFromAction",
                "componentPath" => "",
                "type" => "action",
                "visibility" => "public",
            })
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_function_metadata_from_http_action(rt: TestRuntime) -> anyhow::Result<()> {
    let t = http_action_udf_test(rt).await?;
    let response = t
        .http_action("http_action", http_request("metadata"), Identity::system())
        .await?;
    must_let!(let Some(body) = response.body().clone());
    let meta: JsonValue = serde_json::from_slice(&body)?;
    assert_eq!(
        meta,
        json!({
            "name": "http_action",
            "componentPath": "",
            "type": "action",
            "visibility": "public",
        })
    );
    Ok(())
}
