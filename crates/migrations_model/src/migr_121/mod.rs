use common::runtime::Runtime;
use database::{
    SystemMetadataModel,
    Transaction,
};
use log_sinks::LogSinksModel;
use value::TableNamespace;

mod log_sinks;

pub async fn run_migration<RT: Runtime>(tx: &mut Transaction<RT>) -> anyhow::Result<()> {
    // Read everything in and back out to fill in `format` field.
    let all_log_sinks = LogSinksModel::new(tx).get_all().await?;
    for log_sink in all_log_sinks {
        SystemMetadataModel::new(tx, TableNamespace::Global)
            .replace(log_sink.id(), log_sink.into_value().try_into()?)
            .await?;
    }
    Ok(())
}
