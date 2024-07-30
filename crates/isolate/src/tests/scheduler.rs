#![allow(clippy::float_cmp)]

use std::time::Duration;

use common::{
    assert_obj,
    runtime::Runtime,
    testing::assert_contains,
    value::ConvexValue,
};
use keybroker::Identity;
use model::scheduled_jobs::{
    types::ScheduledJobState,
    virtual_table::PublicScheduledJob,
};
use must_let::must_let;
use rand::RngCore;
use runtime::testing::TestRuntime;

use crate::{
    test_helpers::{
        UdfTest,
        UdfTestType,
    },
    tests::action::action_udf_test,
};

#[convex_macro::test_runtime]
async fn test_schedule_after(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let (_, outcome) = t
            .mutation_outcome(
                "scheduler:scheduleAfter",
                assert_obj!("delayMs" => ConvexValue::Float64(2000.0)),
                Identity::system(),
            )
            .await?;

        let result = t.query("scheduler:getScheduledJobs", assert_obj!()).await?;
        must_let!(let ConvexValue::Array(scheduled_jobs) = result);
        assert_eq!(scheduled_jobs.len(), 1);
        must_let!(let ConvexValue::Object(job_obj) = scheduled_jobs[0].clone());

        let job = PublicScheduledJob::try_from(job_obj)?;
        assert_eq!(job.state, ScheduledJobState::Pending);

        let expected_ts = (outcome.unix_timestamp + Duration::from_secs(2)).as_secs_f64() * 1000.0;
        assert!((job.scheduled_time - expected_ts).abs() < 0.1);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_with_arbitrary_json(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.mutation("scheduler:scheduleWithArbitraryJson", assert_obj!())
            .await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_at_unix_timestamp(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let ts = t.rt.unix_timestamp().as_secs_f64();
        t.mutation(
            "scheduler:scheduleAtTimestamp",
            assert_obj!("ts" => ts * 1000.0),
        )
        .await?;
        let result = t.query("scheduler:getScheduledJobs", assert_obj!()).await?;
        must_let!(let ConvexValue::Array(scheduled_jobs) = result);
        assert_eq!(scheduled_jobs.len(), 1);
        must_let!(let ConvexValue::Object(job_obj) = scheduled_jobs[0].clone());
        let job = PublicScheduledJob::try_from(job_obj)?;
        assert_eq!(job.state, ScheduledJobState::Pending);

        let expected_ts = ts * 1000.0;
        assert!((expected_ts - job.scheduled_time).abs() < 0.1);

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_at_date(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let ts = t.rt.unix_timestamp().as_secs_f64();
        t.mutation("scheduler:scheduleAtDate", assert_obj!("ts" => ts * 1000.0))
            .await?;
        let result = t.query("scheduler:getScheduledJobs", assert_obj!()).await?;
        must_let!(let ConvexValue::Array(scheduled_jobs) = result);
        assert_eq!(scheduled_jobs.len(), 1);
        must_let!(let ConvexValue::Object(job_obj) = scheduled_jobs[0].clone());
        let job = PublicScheduledJob::try_from(job_obj)?;
        assert_eq!(job.state, ScheduledJobState::Pending);

        let expected_ts = ts * 1000.0;
        assert!((expected_ts - job.scheduled_time).abs() < 0.1);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_invalid_schedule(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {

        let err = t
            .mutation_js_error("scheduler:scheduleAfter", assert_obj!("delayMs" => "abc"))
            .await?;
        assert_contains(&err, "`delayMs` must be a number");

        let err = t
            .mutation_js_error(
                "scheduler:scheduleAfter",
                assert_obj!("delayMs" => ConvexValue::Float64(-2000.0)),
            )
            .await?;
        assert_contains(&err, "`delayMs` must be non-negative");

        let err = t
            .mutation_js_error(
                "scheduler:scheduleAfter",
                assert_obj!("delayMs" => ConvexValue::Float64(10.0 * 365.0 * 24.0 * 3600.0 * 1000.0)),
            )
            .await?;
        assert_contains(&err, "more than 5 years in the future");

        let ts = t.rt.unix_timestamp().as_secs_f64();
        let err = t
            .mutation_js_error(
                "scheduler:scheduleAtTimestamp",
                assert_obj!("ts" => ConvexValue::Float64((ts - 6.0 * 365.0 * 24.0 * 3600.0) * 1000.0)),
            )
            .await?;
        assert_contains(&err, "more than 5 years in the past");

        let err = t
            .mutation_js_error(
                "scheduler:scheduleAtTimestamp",
                assert_obj!("ts" => ConvexValue::Float64((ts + 6.0 * 365.0 * 24.0 * 3600.0) * 1000.0)),
            )
            .await?;
        assert_contains(&err, "more than 5 years in the future");

        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_schedule_missing_function(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Invalid path
        let err = t
            .mutation_js_error(
                "scheduler:scheduleByName",
                assert_obj!("udfPath" => "@#@!invalid@#file$$path"),
            )
            .await?;
        assert_contains(
            &err,
            "can only contain alphanumeric characters, underscores, or periods",
        );

        // Missing module
        let err = t
            .mutation_js_error(
                "scheduler:scheduleByName",
                assert_obj!("udfPath" => "missing_file_path:default"),
            )
            .await?;
        assert_contains(
            &err,
            "Attempted to schedule function at nonexistent path: missing_file_path",
        );

        // Missing function
        let err = t
            .mutation_js_error(
                "scheduler:scheduleByName",
                assert_obj!("udfPath" => "basic:missing_export"),
            )
            .await?;
        assert_contains(
            &err,
            "Attempted to schedule function, but no exported function missing_export found in the \
             file: basic.js. Did you forget to export it?",
        );

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_too_many(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Scheduling a hundred should work.
        t.mutation(
            "scheduler:scheduleMany",
            assert_obj!("limit" => 100, "obj" => {}),
        )
        .await?;

        // Scheduling ten thousand should not.
        let err = t
            .mutation_js_error(
                "scheduler:scheduleMany",
                assert_obj!("limit" => 10000, "obj" => {}),
            )
            .await?;
        assert_contains(
            &err,
            "Too many functions scheduled by this mutation (limit: 1000)",
        );

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_many(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let mut log_lines = t
            .mutation_log_lines(
                "scheduler:scheduleMany",
                assert_obj!("limit" => 950, "obj" => {}),
            )
            .await?;
        let last_line = log_lines.pop().unwrap().to_pretty_string();
        assert_contains(
            &last_line,
            "[WARN] Many functions scheduled by this mutation (actual: 950, limit: 1000)",
        );

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_arguments_too_large(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Scheduling a hundred functions with 1KB argument should work.
        let bytes_1k = t.rt.with_rng(|rng| {
            let mut bytes = [0u8; 1024];
            rng.fill_bytes(&mut bytes);
            bytes.to_vec()
        });
        t.mutation(
            "scheduler:scheduleMany",
            assert_obj!("limit" => 100, "obj" => {
                "bytes" => ConvexValue::Bytes(bytes_1k.try_into()?)
            }),
        )
        .await?;

        // Scheduling a hundred functions with 100KB arguments should not.
        let bytes_100k = t.rt.with_rng(|rng| {
            let mut bytes = [0u8; 100 * 1024];
            rng.fill_bytes(&mut bytes);
            bytes.to_vec()
        });
        let err = t
            .mutation_js_error(
                "scheduler:scheduleMany",
                assert_obj!("limit" => 100, "obj" => {
                    "bytes" => ConvexValue::Bytes(bytes_100k.try_into()?)
                }),
            )
            .await?;
        assert_contains(
            &err,
            "Too large total size of the arguments of scheduled functions from this mutation",
        );

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_arguments_large(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let bytes_1m = t.rt.with_rng(|rng| {
            let mut bytes = [0u8; 1000 * 1024];
            rng.fill_bytes(&mut bytes);
            bytes.to_vec()
        });
        let mut log_lines = t
            .mutation_log_lines(
                "scheduler:scheduleMany",
                assert_obj!("limit" => 7, "obj" => {
                    "bytes" => ConvexValue::Bytes(bytes_1m.try_into()?)
                }),
            )
            .await?;
        let last_line = log_lines.pop().unwrap().to_pretty_string();
        assert_contains(
            &last_line,
            "[WARN] Large total size of the arguments of scheduled functions from this mutation",
        );

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_schedule_by_string(rt: TestRuntime) -> anyhow::Result<()> {
    let t = action_udf_test(rt).await?;
    let job_path = t
        .action("scheduler:scheduleByString", assert_obj!())
        .await?;
    must_let!(let ConvexValue::String(job_path) = job_path);
    assert_eq!(job_path.to_string(), "basic.js:insertObject".to_string());
    Ok(())
}
