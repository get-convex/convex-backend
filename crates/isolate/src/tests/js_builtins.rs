use std::{
    io::Write,
    path::Path,
    process::{
        Command,
        Stdio,
    },
    time::Duration,
};

use common::{
    assert_obj,
    runtime::Runtime,
    testing::assert_contains,
};
use must_let::must_let;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use value::ConvexValue;

use crate::{
    test_helpers::{
        UdfTest,
        UdfTestConfig,
        UdfTestType,
        DEFAULT_CONFIG,
    },
    ConcurrencyLimiter,
    IsolateConfig,
};

#[convex_macro::test_runtime]
async fn test_url(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/url", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());

        assert_contains(
            &t.query_js_error("js_builtins/url:setHostUnimplemented", assert_obj!())
                .await?,
            "Not implemented: set host",
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_crypto(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/crypto:test", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());

        assert_contains(
            &t.query_js_error("js_builtins/crypto:methodNotImplemented", assert_obj!())
                .await?,
            "Not implemented: wrapKey for SubtleCrypto",
        );
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_crypto_in_action(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.action("js_builtins/crypto:testAction", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_crypto_interop_from_node(rt: TestRuntime) -> anyhow::Result<()> {
    let output = Command::new("npm")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../npm-packages/udf-tests"))
        .args(["-s", "run", "generate_crypto_interop_data"])
        .stderr(Stdio::inherit())
        .output()?
        .exit_ok()?;
    let data = str::from_utf8(&output.stdout)?;
    let t = UdfTest::default_with_config(
        UdfTestConfig {
            isolate_config: IsolateConfig::new_with_max_user_timeout(
                "test",
                Some(Duration::from_secs(30)),
                ConcurrencyLimiter::unlimited(),
            ),
            ..DEFAULT_CONFIG.clone()
        },
        1,
        rt,
    )
    .await?;
    assert_eq!(
        t.query(
            "js_builtins/crypto:consumeInteropData",
            assert_obj! {"json" => data}
        )
        .await?,
        ConvexValue::Null
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_crypto_interop_to_node(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: String = t
        .action("js_builtins/crypto:generateInteropData", assert_obj!())
        .await?
        .try_into()?;
    let mut cmd = Command::new("npm")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../npm-packages/udf-tests"))
        .args(["-s", "run", "consume_crypto_interop_data"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    cmd.stdin.take().unwrap().write_all(data.as_bytes())?;
    let output = cmd.wait_with_output()?.exit_ok()?;
    assert_eq!(str::from_utf8(&output.stdout)?, "ok\n");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_url_search_params(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/urlSearchParams", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_headers(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/headers", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_blob(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/blob", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_file(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/file", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_stream(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/stream", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_request(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/request", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_response(rt: TestRuntime) -> anyhow::Result<()> {
    // TODO: Enable when we implement actions.
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/response", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        must_let!(let ConvexValue::String(r) = t.action("js_builtins/response:responseAction", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_text_encoder(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/textEncoder", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_event(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/event", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_abort_controller(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/abort_controller", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_event_target(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/event_target", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_set_timeout(rt: TestRuntime) -> anyhow::Result<()> {
    // TODO: Enable when we implement actions.
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        let start = t.rt.monotonic_now();
        must_let!(let ConvexValue::String(r) = t.action("js_builtins/setTimeout:sleep", assert_obj!("ms" => 3000.0)).await?);
        assert_eq!(String::from(r), "success".to_string());
        // TestRuntime mocks time so this is deterministic and fast.
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_secs(3));
        assert!(elapsed <= Duration::from_secs(4));

        let e = t
            .action_js_error("js_builtins/setTimeout:setTimeoutThrows", assert_obj!())
            .await?;
        assert_contains(&e, "THROWN WITHIN setTimeout");

        let start = t.rt.monotonic_now();
        t.action("js_builtins/setTimeout:danglingSetTimeout", assert_obj!())
            .await?;
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(1));

        must_let!(let ConvexValue::String(r) = t.action("js_builtins/setTimeout", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_structured_clone(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::String(r) = t.query("js_builtins/structuredClone", assert_obj!()).await?);
        assert_eq!(String::from(r), "success".to_string());

        let e = t.query_js_error("js_builtins/structuredClone:withTransfer", assert_obj!()).await?;
        assert_contains(&e, "structuredClone with transfer not supported");
        Ok(())
    })
    .await
}
