use std::{
    str::FromStr,
    sync::LazyLock,
};

use common::{
    assert_obj,
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        PublicFunctionPath,
    },
    log_lines::TRUNCATED_LINE_SUFFIX,
    testing::assert_contains,
    types::{
        AllowedVisibility,
        UdfType,
    },
    value::{
        ConvexArray,
        ConvexValue,
    },
    version::Version,
};
use database::SystemMetadataModel;
use keybroker::Identity;
use model::udf_config::UdfConfigModel;
use must_let::must_let;
use runtime::{
    prod::ProdRuntime,
    testing::TestRuntime,
};
use sync_types::CanonicalizedUdfPath;
use udf::validation::ValidatedPathAndArgs;
use value::{
    assert_val,
    ConvexBytes,
    TableNamespace,
};

use crate::{
    concurrency_limiter::ConcurrencyLimiter,
    environment::helpers::MAX_LOG_LINES,
    test_helpers::{
        UdfTest,
        UdfTestConfig,
    },
    tests::logging::nested_function_udf_test,
    IsolateConfig,
};

static MAX_ISOLATE_WORKERS: usize = 1;

static TIMEOUT_CONFIG: LazyLock<UdfTestConfig> = LazyLock::new(|| UdfTestConfig {
    isolate_config: IsolateConfig::new("timeout_test", ConcurrencyLimiter::unlimited()),
    udf_server_version: Version::parse("1000.0.0").unwrap(),
});

