use std::collections::BTreeMap;

use common::{
    components::ComponentId,
    runtime::Runtime,
    types::ModuleEnvironment,
};
use database::Transaction;
use keybroker::Identity;
use model::{
    config::{
        module_loader::ModuleLoader,
        types::ModuleConfig,
    },
    modules::ModuleModel,
    source_packages::{
        types::SourcePackage,
        upload_download::download_package,
    },
};
use runtime::prod::ProdRuntime;
use sync_types::CanonicalizedModulePath;

use crate::{
    test_helpers::ApplicationTestExt,
    tests::NODE_SOURCE,
    Application,
};
const SOURCE_MAP: &str = "{}";

#[convex_macro::prod_rt_test]
async fn test_source_package(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;

    let path: CanonicalizedModulePath = "b.js".parse()?;
    let config = ModuleConfig {
        path: path.clone().into(),
        source: NODE_SOURCE.into(),
        source_map: Some(SOURCE_MAP.to_owned()),
        environment: ModuleEnvironment::Node,
    };
    let mut modules = BTreeMap::new();
    modules.insert(path.clone(), Some(config));
    let mut tx = application.begin(Identity::system()).await?;
    let package = assemble_package(&mut tx, application.modules_cache(), modules).await?;

    let SourcePackage {
        storage_key,
        sha256,
        ..
    } = application.upload_package(&package, None, None).await?;

    let result =
        download_package(application.modules_storage().clone(), storage_key, sha256).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(&*result[&path].source, NODE_SOURCE);
    assert_eq!(
        result[&path].source_map.as_ref().map(|s| &s[..]),
        Some(SOURCE_MAP)
    );

    Ok(())
}

pub async fn assemble_package<RT: Runtime>(
    tx: &mut Transaction<RT>,
    module_loader: &dyn ModuleLoader<RT>,
    modifications: BTreeMap<CanonicalizedModulePath, Option<ModuleConfig>>,
) -> anyhow::Result<Vec<ModuleConfig>> {
    let existing_modules = ModuleModel::new(tx)
        .get_application_modules(ComponentId::test_user(), module_loader)
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
