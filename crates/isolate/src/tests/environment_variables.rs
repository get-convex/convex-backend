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
use serde_json::json;
use value::assert_val;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_get_environment_variable(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async |t| {
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
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_get_environment_variable_null(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async |t| {
        let v = t
            .query("environmentVariables:getEnvironmentVariable", assert_obj!())
            .await?;
        assert_eq!(v, ConvexValue::Null);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_console_log(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async |t| {
        let v = t
            .query_log_lines("environmentVariables:log", assert_obj!())
            .await?;
        assert_eq!(
            v.into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect::<Vec<_>>(),
            vec!["[LOG] [process.env]"]
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_system_environment_variables(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async |t| {
        let v = t
            .query("environmentVariables:getCloudUrl", assert_obj!())
            .await?;
        assert_eq!(v, ConvexValue::try_from("https://carnitas.convex.cloud")?);

        let v = t
            .query("environmentVariables:getSiteUrl", assert_obj!())
            .await?;
        assert_eq!(v, ConvexValue::try_from("https://carnitas.convex.site")?);
        Ok(())
    })
    .await
}

async fn test_environment_variable_reads_recorded(
    rt: TestRuntime,
    env_var_name: &str,
    read_function: &str,
) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async |t| {
        let (outcome, mut token) = t
            .raw_query(
                read_function,
                vec![assert_val!({})],
                Identity::system(),
                None,
            )
            .await?;
        assert_eq!(outcome.result.unwrap().json_value(), json!(null));

        let mut tx = t.database.begin_system().await?;
        let unrelated_variable =
            EnvironmentVariable::new("UNRELATED_NAME".parse()?, "TEST_VALUE".parse()?);
        EnvironmentVariablesModel::new(&mut tx)
            .create(unrelated_variable, &HashSet::new())
            .await?;
        let new_ts = t.database.commit(tx).await?;
        token = t
            .database
            .log()
            .refresh_token(token, new_ts)?
            .expect("Should not be invalidated by creating an unrelated environment variable");

        let mut tx = t.database.begin_system().await?;
        let mut related_variable =
            EnvironmentVariable::new(env_var_name.parse()?, "TEST_VALUE".parse()?);
        let env_var_id = EnvironmentVariablesModel::new(&mut tx)
            .create(related_variable.clone(), &HashSet::new())
            .await?;
        let new_ts = t.database.commit(tx).await?;
        assert!(
            t.database.log().refresh_token(token, new_ts)?.is_none(),
            "Should be invalidated by creating a used environment variable"
        );

        let (outcome, token) = t
            .raw_query(
                read_function,
                vec![assert_val!({})],
                Identity::system(),
                None,
            )
            .await?;
        assert_eq!(outcome.result.unwrap().json_value(), json!("TEST_VALUE"));
        let mut tx = t.database.begin_system().await?;
        related_variable.value = "TEST_VALUE_2".parse()?;
        EnvironmentVariablesModel::new(&mut tx)
            .edit([(env_var_id, related_variable)].into_iter().collect())
            .await?;
        let new_ts = t.database.commit(tx).await?;
        assert!(
            t.database.log().refresh_token(token, new_ts)?.is_none(),
            "Should be invalidated by editing a used environment variable"
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_global_environment_variable_reads_recorded(rt: TestRuntime) -> anyhow::Result<()> {
    test_environment_variable_reads_recorded(
        rt,
        "TEST_NAME",
        "environmentVariables:getGlobalEnvironmentVariable",
    )
    .await
}

#[convex_macro::test_runtime]
async fn test_query_environment_variable_reads_recorded(rt: TestRuntime) -> anyhow::Result<()> {
    test_environment_variable_reads_recorded(
        rt,
        "TEST_NAME_2",
        "environmentVariables:getOtherEnvironmentVariable",
    )
    .await
}
