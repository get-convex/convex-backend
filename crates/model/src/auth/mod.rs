use std::{
    collections::BTreeSet,
    sync::LazyLock,
};

use common::{
    auth::AuthInfo,
    document::{
        ParseDocument,
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
};
use database::{
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    TableName,
    TableNamespace,
};

use self::types::AuthInfoPersisted;
use crate::{
    auth::types::AuthDiff,
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static AUTH_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_auth".parse().expect("Invalid built-in auth table"));

pub struct AuthTable;
impl SystemTable for AuthTable {
    fn table_name(&self) -> &'static TableName {
        &AUTH_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParseDocument::<AuthInfoPersisted>::parse(document).map(|_| ())
    }
}

pub struct AuthInfoModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> AuthInfoModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[fastrace::trace]
    pub async fn put(&mut self, auth_infos: Vec<AuthInfo>) -> anyhow::Result<AuthDiff> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("put_auth_config"));
        }
        // Read out existing info first so we can diff it.
        let existing: Vec<ParsedDocument<AuthInfo>> = self.get_inner().await?;
        // Put the new data into a BTreeSet.
        let mut new_set = auth_infos.into_iter().collect::<BTreeSet<_>>();
        let mut to_delete = vec![];

        let mut removed_auth_infos: BTreeSet<AuthInfo> = BTreeSet::new();

        // After this loop, `new_set` will contain only values that don't already exist
        // in the database, and `to_delete` will contain the ids of documents that need
        // to be deleted.
        for info in existing.into_iter() {
            if !new_set.remove(&info) {
                to_delete.push(info.id());
                removed_auth_infos.insert(info.into_value());
            }
        }
        // Make the changes.
        for id in to_delete {
            SystemMetadataModel::new_global(self.tx).delete(id).await?;
        }
        for info in new_set.clone().into_iter() {
            SystemMetadataModel::new_global(self.tx)
                .insert(&AUTH_TABLE, AuthInfoPersisted(info).try_into()?)
                .await?;
        }
        AuthDiff::new(new_set, removed_auth_infos)
    }

    pub async fn get(&mut self) -> anyhow::Result<Vec<ParsedDocument<AuthInfo>>> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("get_auth_config"));
        }
        self.get_inner().await
    }

    async fn get_inner(&mut self) -> anyhow::Result<Vec<ParsedDocument<AuthInfo>>> {
        let auth_query = Query::full_table_scan(AUTH_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, auth_query)?;
        let mut auth_infos = vec![];
        while let Some(auth_value) = query_stream.next(self.tx, None).await? {
            let parsed: ParsedDocument<AuthInfoPersisted> = auth_value.parse()?;
            auth_infos.push(parsed.map(|ai| Ok(ai.0))?);
        }
        Ok(auth_infos)
    }
}
