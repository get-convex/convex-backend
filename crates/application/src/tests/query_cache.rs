use std::time::Duration;

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        PublicFunctionPath,
    },
    pause::PauseClient,
    types::FunctionCaller,
    RequestId,
};
use keybroker::{
    testing::TestUserIdentity,
    Identity,
    UserIdentity,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use value::ConvexValue;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

async fn run_query(
    application: &Application<TestRuntime>,
    path: &str,
    arg: JsonValue,
    identity: Identity,
    expect_cached: bool,
) -> anyhow::Result<ConvexValue> {
    let result = application
        .read_only_udf(
            RequestId::new(),
            PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: path.parse()?,
            }),
            vec![arg],
            identity,
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
        )
        .await?;
    let (function_log, _) = application.function_log().stream(0.0).await;
    let last_log_entry = function_log.last().unwrap();
    assert_eq!(last_log_entry.cached_result, expect_cached);
    Ok(result.result?)
}

async fn insert_object(application: &Application<TestRuntime>) -> anyhow::Result<ConvexValue> {
    let result = application
        .mutation_udf(
            RequestId::new(),
            PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: "basic:insertObject".parse()?,
            }),
            vec![json!({})],
            Identity::system(),
            None,
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
            PauseClient::new(),
        )
        .await??;
    Ok(result.value)
}

#[convex_macro::test_runtime]
async fn test_query_cache(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let result1 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    rt.advance_time(Duration::from_secs(1)).await;
    // Same query, so it's cached.
    let result2 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::system(),
        true,
    )
    .await?;

    // The query gets the current time, but the result is cached so the results
    // should match.
    // It's a bit weird to be using Date.now() to test this, since we just want
    // to know that the query was cached. The purpose is to assert that the
    // function is not re-executing. If it were re-executing, the Date.now()
    // would be different.
    assert_eq!(result1, result2);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_data_invalidation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let result1 = run_query(
        &application,
        "basic:listAllObjects",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    insert_object(&application).await?;
    let result2 = run_query(
        &application,
        "basic:listAllObjects",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    assert_ne!(result1, result2);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_time_invalidation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let time_result1 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    let do_nothing_result1 = run_query(
        &application,
        "basic:doNothing",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    rt.advance_time(Duration::from_mins(20)).await;

    // Write a new object to bump timestamps.
    // It doesn't have to succeed; it'll still bump the timestamp.
    let _ = insert_object(&application).await;

    // After 20 minutes, the time query is not cached anymore.
    let time_result2 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    assert_ne!(time_result1, time_result2);
    // The doNothing query is still cached.
    let do_nothing_result2 = run_query(
        &application,
        "basic:doNothing",
        json!({}),
        Identity::system(),
        true,
    )
    .await?;
    assert_eq!(do_nothing_result1, do_nothing_result2);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_precise_data_invalidation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let value1 = insert_object(&application).await?;
    must_let!(let ConvexValue::Object(obj1) = value1);
    must_let!(let ConvexValue::String(id1) = obj1.get("_id").unwrap());
    let result1 = run_query(
        &application,
        "basic:getObject",
        json!({"id": **id1}),
        Identity::system(),
        false,
    )
    .await?;
    // Inserting a new object doesn't invalidate the cache for the old object.
    insert_object(&application).await?;
    let result2 = run_query(
        &application,
        "basic:getObject",
        json!({"id": **id1}),
        Identity::system(),
        true,
    )
    .await?;
    assert_eq!(result1, result2);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_arg_invalidation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let result1 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    rt.advance_time(Duration::from_secs(1)).await;
    let result2 = run_query(
        &application,
        "basic:readTimeMs",
        json!({"cacheBuster": 1}),
        Identity::system(),
        false,
    )
    .await?;
    assert_ne!(result1, result2);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_auth_invalidation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Run query as first user
    let result1 = run_query(
        &application,
        "auth:getIdentifier",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;
    // Run query as system
    let result2 = run_query(
        &application,
        "auth:getIdentifier",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    // Results should be different since they're from different auth
    assert_ne!(result1, result2);
    // Run query as first user again
    let result3 = run_query(
        &application,
        "auth:getIdentifier",
        json!({}),
        Identity::user(UserIdentity::test()),
        true,
    )
    .await?;
    // Results should be the same since it's the same user
    assert_eq!(result1, result3);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_without_checking_auth(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let result1 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;
    rt.advance_time(Duration::from_secs(1)).await;
    // The query doesn't read ctx.auth, so it's cached across identities.
    let result2 = run_query(
        &application,
        "basic:readTimeMs",
        json!({}),
        Identity::system(),
        true,
    )
    .await?;
    // Result is the same because the query doesn't re-execute and get a new
    // Date.now().
    assert_eq!(result1, result2);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_cache_with_conditional_auth_check(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // First run query that doesn't check auth
    let result1 = run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;
    rt.advance_time(Duration::from_secs(1)).await;
    // The query is cached across identities.
    let result2 = run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::system(),
        true,
    )
    .await?;
    assert_eq!(result1, result2);

    insert_object(&application).await?;

    // Now that there's an object, the query checks auth.
    let result3 = run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;
    let result4 = run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::system(),
        false,
    )
    .await?;
    assert_ne!(result1, result3);
    assert_ne!(result1, result4);
    assert_ne!(result3, result4); // different auth

    Ok(())
}
