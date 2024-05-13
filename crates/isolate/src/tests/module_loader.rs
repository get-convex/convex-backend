use std::str::FromStr;

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
    },
    types::UdfType,
};
use errors::ErrorMetadataAnyhowExt;
use keybroker::Identity;
use model::modules::ModuleModel;
use must_let::must_let;
use runtime::testing::TestRuntime;
use sync_types::UdfPath;

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_get_analyzed_function(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let mut tx = t.database.begin(Identity::system()).await?;

    let root_component_path = |s: &str| CanonicalizedComponentFunctionPath {
        component: ComponentId::Root,
        udf_path: UdfPath::from_str(s).unwrap().canonicalize(),
    };

    let udf_type = ModuleModel::new(&mut tx)
        .get_analyzed_function(&root_component_path("basic:count"))
        .await?
        .map(|f| f.udf_type);
    must_let!(let Ok(UdfType::Query) = udf_type);

    let udf_type = ModuleModel::new(&mut tx)
        .get_analyzed_function(&root_component_path("basic:insertObject"))
        .await?
        .map(|f| f.udf_type);
    must_let!(let Ok(UdfType::Mutation) = udf_type);

    let udf_type = ModuleModel::new(&mut tx)
        .get_analyzed_function(&root_component_path("notExistingModule"))
        .await?
        .map(|f| f.udf_type);
    assert_eq!(udf_type.unwrap_err().short_msg(), "ModuleNotFound");

    let udf_type = ModuleModel::new(&mut tx)
        .get_analyzed_function(&root_component_path("basic:notExistingFunction"))
        .await?
        .map(|f| f.udf_type);
    assert_eq!(udf_type.unwrap_err().short_msg(), "FunctionNotFound");

    Ok(())
}
