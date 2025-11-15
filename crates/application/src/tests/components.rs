use common::{
    bootstrap_model::{
        components::ComponentState,
        tables::TableState,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
    },
    pause::PauseController,
    runtime::Runtime,
    testing::assert_contains,
    types::{
        EnvironmentVariable,
        FunctionCaller,
    },
    RequestId,
};
use database::{
    BootstrapComponentsModel,
    TableModel,
    UserFacingModel,
    PERFORM_BACKFILL_LABEL,
};
use errors::ErrorMetadataAnyhowExt;
use futures::FutureExt;
use itertools::Itertools;
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::CanonicalizedUdfPath;
use value::{
    assert_obj,
    ConvexObject,
    ConvexValue,
    TableName,
    TableNamespace,
};

use crate::{
    deploy_config::SchemaStatus,
    test_helpers::ApplicationTestExt,
    Application,
    FunctionError,
    FunctionReturn,
};

async fn run_function<RT: Runtime>(
    application: &Application<RT>,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
    run_component_function(application, udf_path, args, ComponentPath::root()).await
}

async fn run_component_function<RT: Runtime>(
    application: &Application<RT>,
    udf_path: CanonicalizedUdfPath,
    args: Vec<JsonValue>,
    component: ComponentPath,
) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
    application
        .any_udf(
            RequestId::new(),
            CanonicalizedComponentFunctionPath {
                component,
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
    assert_eq!(ConvexValue::Null, result.value.unpack()?);
    let result =
        run_function(&application, "componentEntry:envVarAction".parse()?, vec![]).await??;
    assert_eq!(ConvexValue::Null, result.value.unpack()?);
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
    assert_eq!(ConvexValue::Null, result.value.unpack()?);
    let result = run_function(
        &application,
        "componentEntry:systemEnvVarAction".parse()?,
        vec![],
    )
    .await??;
    assert_eq!(ConvexValue::Null, result.value.unpack()?);
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
async fn test_paginate_within_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    let err = run_function(
        &application,
        "errors:tryPaginateWithinComponent".parse()?,
        vec![],
    )
    .await?
    .unwrap_err();
    assert_contains(&err.error, "paginate() is only supported in the app");
    Ok(())
}
#[convex_macro::test_runtime]
async fn test_delete_tables_in_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("mounted").await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut components_model = BootstrapComponentsModel::new(&mut tx);
    let (_, component_id) = components_model.must_component_path_to_ids(&component_path())?;

    // Create a table in a new namespace
    let table_namespace = TableNamespace::from(component_id);
    let mut user_facing_model = UserFacingModel::new(&mut tx, table_namespace);
    let table_name: TableName = "test".parse()?;
    user_facing_model
        .insert(table_name.clone(), assert_obj!())
        .await?;
    application.commit_test(tx).await?;

    // Confirm table exists and document is present
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let count = table_model.must_count(table_namespace, &table_name).await?;
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

#[convex_macro::test_runtime]
async fn test_date_now_within_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    let result = run_function(&application, "componentEntry:dateNow".parse()?, vec![])
        .await??
        .value;
    must_let!(let ConvexValue::Array(dates) = result.unpack()?);
    assert_eq!(dates.len(), 2);
    must_let!(let ConvexValue::Float64(parent_date) = dates[0].clone());
    must_let!(let ConvexValue::Float64(child_date) = dates[1].clone());
    // Today these are equal because we're using TestRuntime.
    // We want the guarantee that child_date <= parent_date for queries (because
    // the child might be cached), and child_date == parent_date for mutations.
    // But right now we actually have the opposite guarantee.
    // TODO: Fix this.
    assert!(child_date >= parent_date);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_math_random_within_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    let result = run_function(&application, "componentEntry:mathRandom".parse()?, vec![])
        .await??
        .value;
    must_let!(let ConvexValue::Array(randoms) = result.unpack()?);
    assert_eq!(randoms.len(), 2);
    must_let!(let ConvexValue::Float64(parent_random) = randoms[0].clone());
    must_let!(let ConvexValue::Float64(child_random) = randoms[1].clone());
    // Ensure that a child component has a different random seed from the parent.
    // TODO: the child's random seed should depend on the parent's, so the
    // entire query can be deterministic.
    assert!(parent_random != child_random);
    Ok(())
}

