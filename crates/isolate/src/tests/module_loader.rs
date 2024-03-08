use std::str::FromStr;

use common::types::UdfType;
use keybroker::Identity;
use runtime::testing::TestRuntime;
use sync_types::UdfPath;

use crate::{
    test_helpers::UdfTest,
    ModuleLoader,
    TransactionModuleLoader,
};

#[convex_macro::test_runtime]
async fn test_log_number(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;
    let module_loader = TransactionModuleLoader;

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("basic:count").unwrap().canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    assert_eq!(udf_type, Ok(UdfType::Query));

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("basic:insertObject")
                .unwrap()
                .canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    assert_eq!(udf_type, Ok(UdfType::Mutation));

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("notExistingModule")
                .unwrap()
                .canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    assert!(udf_type
        .unwrap_err()
        .contains("Couldn't find JavaScript module 'notExistingModule.js'"));

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("basic:notExistingFunction")
                .unwrap()
                .canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    assert!(udf_type
        .unwrap_err()
        .contains(r#"Couldn't find "notExistingFunction" in module "basic.js"."#));

    Ok(())
}
