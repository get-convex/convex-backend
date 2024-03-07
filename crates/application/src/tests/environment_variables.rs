use errors::ErrorMetadataAnyhowExt;
use keybroker::Identity;
use model::environment_variables::{
    types::{
        EnvVarName,
        EnvVarValue,
        EnvironmentVariable,
    },
    EnvironmentVariablesModel,
};
use runtime::testing::TestRuntime;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
    EnvVarChange,
};

// Name conflict with an environment variable outside the change
#[convex_macro::test_runtime]
async fn test_env_variable_uniqueness(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;
    let name1: EnvVarName = "name1".parse()?;
    let name2: EnvVarName = "name2".parse()?;
    let value1: EnvVarValue = "value1".parse()?;
    let value2: EnvVarValue = "value2".parse()?;

    let mut tx = application.begin(Identity::system()).await?;
    let audit_events = application
        .create_environment_variables(
            &mut tx,
            vec![
                EnvironmentVariable::new(name1.clone(), value1.clone()),
                EnvironmentVariable::new(name2.clone(), value2.clone()),
            ],
        )
        .await?;
    assert_eq!(audit_events.len(), 2);

    must_let::must_let!(
        let Err(e) = application
            .create_environment_variables(
                &mut tx,
                vec![EnvironmentVariable::new(name1.clone(), value2)],
            )
            .await
    );
    println!("{e:?}");
    assert_eq!(e.short_msg(), "EnvVarNameNotUnique");

    Ok(())
}

// An user can delete an environment variable and create a new one with the same
// name in one request
#[convex_macro::test_runtime]
async fn test_env_variable_delete_and_create(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let name: EnvVarName = "name".parse()?;
    let value: EnvVarValue = "value".parse()?;
    let value2: EnvVarValue = "value2".parse()?;

    let mut tx = application.begin(Identity::system()).await?;
    application
        .create_environment_variables(
            &mut tx,
            vec![EnvironmentVariable::new(name.clone(), value.clone())],
        )
        .await?;

    EnvironmentVariablesModel::new(&mut tx)
        .get(&name)
        .await?
        .unwrap()
        .id();

    application
        .update_environment_variables(
            &mut tx,
            vec![
                EnvVarChange::Unset(name.clone()),
                EnvVarChange::Set(EnvironmentVariable::new(name.clone(), value2.clone())),
            ],
        )
        .await?;

    let after = EnvironmentVariablesModel::new(&mut tx)
        .get(&name)
        .await?
        .unwrap()
        .into_value();
    assert_eq!(after.value, value2);

    Ok(())
}

// Test that the env var count limit is enforced by create_environment_variables
#[convex_macro::test_runtime]
async fn test_env_var_limit_create(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let value: EnvVarValue = "value".parse()?;
    let env_vars_full: Vec<EnvironmentVariable> = (1..=100)
        .collect::<Vec<i32>>()
        .iter()
        .map(|i| -> anyhow::Result<EnvironmentVariable> {
            Ok(EnvironmentVariable::new(
                format!("var{i}").parse()?,
                value.clone(),
            ))
        })
        .try_collect()?;

    let mut tx = application.begin(Identity::system()).await?;
    application
        .create_environment_variables(&mut tx, env_vars_full)
        .await?;

    application
        .create_environment_variables(
            &mut tx,
            vec![
                EnvironmentVariable::new("new_var".parse()?, value.clone()),
                EnvironmentVariable::new("new_varadcds".parse()?, value.clone()),
            ],
        )
        .await
        .unwrap_err();

    Ok(())
}

// Test that the env var count limit is enforced by update_environment_variables
#[convex_macro::test_runtime]
async fn test_env_var_limit_update(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let value: EnvVarValue = "value".parse()?;
    let env_vars_full: Vec<EnvironmentVariable> = (1..=100)
        .collect::<Vec<i32>>()
        .iter()
        .map(|i| -> anyhow::Result<EnvironmentVariable> {
            Ok(EnvironmentVariable::new(
                format!("var{i}").parse()?,
                value.clone(),
            ))
        })
        .try_collect()?;

    let mut tx = application.begin(Identity::system()).await?;
    application
        .create_environment_variables(&mut tx, env_vars_full)
        .await?;

    application
        .update_environment_variables(
            &mut tx,
            vec![EnvVarChange::Set(EnvironmentVariable::new(
                "other_new_var".parse()?,
                value.clone(),
            ))],
        )
        .await
        .unwrap_err();

    Ok(())
}

// Name conflict with system environment variable
#[convex_macro::test_runtime]
async fn test_env_variable_cannot_set_system_name(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application.load_udf_tests_modules().await?;

    let mut tx = application.begin(Identity::system()).await?;
    let error = application
        .create_environment_variables(
            &mut tx,
            vec![EnvironmentVariable::new(
                "CONVEX_SITE_URL".parse()?,
                "foo".parse()?,
            )],
        )
        .await
        .unwrap_err();
    assert!(format!("{}", error).contains(
        "Environment variable with name \"CONVEX_SITE_URL\" is built-in and cannot be overridden"
    ));

    Ok(())
}
