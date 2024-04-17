use common::assert_obj;
use runtime::testing::TestRuntime;

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

const EXPECTED: &str = r#"
Uncaught Error: Oh bother!
    at throwsTheError (../convex/sourceMaps.ts:11:0)
    at callsSomethingElse (../convex/sourceMaps.ts:16:2)
"#;

#[convex_macro::test_runtime]
async fn test_source_mapping(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .query_js_error("sourceMaps:throwsError", assert_obj!())
            .await?;
        assert!(format!("{e}").starts_with(EXPECTED.trim()), "{e:?}");
        Ok(())
    })
    .await
}
