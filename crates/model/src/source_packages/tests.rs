#[convex_macro::test_runtime]
async fn test_source_package(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;

    // Initialize config with a source package
    let mut tx = database.begin(Identity::system()).await?;
    let config_metadata = ConfigMetadata::test_example();
    let module = ModuleConfig {
        path: "a/b/c.js".parse()?,
        source: "// some js".to_string(),
        source_map: None,
        environment: ModuleEnvironment::Isolate,
    };
    let source_package = SourcePackage {
        storage_key: "sk".try_into()?,
        sha256: Sha256::hash(b"sk"),
    };
    tx.apply_config(
        config_metadata,
        vec![module.clone()],
        "1000.0.0".parse()?,
        Some(source_package.clone()),
        btreemap! {
            module.path.clone().canonicalize() => AnalyzedModule{
                functions: vec![],
                http_routes: None,
                cron_specs: None,
                source_mapped: None,
            }
        },
        None,
    )
    .await?;
    database.commit(tx).await?;

    // Read the source package back out
    let mut tx = database.begin(Identity::system()).await?;
    let module_metadata = ModuleModel::new(&mut tx)
        .get_module_metadata(module.path.canonicalize())
        .await
        .expect("getting module should succeed")
        .expect("Should find the module");
    assert_eq!(module_metadata.latest_version, 0);
    let module_version = ModuleModel::new(&mut tx)
        .get_module_version(module_metadata.id().clone(), module_metadata.latest_version)
        .await
        .expect("getting module version should succeed");
    assert_eq!(module_version.source, module.source);
    assert_eq!(module_version.source_map, module.source_map);
    assert_eq!(module_version.version, 0);
    assert!(module_version.source_package_id.is_some());
    let source_package_read = tx
        .get_source_package(module_version.source_package_id.clone().unwrap())
        .await?;
    assert_eq!(source_package_read.into_value(), source_package);

    Ok(())
}
