use std::collections::BTreeMap;

use common::runtime::{
    Runtime,
    SpawnHandle,
};
use convex::{
    base_client::BaseConvexClient,
    QueryResults,
    SubscriberId,
    Value,
};
use runtime::testing::TestRuntime;
use serde_json::Value as JsonValue;
use sync_types::UdfPath;
use tokio::{
    sync::{
        broadcast,
        mpsc,
        oneshot,
    },
    time::Instant,
};

use super::server::ServerThread;

pub enum BaseClientRequest {
    Subscribe(
        UdfPath,
        BTreeMap<String, Value>,
        oneshot::Sender<SubscriberId>,
    ),
    Unsubscribe(SubscriberId, oneshot::Sender<()>),
    Listen(broadcast::Sender<QueryResults>),
}

pub struct BaseClientThread {
    tx: mpsc::UnboundedSender<BaseClientRequest>,
}

impl BaseClientThread {
    pub fn new(rt: TestRuntime, server_thread: ServerThread) -> (Self, Box<dyn SpawnHandle>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = rt.spawn("BaseClientThread", async move {
            Self::go(server_thread, rx)
                .await
                .expect("BaseClientThread failed")
        });
        (Self { tx }, handle)
    }

    pub async fn subscribe(
        &self,
        udf_path: UdfPath,
        args: BTreeMap<String, Value>,
    ) -> anyhow::Result<SubscriberId> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(BaseClientRequest::Subscribe(udf_path, args, tx))?;
        let subscriber_id = rx.await?;
        Ok(subscriber_id)
    }

    pub async fn unsubscribe(&self, subscriber_id: SubscriberId) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(BaseClientRequest::Unsubscribe(subscriber_id, tx))?;
        rx.await?;
        Ok(())
    }

    pub async fn listen(&self) -> anyhow::Result<broadcast::Receiver<QueryResults>> {
        let (tx, rx) = broadcast::channel(16);
        self.tx.send(BaseClientRequest::Listen(tx))?;
        Ok(rx)
    }

    pub(crate) async fn go(
        server_thread: ServerThread,
        mut rx: mpsc::UnboundedReceiver<BaseClientRequest>,
    ) -> anyhow::Result<()> {
        let mut client = BaseConvexClient::new();
        'connect_loop: loop {
            // First, try to connect to the server.
            let (client_tx, mut server_rx) = server_thread.connect()?;
            client.resend_ongoing_queries_mutations();

            let mut listeners = vec![];
            loop {
                // Send out all messages we have queued up.
                while let Some(msg) = client.pop_next_message() {
                    let _ = client_tx.send((msg, Instant::now()));
                }
                tokio::select! {
                    request = rx.recv() => {
                        match request {
                            Some(request) => {
                                match request {
                                    BaseClientRequest::Subscribe(udf_path, args, sender) => {
                                        let subscriber_id = client.subscribe(udf_path, args);
                                        let _ = sender.send(subscriber_id);
                                    },
                                    BaseClientRequest::Unsubscribe(subscriber_id, sender) => {
                                        client.unsubscribe(subscriber_id);
                                        let _ = sender.send(());
                                    },
                                    BaseClientRequest::Listen(sender) => {
                                        if sender.send(client.latest_results().clone()).is_ok() {
                                            listeners.push(sender);
                                        }
                                    },
                                }
                            }
                            None => {
                                tracing::error!("Client thread shutting down...");
                                break 'connect_loop;
                            },
                        }
                    },
                    // Wait for an incoming message from the server.
                    message = server_rx.next() => {
                        match message {
                            Some((msg, _)) => {
                                tracing::debug!("Received message from sync worker: {msg:?}");
                                let json_msg = JsonValue::from(msg);
                                match client.receive_message(json_msg.try_into().unwrap()) {
                                    Ok(Some(query_results)) => {
                                        tracing::debug!("Received query results: {query_results:?}");
                                        listeners.retain(|sender| {
                                            sender.send(query_results.clone()).is_ok()
                                        });
                                    },
                                    Ok(None) => {
                                        tracing::debug!("Received no query results");
                                    },
                                    Err(reason) => {
                                        tracing::error!("Received error from sync worker: {reason:?}");
                                        continue 'connect_loop;
                                    },
                                }
                            },
                            None => {
                                tracing::error!("Server thread disconnected");
                                continue 'connect_loop;
                            },
                        }
                    },
                }
            }
        }
        Ok(())
    }
}
