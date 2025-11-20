use std::sync::LazyLock;

use common::{
    self,
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    system_tables::{
        SystemIndex,
        SystemTable,
    },
    Transaction,
};
use value::{
    TableName,
    TableNamespace,
};

pub mod types;
use types::{
    LogSinksRow,
    SinkState,
    SinkType,
};

pub static LOG_SINKS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_log_sinks"
        .parse()
        .expect("Invalid built-in _log_sinks table")
});

pub struct LogSinksTable;
impl SystemTable for LogSinksTable {
    type Metadata = LogSinksRow;

    fn table_name() -> &'static TableName {
        &LOG_SINKS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
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
        let result = self
            .tx
            .query_system(
                TableNamespace::Global,
                &SystemIndex::<LogSinksTable>::by_id(),
            )?
            .all()
            .await?
            .into_iter()
            .map(|arc_row| (*arc_row).clone())
            .collect();
        Ok(result)
    }
}
