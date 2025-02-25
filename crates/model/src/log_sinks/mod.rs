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
    patch_value,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
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
use types::{
    LogSinksRow,
    SinkConfig,
    SinkState,
    SinkType,
    LOG_SINKS_LIMIT,
};

pub static LOG_SINKS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_log_sinks"
        .parse()
        .expect("Invalid built-in _log_sinks table")
});

pub struct LogSinksTable;
impl SystemTable for LogSinksTable {
    fn table_name(&self) -> &'static TableName {
        &LOG_SINKS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<LogSinksRow>::try_from(document).map(|_| ())
    }
}

pub struct LogSinksModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> LogSinksModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get_by_provider(
        &mut self,
        provider: SinkType,
    ) -> anyhow::Result<Option<ParsedDocument<LogSinksRow>>> {
        let mut result: Vec<_> = self
            .get_by_provider_including_tombstoned(provider.clone())
            .await?
            .into_iter()
            .filter(|doc| doc.status != SinkState::Tombstoned)
            .collect();
        anyhow::ensure!(
            result.len() <= 1,
            "Multiple sinks found of the same type: {:?}",
            provider
        );
        Ok(result.pop())
    }

    async fn get_by_provider_including_tombstoned(
        &mut self,
        provider: SinkType,
    ) -> anyhow::Result<Vec<ParsedDocument<LogSinksRow>>> {
        let result: Vec<_> = self
            .get_all()
            .await?
            .into_iter()
            .filter(|doc| doc.config.sink_type() == provider)
            .collect();
        Ok(result)
    }

    pub async fn get_all(&mut self) -> anyhow::Result<Vec<ParsedDocument<LogSinksRow>>> {
        let mut result: Vec<_> = vec![];

        let value_query = Query::full_table_scan(LOG_SINKS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, value_query)?;
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            let row: ParsedDocument<LogSinksRow> = doc.try_into()?;
            result.push(row);
        }

        Ok(result)
    }

    pub async fn patch_status(
        &mut self,
        id: ResolvedDocumentId,
        status: SinkState,
    ) -> anyhow::Result<()> {
        SystemMetadataModel::new_global(self.tx)
            .patch(
                id,
                patch_value!("status" => Some(ConvexValue::Object(status.try_into()?)))?,
            )
            .await?;
        Ok(())
    }

    pub async fn mark_for_removal(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        self.patch_status(id, SinkState::Tombstoned).await?;
        Ok(())
    }

    pub async fn add_or_update(&mut self, config: SinkConfig) -> anyhow::Result<()> {
        let sink_type = config.sink_type();
        let row = LogSinksRow {
            status: SinkState::Pending,
            config,
        };

        // Filter to non-tombstoned log sinks
        let sinks = self
            .get_all()
            .await?
            .into_iter()
            .filter(|row| row.status != SinkState::Tombstoned)
            .collect::<Vec<_>>();
        if sinks.len() >= LOG_SINKS_LIMIT {
            return Err(ErrorMetadata::bad_request(
                "LogSinkQuotaExceeded",
                "Cannot add more LogSinks, the quota for this project has been reached.",
            )
            .into());
        }

        if let Some(row) = self.get_by_provider(sink_type.clone()).await? {
            self.mark_for_removal(row.id()).await?;
        }

        SystemMetadataModel::new_global(self.tx)
            .insert(&LOG_SINKS_TABLE, row.try_into()?)
            .await?;
        Ok(())
    }

    // It's generally not safe to delete an existing sink without marking it
    // Tombstoned first since the LogManager will not know to remove the sink.
    // However, we can do this during startup before the LogManager has started
    // (like when adding a local log sink)
    pub async fn add_on_startup(&mut self, config: SinkConfig) -> anyhow::Result<()> {
        // Search for matching provider
        if let Some(sink) = self.get_by_provider(config.sink_type()).await? {
            SystemMetadataModel::new_global(self.tx)
                .delete(sink.id())
                .await?;
        };
        self.add_or_update(config).await?;
        Ok(())
    }

    pub async fn clear(&mut self) -> anyhow::Result<()> {
        let providers = self.get_all().await?;

        for sink in providers {
            self.patch_status(sink.id(), SinkState::Tombstoned).await?;
        }
        Ok(())
    }
}
