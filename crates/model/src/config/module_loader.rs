use std::sync::Arc;

use async_trait::async_trait;
use common::{
    components::CanonicalizedComponentModulePath,
    document::ParsedDocument,
    runtime::Runtime,
};
use database::Transaction;

use crate::modules::{
    module_versions::FullModuleSource,
    types::ModuleMetadata,
    ModuleModel,
};

#[async_trait]
pub trait ModuleLoader<RT: Runtime>: Sync + Send + 'static {
    async fn get_module_with_metadata(
        &self,
        tx: &mut Transaction<RT>,
        module_metadata: ParsedDocument<ModuleMetadata>,
    ) -> anyhow::Result<Arc<FullModuleSource>>;

    async fn get_module(
        &self,
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentModulePath,
    ) -> anyhow::Result<Option<Arc<FullModuleSource>>> {
        let module_metadata = match ModuleModel::new(tx).get_metadata(path).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        self.get_module_with_metadata(tx, module_metadata)
            .await
            .map(Some)
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod test_module_loader {
    use std::sync::Arc;

    use anyhow::Context;
    use async_trait::async_trait;
    use common::{
        document::ParsedDocument,
        runtime::Runtime,
    };
    use database::Transaction;
    use storage::Storage;

    use super::ModuleLoader;
    use crate::{
        modules::{
            module_versions::FullModuleSource,
            types::ModuleMetadata,
        },
        source_packages::{
            upload_download::download_package,
            SourcePackageModel,
        },
    };

    // Loads module versions directly from storage.
    pub struct UncachedModuleLoader {
        pub modules_storage: Arc<dyn Storage>,
    }

    #[async_trait]
    impl<RT: Runtime> ModuleLoader<RT> for UncachedModuleLoader {
        async fn get_module_with_metadata(
            &self,
            tx: &mut Transaction<RT>,
            module_metadata: ParsedDocument<ModuleMetadata>,
        ) -> anyhow::Result<Arc<FullModuleSource>> {
            let source_package_id = module_metadata.source_package_id;
            let namespace = tx
                .table_mapping()
                .tablet_namespace(module_metadata.id().table().tablet_id)?;
            let source_package = SourcePackageModel::new(tx, namespace)
                .get(source_package_id)
                .await?
                .into_value();
            let package = download_package(
                self.modules_storage.clone(),
                source_package.storage_key,
                source_package.sha256,
            )
            .await?;
            let module_config = package
                .get(&module_metadata.path)
                .with_context(|| format!("Missing module source {}", module_metadata.id()))?;
            Ok(Arc::new(FullModuleSource {
                source: module_config.source.clone(),
                source_map: module_config.source_map.clone(),
            }))
        }
    }
}
