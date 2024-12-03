use std::time::Duration;

use common::runtime::{
    Runtime,
    SpawnHandle,
};
use convex::Value;
use runtime::testing::TestRuntime;
use sync_types::{
    Timestamp,
    UdfPath,
};
use tokio::sync::{
    mpsc,
    oneshot,
};

use super::server::ServerThread;

mod environment;
mod go;
mod js_protocol;
mod state;

type QueryToken = String;

pub enum JsClientThreadRequest {
    AddQuery {
        udf_path: UdfPath,
        args: Value,
        sender: oneshot::Sender<QueryToken>,
    },
    QueryResult {
        token: QueryToken,
        sender: oneshot::Sender<Option<Value>>,
    },
    RemoveQuery {
        token: QueryToken,
        sender: oneshot::Sender<()>,
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

    pub async fn add_query(&self, udf_path: UdfPath, args: Value) -> anyhow::Result<QueryToken> {
        let (sender, receiver) = oneshot::channel();
        self.tx.send(JsClientThreadRequest::AddQuery {
            udf_path,
            args,
            sender,
        })?;
        Ok(receiver.await?)
    }

    pub async fn query_result(&self, token: QueryToken) -> anyhow::Result<Option<Value>> {
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
