use common::runtime::Runtime;
use database::Database;
use errors::ErrorMetadata;
use keybroker::Identity;
use model::{
    backend_info::BackendInfoModel,
    log_sinks::{
        types::{
            SinkConfig,
            SinkType,
        },
        LogSinksModel,
    },
};

use crate::Application;

pub async fn add_local_log_sink_on_startup<RT: Runtime>(
    db: Database<RT>,
    path: String,
) -> anyhow::Result<()> {
    let mut tx = db.begin(Identity::system()).await?;
    let mut log_sink_model = LogSinksModel::new(&mut tx);
    log_sink_model
        .add_on_startup(SinkConfig::Local(path.clone()))
        .await?;
    db.commit_with_write_source(tx, "add_local_log_sink_startup")
        .await?;
    tracing::info!("Local log sink configured at {path}.");
    Ok(())
}

impl<RT: Runtime> Application<RT> {
    pub async fn add_log_sink(&self, config: SinkConfig) -> anyhow::Result<()> {
        let mut tx = self.begin(Identity::system()).await?;
        let mut model = LogSinksModel::new(&mut tx);
        model.add_or_update(config).await?;
        self.commit(tx, "add_log_sink").await?;
        Ok(())
    }

    pub async fn remove_log_sink(&self, sink_type: SinkType) -> anyhow::Result<()> {
        let mut tx = self.begin(Identity::system()).await?;
        let mut model = LogSinksModel::new(&mut tx);

        let Some(row) = model.get_by_provider(sink_type.clone()).await? else {
            return Err(ErrorMetadata::bad_request(
                "SinkDoesntExist",
                "Cannot remove a sink that is not configured for this project.",
            )
            .into());
        };

        model.mark_for_removal(row.id()).await?;
        self.commit(tx, "remove_log_sink").await?;

        Ok(())
    }

    pub async fn ensure_log_streaming_allowed(&self, identity: Identity) -> anyhow::Result<()> {
        let mut tx = self.begin(identity).await?;
        BackendInfoModel::new(&mut tx)
            .ensure_log_streaming_allowed()
            .await
    }
}
