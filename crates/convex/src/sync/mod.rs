use async_trait::async_trait;
use convex_sync_types::{
    ClientMessage,
    Timestamp,
};
use futures::channel::mpsc;
use url::Url;

use crate::value::Value;

#[cfg(test)]
pub mod testing;
pub mod web_socket_manager;

/// Upon a protocol failure, an explanation of the failure to pass in on
/// reconnect
#[derive(Debug)]
pub struct ReconnectRequest {
    pub reason: ReconnectProtocolReason,
    pub max_observed_timestamp: Option<Timestamp>,
}

pub type ReconnectProtocolReason = String;

pub type ServerMessage = convex_sync_types::ServerMessage<Value>;

#[derive(Debug)]
pub enum ProtocolResponse {
    ServerMessage(ServerMessage),
    Failure,
}

#[async_trait]
pub trait SyncProtocol: Send + Sized {
    async fn open(ws_url: Url, on_response: mpsc::Sender<ProtocolResponse>)
        -> anyhow::Result<Self>;
    async fn send(&mut self, message: ClientMessage) -> anyhow::Result<()>;
    async fn reconnect(&mut self, request: ReconnectRequest);
}
