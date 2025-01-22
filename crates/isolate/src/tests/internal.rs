use std::{
    collections::BTreeMap,
    str::FromStr,
};

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
        PublicFunctionPath,
    },
    types::{
        AllowedVisibility,
        MemberId,
        UdfType,
    },
    version::Version,
};
use keybroker::{
    AdminIdentity,
    Identity,
    DEV_INSTANCE_NAME,
};
use model::{
    config::ConfigModel,
    source_packages::SourcePackageModel,
    udf_config::types::UdfConfig,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use sync_types::CanonicalizedUdfPath;
use udf::validation::ValidatedPathAndArgs;
use value::{
    ConvexArray,
    TableNamespace,
};

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_udf_visibility(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;

    let internal_function = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: CanonicalizedUdfPath::from_str("internal.js:myInternalMutation")?,
    };
    let public_function = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: CanonicalizedUdfPath::from_str("internal.js:publicMutation")?,
    };
    let non_existent_function = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: CanonicalizedUdfPath::from_str("internal.js:doesNotExist")?,
    };

    let post_internal_npm_version = Version::parse("1.0.0").unwrap();

    let mut tx = t.database.begin(Identity::system()).await?;
    let (config_metadata, module_configs, _udf_config) =
        ConfigModel::new(&mut tx, ComponentId::test_user())
            .get_with_module_source(t.module_loader.as_ref())
            .await?;
    let modules_by_path = module_configs
        .iter()
        .map(|c| (c.path.clone().canonicalize(), c.clone()))
        .collect();
    let udf_config = UdfConfig::new_for_test(&t.rt, "1000.0.0".parse()?);
    let analyze_results = t
        .isolate
        .analyze(
            udf_config,
            modules_by_path,
            BTreeMap::new(),
            DEV_INSTANCE_NAME.to_string(),
        )
        .await??;

    let source_package = SourcePackageModel::new(&mut tx, TableNamespace::test_user())
        .get_latest()
        .await?
        .unwrap();
    drop(tx);

    // Newer version + analyze results
    tx = t.database.begin(Identity::system()).await?;
    ConfigModel::new(&mut tx, ComponentId::test_user())
        .apply(
            config_metadata.clone(),
            module_configs.clone(),
            UdfConfig::new_for_test(&t.rt, post_internal_npm_version.clone()),
            Some(source_package.into_value()),
            analyze_results.clone(),
            None,
        )
        .await?;
    t.database.commit(tx).await?;

    tx = t.database.begin(Identity::Unknown).await?;

    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(internal_function.clone()),
        ConvexArray::empty(),
        UdfType::Mutation,
    )
    .await;
    must_let!(let Ok(Err(js_error)) = result);
    assert!(js_error
        .message
        .starts_with("Could not find public function for 'internal:myInternalMutation'"));

    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(public_function.clone()),
        ConvexArray::empty(),
        UdfType::Mutation,
    )
    .await;
    must_let!(let Ok(Ok(_)) = result);

    // Error message should be the same so we don't leak information about which
    // internal functions exist
    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(non_existent_function.clone()),
        ConvexArray::empty(),
        UdfType::Mutation,
    )
    .await;
    must_let!(let Ok(Err(js_error)) = result);
    assert!(js_error
        .message
        .starts_with("Could not find public function for 'internal:doesNotExist'"));

    // Calling query as a mutation should fail
    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(public_function.clone()),
        ConvexArray::empty(),
        UdfType::Query,
    )
    .await;
    must_let!(let Ok(Err(js_error)) = result);
    assert_eq!(
        js_error.message,
        "Trying to execute internal.js:publicMutation as Query, but it is defined as Mutation."
    );

    tx = t
        .database
        .begin(Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
            "happy-animal-123".to_string(),
            MemberId(123),
        )))
        .await?;

    // Admins should be allowed to call internal functions from public APIs
    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(internal_function.clone()),
        ConvexArray::empty(),
        UdfType::Mutation,
    )
    .await;
    must_let!(let Ok(Ok(_)) = result);

    // Calling a missing function should fail even as admin.
    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(non_existent_function.clone()),
        ConvexArray::empty(),
        UdfType::Mutation,
    )
    .await;
    must_let!(let Ok(Err(js_error)) = result);
    assert!(js_error
        .message
        .starts_with("Could not find public function for 'internal:doesNotExist'"));

    // Calling query as a mutation should fail even with admin.
    let result = ValidatedPathAndArgs::new(
        AllowedVisibility::PublicOnly,
        &mut tx,
        PublicFunctionPath::Component(public_function.clone()),
        ConvexArray::empty(),
        UdfType::Query,
    )
    .await;
    must_let!(let Ok(Err(js_error)) = result);
    assert_eq!(
        js_error.message,
        "Trying to execute internal.js:publicMutation as Query, but it is defined as Mutation."
    );

    Ok(())
}
