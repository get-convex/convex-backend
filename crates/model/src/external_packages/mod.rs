use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use anyhow::Context;
use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        IndexName,
        NodeDependency,
    },
};
use database::{
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    id_v6::DeveloperDocumentId,
    TableName,
    TableNamespace,
};

use self::types::{
    ExternalDepsPackage,
    ExternalDepsPackageId,
};
use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

const NUM_EXTERNAL_DEPS_CACHE_ENTRIES: usize = 10;

pub static EXTERNAL_PACKAGES_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_external_deps_packages"
        .parse()
        .expect("invalid built-in _external_packages table")
});

pub struct ExternalPackagesTable;
impl SystemTable for ExternalPackagesTable {
    fn table_name(&self) -> &'static TableName {
        &EXTERNAL_PACKAGES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<ExternalDepsPackage>::try_from(document).map(|_| ())
    }
}

pub struct ExternalPackagesModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ExternalPackagesModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[minitrace::trace]
    pub async fn get(
        &mut self,
        external_deps_package_id: ExternalDepsPackageId,
    ) -> anyhow::Result<ParsedDocument<ExternalDepsPackage>> {
        let id: DeveloperDocumentId = external_deps_package_id.into();
        let document_id = id.to_resolved(
            &self
                .tx
                .table_mapping()
                .namespace(TableNamespace::Global)
                .number_to_tablet(),
        )?;
        self.tx
            .get(document_id)
            .await?
            .context("Couldn't find external package")?
            .try_into()
    }

    pub async fn put(
        &mut self,
        external_deps_package: ExternalDepsPackage,
    ) -> anyhow::Result<ExternalDepsPackageId> {
        let id = SystemMetadataModel::new_global(self.tx)
            .insert(&EXTERNAL_PACKAGES_TABLE, external_deps_package.try_into()?)
            .await?;
        let doc_id: DeveloperDocumentId = id.into();
        Ok(doc_id.into())
    }

    #[minitrace::trace]
    pub async fn get_cached_package_match(
        &mut self,
        deps: Vec<NodeDependency>,
    ) -> anyhow::Result<Option<(ExternalDepsPackageId, ExternalDepsPackage)>> {
        let index_query = Query::index_range(IndexRange {
            index_name: IndexName::by_creation_time(EXTERNAL_PACKAGES_TABLE.clone()),
            range: vec![],
            order: Order::Desc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, index_query)?;
        let deps_map: BTreeMap<String, String> = deps
            .into_iter()
            .map(|dep| (dep.package, dep.version))
            .collect();

        // Check at most NUM_EXTERNAL_DEPS_CACHE_ENTRIES entries for a match
        let mut cache_entries_checked = 0;
        while let Some(doc) = query_stream.next(self.tx, None).await?
            && cache_entries_checked < NUM_EXTERNAL_DEPS_CACHE_ENTRIES
        {
            let row: ParsedDocument<ExternalDepsPackage> = doc.try_into()?;
            let (id, pkg) = row.into_id_and_value();

            let pkg_deps_map: BTreeMap<String, String> = pkg
                .deps
                .clone()
                .into_iter()
                .map(|dep| (dep.package, dep.version))
                .collect();
            if pkg_deps_map.eq(&deps_map) {
                return Ok(Some((DeveloperDocumentId::from(id).into(), pkg)));
            }

            cache_entries_checked += 1;
        }
        Ok(None)
    }
}
