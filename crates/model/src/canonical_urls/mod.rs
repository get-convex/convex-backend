use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    http::RequestDestination,
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
    types::TableName,
};
use database::{
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::TableNamespace;

use self::types::CanonicalUrl;
use crate::SystemTable;

pub mod types;

pub static CANONICAL_URLS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_canonical_urls"
        .parse()
        .expect("Invalid built-in table name")
});

pub struct CanonicalUrlsTable;

impl SystemTable for CanonicalUrlsTable {
    fn table_name(&self) -> &'static TableName {
        &CANONICAL_URLS_TABLE
    }

    fn indexes(&self) -> Vec<crate::SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<CanonicalUrl>::try_from(document).map(|_| ())
    }
}

pub struct CanonicalUrlsModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> CanonicalUrlsModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get_canonical_urls(
        &mut self,
    ) -> anyhow::Result<BTreeMap<RequestDestination, ParsedDocument<CanonicalUrl>>> {
        let query = Query::full_table_scan(CANONICAL_URLS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let mut canonical_urls = BTreeMap::new();
        while let Some(document) = query_stream.next(self.tx, None).await? {
            let canonical_url = ParsedDocument::<CanonicalUrl>::try_from(document)?;
            canonical_urls.insert(canonical_url.request_destination, canonical_url);
        }
        Ok(canonical_urls)
    }

    pub async fn set_canonical_url(
        &mut self,
        request_destination: RequestDestination,
        url: String,
    ) -> anyhow::Result<()> {
        if let Some(existing_canonical_url) =
            self.get_canonical_urls().await?.get(&request_destination)
        {
            if existing_canonical_url.url == url {
                // Url isn't changing, so no-op.
                return Ok(());
            } else {
                // Delete the existing canonical url.
                SystemMetadataModel::new_global(self.tx)
                    .delete(existing_canonical_url.id())
                    .await?;
            }
        }
        let canonical_url = CanonicalUrl {
            request_destination,
            url,
        };
        SystemMetadataModel::new_global(self.tx)
            .insert(&CANONICAL_URLS_TABLE, canonical_url.try_into()?)
            .await?;
        Ok(())
    }
}
