/// Testing helpers for the protocol module.
use std::{
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use convex_sync_types::{
    ClientMessage,
    SessionId,
};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use url::Url;
use uuid::Uuid;

use super::{
    ReconnectRequest,
    WebSocketState,
};
use crate::sync::{
    ProtocolResponse,
    ServerMessage,
    SyncProtocol,
};

#[derive(Debug)]
struct TestProtocolInner {
    closed: bool,
    sent_messages: Vec<ClientMessage>,
}
#[derive(Debug, Clone)]
pub struct TestProtocolManager {
    inner: Arc<Mutex<TestProtocolInner>>,
    response_sender: mpsc::Sender<ProtocolResponse>,
}

impl TestProtocolManager {
    pub async fn fake_server_response(&mut self, message: ServerMessage) -> anyhow::Result<()> {
        self.response_sender
            .send(ProtocolResponse::ServerMessage(message))
            .await?;
        Ok(())
    }

    pub async fn wait_until_n_messages_sent(&self, n: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.inner.lock().sent_messages.len() < n {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("Test timed out waiting for messages to be sent");
    }

    pub async fn take_sent(&self) -> Vec<ClientMessage> {
        std::mem::take(&mut self.inner.lock().sent_messages)
    }
}

#[async_trait]
impl SyncProtocol for TestProtocolManager {
    async fn open(
        _ws_url: Url,
        response_sender: mpsc::Sender<ProtocolResponse>,
        _on_state_change: Option<mpsc::Sender<WebSocketState>>,
        _client_id: &str,
    ) -> anyhow::Result<Self> {
        let mut test_protocol = TestProtocolManager {
            inner: Arc::new(Mutex::new(TestProtocolInner {
                closed: false,
                sent_messages: vec![],
            })),
            response_sender,
        };

        let session_id = Uuid::nil();
        let connection_count = 0;

        test_protocol
            .send(ClientMessage::Connect {
                session_id: SessionId::new(session_id),
                connection_count,
                last_close_reason: "InitialConnect".to_string(),
                max_observed_timestamp: None,
            })
            .await?;

        Ok(test_protocol)
    }

    async fn send(&mut self, message: ClientMessage) -> anyhow::Result<()> {
        if self.inner.lock().closed {
            anyhow::ensure!(!self.inner.lock().closed, "Websocket is closed");
        }
        self.inner.lock().sent_messages.push(message);

        Ok(())
    }

    async fn reconnect(&mut self, request: ReconnectRequest) {
        panic!("Test reconnected {request:?}");
    }
}
