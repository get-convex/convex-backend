use std::time::Duration;

use common::{
    errors::JsError,
    runtime::{
        Runtime,
        SpawnHandle,
    },
    value::{
        ConvexObject,
        ConvexValue,
    },
};
use js_protocol::SyncMutationStatus;
use runtime::testing::TestRuntime;
use serde::Serialize;
use sync_types::{
    Timestamp,
    UdfPath,
};
use tokio::sync::{
    mpsc,
    oneshot,
};
use uuid::Uuid;

use super::server::ServerThread;

mod environment;
mod go;
mod js_protocol;
mod state;

pub type QueryToken = String;
pub type SyncQuerySubscriptionId = String;
pub type MutationId = String;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationInfo {
    pub mutation_path: String,
    pub opt_update_args: ConvexObject,
    pub server_args: ConvexObject,
}

pub enum JsClientThreadRequest {
    AddQuery {
        udf_path: UdfPath,
        args: ConvexObject,
        sender: oneshot::Sender<QueryToken>,
    },
    QueryResult {
        token: QueryToken,
        sender: oneshot::Sender<Option<ConvexValue>>,
    },
    RemoveQuery {
        token: QueryToken,
        sender: oneshot::Sender<()>,
    },
    RunMutation {
        udf_path: UdfPath,
        args: ConvexObject,
        sender: oneshot::Sender<Result<ConvexValue, JsError>>,
    },

    AddSyncQuery {
        id: String,
        name: String,
        args: ConvexObject,
        sender: oneshot::Sender<()>,
    },
    SyncQueryResult {
        id: SyncQuerySubscriptionId,
        sender: oneshot::Sender<Option<Result<ConvexValue, JsError>>>,
    },
    RemoveSyncQuery {
        id: SyncQuerySubscriptionId,
        sender: oneshot::Sender<()>,
    },

    RequestSyncMutation {
        id: String,
        mutation_info: MutationInfo,
        sender: oneshot::Sender<()>,
    },
    GetSyncMutationStatus {
        id: MutationId,
        sender: oneshot::Sender<Option<SyncMutationStatus>>,
    },

    MaxObservedTimestamp {
        sender: oneshot::Sender<Option<Timestamp>>,
    },
    DisconnectNetwork {
        sender: oneshot::Sender<bool>,
    },
    ReconnectNetwork {
        sender: oneshot::Sender<bool>,
    },
}

#[derive(Clone)]
pub struct JsClientThread {
    rt: TestRuntime,
    tx: mpsc::UnboundedSender<JsClientThreadRequest>,
}

impl JsClientThread {
    pub fn new(rt: TestRuntime, server: ServerThread) -> (Self, Box<dyn SpawnHandle>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let rt_ = rt.clone();
        let handle = rt.spawn_thread(move || async move {
            Self::go(rt_, server, rx).await.expect("JsThread failed");
        });
        (Self { rt, tx }, handle)
    }

    pub async fn add_query(
        &self,
        udf_path: UdfPath,
        args: ConvexObject,
    ) -> anyhow::Result<QueryToken> {
        let (sender, receiver) = oneshot::channel();
        self.tx.send(JsClientThreadRequest::AddQuery {
            udf_path,
            args,
            sender,
        })?;
        Ok(receiver.await?)
    }

