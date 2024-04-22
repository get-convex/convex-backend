use std::str::FromStr;

use common::types::UdfType;
use errors::ErrorMetadataAnyhowExt;
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use sync_types::UdfPath;

use crate::{
    test_helpers::UdfTest,
    ModuleLoader,
    TransactionModuleLoader,
};

#[convex_macro::test_runtime]
async fn test_get_analyzed_function(rt: TestRuntime) -> anyhow::Result<()> {
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
    must_let!(let Ok(UdfType::Query) = udf_type);

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("basic:insertObject")
                .unwrap()
                .canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    must_let!(let Ok(UdfType::Mutation) = udf_type);

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("notExistingModule")
                .unwrap()
                .canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    assert_eq!(udf_type.unwrap_err().short_msg(), "ModuleNotFound");

    let udf_type = module_loader
        .get_analyzed_function(
            &mut tx,
            &UdfPath::from_str("basic:notExistingFunction")
                .unwrap()
                .canonicalize(),
        )
        .await?
        .map(|f| f.udf_type);
    assert_eq!(udf_type.unwrap_err().short_msg(), "FunctionNotFound");

    Ok(())
}
