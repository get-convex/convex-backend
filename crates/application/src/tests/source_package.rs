use std::collections::BTreeMap;

use anyhow::Context;
use common::{
    components::{
        CanonicalizedComponentModulePath,
        ComponentDefinitionId,
    },
    runtime::Runtime,
    types::ModuleEnvironment,
};
use database::Transaction;
use keybroker::Identity;
use model::{
    config::{
        module_loader::TransactionModuleLoader,
        types::ModuleConfig,
    },
    modules::ModuleModel,
    source_packages::types::SourcePackage,
};
use node_executor::source_package::download_package;
use runtime::prod::ProdRuntime;

use crate::{
    test_helpers::ApplicationTestExt,
    tests::NODE_SOURCE,
    Application,
};
const SOURCE_MAP: &str = "{}";

#[convex_macro::prod_rt_test]
async fn test_source_package(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;

    let path = CanonicalizedComponentModulePath {
        component: ComponentDefinitionId::Root,
        module_path: "b.js".parse()?,
    };
    let config = ModuleConfig {
        path: path.as_root_module_path()?.clone().into(),
        source: NODE_SOURCE.to_owned(),
        source_map: Some(SOURCE_MAP.to_owned()),
        environment: ModuleEnvironment::Node,
    };
    let mut modules = BTreeMap::new();
    modules.insert(path.clone(), Some(config));
    let mut tx = application.begin(Identity::system()).await?;
    let package = assemble_package(&mut tx, modules).await?;

    let SourcePackage {
        storage_key,
        sha256,
        ..
    } = application
        .upload_package(&package, None)
        .await?
        .context("With functions should upload")?;

    let result =
        download_package(application.modules_storage().clone(), storage_key, sha256).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(&result[path.as_root_module_path()?].source, NODE_SOURCE);
    assert_eq!(
        result[&path.as_root_module_path()?]
            .source_map
            .as_ref()
            .map(|s| &s[..]),
        Some(SOURCE_MAP)
    );

    Ok(())
}

pub async fn assemble_package<RT: Runtime>(
    tx: &mut Transaction<RT>,
    modifications: BTreeMap<CanonicalizedComponentModulePath, Option<ModuleConfig>>,
) -> anyhow::Result<Vec<ModuleConfig>> {
    let existing_modules = ModuleModel::new(tx)
        .get_application_modules(ComponentDefinitionId::Root, &TransactionModuleLoader)
        .await?;
    let mut modules = BTreeMap::new();
    for (path, module) in existing_modules {
        if modifications.contains_key(&path) {
            continue;
        }
        anyhow::ensure!(modules.insert(path, module).is_none());
    }
    for (path, module_edit) in modifications {
        let module = match module_edit {
            Some(m) => m,
            None => continue,
        };
        anyhow::ensure!(modules.insert(path, module).is_none());
    }
    Ok(modules.into_values().collect())
}
