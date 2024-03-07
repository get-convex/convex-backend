use std::collections::HashSet;

use common::{
    assert_obj,
    value::ConvexValue,
};
use keybroker::Identity;
use model::environment_variables::{
    types::EnvironmentVariable,
    EnvironmentVariablesModel,
};
use runtime::testing::TestRuntime;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_get_environment_variable(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;
    let environment_variable =
        EnvironmentVariable::new("TEST_NAME".parse()?, "TEST_VALUE".parse()?);
    EnvironmentVariablesModel::new(&mut tx)
        .create(environment_variable, &HashSet::new())
        .await?;
    t.database.commit(tx).await?;
    let value = t
        .query("environmentVariables:getEnvironmentVariable", assert_obj!())
        .await?;
    assert_eq!(value, ConvexValue::try_from("TEST_VALUE")?);

    // Environment variables should also be available at import time.
    let value = t
        .query(
            "environmentVariables:getGlobalEnvironmentVariable",
            assert_obj!(),
        )
        .await?;
    assert_eq!(value, ConvexValue::try_from("TEST_VALUE")?);
    // In actions as well.
    let value = t
        .action(
            "environmentVariables:actionGetEnvironmentVariable",
            assert_obj!(),
        )
        .await?;
    assert_eq!(value, ConvexValue::try_from("TEST_VALUE")?);
    let value = t
        .action(
            "environmentVariables:actionGetGlobalEnvironmentVariable",
            assert_obj!(),
        )
        .await?;
    assert_eq!(value, ConvexValue::try_from("TEST_VALUE")?);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_get_environment_variable_null(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let v = t
        .query("environmentVariables:getEnvironmentVariable", assert_obj!())
        .await?;
    assert_eq!(v, ConvexValue::Null);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_console_log(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let v = t
        .query_log_lines("environmentVariables:log", assert_obj!())
        .await?;
    assert_eq!(
        v.into_iter()
            .map(|l| l.to_pretty_string())
            .collect::<Vec<_>>(),
        vec!["[LOG] [process.env]"]
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_system_environment_variables(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let v = t
        .query("environmentVariables:getCloudUrl", assert_obj!())
        .await?;
    assert_eq!(v, ConvexValue::try_from("https://carnitas.convex.cloud")?);

    let v = t
        .query("environmentVariables:getSiteUrl", assert_obj!())
        .await?;
    assert_eq!(v, ConvexValue::try_from("https://carnitas.convex.site")?);
    Ok(())
}
