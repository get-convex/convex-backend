use common::{
    assert_obj,
    value::ConvexValue,
};
use keybroker::Identity;
use runtime::testing::TestRuntime;
use serde_json::json;

use crate::test_helpers::UdfTest;

// Note: These tests use UdfTest::default() instead of run_test_with_isolate2
// because isolate2 is being deprecated and doesn't support
// custom_log_attributes.

#[convex_macro::test_runtime]
async fn test_query_with_attributes(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (outcome, _token) = t
        .raw_query(
            "custom_log_attributes:queryWithAttributes",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
            None,
        )
        .await?;

    // Verify the function succeeded
    assert!(
        outcome.result.is_ok(),
        "Function failed with error: {:?}",
        outcome.result.as_ref().err()
    );

    // Verify custom log attributes were set
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert_eq!(attrs.get("user_id"), Some(&json!("user_123")));
    assert_eq!(attrs.get("operation"), Some(&json!("test_query")));
    assert_eq!(attrs.get("count"), Some(&json!(42)));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_with_attributes(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let outcome = t
        .raw_mutation(
            "custom_log_attributes:mutationWithAttributes",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
        )
        .await?;

    // Verify the function succeeded
    assert!(outcome.result.is_ok());

    // Verify custom log attributes were set
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert_eq!(attrs.get("user_id"), Some(&json!("user_456")));
    assert_eq!(attrs.get("action_type"), Some(&json!("create")));
    assert_eq!(attrs.get("success"), Some(&json!(true)));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_with_attributes(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (outcome, _log_lines) = t
        .raw_action(
            "custom_log_attributes:actionWithAttributes",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
        )
        .await?;

    // Verify the function succeeded
    assert!(outcome.result.is_ok());

    // Verify custom log attributes were set
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert_eq!(attrs.get("external_api"), Some(&json!("test_service")));
    assert_eq!(attrs.get("request_id"), Some(&json!("req_789")));
    assert_eq!(attrs.get("latency_ms"), Some(&json!(150)));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multiple_set_calls_merge(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let outcome = t
        .raw_mutation(
            "custom_log_attributes:multipleSetCalls",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
        )
        .await?;

    // Verify the function succeeded
    assert!(outcome.result.is_ok());

    // Verify attributes were merged correctly
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert_eq!(attrs.get("first_key"), Some(&json!("first_value")));
    assert_eq!(attrs.get("second_key"), Some(&json!("second_value")));
    // shared_key should have the value from the second call
    assert_eq!(attrs.get("shared_key"), Some(&json!("overwritten")));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_various_value_types(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (outcome, _token) = t
        .raw_query(
            "custom_log_attributes:variousTypes",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
            None,
        )
        .await?;

    // Verify the function succeeded
    assert!(outcome.result.is_ok());

    // Verify various types are handled correctly
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert_eq!(attrs.get("string_val"), Some(&json!("hello")));
    assert_eq!(attrs.get("number_val"), Some(&json!(123.456)));
    assert_eq!(attrs.get("bool_true"), Some(&json!(true)));
    assert_eq!(attrs.get("bool_false"), Some(&json!(false)));
    assert_eq!(attrs.get("int_val"), Some(&json!(42)));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_empty_attributes(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (outcome, _token) = t
        .raw_query(
            "custom_log_attributes:emptyAttributes",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
            None,
        )
        .await?;

    // Verify the function succeeded
    assert!(outcome.result.is_ok());

    // Empty attributes should result in an empty map (not None)
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert!(attrs.is_empty());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_dot_separated_keys(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (outcome, _token) = t
        .raw_query(
            "custom_log_attributes:dotSeparatedKeys",
            vec![ConvexValue::Object(assert_obj!())],
            Identity::system(),
            None,
        )
        .await?;

    // Verify the function succeeded
    assert!(
        outcome.result.is_ok(),
        "Function failed with error: {:?}",
        outcome.result.as_ref().err()
    );

    // Verify OTel-style dot-separated keys were set correctly
    let attrs = outcome
        .custom_log_attributes
        .expect("custom_log_attributes should be set");
    assert_eq!(attrs.get("http.method"), Some(&json!("POST")));
    assert_eq!(attrs.get("http.status_code"), Some(&json!(200)));
    assert_eq!(attrs.get("user.id"), Some(&json!("user_123")));
    assert_eq!(attrs.get("service.name"), Some(&json!("my_service")));

    Ok(())
}