#[convex_macro::prod_rt_test]
async fn test_time_out(rt: ProdRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(TIMEOUT_CONFIG.clone(), MAX_ISOLATE_WORKERS, rt.clone())
        .await?;
    let e = t
        .query_js_error("adversarial:simpleLoop", assert_obj!())
        .await?;
    assert_contains(&e, "Function execution timed out");
    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_wasm_simple_loop(rt: ProdRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(TIMEOUT_CONFIG.clone(), MAX_ISOLATE_WORKERS, rt.clone())
        .await?;
    let e = t
        .query_js_error("adversarialWasm:simpleLoop", assert_obj!())
        .await?;
    assert_contains(&e, "Function execution timed out");
    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_wasm_allocating_loop(rt: ProdRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(TIMEOUT_CONFIG.clone(), MAX_ISOLATE_WORKERS, rt.clone())
        .await?;
    let e = t
        .query_js_error("adversarialWasm:allocatingLoop", assert_obj!())
        .await?;
    assert_contains(&e, "Function execution timed out");
    Ok(())
}

// We don't allow setTimeout in queries, so it's not possible to get
// a query to execute for a precise amount of time. This makes this
// test flaky:
#[ignore]
#[convex_macro::prod_rt_test]
async fn test_almost_time_out(rt: ProdRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(TIMEOUT_CONFIG.clone(), MAX_ISOLATE_WORKERS, rt.clone())
        .await?;
    let mut log_lines = t.query_log_lines("adversarial:slow", assert_obj!()).await?;
    assert_contains(
        &log_lines.pop().unwrap().to_pretty_string_test_only(),
        "[WARN] Function execution took a long time",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_db_loop(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("adversarial:populate", assert_obj!()).await?;
    let e = t
        .query_js_error("adversarial:dbLoop", assert_obj!())
        .await?;
    assert_contains(&e, "Too many documents read");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_read_too_many_documents(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("adversarial:populate", assert_obj!()).await?;
    let e = t
        .query_js_error("adversarial:queryLeak", assert_obj!())
        .await?;
    assert_contains(&e, "Too many documents read in a single function execution");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_read_many_documents(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("adversarial:populate", assert_obj!()).await?;
    let mut log_lines = t
        .query_log_lines("adversarial:queryATon", assert_obj!())
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Many documents read in a single function execution",
    );
    assert_contains(&last_line, "Consider using smaller limits in your queries");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_reads_too_many(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.add_index(IndexMetadata::new_backfilling(
        *t.database.now_ts_for_reads(),
        "test.by_hello".parse()?,
        IndexedFields::try_from(vec!["hello".parse()?])?,
    ))
    .await?;
    t.backfill_indexes().await?;
    let e = t
        .query_js_error("adversarial:queryTooManyTimes", assert_obj!())
        .await?;
    assert_contains(&e, "Too many reads in a single function execution");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_reads_many(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.add_index(IndexMetadata::new_backfilling(
        *t.database.now_ts_for_reads(),
        "test.by_hello".parse()?,
        IndexedFields::try_from(vec!["hello".parse()?])?,
    ))
    .await?;
    t.backfill_indexes().await?;
    let mut log_lines = t
        .query_log_lines("adversarial:queryManyTimes", assert_obj!())
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Many reads in a single function execution",
    );
    assert_contains(&last_line, "Consider using smaller limits in your queries");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_console_loop(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut log_lines = t
        .query_log_lines("adversarial:consoleLoop", assert_obj!())
        .await?;
    assert_eq!(log_lines.len(), MAX_LOG_LINES);
    assert_contains(
        &log_lines.pop().unwrap().to_pretty_string_test_only(),
        "Log overflow",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_console_loop_from_subfunction(rt: TestRuntime) -> anyhow::Result<()> {
    let t = nested_function_udf_test(rt).await?;
    let log_lines = t
        .query_log_lines("adversarial:consoleLoopFromSubfunction", assert_obj!())
        .await?;
    let log_lines_flat: Vec<_> = log_lines
        .into_iter()
        .flat_map(|log_line| log_line.to_pretty_strings())
        .collect();
    assert_eq!(log_lines_flat.len(), 256);
    let mut expected_log_lines = vec![];
    // From child.
    expected_log_lines.extend(["[LOG] 'are we there yet'"].repeat(200).into_iter());
    // From parent.
    expected_log_lines.push("[LOG] 'we get there when we get there'");
    // From child, truncated.
    expected_log_lines.extend(["[LOG] 'are we there yet'"].repeat(54).into_iter());
    expected_log_lines.push("[ERROR] Log overflow (maximum 256). Remaining log lines omitted.");
    assert_eq!(
        log_lines_flat,
        expected_log_lines
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_console_line_too_long(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let log_lines = t
        .query_log_lines("adversarial:consoleLongLine", assert_obj!())
        .await?;
    assert_contains(
        &log_lines[0].clone().to_pretty_string_test_only(),
        TRUNCATED_LINE_SUFFIX,
    );
    // The limit is 32768 MAX_LOG_LINE_LENGTH, but we don't count the [INFO]
    // prefix or the truncated line suffix, so just check that we're close
    assert!(log_lines[0].clone().to_pretty_string_test_only().len() > 32700);
    assert!(log_lines[0].clone().to_pretty_string_test_only().len() < 32900);

    assert_contains(
        &log_lines[1].clone().to_pretty_string_test_only(),
        TRUNCATED_LINE_SUFFIX,
    );
    // The limit is 32768 MAX_LOG_LINE_LENGTH, but we don't count the [INFO]
    // prefix or the truncated line suffix, so just check that we're close
    assert!(log_lines[1].clone().to_pretty_string_test_only().len() > 32700);
    assert!(log_lines[1].clone().to_pretty_string_test_only().len() < 32900);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_console_line_too_long_char_boundary(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let log_lines = t
        .query_log_lines("adversarial:consoleLongLineCharBoundary", assert_obj!())
        .await?;
    assert_eq!(log_lines.len(), 4);

    assert_contains(
        &log_lines[0].clone().to_pretty_string_test_only(),
        TRUNCATED_LINE_SUFFIX,
    );
    assert_contains(
        &log_lines[1].clone().to_pretty_string_test_only(),
        TRUNCATED_LINE_SUFFIX,
    );
    assert_contains(
        &log_lines[2].clone().to_pretty_string_test_only(),
        TRUNCATED_LINE_SUFFIX,
    );
    assert_contains(
        &log_lines[3].clone().to_pretty_string_test_only(),
        TRUNCATED_LINE_SUFFIX,
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_unsupported_apis(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.query("adversarial:tryUnsupportedAPIs", assert_obj!())
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_big_read(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let ids = t
        .mutation("adversarial:populateBigRead", assert_obj!())
        .await?;
    let e = t
        .query_js_error("adversarial:bigRead", assert_obj!("ids" => ids))
        .await?;
    assert_contains(&e, "Too many reads");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_big_return(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("adversarial:returnTooLarge", assert_obj!())
        .await?;
    assert_contains(&e, "Array length is too long");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_iterate_twice(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("adversarial:iterateTwice", assert_obj!())
        .await?;
    assert_contains(&e, "This query is closed and can't emit any more values.");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_iterate_consumed(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("adversarial:iterateConsumed", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "This query has been chained with another operator and can't be reused.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_reads_too_large(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // 16 documents per write * 256KiB per document * 5 writes = 20 MiB, which is
    // higher that the limit on reads.
    let count_per_write = 16.0;
    let mut ids: Vec<ConvexValue> = vec![];
    for _ in 0..5 {
        let more_ids = t
            .mutation(
                "adversarial:bigWrite",
                assert_obj!("count" => count_per_write),
            )
            .await?;
        must_let!(let ConvexValue::Array(more_ids) = more_ids);
        ids.extend(Vec::<ConvexValue>::from(more_ids));
    }
    let e = t
        .query_js_error("adversarial:bigRead", assert_obj!("ids" => ids.clone()))
        .await?;
    assert_contains(&e, "Too many bytes read in a single function execution");

    let ret_val = t.query("adversarial:readUntilError", assert_obj!()).await?;
    must_let!(let ConvexValue::Array(ret_val) = ret_val);
    assert!(!ret_val.is_empty());
    assert!(ret_val.len() < ids.len());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_reads_large(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // 6 documents per write * 256KiB per document * 5 writes = 7 MiB, which is
    // close to the limit on reads.
    let count_per_write = 6.0;
    let mut ids: Vec<ConvexValue> = vec![];
    for _ in 0..5 {
        let more_ids = t
            .mutation(
                "adversarial:bigWrite",
                assert_obj!("count" => count_per_write),
            )
            .await?;
        must_let!(let ConvexValue::Array(more_ids) = more_ids);
        ids.extend(Vec::<ConvexValue>::from(more_ids));
    }

    let mut log_lines = t
        .query_log_lines("adversarial:bigRead", assert_obj!("ids" => ids.clone()))
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Many bytes read in a single function execution",
    );
    assert_contains(&last_line, "Consider using smaller limits in your queries");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let count = 64.0;
    let e = t
        .mutation_js_error("adversarial:bigWrite", assert_obj!("count" => count))
        .await?;
    assert_contains(&e, "Too many bytes written in a single function execution");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let count = 30.0;
    let mut log_lines = t
        .mutation_log_lines("adversarial:bigWrite", assert_obj!("count" => count))
        .await?;

    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Many bytes written in a single function execution",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_big_document(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (id, outcome) = t
        .mutation_outcome("adversarial:bigDocument", assert_obj!(), Identity::system())
        .await?;
    let last_line = outcome
        .log_lines
        .last()
        .unwrap()
        .clone()
        .to_pretty_string_test_only();
    assert_contains(
        &last_line,
        &format!("[WARN] Large document written with ID {id}"),
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_nested_document(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let (id, outcome) = t
        .mutation_outcome(
            "adversarial:nestedDocument",
            assert_obj!(),
            Identity::system(),
        )
        .await?;
    let last_line = outcome
        .log_lines
        .last()
        .unwrap()
        .clone()
        .to_pretty_string_test_only();
    assert_contains(
        &last_line,
        &format!("[WARN] Deeply nested document written with ID {id}"),
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_oom(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t.query_js_error("adversarial:oom", assert_obj!()).await?;
    assert_contains(&e, "JavaScript execution ran out of memory");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_too_many(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .mutation_js_error("adversarial:tooManyWrites", assert_obj!())
        .await?;
    assert_contains(&e, "Too many writes in a single function execution");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_many(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut log_lines = t
        .mutation_log_lines("adversarial:manyWrites", assert_obj!())
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Many writes in a single function execution",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_args_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: ConvexValue = ConvexValue::Bytes(ConvexBytes::try_from(vec![0; 9_000_000])?);
    let e = t
        .query_js_error("basic:readTime", assert_obj!("data" => data))
        .await?;
    assert_contains(
        &e,
        "Arguments for basic.js:readTime are too large (actual: 8.58 MiB, limit: 8 MiB)",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_args_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: ConvexValue = ConvexValue::Bytes(ConvexBytes::try_from(vec![0; 9_000_000])?);
    let e = t
        .mutation_js_error("basic:simpleMutation", assert_obj!("data" => data))
        .await?;
    assert_contains(
        &e,
        "Arguments for basic.js:simpleMutation are too large (actual: 8.58 MiB, limit: 8 MiB)",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_args_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: ConvexValue = ConvexValue::Bytes(ConvexBytes::try_from(vec![0; 9_000_000])?);
    let e = t
        .action_js_error("basic:simpleAction", assert_obj!("data" => data))
        .await?;
    assert_contains(
        &e,
        "Arguments for basic.js:simpleAction are too large (actual: 8.58 MiB, limit: 8 MiB)",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_args_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: ConvexValue = ConvexValue::Bytes(ConvexBytes::try_from(vec![0; 8_000_000])?);
    let mut log_lines = t
        .query_log_lines("basic:readTime", assert_obj!("data" => data))
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Large size of the function arguments (actual: 8000011 bytes, limit: 8388608 \
         bytes).",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_args_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: ConvexValue = ConvexValue::Bytes(ConvexBytes::try_from(vec![0; 8_000_000])?);
    let mut log_lines = t
        .mutation_log_lines("basic:simpleMutation", assert_obj!("data" => data))
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Large size of the function arguments (actual: 8000011 bytes, limit: 8388608 \
         bytes).",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_args_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let data: ConvexValue = ConvexValue::Bytes(ConvexBytes::try_from(vec![0; 8_000_000])?);
    let mut log_lines = t
        .action_log_lines("basic:simpleAction", assert_obj!("data" => data))
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Large size of the action arguments (actual: 8000011 bytes, limit: 8388608 bytes).",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_result_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error(
            "adversarial:queryResultSized",
            assert_obj!("size" => 9_000_000.0),
        )
        .await?;
    assert_contains(
        &e,
        "Function adversarial.js:queryResultSized return value is too large (actual: 8.58 MiB, \
         limit: 8 MiB)",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_result_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .mutation_js_error(
            "adversarial:mutationResultSized",
            assert_obj!("size" => 9_000_000.0),
        )
        .await?;
    assert_contains(
        &e,
        "Function adversarial.js:mutationResultSized return value is too large (actual: 8.58 MiB, \
         limit: 8 MiB)",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_result_too_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .action_js_error(
            "adversarial:actionResultSized",
            assert_obj!("size" => 9_000_000.0),
        )
        .await?;
    assert_contains(
        &e,
        "Function adversarial.js:actionResultSized return value is too large (actual: 8.58 MiB, \
         limit: 8 MiB)",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_result_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut log_lines = t
        .query_log_lines(
            "adversarial:queryResultSized",
            assert_obj!("size" => 8_000_000.0),
        )
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Large size of the function return value (actual: 8000002 bytes, limit: 8388608 \
         bytes).",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_result_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut log_lines = t
        .mutation_log_lines(
            "adversarial:mutationResultSized",
            assert_obj!("size" => 8_000_000.0),
        )
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Large size of the function return value (actual: 8000002 bytes, limit: 8388608 \
         bytes).",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_result_big(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut log_lines = t
        .action_log_lines(
            "adversarial:actionResultSized",
            assert_obj!("size" => 8_000_000.0),
        )
        .await?;
    let last_line = log_lines.pop().unwrap().to_pretty_string_test_only();
    assert_contains(
        &last_line,
        "[WARN] Large size of the action return value (actual: 8000002 bytes, limit: 8388608 \
         bytes).",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_no_eval(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // TODO: Reenable this test.
    // let e = t.transaction("adversarial:tryEval", assert_obj!()).unwrap_err();
    // assert!(format!("{e}").contains("Code generation from strings disallowed for
    // this context"));
    let e = t
        .mutation_js_error("adversarial:tryNewFunction", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Code generation from strings disallowed for this context",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_convex_global(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("adversarial:deleteConvexGlobal", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "The Convex database and auth objects are being used outside of a Convex backend.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_throw_system_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_outcome(
            "adversarial:throwSystemError",
            assert_obj!(),
            Identity::system(),
        )
        .await
        .unwrap_err();
    assert_contains(&e, "I can't go for that");

    // Check that system errors work after an `.await` -- the code after the `await`
    // runs in the microtask queue rather than the direct call stack entered by
    // `Isolate::run`.
    let e = t
        .query_outcome(
            "adversarial:throwSystemErrorAfterAwait",
            assert_obj!(),
            Identity::system(),
        )
        .await
        .unwrap_err();
    assert_contains(&e, "I can't go for that");
    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_slow_syscall(rt: ProdRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(TIMEOUT_CONFIG.clone(), MAX_ISOLATE_WORKERS, rt.clone())
        .await?;
    let r = t.query("adversarial:slowSyscall", assert_obj!()).await?;
    assert_eq!(r, assert_val!(1017.));
    Ok(())
}

#[convex_macro::prod_rt_test]
#[ignore]
async fn test_really_slow_syscall(rt: ProdRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default_with_config(TIMEOUT_CONFIG.clone(), MAX_ISOLATE_WORKERS, rt.clone())
        .await?;
    let e = t
        .query("adversarial:reallySlowSyscall", assert_obj!())
        .await
        .unwrap_err();
    assert_contains(&e, "Hit maximum total syscall duration");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_path_within_deps(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error_no_validation("_deps/nonexistent", assert_obj!())
        .await
        .unwrap_err();
    assert_contains(
        &e,
        "Refusing to run _deps/nonexistent.js:default within the '_deps' directory",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_udf_type_mismatch(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("adversarial:populate", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Trying to execute adversarial.js:populate as Query, but it is defined as Mutation.",
    );

    let e = t
        .mutation_js_error("adversarial:simpleLoop", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Trying to execute adversarial.js:simpleLoop as Mutation, but it is defined as Query.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_create_system_field(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .mutation_js_error("basic:insertObject", assert_obj!("_systemField" => 0))
        .await?;
    assert_contains(
        &e,
        "Field '_systemField' starts with an underscore, which is only allowed for system fields \
         like '_id'",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_atomics_wait(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("adversarial:atomicsWait", assert_obj!())
        .await?;
    assert_contains(&e, "Atomics.wait cannot be called in this context");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_big_memory_usage(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.query("adversarial:bigMemoryUsage", assert_obj!()).await?;
    // Regression test. Previously this would error on the second query with
    // "Possible memory leak: not enough room for user heap"
    t.query("adversarial:bigMemoryUsage", assert_obj!()).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_not_implemented_builtin(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let not_implemented_error = t
        .query_js_error("adversarial:useNotImplementedBuiltin", assert_obj!())
        .await?;
    assert_contains(&not_implemented_error, "Not implemented");
    // Has the original function somewhere in the stack trace
    assert_contains(&not_implemented_error, "convex/adversarial.ts:");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_stubbed_unsupported_apis(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;

    let not_implemented_error = t
        .query_js_error("adversarial:setTimeoutThrows", assert_obj!())
        .await?;
    assert_contains(
        &not_implemented_error,
        "Can't use setTimeout in queries and mutations. Please consider using an action",
    );

    let not_implemented_error = t
        .query_js_error("adversarial:setIntervalThrows", assert_obj!())
        .await?;
    assert_contains(
        &not_implemented_error,
        "Can't use setInterval in queries and mutations. Please consider using an action",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_never_pushed(rt: TestRuntime) -> anyhow::Result<()> {
    // Test that if you've never pushed functions, we generate an appropriate
    // error message telling you to push.
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;

    // Delete the UDF config to simulate it never having existed.
    let mut udf_config_model = UdfConfigModel::new(&mut tx, TableNamespace::test_user());
    must_let!(let Some(config) = udf_config_model.get().await?);
    SystemMetadataModel::new(&mut tx, TableNamespace::test_user())
        .delete(config.id())
        .await?;

    let path = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: CanonicalizedUdfPath::from_str("myFunc.js:default")?,
    };
    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(path),
        ConvexArray::empty(),
        UdfType::Mutation,
    )
    .await?;
    must_let!(let Err(js_error) = result);
    assert_contains(&js_error, "Could not find public function for 'myFunc'");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_invoke_function_directly(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut outcome = t
        .raw_query(
            "adversarial:invokeFunctionDirectly",
            vec![assert_val!({})],
            Identity::system(),
            None,
        )
        .await?;
    let log_line = outcome
        .log_lines
        .pop()
        .unwrap()
        .to_pretty_string_test_only();
    assert_contains(
        &log_line,
        "[WARN] 'Convex functions should not directly call other Convex functions. Consider \
         calling a helper function instead.",
    );
    Ok(())
}
