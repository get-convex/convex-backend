use std::sync::LazyLock;

use common::{
    document::{
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

pub mod types;
use types::UdfConfig;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    config::types::UdfServerVersionDiff,
    SystemIndex,
    SystemTable,
};

pub static UDF_CONFIG_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_udf_config"
        .parse()
        .expect("Invalid built-in UDF config table")
});

pub struct UdfConfigTable;
impl SystemTable for UdfConfigTable {
    fn table_name(&self) -> &'static TableName {
        &UDF_CONFIG_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<UdfConfig>::try_from(document).map(|_| ())
    }
}

pub struct UdfConfigModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> UdfConfigModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get(&mut self) -> anyhow::Result<Option<ParsedDocument<UdfConfig>>> {
        let index_query = Query::full_table_scan(UDF_CONFIG_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, index_query)?;
        let config = query_stream
            .expect_at_most_one(self.tx)
            .await?
            .map(|document| document.try_into())
            .transpose()?;
        Ok(config)
    }

    pub async fn set(
        &mut self,
        new_config: UdfConfig,
    ) -> anyhow::Result<Option<UdfServerVersionDiff>> {
        if !(self.tx.identity().is_admin() || self.tx.identity().is_system()) {
            anyhow::bail!(unauthorized_error("set_udf_config"));
        }
        let new_server_version = new_config.server_version.clone();
        let value = new_config.try_into()?;

        let existing_doc = self.get().await?;
        let opt_previous_version = if let Some(existing_doc) = existing_doc {
            SystemMetadataModel::new(self.tx)
                .replace(existing_doc.id(), value)
                .await?;
            Some(existing_doc.into_value().server_version)
        } else {
            SystemMetadataModel::new(self.tx)
                .insert(&UDF_CONFIG_TABLE, value)
                .await?;
            None
        };

        let version_diff = match opt_previous_version {
            Some(previous_version) => {
                if previous_version != new_server_version {
                    Some(UdfServerVersionDiff {
                        previous_version: previous_version.to_string(),
                        next_version: new_server_version.to_string(),
                    })
                } else {
                    None
                }
            },
            None => Some(UdfServerVersionDiff {
                previous_version: "Unspecified version".to_string(),
                next_version: new_server_version.to_string(),
            }),
        };
        Ok(version_diff)
    }
}
