use std::sync::LazyLock;

use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    patch_value,
    SystemMetadataModel,
    Transaction,
};
use value::{
    ConvexValue,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;
use types::AuditLogConfig;

pub static AUDIT_LOG_CONFIG_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_audit_log_config"
        .parse()
        .expect("Invalid built-in _audit_log_config table")
});

pub struct AuditLogConfigTable;
impl SystemTable for AuditLogConfigTable {
    type Metadata = AuditLogConfig;

    fn table_name() -> &'static TableName {
        &AUDIT_LOG_CONFIG_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct AuditLogConfigModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> AuditLogConfigModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    /// Get the current audit log config, if one exists.
    pub async fn get(&mut self) -> anyhow::Result<Option<ParsedDocument<AuditLogConfig>>> {
        let result = self
            .tx
            .query_system(
                TableNamespace::Global,
                &SystemIndex::<AuditLogConfigTable>::by_id(),
            )?
            .unique()
            .await?
            .map(|arc_row| (*arc_row).clone());
        Ok(result)
    }

    /// Get the existing config row ID, or create a new row with defaults.
    async fn get_or_create(&mut self) -> anyhow::Result<ResolvedDocumentId> {
        if let Some(existing) = self.get().await? {
            Ok(existing.id())
        } else {
            let config = AuditLogConfig {
                firehose_stream_name: None,
                include_in_log_streams: false,
            };
            let id = SystemMetadataModel::new_global(self.tx)
                .insert(&AUDIT_LOG_CONFIG_TABLE, config.try_into()?)
                .await?;
            Ok(id)
        }
    }

    /// Set or unset the firehose stream name, creating the config row if
    /// needed.
    pub async fn set_firehose_stream_name(
        &mut self,
        firehose_stream_name: Option<String>,
    ) -> anyhow::Result<()> {
        let id = self.get_or_create().await?;
        let value = firehose_stream_name
            .map(ConvexValue::try_from)
            .transpose()?;
        SystemMetadataModel::new_global(self.tx)
            .patch(id, patch_value!("firehoseStreamName" => value)?)
            .await?;
        Ok(())
    }
}
