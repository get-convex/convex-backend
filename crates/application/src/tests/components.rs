use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    pause::PauseClient,
    types::FunctionCaller,
    RequestId,
};
use itertools::Itertools;
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::CanonicalizedUdfPath;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
    RedactedActionError,
    RedactedActionReturn,
    RedactedMutationError,
    RedactedMutationReturn,
    RedactedQueryReturn,
};

async fn run_query(
    rt: TestRuntime,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> anyhow::Result<RedactedQueryReturn> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules().await?;
    application
        .read_only_udf(
            RequestId::new(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path,
            },
            args,
            Identity::system(),
            FunctionCaller::Test,
        )
        .await
}

async fn run_mutation(
    rt: TestRuntime,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules().await?;
    application
        .mutation_udf(
            RequestId::new(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path,
            },
            args,
            Identity::system(),
            None,
            FunctionCaller::Test,
            PauseClient::new(),
        )
        .await
}

async fn run_action(
    rt: TestRuntime,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules().await?;
    application
        .action_udf(
            RequestId::new(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path,
            },
            args,
            Identity::system(),
            FunctionCaller::Test,
        )
        .await
}

#[convex_macro::test_runtime]
async fn test_run_component_query(rt: TestRuntime) -> anyhow::Result<()> {
    let result = run_query(rt, "componentEntry:list".parse()?, vec![]).await?;
    assert!(result.result.is_ok());
    assert_eq!(result.log_lines.iter().collect_vec().len(), 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_run_component_mutation(rt: TestRuntime) -> anyhow::Result<()> {
    let result = run_mutation(
        rt,
        "componentEntry:insert".parse()?,
        vec![json!({"channel": "random", "text": "convex is kewl"})],
    )
    .await?;
    assert!(result.is_ok());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_run_component_action(rt: TestRuntime) -> anyhow::Result<()> {
    let result = run_action(rt, "componentEntry:hello".parse()?, vec![]).await?;
    assert!(result.is_ok());
    must_let!(let Ok(RedactedActionReturn{value: _, log_lines}) = result);
    // No logs returned because only the action inside the component logs.
    assert_eq!(log_lines.iter().collect_vec().len(), 0);
    Ok(())
}
