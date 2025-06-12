use std::collections::HashSet;

use common::{
    assert_obj,
    types::EnvironmentVariable,
    value::ConvexValue,
};
use keybroker::Identity;
use model::environment_variables::EnvironmentVariablesModel;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::ConvexObject;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_system_udf(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;
    let environment_variable = EnvironmentVariable::new("A".parse()?, "B".parse()?);
    EnvironmentVariablesModel::new(&mut tx)
        .create(environment_variable, &HashSet::new())
        .await?;
    t.database.commit(tx).await?;

    // nonexistent returns Null
    must_let!(let ConvexValue::Null = t.query(
        "_system/cli/queryEnvironmentVariables:get",
        assert_obj!("name" => "nonexistent".to_string()),
    )
    .await?);

    // return environment variable
    must_let!(let ConvexValue::Object(obj) = t.query(
        "_system/cli/queryEnvironmentVariables:get",
        assert_obj!("name" => "A".to_string()),
    )
    .await?);
    must_let!(let Some(ConvexValue::String(name)) = ConvexObject::get(&obj, "name"));
    assert_eq!(name.to_string(), "A");
    must_let!(let Some(ConvexValue::String(value)) = ConvexObject::get(&obj, "value"));
    assert_eq!(value.to_string(), "B");

    // query a system environment variable
    must_let!(let ConvexValue::Object(obj) = t.query(
        "_system/cli/queryEnvironmentVariables:get",
        assert_obj!("name" => "CONVEX_CLOUD_URL".to_string()),
    )
    .await?);
    must_let!(let Some(ConvexValue::String(name)) = ConvexObject::get(&obj, "name"));
    assert_eq!(name.to_string(), "CONVEX_CLOUD_URL");
    must_let!(let Some(ConvexValue::String(value)) = ConvexObject::get(&obj, "value"));
    assert_eq!(value.to_string(), "https://carnitas.convex.cloud");

    // calling with empty argument fails
    let error = t
        .query_js_error("_system/cli/queryEnvironmentVariables:get", assert_obj!())
        .await?;

    assert!(error.message.contains("missing the required field `name`"));
    Ok(())
}
