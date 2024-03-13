use std::sync::LazyLock;

use anyhow::Context;
use common::{
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
    id_v6::DocumentIdV6,
    TableName,
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
}

impl<'a, RT: Runtime> SourcePackageModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn put(&mut self, source_package: SourcePackage) -> anyhow::Result<SourcePackageId> {
        let document_id = SystemMetadataModel::new(self.tx)
            .insert(&SOURCE_PACKAGES_TABLE, source_package.try_into()?)
            .await?;
        let id: DocumentIdV6 = document_id.try_into()?;
        Ok(id.into())
    }

    pub async fn get(
        &mut self,
        source_package_id: SourcePackageId,
    ) -> anyhow::Result<ParsedDocument<SourcePackage>> {
        let id: DocumentIdV6 = source_package_id.into();
        let document_id = id.to_resolved(&self.tx.table_mapping().inject_table_id())?;
        self.tx
            .get(document_id)
            .await?
            .context("Couldn't find source package")?
            .try_into()
    }

    pub async fn get_latest(&mut self) -> anyhow::Result<Option<ParsedDocument<SourcePackage>>> {
        let mut source_package_ids = vec![];

        for module in ModuleModel::new(self.tx).get_all_metadata().await? {
            let module_version = ModuleModel::new(self.tx)
                .get_version(module.id(), module.latest_version)
                .await?
                .into_value();
            source_package_ids.push(module_version.source_package_id);
        }

        // If there are no modules, or if all the modules lack a source_package_id -
        // then return None
        let Some(Some(source_package_id)) = source_package_ids.pop() else {
            return Ok(None);
        };

        // They should all match
        anyhow::ensure!(source_package_ids
            .into_iter()
            .all(|id| id.as_ref() == Some(&source_package_id)));

        Ok(Some(self.get(source_package_id).await?))
    }
}
