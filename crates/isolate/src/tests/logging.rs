use common::{
    assert_obj,
    testing::assert_contains,
};
use itertools::Itertools;
use regex::Regex;
use runtime::testing::TestRuntime;

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
                .map(|l| l.to_pretty_string())
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
        let line = log_lines.first().unwrap().clone().to_pretty_string();
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

#[convex_macro::test_runtime]
async fn test_console_trace(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let log_lines = t
            .query_log_lines("logging:consoleTrace", assert_obj!())
            .await?;

        let line = log_lines.first().unwrap().clone().to_pretty_string();
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
        let line = log_lines.first().unwrap().clone().to_pretty_string();
        let regex = Regex::new(r"^\[INFO\] default: (\d+)ms$").unwrap();
        assert!(regex.is_match(&line));

        let line = log_lines.get(1).unwrap().clone().to_pretty_string();
        assert!(regex.is_match(&line));

        let line = log_lines.get(2).unwrap().clone().to_pretty_string();
        assert_eq!(&line, "[WARN] Timer 'foo' already exists");

        let line = log_lines.get(3).unwrap().clone().to_pretty_string();
        let regex = Regex::new(r"^\[INFO\] foo: (\d+)ms 'bar' 'baz'$").unwrap();
        assert!(regex.is_match(&line));

        let line = log_lines.get(4).unwrap().clone().to_pretty_string();
        let regex = Regex::new(r"^\[INFO\] foo: (\d+)ms$").unwrap();
        assert!(regex.is_match(&line));
        Ok(())
    })
    .await
}
