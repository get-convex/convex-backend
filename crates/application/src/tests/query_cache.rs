use std::time::Duration;

use common::{
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        PublicFunctionPath,
    },
    pause::PauseController,
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
use value::{
    val,
    ConvexValue,
};

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

fn udf_path(path: &str) -> PublicFunctionPath {
    PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: path.parse().unwrap(),
    })
}

async fn run_query_with_journal(
    application: &Application<TestRuntime>,
    path: &str,
    arg: JsonValue,
    identity: Identity,
    journal: Option<Option<String>>,
    expect_cached: bool,
) -> anyhow::Result<(ConvexValue, Option<String>)> {
    let ts = application.now_ts_for_reads();
    let result = application
        .read_only_udf_at_ts(
            RequestId::new(),
            udf_path(path),
            vec![arg],
            identity,
            *ts,
            journal,
            FunctionCaller::Action {
                parent_scheduled_job: None,
                parent_execution_id: None,
            },
        )
        .await?;
    let (function_log, _) = application.function_log().stream(0.0).await;
    let last_log_entry = function_log.last().unwrap();
    assert_eq!(last_log_entry.cached_result, expect_cached);
    Ok((result.result?.unpack()?, result.journal))
}

async fn run_query(
    application: &Application<TestRuntime>,
    path: &str,
    arg: JsonValue,
    identity: Identity,
    expect_cached: bool,
) -> anyhow::Result<ConvexValue> {
    Ok(
        run_query_with_journal(application, path, arg, identity, None, expect_cached)
            .await?
            .0,
    )
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
                parent_execution_id: None,
            },
            None,
        )
        .await??;
    result.value.unpack()
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
async fn test_query_cache_error_not_cached(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Run query that throws an error first time
    let error1 = run_query(
        &application,
        "custom_errors:queryThrows",
        json!({}),
        Identity::system(),
        false,
    )
    .await;
    assert!(error1.is_err(), "Expected first query to throw an error");

    // Run same query again - should throw error again and not be cached
    let error2 = run_query(
        &application,
        "custom_errors:queryThrows",
        json!({}),
        Identity::system(),
        false,
    )
    .await;
    assert!(error2.is_err(), "Expected second query to throw an error");

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
async fn test_query_cache_unauthed_race(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Run the same query as different users, in parallel.
    // In this case we don't know that the query doesn't check auth, so
    // neither request waits for the other.
    // To make sure they run in parallel, run each query up until they try
    // to run a function, which is after they have checked the cache and decided
    // that it's a cache miss.
    let mut first_query = Box::pin(run_query(
        &application,
        "basic:listAllObjects",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    ));
    let first_hold_guard = pause_controller.hold("run_function");
    let first_pause_guard = tokio::select! {
        _ = &mut first_query => {
            panic!("First query completed before pause");
        }
        pause_guard = first_hold_guard.wait_for_blocked() => {
            pause_guard.unwrap()
        }
    };
    let second_hold_guard = pause_controller.hold("run_function");
    let mut second_query = Box::pin(run_query(
        &application,
        "basic:listAllObjects",
        json!({}),
        Identity::system(),
        false,
    ));
    let second_pause_guard = tokio::select! {
        _ = &mut second_query => {
            panic!("Second query completed before pause");
        }
        pause_guard = second_hold_guard.wait_for_blocked() => {
            pause_guard.unwrap()
        }
    };
    first_pause_guard.unpause();
    first_query.await?;
    second_pause_guard.unpause();
    second_query.await?;

    // Insert an object to invalidate the cache.
    // Then run both queries again in parallel.
    // In this case we can guess that the query doesn't check auth, so
    // the second request should wait for the first and use the cached value.
    insert_object(&application).await?;

    // Rerun queries in parallel
    let mut first_query = Box::pin(run_query(
        &application,
        "basic:listAllObjects",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    ));
    let first_hold_guard = pause_controller.hold("run_function");
    let first_pause_guard = tokio::select! {
        _ = &mut first_query => {
            panic!("First query completed before pause");
        }
        pause_guard = first_hold_guard.wait_for_blocked() => {
            pause_guard.unwrap()
        }
    };
    let mut second_query = Box::pin(run_query(
        &application,
        "basic:listAllObjects",
        json!({}),
        Identity::system(),
        true, // cache hit
    ));
    // The second one never gets to run_function, so use perform_cache_op instead
    // to pause it when it's waiting for the first query to finish.
    let second_hold_guard = pause_controller.hold("perform_cache_op");
    let second_pause_guard = tokio::select! {
        _ = &mut second_query => {
            panic!("Second query completed before pause");
        }
        pause_guard = second_hold_guard.wait_for_blocked() => {
            pause_guard.unwrap()
        }
    };
    first_pause_guard.unpause();
    first_query.await?;
    second_pause_guard.unpause();
    second_query.await?;

    Ok(())
}

async fn test_query_cache_with_conditional_auth_check_inner(
    rt: TestRuntime,
    subquery: bool,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let query = if subquery {
        "auth:conditionallyCheckAuthInSubquery"
    } else {
        "auth:conditionallyCheckAuth"
    };

    // First run query that doesn't check auth
    let result1 = run_query(
        &application,
        query,
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;
    rt.advance_time(Duration::from_secs(1)).await;
    // The query is cached across identities.
    let result2 = run_query(&application, query, json!({}), Identity::system(), true).await?;
    assert_eq!(result1, result2);

    insert_object(&application).await?;

    // Now that there's an object, the query checks auth.
    let result3 = run_query(
        &application,
        query,
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;
    let result4 = run_query(&application, query, json!({}), Identity::system(), false).await?;
    assert_ne!(result1, result3);
    assert_ne!(result1, result4);
    assert_ne!(result3, result4); // different auth

    Ok(())
}
#[convex_macro::test_runtime]
async fn test_query_cache_with_conditional_auth_check(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_cache_with_conditional_auth_check_inner(rt, false).await
}
#[convex_macro::test_runtime]
async fn test_query_cache_with_conditional_auth_check_in_subquery(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    test_query_cache_with_conditional_auth_check_inner(rt, true).await
}

#[convex_macro::test_runtime]
async fn test_query_cache_conditional_auth_check_race(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    // Run query as one user.
    run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    )
    .await?;

    // Insert an object to make the query check auth.
    // Then run two queries in parallel, for different users.
    // We guess that the queries don't check auth, so one waits for the other.
    // But the queries actually do check auth, so the second one re-executes
    // after waiting.
    insert_object(&application).await?;

    // Run queries in parallel
    let mut first_query = Box::pin(run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::user(UserIdentity::test()),
        false,
    ));
    let first_hold_guard = pause_controller.hold("run_function");
    let first_pause_guard = tokio::select! {
        _ = &mut first_query => {
            panic!("First query completed before pause");
        }
        pause_guard = first_hold_guard.wait_for_blocked() => {
            pause_guard.unwrap()
        }
    };
    let mut second_query = Box::pin(run_query(
        &application,
        "auth:conditionallyCheckAuth",
        json!({}),
        Identity::system(),
        false, // cache miss
    ));
    // Pause the second query when it's waiting for the first query to finish.
    let second_hold_guard = pause_controller.hold("perform_cache_op");
    let second_pause_guard = tokio::select! {
        _ = &mut second_query => {
            panic!("Second query completed before pause");
        }
        pause_guard = second_hold_guard.wait_for_blocked() => {
            pause_guard.unwrap()
        }
    };
    first_pause_guard.unpause();
    let result1 = first_query.await?;
    assert_eq!(result1, val!("https://testauth.fake.domain|testauth|123"));
    second_pause_guard.unpause();
    let result2 = second_query.await?;
    assert_eq!(result2, val!("No user"));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_cache_paginated_query(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    application
        .add_index(IndexMetadata::new_enabled(
            "test.by_hello".parse()?,
            IndexedFields::try_from(vec!["hello".parse()?])?,
        ))
        .await?;

    for i in 1..=5 {
        assert!(application
            .mutation_udf(
                RequestId::new(),
                udf_path("query:insert"),
                vec![json!({"number": i * 10})],
                Identity::system(),
                None,
                FunctionCaller::Test,
                None
            )
            .await?
            .is_ok());
    }

    let (page1_1, journal1_1) = run_query_with_journal(
        &application,
        "query:paginateIndex",
        json!({ "paginationOpts": { "numItems": 2, "cursor": null } }),
        Identity::system(),
        None,
        false,
    )
    .await?;
    assert!(journal1_1.is_some());
    assert_eq!(page1_1["page"][0]["hello"], ConvexValue::Float64(10.0));
    assert_eq!(page1_1["page"][1]["hello"], ConvexValue::Float64(20.0));

    // Rerunning the query as-is should yield a cache hit
    let (page1_2, journal1_2) = run_query_with_journal(
        &application,
        "query:paginateIndex",
        json!({ "paginationOpts": { "numItems": 2, "cursor": null } }),
        Identity::system(),
        None,
        true, /* expect_cached */
    )
    .await?;
    assert_eq!(page1_1, page1_2);
    // TODO: consider making journal encoding deterministic,
    // since these journals should be the same.
    assert_ne!(journal1_1, journal1_2);

    // Rerunning the query, but passing in the journal from the first run,
    // should _also_ result in a cache hit
    let (page1_3, journal1_3) = run_query_with_journal(
        &application,
        "query:paginateIndex",
        json!({ "paginationOpts": { "numItems": 2, "cursor": null } }),
        Identity::system(),
        Some(journal1_1.clone()),
        true, /* expect_cached */
    )
    .await?;
    assert_eq!(page1_1, page1_3);
    // TODO: consider making journal encoding deterministic,
    // since these journals should be the same.
    assert_ne!(journal1_1, journal1_3);

    // Load a second page.
    let (page2_1, journal2_1) = run_query_with_journal(
        &application,
        "query:paginateIndex",
        json!({ "paginationOpts": { "numItems": 2, "cursor": page1_1["continueCursor"].to_internal_json() } }),
        Identity::system(),
        None,
        false,
    )
    .await?;
    assert!(journal2_1.is_some());
    assert_eq!(page2_1["page"][0]["hello"], ConvexValue::Float64(30.0));
    assert_eq!(page2_1["page"][1]["hello"], ConvexValue::Float64(40.0));

    // Insert an item into the first page.
    assert!(application
        .mutation_udf(
            RequestId::new(),
            udf_path("query:insert"),
            vec![json!({"number": 15})],
            Identity::system(),
            None,
            FunctionCaller::Test,
            None
        )
        .await?
        .is_ok());

    // Re-query the first page.
    let (page1_4, journal1_4) = run_query_with_journal(
        &application,
        "query:paginateIndex",
        json!({ "paginationOpts": { "numItems": 2, "cursor": null } }),
        Identity::system(),
        Some(journal1_1.clone()),
        false,
    )
    .await?;
    assert!(journal1_4.is_some());
    assert_eq!(page1_4["page"][0]["hello"], ConvexValue::Float64(10.0));
    assert_eq!(page1_4["page"][1]["hello"], ConvexValue::Float64(15.0)); // new!
    assert_eq!(page1_4["page"][2]["hello"], ConvexValue::Float64(20.0));

    // The second page should still be cached.
    let (page2_2, journal2_2) = run_query_with_journal(
        &application,
        "query:paginateIndex",
        json!({ "paginationOpts": { "numItems": 2, "cursor": page1_1["continueCursor"].to_internal_json() } }),
        Identity::system(),
        None,
        true /* expect_cached */,
    )
    .await?;
    assert!(journal2_2.is_some());
    assert_eq!(page2_1, page2_2);

    Ok(())
}
