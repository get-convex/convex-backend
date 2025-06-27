use std::sync::{
    Arc,
    LazyLock,
};

use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    unauthorized_error,
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
    type Metadata = UdfConfig;

    fn table_name() -> &'static TableName {
        &UDF_CONFIG_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct UdfConfigModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> UdfConfigModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    pub async fn get(&mut self) -> anyhow::Result<Option<Arc<ParsedDocument<UdfConfig>>>> {
        let config = self
            .tx
            .query_system(self.namespace, &SystemIndex::<UdfConfigTable>::by_id())?
            .unique()
            .await?;
        Ok(config)
    }

    #[fastrace::trace]
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
            SystemMetadataModel::new(self.tx, self.namespace)
                .replace(existing_doc.id(), value)
                .await?;
            Some(existing_doc.server_version.clone())
        } else {
            SystemMetadataModel::new(self.tx, self.namespace)
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
