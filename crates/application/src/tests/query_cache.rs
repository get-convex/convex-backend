use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        PublicFunctionPath,
    },
    types::FunctionCaller,
    RequestId,
};
use keybroker::Identity;
use runtime::testing::TestRuntime;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_query_caching(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let (initial_function_log, _) = application.function_log().stream(0.0).await;

    let path = PublicFunctionPath::Component(CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: "basic:readTimeMs".parse()?,
    });

    // Run query first time
    let result1 = application
        .read_only_udf(
            RequestId::new(),
            path.clone(),
            vec![],
            Identity::system(),
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
        )
        .await?;

    // Run same query second time
    let result2 = application
        .read_only_udf(
            RequestId::new(),
            path,
            vec![],
            Identity::system(),
            FunctionCaller::Action {
                parent_scheduled_job: None,
            },
        )
        .await?;

    // The query gets the current time, but the result is cached so the results
    // should match.
    assert_eq!(result1.result, result2.result);

    // Check execution logs show second query was cached
    let (final_function_log, _) = application.function_log().stream(0.0).await;
    let entries = &final_function_log[initial_function_log.len()..];
    assert_eq!(entries.len(), 2);
    assert!(!entries[0].cached_result);
    assert!(entries[1].cached_result);

    Ok(())
}