    pub async fn query_result(&self, token: QueryToken) -> anyhow::Result<Option<ConvexValue>> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::QueryResult { token, sender })?;
        Ok(receiver.await?)
    }

    pub async fn remove_query(&self, token: QueryToken) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::RemoveQuery { token, sender })?;
        Ok(receiver.await?)
    }

    pub async fn run_mutation(
        &self,
        udf_path: UdfPath,
        args: ConvexObject,
    ) -> anyhow::Result<Result<ConvexValue, JsError>> {
        let (sender, receiver) = oneshot::channel();
        self.tx.send(JsClientThreadRequest::RunMutation {
            udf_path,
            args,
            sender,
        })?;
        Ok(receiver.await?)
    }

    pub async fn add_sync_query(
        &self,
        name: &str,
        args: ConvexObject,
    ) -> anyhow::Result<SyncQuerySubscriptionId> {
        let (sender, receiver) = oneshot::channel();
        let id = Uuid::new_v4().to_string();
        self.tx.send(JsClientThreadRequest::AddSyncQuery {
            id: id.clone(),
            name: name.to_string(),
            args,
            sender,
        })?;
        receiver.await?;
        Ok(id)
    }

    pub async fn sync_query_result(
        &self,
        id: SyncQuerySubscriptionId,
    ) -> anyhow::Result<Option<Result<ConvexValue, JsError>>> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::SyncQueryResult { id, sender })?;
        Ok(receiver.await?)
    }

    pub async fn remove_sync_query(&self, id: SyncQuerySubscriptionId) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::RemoveSyncQuery { id, sender })?;
        Ok(receiver.await?)
    }

    pub async fn request_sync_mutation(
        &self,
        mutation_info: MutationInfo,
    ) -> anyhow::Result<MutationId> {
        let (sender, receiver) = oneshot::channel();
        let id = Uuid::new_v4().to_string();
        self.tx.send(JsClientThreadRequest::RequestSyncMutation {
            id: id.clone(),
            mutation_info,
            sender,
        })?;
        receiver.await?;
        Ok(id)
    }

    pub async fn get_sync_mutation_status(
        &self,
        id: MutationId,
    ) -> anyhow::Result<Option<SyncMutationStatus>> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::GetSyncMutationStatus { id, sender })?;
        Ok(receiver.await?)
    }

    pub async fn max_observed_timestamp(&self) -> anyhow::Result<Option<Timestamp>> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::MaxObservedTimestamp { sender })?;
        Ok(receiver.await?)
    }

    pub async fn wait_for_server_ts(&self, ts: Timestamp) -> anyhow::Result<()> {
        while self.max_observed_timestamp().await? < Some(ts) {
            // Since we virtualize time, the actual duration we sleep here doesn't matter
            // much.
            self.rt.wait(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn wait_for_sync_mutation_reflected_locally(
        &self,
        id: MutationId,
    ) -> anyhow::Result<()> {
        while !matches!(
            self.get_sync_mutation_status(id.clone()).await?,
            Some(SyncMutationStatus::ReflectedLocallyButWaitingForNetwork)
                | Some(SyncMutationStatus::Reflected)
        ) {
            // Since we virtualize time, the actual duration we sleep here doesn't matter
            // much.
            self.rt.wait(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn wait_for_sync_mutation_reflected(&self, id: MutationId) -> anyhow::Result<()> {
        while !matches!(
            self.get_sync_mutation_status(id.clone()).await?,
            Some(SyncMutationStatus::Reflected)
        ) {
            // Since we virtualize time, the actual duration we sleep here doesn't matter
            // much.
            self.rt.wait(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn wait_for_sync_mutation_reflected_on_network(
        &self,
        id: MutationId,
    ) -> anyhow::Result<()> {
        while !matches!(
            self.get_sync_mutation_status(id.clone()).await?,
            Some(SyncMutationStatus::ReflectedOnNetworkButNotLocally)
                | Some(SyncMutationStatus::Reflected)
        ) {
            // Since we virtualize time, the actual duration we sleep here doesn't matter
            // much.
            self.rt.wait(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn disconnect_network(&self) -> anyhow::Result<bool> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::DisconnectNetwork { sender })?;
        Ok(receiver.await?)
    }

    pub async fn reconnect_network(&self) -> anyhow::Result<bool> {
        let (sender, receiver) = oneshot::channel();
        self.tx
            .send(JsClientThreadRequest::ReconnectNetwork { sender })?;
        Ok(receiver.await?)
    }
}
