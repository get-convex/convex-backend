use common::{
    assert_obj,
    value::ConvexValue,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::assert_val;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_bigint(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let value = t.query("values:intQuery", assert_obj!()).await?;
    must_let!(let ConvexValue::Int64(v) = value);
    assert_eq!(v, 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_empty_key(rt: TestRuntime) -> anyhow::Result<()> {
    // Check that an object with an empty key round trips through mutation and
    // query.
    let t = UdfTest::default(rt).await?;
    let id = t
        .mutation("values:insertObject", assert_obj!("obj" => {"" => "hi"}))
        .await?;
    let value = t.query("values:getObject", assert_obj!("id" => id)).await?;
    must_let!(let ConvexValue::Object(o) = value);
    assert_eq!(o.len(), 3);
    assert_eq!(o.get("").unwrap().clone(), assert_val!("hi"));
    Ok(())
}
