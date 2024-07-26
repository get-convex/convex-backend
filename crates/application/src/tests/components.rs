use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    pause::PauseClient,
    types::{
        EnvironmentVariable,
        FunctionCaller,
    },
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
use value::ConvexValue;

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
    application: &Application<TestRuntime>,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
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
    let application = Application::new_for_tests(&rt).await?;
    let result = run_action(&application, "componentEntry:hello".parse()?, vec![]).await?;
    assert!(result.is_ok());
    must_let!(let Ok(RedactedActionReturn{value: _, log_lines}) = result);
    // No logs returned because only the action inside the component logs.
    assert_eq!(log_lines.iter().collect_vec().len(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_run_component_action_with_env_var_set(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    application
        .create_one_environment_variable(
            &mut tx,
            EnvironmentVariable {
                name: "NAME".parse()?,
                value: "emma".parse()?,
            },
        )
        .await?;
    application.commit_test(tx).await?;
    let result = run_action(&application, "componentEntry:hello".parse()?, vec![]).await?;
    assert!(result.is_ok());
    must_let!(let Ok(RedactedActionReturn{value, log_lines}) = result);
    must_let!(let ConvexValue::String(name) = value);
    assert_eq!(name.to_string(), "emma".to_string());

    // No logs returned because only the action inside the component logs.
    assert_eq!(log_lines.iter().collect_vec().len(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_system_env_var_works_in_app_definition(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let result = run_action(&application, "componentEntry:url".parse()?, vec![]).await?;
    assert!(result.is_ok());
    must_let!(let Ok(RedactedActionReturn{value, log_lines: _}) = result);
    must_let!(let ConvexValue::String(name) = value);
    assert_eq!(name.to_string(), "http://127.0.0.1:8000".to_string());
    Ok(())
}
