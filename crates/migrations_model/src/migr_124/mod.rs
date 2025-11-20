use common::runtime::Runtime;
use database::{
    SystemMetadataModel,
    Transaction,
};
use log_sinks::{
    types::{
        SinkConfig,
        SinkType,
    },
    LogSinksModel,
};
use value::TableNamespace;

use crate::migr_124::log_sinks::types::{
    webhook::generate_webhook_hmac_secret,
    LogSinksRow,
};

mod log_sinks;

pub async fn run_migration<RT: Runtime>(tx: &mut Transaction<RT>) -> anyhow::Result<()> {
    let Some(log_sink) = LogSinksModel::new(tx)
        .get_by_provider(SinkType::Webhook)
        .await?
    else {
        return Ok(());
    };
    let SinkConfig::Webhook(mut webhook_config) = log_sink.config.clone() else {
        return Ok(());
    };
    if webhook_config.hmac_secret.is_some() {
        return Ok(());
    }
    webhook_config.hmac_secret = Some(generate_webhook_hmac_secret(tx.runtime()));
    let log_sink_row = LogSinksRow {
        status: log_sink.status.clone(),
        config: SinkConfig::Webhook(webhook_config),
    };
    SystemMetadataModel::new(tx, TableNamespace::Global)
        .replace(log_sink.id(), log_sink_row.try_into()?)
        .await?;
    Ok(())
}
