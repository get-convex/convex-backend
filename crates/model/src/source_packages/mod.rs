use std::sync::LazyLock;

use anyhow::Context;
use common::{
    components::ComponentId,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    runtime::Runtime,
};
use database::{
    SystemMetadataModel,
    Transaction,
};
use value::{
    id_v6::DeveloperDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    modules::ModuleModel,
    source_packages::types::{
        SourcePackage,
        SourcePackageId,
    },
    SystemIndex,
    SystemTable,
};

pub mod types;
pub mod upload_download;

pub static SOURCE_PACKAGES_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_source_packages"
        .parse()
        .expect("invalid built-in source_packages table")
});

pub struct SourcePackagesTable;
impl SystemTable for SourcePackagesTable {
    fn table_name(&self) -> &'static TableName {
        &SOURCE_PACKAGES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<SourcePackage>::try_from(document).map(|_| ())
    }
}

pub struct SourcePackageModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SourcePackageModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    pub async fn put(&mut self, source_package: SourcePackage) -> anyhow::Result<SourcePackageId> {
        let document_id = SystemMetadataModel::new(self.tx, self.namespace)
            .insert(&SOURCE_PACKAGES_TABLE, source_package.try_into()?)
            .await?;
        let id: DeveloperDocumentId = document_id.into();
        Ok(id.into())
    }

    pub async fn get(
        &mut self,
        source_package_id: SourcePackageId,
    ) -> anyhow::Result<ParsedDocument<SourcePackage>> {
        let id: DeveloperDocumentId = source_package_id.into();
        let document_id = id.to_resolved(
            &self
                .tx
                .table_mapping()
                .namespace(self.namespace)
                .inject_table_id(),
        )?;
        self.tx
            .get(document_id)
            .await?
            .context("Couldn't find source package")?
            .try_into()
    }

    pub async fn get_latest(&mut self) -> anyhow::Result<Option<ParsedDocument<SourcePackage>>> {
        let mut source_package_ids = vec![];

        for module in ModuleModel::new(self.tx)
            .get_all_metadata(ComponentId::TODO())
            .await?
        {
            source_package_ids.push(module.source_package_id);
        }

        // If there are no modules - then return None
        let Some(source_package_id) = source_package_ids.pop() else {
            return Ok(None);
        };

        // They should all match
        anyhow::ensure!(source_package_ids
            .into_iter()
            .all(|id| &id == &source_package_id));

        Ok(Some(self.get(source_package_id).await?))
    }
}
