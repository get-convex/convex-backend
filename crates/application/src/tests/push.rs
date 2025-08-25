use common::knobs::MAX_USER_MODULES;
use keybroker::Identity;
use model::{
    config::types::ConfigFile,
    source_packages::{
        types::NodeVersion,
        SourcePackageModel,
    },
};
use runtime::testing::TestRuntime;
use value::TableNamespace;

use crate::{
    deploy_config::{
        AppDefinitionConfigJson,
        ModuleJson,
        StartPushRequest,
    },
    test_helpers::ApplicationTestExt as _,
    Application,
};

fn make_modules() -> Vec<ModuleJson> {
    let mut functions: Vec<_> = (0..*MAX_USER_MODULES)
        .map(|i| ModuleJson {
            environment: None,
            source_map: None,
            path: format!("mod{i}.js"),
            source: format!("// {i}"),
        })
        .collect();
    functions.extend((0..*MAX_USER_MODULES).map(|i| ModuleJson {
        environment: None,
        source_map: None,
        path: format!("_deps/mod{i}.js"),
        source: format!("// dep {i}"),
    }));
    functions
}

#[convex_macro::test_runtime]
async fn test_max_size_push(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    for _ in 0..2 {
        application
            .run_test_push(StartPushRequest {
                admin_key: "".into(),
                dry_run: false,
                functions: "convex/".into(),
                app_definition: AppDefinitionConfigJson {
                    definition: None,
                    dependencies: vec![],
                    schema: None,
                    functions: make_modules(),
                    udf_server_version: "1.3939.3939".into(),
                },
                component_definitions: vec![],
                node_dependencies: vec![],
                node_version: None,
            })
            .await?;
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_max_size_push_no_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    for _ in 0..2 {
        application
            .push_config_no_components(
                Identity::system(),
                ConfigFile {
                    auth_info: None,
                    functions: "convex/".into(),
                },
                make_modules()
                    .into_iter()
                    .map(|m| m.try_into().unwrap())
                    .collect(),
                "1.3939.3939".parse().unwrap(),
                None,
                None,
                None,
            )
            .await?;
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_change_node_version(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let node_version = SourcePackageModel::new(&mut tx, TableNamespace::Global)
        .get_latest()
        .await?
        .and_then(|p| p.node_version);
    assert!(node_version.is_none());

    application
        .run_test_push(StartPushRequest {
            admin_key: "".into(),
            dry_run: false,
            functions: "convex/".into(),
            app_definition: AppDefinitionConfigJson {
                definition: None,
                dependencies: vec![],
                schema: None,
                functions: make_modules(),
                udf_server_version: "1.3939.3939".into(),
            },
            component_definitions: vec![],
            node_dependencies: vec![],
            node_version: Some("22".to_string()),
        })
        .await?;

    let mut tx = application.begin(Identity::system()).await?;
    let node_version = SourcePackageModel::new(&mut tx, TableNamespace::Global)
        .get_latest()
        .await?
        .and_then(|p| p.node_version);
    assert_eq!(node_version, Some(NodeVersion::V22x));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_change_node_version_no_components(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let node_version = SourcePackageModel::new(&mut tx, TableNamespace::Global)
        .get_latest()
        .await?
        .and_then(|p| p.node_version);
    assert!(node_version.is_none());

    application
        .push_config_no_components(
            Identity::system(),
            ConfigFile {
                auth_info: None,
                functions: "convex/".into(),
            },
            make_modules()
                .into_iter()
                .map(|m| m.try_into().unwrap())
                .collect(),
            "1.3939.3939".parse().unwrap(),
            None,
            None,
            Some(NodeVersion::V22x),
        )
        .await?;

    let mut tx = application.begin(Identity::system()).await?;
    let node_version = SourcePackageModel::new(&mut tx, TableNamespace::Global)
        .get_latest()
        .await?
        .and_then(|p| p.node_version);
    assert_eq!(node_version, Some(NodeVersion::V22x));
    Ok(())
}
