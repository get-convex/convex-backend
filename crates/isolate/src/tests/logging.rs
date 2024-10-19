use common::{
    assert_obj,
    log_lines::LogLine,
    testing::{
        assert_contains,
        TestPersistence,
    },
};
use itertools::Itertools;
use must_let::must_let;
use regex::Regex;
use runtime::testing::TestRuntime;
use semver::Version;

use crate::{
    test_helpers::{
        UdfTest,
        UdfTestConfig,
        UdfTestType,
    },
    ConcurrencyLimiter,
    IsolateConfig,
};

/// Tests to ensure that our logging is reasonable for basic JS types.
///
/// Feel free to adjust these (the specific strings don't matter), but make sure
/// these all stay readable.

#[convex_macro::test_runtime]
async fn test_log_string(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:logString", assert_obj!())
            .await?;
        assert_eq!(
            vec!["[LOG] 'myString'"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_number(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:logNumber", assert_obj!())
            .await?;
        assert_eq!(
            vec!["[LOG] 42"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_undefined(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:logUndefined", assert_obj!())
            .await?;
        assert_eq!(
            vec!["[LOG] undefined"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_null(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t.query_log_lines("logging:logNull", assert_obj!()).await?;
        assert_eq!(
            vec!["[LOG] null"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_function(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:logFunction", assert_obj!())
            .await?;
        assert_eq!(
            vec!["[LOG] [Function: myFunction]"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_instance(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:logInstance", assert_obj!())
            .await?;
        assert_eq!(
            vec!["[LOG] MyClass {}"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_object(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:logObject", assert_obj!())
            .await?;
        assert_eq!(
            vec!["[LOG] {\n  property: 'value',\n  nested_object: {}\n}"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_array(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t.query_log_lines("logging:logArray", assert_obj!()).await?;
        assert_eq!(
            vec!["[LOG] [ 'string', 42 ]"],
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect_vec()
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_log_document(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .mutation_log_lines("logging:logDocument", assert_obj!())
            .await?;
        let line = log_lines.first().unwrap().clone().to_pretty_string_test_only();
        let pattern = Regex::new(
            r#"\[LOG\] \{\n  _creationTime: \d+,\n  _id: '[0-9A-Za-z-_]+',\n  property: 'value'\n\}"#,
        )?;

        assert!(
            pattern.is_match(&line),
            "Log line didn't match pattern {}",
            line
        );
        Ok(())
    }).await
}

pub(crate) async fn nested_function_udf_test(
    rt: TestRuntime,
) -> anyhow::Result<UdfTest<TestRuntime, TestPersistence>> {
    UdfTest::default_with_config(
        UdfTestConfig {
            isolate_config: IsolateConfig::new(
                "nested_function_test",
                ConcurrencyLimiter::unlimited(),
            ),
            udf_server_version: Version::parse("1000.0.0")?,
        },
        // we need at least 2 threads since functions will request and block
        // on the execution of other UDFs
        2,
        rt,
    )
    .await
}

#[convex_macro::test_runtime]
async fn test_log_from_subfunction(rt: TestRuntime) -> anyhow::Result<()> {
    let t = nested_function_udf_test(rt).await?;
    let log_lines = t
        .query_log_lines("logging:logFromSubfunction", assert_obj!())
        .await?;
    let mut log_lines_iter = log_lines.into_iter();
    must_let!(let Some(first_log_line) = log_lines_iter.next());
    assert_eq!(
        "[LOG] 'from parent'",
        first_log_line.to_pretty_string_test_only()
    );
    must_let!(let Some(second_log_line) = log_lines_iter.next());
    must_let!(let LogLine::SubFunction { path, log_lines } = second_log_line);
    assert_eq!(&*path.component.to_string(), "");
    assert_eq!(&*path.udf_path.to_string(), "logging.js:logString");
    assert_eq!(
        vec!["[LOG] 'myString'"],
        log_lines
            .into_iter()
            .map(|l| l.to_pretty_string_test_only())
            .collect_vec()
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_console_trace(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:consoleTrace", assert_obj!())
            .await?;

        let line = log_lines
            .first()
            .unwrap()
            .clone()
            .to_pretty_string_test_only();
        assert!(line.starts_with("[LOG] 'myString' \n"));
        // Has the original function somewhere in the stack trace
        assert_contains(&line, "convex/logging.ts:");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_error_stack(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.query("logging:errorStack", assert_obj!()).await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_console_time(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:consoleTime", assert_obj!())
            .await?;
        assert_eq!(log_lines.len(), 5);
        let line = log_lines
            .first()
            .unwrap()
            .clone()
            .to_pretty_string_test_only();
        let regex = Regex::new(r"^\[INFO\] default: (\d+)ms$").unwrap();
        assert!(regex.is_match(&line));

        let line = log_lines
            .get(1)
            .unwrap()
            .clone()
            .to_pretty_string_test_only();
        assert!(regex.is_match(&line));

        let line = log_lines
            .get(2)
            .unwrap()
            .clone()
            .to_pretty_string_test_only();
        assert_eq!(&line, "[WARN] Timer 'foo' already exists");

        let line = log_lines
            .get(3)
            .unwrap()
            .clone()
            .to_pretty_string_test_only();
        let regex = Regex::new(r"^\[INFO\] foo: (\d+)ms 'bar' 'baz'$").unwrap();
        assert!(regex.is_match(&line));

        let line = log_lines
            .get(4)
            .unwrap()
            .clone()
            .to_pretty_string_test_only();
        let regex = Regex::new(r"^\[INFO\] foo: (\d+)ms$").unwrap();
        assert!(regex.is_match(&line));
        Ok(())
    })
    .await
}
