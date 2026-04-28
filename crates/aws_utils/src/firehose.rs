use anyhow::Context;

use crate::must_config_from_env;

/// Singleton firehose client to share the connection pool / TLS connector
/// across instances.
static FIREHOSE_CLIENT: tokio::sync::OnceCell<aws_sdk_firehose::Client> =
    tokio::sync::OnceCell::const_new();

pub async fn firehose_client() -> anyhow::Result<aws_sdk_firehose::Client> {
    FIREHOSE_CLIENT
        .get_or_try_init(|| async {
            let config = must_config_from_env()
                .await
                .context("AWS env variables are required when using AWS Firehose")?
                .load()
                .await;
            let client = aws_sdk_firehose::Client::new(&config);
            anyhow::Ok(client)
        })
        .await
        .cloned()
}
