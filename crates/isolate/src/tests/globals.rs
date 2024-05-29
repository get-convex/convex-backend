use std::time::Duration;

use common::assert_obj;
use runtime::testing::TestRuntime;
use value::{
    numeric::is_integral,
    ConvexValue,
};

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

/// Tests to ensure that the JavaScript environment will be recognizeable to
/// library code running in it. We aim to implement a subset of
/// https://common-min-api.proposal.wintercg.org/

#[convex_macro::test_runtime]
async fn test_globals(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.query("globals:globals", assert_obj!()).await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_date(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let global_date1 = t.query("globals:getGlobalDate", assert_obj!()).await?;
        t.rt.advance_time(Duration::from_secs(1)).await;
        let udf_date1 = t.query("globals:getDate", assert_obj!()).await?;

        // The UDF execution phase date should be higher than the import phase one.
        assert!(udf_date1 > global_date1);

        let global_date2 = t.query("globals:getGlobalDate", assert_obj!()).await?;
        t.rt.advance_time(Duration::from_secs(1)).await;
        let udf_date2 = t.query("globals:getDate", assert_obj!()).await?;
        // Global date should not change between reruns.
        assert_eq!(global_date1, global_date2);
        // The UDF execute date should advance.
        assert!(udf_date2 > udf_date1);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_date_now_integral(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let ConvexValue::Float64(n) = t.query("globals:getDateNow", assert_obj!()).await? else {
            panic!("Expected Float64 from getDateNow");
        };
        assert!(is_integral(n).is_some());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_rand(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let global_rand1 = t.query("globals:getGlobalRandom", assert_obj!()).await?;
        let udf_rand1 = t.query("globals:getRandom", assert_obj!()).await?;

        assert_ne!(udf_rand1, global_rand1);

        let global_rand2 = t.query("globals:getGlobalRandom", assert_obj!()).await?;
        let udf_rand2 = t.query("globals:getRandom", assert_obj!()).await?;
        // Global rand should not change between runs.
        assert_eq!(global_rand1, global_rand2);
        assert_ne!(udf_rand2, udf_rand1);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_finalization_registry(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let ConvexValue::Null = t
            .query("globals:createFinalizationRegistry", assert_obj!())
            .await?
        else {
            panic!("Expected null from createFinalizationRegistry");
        };
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_weak_ref(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let ConvexValue::Null = t.query("globals:createWeakRef", assert_obj!()).await? else {
            panic!("Expected null from createWeakRef");
        };
        Ok(())
    })
    .await
}
