use common::{
    document::{
        ParseDocument,
        ParsedDocument,
    },
    runtime::Runtime,
};
use database::Database;
use errors::ErrorMetadata;
use futures::FutureExt;
use keybroker::Identity;
use model::{
    backend_info::BackendInfoModel,
    log_sinks::{
        types::{
            LogSinksRow,
            SinkConfig,
            SinkState,
            SinkType,
        },
        LogSinksModel,
    },
};
use usage_tracking::FunctionUsageTracker;
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
    TableNamespace,
};

use crate::Application;

pub struct LogSinkWithId {
    pub id: ResolvedDocumentId,
    pub config: SinkConfig,
    pub status: SinkState,
}

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
    pub async fn add_log_sink(&self, config: SinkConfig) -> anyhow::Result<ResolvedDocumentId> {
        let (_ts, id) = self
            .execute_with_occ_retries(
                Identity::system(),
                FunctionUsageTracker::new(),
                "add_log_sink",
                |tx| {
                    let config = config.clone();
                    async move { LogSinksModel::new(tx).add_or_update(config).await }
                        .boxed()
                        .into()
                },
            )
            .await?;
        Ok(id)
    }

    pub async fn patch_log_sink_config(
        &self,
        id: &String,
        config: SinkConfig,
    ) -> anyhow::Result<()> {
        let developer_id = DeveloperDocumentId::decode(id).map_err(|_| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidLogStreamId",
                "The log stream id is invalid"
            ))
        })?;
        self.execute_with_occ_retries(
            Identity::system(),
            FunctionUsageTracker::new(),
            "patch_log_sink_config",
            |tx| {
                let config = config.clone();
                async move {
                    let id = tx.resolve_developer_id(&developer_id, TableNamespace::Global)?;
                    LogSinksModel::new(tx).patch_config(id, config).await
                }
                .boxed()
                .into()
            },
        )
        .await?;
        Ok(())
    }

    pub async fn reset_log_sink_to_pending(&self, id: &String) -> anyhow::Result<()> {
        let developer_id = DeveloperDocumentId::decode(id).map_err(|_| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidLogStreamId",
                "The log stream id is invalid"
            ))
        })?;
        self.execute_with_occ_retries(
            Identity::system(),
            FunctionUsageTracker::new(),
            "reset_log_sink_to_pending",
            |tx| {
                async move {
                    let id = tx.resolve_developer_id(&developer_id, TableNamespace::Global)?;
                    LogSinksModel::new(tx)
                        .patch_status(id, SinkState::Pending)
                        .await
                }
                .boxed()
                .into()
            },
        )
        .await?;
        Ok(())
    }

    pub async fn get_log_sink(&self, sink_type: &SinkType) -> anyhow::Result<Option<SinkConfig>> {
        let mut tx = self.begin(Identity::system()).await?;
        let mut model = LogSinksModel::new(&mut tx);
        let sink = model
            .get_by_provider(sink_type.clone())
            .await?
            .map(|sink| sink.into_value().config);
        Ok(sink)
    }

    pub async fn get_log_sink_by_id(&self, id: &String) -> anyhow::Result<Option<LogSinkWithId>> {
        let mut tx = self.begin(Identity::system()).await?;

        let id = tx.resolve_developer_id(
            &DeveloperDocumentId::decode(id).map_err(|_| {
                anyhow::anyhow!(ErrorMetadata::bad_request(
                    "InvalidLogStreamId",
                    "The log stream id is invalid"
                ))
            })?,
            TableNamespace::Global,
        )?;

        let Some(doc) = tx.get(id).await? else {
            return Ok(None);
        };

        let row: ParsedDocument<LogSinksRow> = doc.parse()?;

        // Check if the stream is tombstoned (deleted)
        if row.status == SinkState::Tombstoned {
            return Ok(None);
        }

        Ok(Some(LogSinkWithId {
            id: row.id(),
            config: row.config.clone(),
            status: row.status.clone(),
        }))
    }

    pub async fn list_log_sinks(&self) -> anyhow::Result<Vec<LogSinkWithId>> {
        let mut tx = self.begin(Identity::system()).await?;
        let mut model = LogSinksModel::new(&mut tx);
        let sinks = model
            .get_all_non_tombstoned()
            .await?
            .into_iter()
            .map(|sink| {
                let id = sink.id();
                let value = sink.into_value();
                LogSinkWithId {
                    id,
                    config: value.config,
                    status: value.status,
                }
            })
            .collect();
        Ok(sinks)
    }

    pub async fn remove_log_sink_by_id(&self, id: String) -> anyhow::Result<()> {
        let developer_id = DeveloperDocumentId::decode(&id).map_err(|_| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidLogStreamId",
                "The log stream id is invalid"
            ))
        })?;
        self.execute_with_occ_retries(
            Identity::system(),
            FunctionUsageTracker::new(),
            "remove_log_sink",
            |tx| {
                async move {
                    let id = tx.resolve_developer_id(&developer_id, TableNamespace::Global)?;
                    let row: ParsedDocument<LogSinksRow> = tx
                        .get(id)
                        .await?
                        .ok_or_else(|| {
                            ErrorMetadata::bad_request(
                                "LogStreamDoesntExist",
                                "No log stream with the given id exists for this deployment.",
                            )
                        })?
                        .parse()?;
                    if row.status == SinkState::Tombstoned {
                        return Err(ErrorMetadata::bad_request(
                            "LogStreamDoesntExist",
                            "No log stream with the given id exists for this deployment.",
                        )
                        .into());
                    }
                    LogSinksModel::new(tx).mark_for_removal(id).await
                }
                .boxed()
                .into()
            },
        )
        .await?;
        Ok(())
    }

    pub async fn ensure_log_streaming_allowed(&self, identity: Identity) -> anyhow::Result<()> {
        let mut tx = self.begin(identity).await?;
        BackendInfoModel::new(&mut tx)
            .ensure_log_streaming_allowed()
            .await
    }
}
