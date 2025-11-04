use runtime::testing::TestRuntime;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_auth_config_doesnt_use_globals(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t| {
        let auth_config = t
            .evaluate_auth_config(
                "
            JSON.stringify = 'haha';
            export default { providers: [] };
            ",
            )
            .await?;
        assert_eq!(auth_config.providers, vec![]);
        Ok(())
    })
    .await
}
