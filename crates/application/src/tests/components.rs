use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
    },
    testing::assert_contains,
    types::{
        EnvironmentVariable,
        FunctionCaller,
    },
    RequestId,
};
use database::{
    TableModel,
    UserFacingModel,
};
use futures::FutureExt;
use itertools::Itertools;
use keybroker::Identity;
use model::components::config::ComponentConfigModel;
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::CanonicalizedUdfPath;
use value::{
    assert_obj,
    ConvexValue,
    TableName,
    TableNamespace,
};

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
    FunctionError,
    FunctionReturn,
};

async fn run_function(
    application: &Application<TestRuntime>,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
    application
        .any_udf(
            RequestId::new(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path,
            },
            args,
            Identity::system(),
            FunctionCaller::Test,
        )
        .boxed()
        .await
}

#[convex_macro::test_runtime]
async fn test_run_component_query(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("with-schema")
        .await?;
    let result = run_function(&application, "componentEntry:list".parse()?, vec![]).await??;
    assert_eq!(result.log_lines.iter().collect_vec().len(), 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_run_component_mutation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("with-schema")
        .await?;
    let result = run_function(
        &application,
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
    application
        .load_component_tests_modules("with-schema")
        .await?;
    let result = run_function(&application, "componentEntry:hello".parse()?, vec![]).await??;
    // No logs returned because only the action inside the component logs.
    assert_eq!(result.log_lines.iter().collect_vec().len(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_env_var_works_in_app_definition(rt: TestRuntime) -> anyhow::Result<()> {
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
    application.load_component_tests_modules("basic").await?;
    let result = run_function(&application, "componentEntry:hello".parse()?, vec![]).await??;
    must_let!(let ConvexValue::String(name) = result.value);
    assert_eq!(name.to_string(), "emma".to_string());

    // No logs returned because only the action inside the component logs.
    assert_eq!(result.log_lines.iter().collect_vec().len(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_system_env_var_works_in_app_definition(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    let result = run_function(&application, "componentEntry:url".parse()?, vec![]).await??;
    must_let!(let ConvexValue::String(name) = result.value);
    assert_eq!(name.to_string(), "http://127.0.0.1:8000".to_string());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_env_vars_not_accessible_in_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
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
    let result =
        run_function(&application, "componentEntry:envVarQuery".parse()?, vec![]).await??;
    assert_eq!(ConvexValue::Null, result.value);
    let result =
        run_function(&application, "componentEntry:envVarAction".parse()?, vec![]).await??;
    assert_eq!(ConvexValue::Null, result.value);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_system_env_vars_not_accessible_in_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    let result = run_function(
        &application,
        "componentEntry:systemEnvVarQuery".parse()?,
        vec![],
    )
    .await??;
    assert_eq!(ConvexValue::Null, result.value);
    let result = run_function(
        &application,
        "componentEntry:systemEnvVarAction".parse()?,
        vec![],
    )
    .await??;
    assert_eq!(ConvexValue::Null, result.value);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_system_error_propagation(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;

    // The system error from the subquery should propagate to the top-level query.
    let error = run_function(
        &application,
        "errors:throwSystemErrorFromQuery".parse()?,
        vec![],
    )
    .await
    .unwrap_err();
    assert_contains(&error, "I can't go for that");

    // Actions throw a JS error into user space when a call to `ctx.runAction`
    // throws a system error, so we don't propagate them here.
    let result = run_function(
        &application,
        "errors:throwSystemErrorFromAction".parse()?,
        vec![],
    )
    .await?
    .unwrap_err();
    assert_contains(&result.error, "Your request couldn't be completed");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_tables_in_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    // Create a table in a new namespace
    let mut tx = application.begin(Identity::system()).await?;
    let table_namespace = TableNamespace::test_component();
    let mut component_config_model = ComponentConfigModel::new(&mut tx);
    component_config_model
        .initialize_component_namespace_for_test(ComponentId::from(table_namespace))
        .await?;
    let mut user_facing_model = UserFacingModel::new(&mut tx, table_namespace);
    let table_name: TableName = "test".parse()?;
    user_facing_model
        .insert(table_name.clone(), assert_obj!())
        .await?;
    application.commit_test(tx).await?;

    // Confirm table exists and document is present
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let count = table_model.count(table_namespace, &table_name).await?;
    assert_eq!(count, 1);
    assert!(table_model.table_exists(table_namespace, &table_name));

    // Delete the table
    application
        .delete_tables(
            &Identity::system(),
            vec![table_name.clone()],
            table_namespace,
        )
        .await?;

    // Confirm table no longer exists
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    assert!(!table_model.table_exists(table_namespace, &table_name));
    Ok(())
}
