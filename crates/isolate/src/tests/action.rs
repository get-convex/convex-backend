use std::time::Duration;

use common::{
    testing::TestPersistence,
    version::Version,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    ConvexValue,
};

use crate::{
    concurrency_limiter::ConcurrencyLimiter,
    test_helpers::{
        UdfTest,
        UdfTestConfig,
    },
    tests::assert_contains,
    IsolateConfig,
};

pub(crate) async fn action_udf_test(
    rt: TestRuntime,
) -> anyhow::Result<UdfTest<TestRuntime, TestPersistence>> {
    UdfTest::default_with_config(
        UdfTestConfig {
            isolate_config: IsolateConfig::new("action_test", ConcurrencyLimiter::unlimited()),
            udf_server_version: Version::parse("1000.0.0")?,
        },
        // we need at least 2 threads since actions will request and block
        // on the execution of other UDFs
        2,
        rt,
    )
    .await
}

#[convex_macro::test_runtime]
async fn test_action_env_var(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;

    let v = t.action("action:getCloudUrl", assert_obj!()).await?;
    assert_eq!(v, ConvexValue::try_from("https://carnitas.convex.cloud")?);

    let v = t.action("action:getSiteUrl", assert_obj!()).await?;
    assert_eq!(v, ConvexValue::try_from("https://carnitas.convex.site")?);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_running_other_udfs(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;
    must_let!(let ConvexValue::Float64(v) = t.action("action:insertObject", assert_obj!()).await?);
    assert!(v == 1.0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_time_out(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::with_timeout(rt, Some(Duration::from_secs(1))).await?;
    let e = t
        .action_js_error("action:sleep", assert_obj!("ms" => 1200.0))
        .await?;
    assert_contains(&e, "Function execution timed out");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_almost_time_out(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::with_timeout(rt, Some(Duration::from_secs(1))).await?;
    let mut log_lines = t
        .action_log_lines("action:sleep", assert_obj!("ms" => 900.0))
        .await?;
    let last_line = log_lines.pop().unwrap();
    assert_contains(
        &last_line.to_pretty_string(),
        "[WARN] Function execution took a long time",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_occ(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(
        UdfTestConfig {
            isolate_config: IsolateConfig::new("action_test", ConcurrencyLimiter::unlimited()),
            udf_server_version: Version::parse("1000.0.0")?,
        },
        // We run one parent action and 16 child actions => 17 threads.
        17,
        rt,
    )
    .await?;
    let e = t.action_js_error("action:occAction", assert_obj!()).await?;
    assert_contains(
        &e,
        "Documents read from or written to the \"objects\" table changed while this mutation",
    );
    assert_contains(
        &e,
        "Another call to this mutation changed the document with ID \"",
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_inner_call_fails_with_system_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;
    let e = t
        .action_js_error("action:innerSystemErrorAction", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Your request couldn't be completed. Try again later.",
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_fails_with_system_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;
    let e = t
        .action("action:systemErrorAction", assert_obj!())
        .await
        .unwrap_err();
    assert_contains(&e, "I can't go for that");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_inner_call_fails_with_uncatcatchable_developer_error(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;
    let e = t
        .action_js_error("action:innerUncatchableDeveloperErrorAction", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Unknown JS syscall: idonotexistandicannotlie",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_fails_with_uncatcatchable_developer_error(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;
    let e = t
        .action_js_error("action:uncatchableDeveloperErrorAction", assert_obj!())
        .await?;
    assert_contains(&e, "Unknown JS syscall: idonotexistandicannotlie");
    Ok(())
}
