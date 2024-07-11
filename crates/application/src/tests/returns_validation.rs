use common::{
    components::{
        ComponentFunctionPath,
        ComponentPath,
    },
    pause::PauseClient,
    types::FunctionCaller,
    RequestId,
};
use keybroker::{
    testing::TestUserIdentity,
    Identity,
    UserIdentity,
};
use runtime::testing::TestRuntime;
use serde_json::json;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
    RedactedActionError,
    RedactedActionReturn,
    RedactedMutationError,
    RedactedMutationReturn,
    RedactedQueryReturn,
};

async fn run_zero_arg_mutation(
    application: &Application<TestRuntime>,
    name: &str,
) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>> {
    let obj = json!({});
    application
        .mutation_udf(
            RequestId::new(),
            ComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: name.parse()?,
            },
            vec![obj],
            Identity::user(UserIdentity::test()),
            None,
            FunctionCaller::HttpEndpoint,
            PauseClient::new(),
        )
        .await
}

async fn run_zero_arg_query(
    application: &Application<TestRuntime>,
    name: &str,
) -> anyhow::Result<RedactedQueryReturn> {
    let obj = json!({});
    application
        .read_only_udf(
            RequestId::new(),
            ComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: name.parse()?,
            },
            vec![obj],
            Identity::user(UserIdentity::test()),
            FunctionCaller::HttpEndpoint,
        )
        .await
}

async fn run_zero_arg_action(
    application: &Application<TestRuntime>,
    name: &str,
) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
    let obj = json!({});
    application
        .action_udf(
            RequestId::new(),
            ComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path: name.parse()?,
            },
            vec![obj],
            Identity::user(UserIdentity::test()),
            FunctionCaller::HttpEndpoint,
        )
        .await
}

#[convex_macro::test_runtime]
async fn test_mutation_bad_output(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_mutation(
        &application,
        "returns_validation:stringOutputReturnsNumberMutation",
    )
    .await?;
    assert!(format!("{}", result.unwrap_err()).contains("ReturnsValidationError"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_bad_output(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_query(
        &application,
        "returns_validation:stringOutputReturnsNumberQuery",
    )
    .await?;
    assert!(format!("{}", result.result.unwrap_err()).contains("ReturnsValidationError"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_bad_output(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_action(
        &application,
        "returns_validation:stringOutputReturnsNumberAction",
    )
    .await?;
    assert!(format!("{}", result.unwrap_err()).contains("ReturnsValidationError"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_extra_fields(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_query(&application, "returns_validation:extraOutputFields").await?;
    assert!(format!("{}", result.result.unwrap_err()).contains("ReturnsValidationError"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mutation_output(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_mutation(
        &application,
        "returns_validation:stringOutputReturnsStringMutation",
    )
    .await?;
    assert!(format!("{}", result.unwrap().value).contains("hello"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_output(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_query(
        &application,
        "returns_validation:stringOutputReturnsStringQuery",
    )
    .await?;
    assert!(format!("{}", result.result.unwrap()).contains("hello"));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_output(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let result = run_zero_arg_action(
        &application,
        "returns_validation:stringOutputReturnsStringAction",
    )
    .await?;
    assert!(format!("{}", result.unwrap().value).contains("hello"));
    Ok(())
}
