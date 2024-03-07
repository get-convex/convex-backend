use common::{
    assert_obj,
    value::ConvexValue,
};
use must_let::must_let;
use runtime::testing::TestRuntime;

use crate::{
    test_helpers::UdfTest,
    tests::assert_contains,
};

#[convex_macro::test_runtime]
async fn test_async_return_resolved_promise(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    must_let!(let ConvexValue::String(r) = t.query("asyncTests:returnsResolved", assert_obj!()).await?);
    assert_eq!(&r[..], "hello world");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_async_return_unresolved_promise(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("asyncTests:neverResolves", assert_obj!())
        .await?;
    assert_contains(&e, "Returned promise will never resolve");
    Ok(())
}

// Regression test.
#[convex_macro::test_runtime]
async fn test_doubly_dangling_syscall(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("asyncTests:syscallAfterDanglingSyscall", assert_obj!())
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_dangling_mutation(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("asyncTests:danglingMutation", assert_obj!())
        .await?;
    must_let!(let ConvexValue::Array(arr) = t.query("asyncTests:queryTestTable", assert_obj!()).await?);
    assert_eq!(arr.len(), 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_doubly_dangling_mutation(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("asyncTests:doublyDanglingMutation", assert_obj!())
        .await?;
    must_let!(let ConvexValue::Array(arr) = t.query("asyncTests:queryTestTable", assert_obj!()).await?);
    assert_eq!(arr.len(), 2);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_dangling_query(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("basic:insertObject", assert_obj!()).await?;
    must_let!(let ConvexValue::Array(arr) = t.query("asyncTests:queryDangling", assert_obj!()).await?);
    assert_eq!(arr.len(), 0);
    Ok(())
}
