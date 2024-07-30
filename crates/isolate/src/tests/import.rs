use std::collections::HashSet;

use common::testing::assert_contains;
use keybroker::Identity;
use model::environment_variables::{
    types::EnvironmentVariable,
    EnvironmentVariablesModel,
};
use runtime::testing::TestRuntime;
use value::assert_obj;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_action_dynamic_import(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.action("import_tests:dynamicImport", assert_obj!())
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_action_dynamic_import_nonexistent(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.action("import_tests:dynamicImportNonexistent", assert_obj!())
        .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_dynamic_import(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let err = t
        .query_js_error("import_tests:dynamicImportQuery", assert_obj!())
        .await?;
    assert_contains(&err, "dynamic module import unsupported");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_dynamic_import_load_failure(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;
    let environment_variable =
        EnvironmentVariable::new("FAIL_MODULE_LOAD".parse()?, "fail".parse()?);
    EnvironmentVariablesModel::new(&mut tx)
        .create(environment_variable, &HashSet::new())
        .await?;
    t.database.commit(tx).await?;

    let err = t
        .action_js_error("import_tests:dynamicImportLoadFailure", assert_obj!())
        .await?;
    assert_contains(&err, "Uncaught Error: boom");
    Ok(())
}
