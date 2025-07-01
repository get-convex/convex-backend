use std::sync::{
    Arc,
    LazyLock,
};

use cmd_util::env::env_config;
use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    backend_info::types::BackendInfoPersisted,
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static BACKEND_INFO_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_backend_info"
        .parse()
        .expect("Invalid built-in backend_info table")
});

pub struct BackendInfoTable;
impl SystemTable for BackendInfoTable {
    type Metadata = BackendInfoPersisted;

    fn table_name() -> &'static TableName {
        &BACKEND_INFO_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct BackendInfoModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> BackendInfoModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get(
        &mut self,
    ) -> anyhow::Result<Option<Arc<ParsedDocument<BackendInfoPersisted>>>> {
        self.tx
            .query_system(
                TableNamespace::Global,
                &SystemIndex::<BackendInfoTable>::by_id(),
            )?
            .unique()
            .await
    }

    pub async fn set(&mut self, backend_info: BackendInfoPersisted) -> anyhow::Result<()> {
        let persisted_backend_info = self.get().await?;
        match persisted_backend_info {
            Some(doc) => {
                SystemMetadataModel::new_global(self.tx)
                    .replace(doc.id().to_owned(), backend_info.try_into()?)
                    .await?;
            },
            None => {
                SystemMetadataModel::new_global(self.tx)
                    .insert(&BACKEND_INFO_TABLE, backend_info.try_into()?)
                    .await?;
            },
        };
        Ok(())
    }

    pub async fn ensure_streaming_export_enabled(&mut self) -> anyhow::Result<()> {
        // For debugging locally, you can set CONVEX_ENABLE_STREAMING_EXPORT=true
        if env_config("CONVEX_ENABLE_STREAMING_EXPORT", false) {
            return Ok(());
        }
        if self.tx.identity().is_system() {
            return Ok(());
        }

        let backend_info = self.get().await?;
        anyhow::ensure!(
            backend_info
                .map(|bi| bi.streaming_export_enabled)
                .unwrap_or_default(),
            ErrorMetadata::forbidden(
                "StreamingExportNotEnabled",
                "Streaming export is only available on the Convex Professional plan. See https://www.convex.dev/plans to upgrade.",
            ),
        );
        Ok(())
    }

    pub async fn is_log_streaming_allowed(&mut self) -> anyhow::Result<bool> {
        let backend_info = self.get().await?;
        // Log streaming is allowed on local-deployments.
        Ok(backend_info
            .map(|bi| bi.log_streaming_enabled)
            .unwrap_or(true))
    }

    pub async fn ensure_log_streaming_allowed(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.is_log_streaming_allowed().await?,
            ErrorMetadata::forbidden(
                "LogStreamingNotEnabled",
                "Log streaming is only available on the Convex Professional plan. See https://www.convex.dev/plans to upgrade."
            )
        );
        Ok(())
    }
}
