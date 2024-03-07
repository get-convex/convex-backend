use common::{
    assert_obj,
    value::ConvexValue,
};
use must_let::must_let;
use runtime::testing::TestRuntime;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_creation_times_within_table_are_monotonic(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("creationTime:createFiveDocuments", assert_obj!())
        .await?;
    must_let!(let ConvexValue::Array(array) = t
            .query("creationTime:getDocumentsByCreationTime", assert_obj!())
            .await?);

    let vector: Vec<_> = array.into();
    assert_eq!(vector.len(), 5);

    let mut prev_creation_time: Option<f64> = None;

    for i in 0..5 {
        must_let!(let ConvexValue::Object(obj)  = vector.get(i).unwrap());

        // Numbers come back in order.
        must_let!(let Some(ConvexValue::Float64(count)) = obj.get("count"));
        assert_eq!(*count, i as f64);

        // Each creation time should be higher than the previous.
        must_let!(let Some(ConvexValue::Float64(creation_time)) = obj.get("_creationTime"));
        if let Some(prev_creation_time) = prev_creation_time {
            assert!(prev_creation_time < *creation_time);
        }
        prev_creation_time = Some(*creation_time)
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_creation_time_between_system_time(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    must_let!(let ConvexValue::Float64(t1) = t.query("basic:readTimeMs", assert_obj!()).await?);

    must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:insertObject",
            assert_obj!(),
        ).await?);
    must_let!(let Some(ConvexValue::Float64(t2)) = obj.get("_creationTime"));
    must_let!(let ConvexValue::Float64(t3) = t.query("basic:readTimeMs", assert_obj!()).await?);

    assert!(t1 < *t2);
    assert!(*t2 < t3);

    Ok(())
}