pub async fn unmount_component(
    application: &Application<TestRuntime>,
) -> anyhow::Result<ComponentId> {
    application.load_component_tests_modules("mounted").await?;
    run_component_function(
        application,
        "messages:insertMessage".parse()?,
        vec![example_message().into()],
        component_path(),
    )
    .await??;

    // Unmount component
    application.load_component_tests_modules("empty").await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut components_model = BootstrapComponentsModel::new(&mut tx);
    let (_, component_id) = components_model.must_component_path_to_ids(&component_path())?;
    Ok(component_id)
}

fn example_message() -> ConvexObject {
    assert_obj!("channel" => "sports", "text" => "the celtics won!")
}

fn component_path() -> ComponentPath {
    ComponentPath::deserialize(Some("component")).unwrap()
}

fn table_name() -> TableName {
    "messages".parse().unwrap()
}

fn system_table_name() -> TableName {
    "_modules".parse().unwrap()
}

#[convex_macro::test_runtime]
async fn test_unmounted_component_state(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let component_id = unmount_component(&application).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut components_model = BootstrapComponentsModel::new(&mut tx);
    let component = components_model
        .load_component(component_id)
        .await?
        .unwrap();
    assert!(matches!(component.state, ComponentState::Unmounted));

    // Remount the component
    application.load_component_tests_modules("mounted").await?;

    let mut tx = application.begin(Identity::system()).await?;
    let mut components_model = BootstrapComponentsModel::new(&mut tx);
    // Component at the same path should be remounted with the same id.
    let (_, new_component_id) = components_model.must_component_path_to_ids(&component_path())?;
    assert_eq!(component_id, new_component_id);
    let component = components_model
        .load_component(component_id)
        .await?
        .unwrap();
    assert!(matches!(component.state, ComponentState::Active));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_unmount_cannot_call_functions(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    unmount_component(&application).await?;
    // Calling component function after the component is unmounted should fail
    let result = run_component_function(
        &application,
        "messages:listMessages".parse()?,
        vec![assert_obj!().into()],
        ComponentPath::deserialize(Some("component"))?,
    )
    .await?;
    assert!(result.is_err());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_writes_to_unmounted_tables_fails(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let component_id = unmount_component(&application).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut user_model = UserFacingModel::new(&mut tx, TableNamespace::from(component_id));
    let error = user_model
        .insert(table_name(), example_message())
        .await
        .unwrap_err();
    assert!(error.is_bad_request());
    assert_eq!(error.short_msg(), "UnmountedComponent");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_data_exists_in_unmounted_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let component_id = unmount_component(&application).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let count = table_model
        .must_count(component_id.into(), &table_name())
        .await?;
    assert_eq!(count, 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_descendents_unmounted(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    unmount_component(&application).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let mut components_model = BootstrapComponentsModel::new(&mut tx);
    let env_vars_child_component = ComponentPath::deserialize(Some("envVars/component"))?;
    let (_, component_id) =
        components_model.must_component_path_to_ids(&env_vars_child_component)?;
    let metadata = components_model
        .load_component(component_id)
        .await?
        .unwrap();
    assert!(matches!(metadata.state, ComponentState::Unmounted));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_unmounted_component_deletes_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let component_id = unmount_component(&application).await?;
    let system_identity = Identity::system();
    let mut tx = application.begin(system_identity.clone()).await?;
    let component = BootstrapComponentsModel::new(&mut tx)
        .load_component(component_id)
        .await?
        .unwrap();
    assert!(matches!(component.state, ComponentState::Unmounted));
    application
        .delete_component(&system_identity, component_id)
        .await?;
    let mut tx = application.begin(system_identity).await?;
    let component = BootstrapComponentsModel::new(&mut tx)
        .load_component(component_id)
        .await?;
    assert!(component.is_none());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_unmounted_component_deletes_component_tables(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let component_id = unmount_component(&application).await?;

    let table_namespace = TableNamespace::from(component_id);
    let mut tx = application.begin(Identity::system()).await?;
    assert!(TableModel::new(&mut tx).table_exists(table_namespace, &table_name()));

    application
        .delete_component(&Identity::system(), component_id)
        .await?;

    let mut tx = application.begin(Identity::system()).await?;
    assert!(!TableModel::new(&mut tx).table_exists(table_namespace, &table_name()));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_unmounted_component_deletes_component_system_tables(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let component_id = unmount_component(&application).await?;

    let table_namespace = TableNamespace::from(component_id);
    let mut tx = application.begin(Identity::system()).await?;
    assert!(TableModel::new(&mut tx).table_exists(table_namespace, &system_table_name()));

    application
        .delete_component(&Identity::system(), component_id)
        .await?;

    let mut tx = application.begin(Identity::system()).await?;
    assert!(!TableModel::new(&mut tx).table_exists(table_namespace, &system_table_name()));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_mounted_component_delete_component_errors_out(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("mounted").await?;
    let mut tx = application.begin(Identity::system()).await?;
    let (_, component_id) =
        BootstrapComponentsModel::new(&mut tx).must_component_path_to_ids(&component_path())?;
    assert!(application
        .delete_component(&Identity::system(), component_id)
        .await
        .is_err());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_infinite_loop_in_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    let err = run_function(&application, "errors:tryInfiniteLoop".parse()?, vec![])
        .await?
        .unwrap_err();
    assert_contains(&err.error, "Cross component call depth limit exceeded");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_component_with_hidden_tables(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("mounted").await?;

    // insert table for import
    let mut tx = application.begin(Identity::system()).await?;
    let (_, component_id) =
        BootstrapComponentsModel::new(&mut tx).must_component_path_to_ids(&component_path())?;
    let mut table_model = TableModel::new(&mut tx);
    let hidden_table_name = "hiddentable".parse()?;
    let tablet_id_and_table_number = table_model
        .insert_table_for_import(TableNamespace::from(component_id), &hidden_table_name, None)
        .await?;
    application.commit_test(tx).await?;

    // Ensure the hidden table exists
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let table_metadata = table_model
        .get_table_metadata(tablet_id_and_table_number.tablet_id)
        .await?
        .into_value();
    assert_eq!(table_metadata.namespace, TableNamespace::from(component_id));
    assert_eq!(table_metadata.state, TableState::Hidden);

    // Delete the component
    let component_id = unmount_component(&application).await?;
    application
        .delete_component(&Identity::system(), component_id)
        .await?;

    // Ensure the hidden table is also deleted
    let mut tx = application.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let table_metadata = table_model
        .get_table_metadata(tablet_id_and_table_number.tablet_id)
        .await?
        .into_value();
    assert_eq!(table_metadata.state, TableState::Deleting);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_component_status_skips_staged_index(
    rt: TestRuntime,
    pause: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_component_tests_modules("basic").await?;
    // Insert a doc into a table
    run_function(&application, "errors:insertDoc".parse()?, vec![])
        .await?
        .unwrap();
    // Don't let the index get backfilled before we can check its status
    let _hold_guard = pause.hold(PERFORM_BACKFILL_LABEL);
    let start_push = application
        .start_push_for_layout("schema_with_index")
        .await?;
    // Confirm new schema was created in the root component
    assert!(start_push
        .schema_change
        .schema_ids
        .get(&ComponentPath::root())
        .unwrap()
        .is_some());
    // Schema status should be in progress since one index is backfilling
    let (schema_status, _) = application
        .load_component_schema_status(&Identity::system(), &start_push.schema_change)
        .await?;
    let SchemaStatus::InProgress { components } = schema_status else {
        anyhow::bail!("Expected InProgress schema, got {:?}", schema_status);
    };
    let component_status = components.get(&ComponentPath::root()).unwrap();
    // Schema status only tracks the one non-staged index added and skips the staged
    // index that was added.
    assert_eq!(component_status.indexes_complete, 0);
    assert_eq!(component_status.indexes_total, 1);
    Ok(())
}
