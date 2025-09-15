use std::time::{
    Duration,
    Instant,
};

use ::errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use anyhow::Context as _;
use axum::{
    body::Bytes,
    extract::{
        ws::{
            CloseFrame,
            Message,
            WebSocket,
            WebSocketUpgrade,
        },
        State,
    },
    response::IntoResponse,
};
use common::{
    errors::{
        report_error,
        report_error_sync,
    },
    http::{
        ExtractClientVersion,
        ExtractResolvedHostname,
        HttpResponseError,
        ResolvedHostname,
    },
    runtime::Runtime,
    version::ClientVersion,
    ws::is_connection_closed_error,
};
use futures::{
    select_biased,
    try_join,
    FutureExt,
    SinkExt,
    StreamExt,
};
use parking_lot::Mutex;
use runtime::prod::ProdRuntime;
use sentry::SentryFutureExt;
use serde_json::Value as JsonValue;
use sync::{
    worker::measurable_unbounded_channel,
    ServerMessage,
    SyncWorker,
    SyncWorkerConfig,
};
use sync_types::{
    ClientMessage,
    IdentityVersion,
    SessionId,
};
use tokio::sync::mpsc;

mod metrics;

use metrics::{
    log_debug_sync_protocol_websockets_total,
    log_sync_protocol_websockets_total,
    log_websocket_client_timeout,
    log_websocket_closed,
    log_websocket_closed_error_not_reported,
    log_websocket_connection_reset,
    log_websocket_message_in,
    log_websocket_message_out,
    log_websocket_ping,
    log_websocket_pong,
    log_websocket_server_error,
    websocket_upgrade_timer,
};

use crate::{
    subs::metrics::log_websocket_client_message_bytes,
    RouterState,
};

/// How often heartbeat pings are sent.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(120);

struct SyncSocketDropToken {}

/// Tracker that exists for the lifetime of a run_sync_socket.
impl SyncSocketDropToken {
    fn new() -> Self {
        log_sync_protocol_websockets_total(1);
        SyncSocketDropToken {}
    }
}

impl Drop for SyncSocketDropToken {
    fn drop(&mut self) {
        log_sync_protocol_websockets_total(-1);
    }
}

// TODO(presley): Remove. Used for debugging.
struct DebugSyncSocketDropToken {
    tag: &'static str,
}

/// Tracker that exists for the lifetime of a run_sync_socket.
impl DebugSyncSocketDropToken {
    fn new(tag: &'static str) -> Self {
        log_debug_sync_protocol_websockets_total(tag, 1);
        DebugSyncSocketDropToken { tag }
    }
}

impl Drop for DebugSyncSocketDropToken {
    fn drop(&mut self) {
        log_debug_sync_protocol_websockets_total(self.tag, -1);
    }
}

// The WebSocket layer for the sync protocol has three asynchronous processes:
//
// 1) A `receive_messages` loop that consumes messages from the WebSocket,
// parses them, and feeds them on a channel to the sync worker.
// 2) A `send_messages` loop that receives messages from the sync worker and
// sends them down to the client. It also periodically sends a ping message.
// 3) The `sync_worker` that actually runs the sync protocol.
//
// If any of these workers fails with an error, we send the error to the client
// on a close frame and close the WebSocket. They can also signal clean shutdown
// by returning `Ok(())`, and once all of them have cleanly exited, we'll
// gracefully close the socket.
async fn run_sync_socket(
    st: RouterState,
    host: ResolvedHostname,
    config: SyncWorkerConfig,
    socket: WebSocket,
    sentry_scope: sentry::Scope,
    on_connect: Box<dyn FnOnce(SessionId) + Send>,
) {
    let _drop_token = SyncSocketDropToken::new();

    let (mut tx, mut rx) = socket.split();

    let last_received = Mutex::new(Instant::now());
    let last_ping_sent = Mutex::new(Instant::now());

    let (client_tx, client_rx) = mpsc::unbounded_channel();
    let receive_messages = async {
        let _receive_message_drop_token = DebugSyncSocketDropToken::new("receive_message");
        while let Some(message_r) = rx.next().await {
            let message = match message_r {
                Ok(message) => message,
                Err(e) if is_connection_closed_error(&e) => {
                    log_websocket_connection_reset();
                    return Err(ErrorMetadata::client_disconnect()).context(e);
                },
                Err(e) => return Err(e.into()),
            };
            *last_received.lock() = Instant::now();

            match message {
                Message::Text(s) => {
                    let client_message_size = s.len();
                    let body: ClientMessage = serde_json::from_str::<JsonValue>(&s)
                        .map_err(|e| anyhow::anyhow!(e))
                        .and_then(|body| body.try_into())
                        .map_err(|e| {
                            anyhow::anyhow!(ErrorMetadata::bad_request(
                                "WSMessageInvalidJson",
                                format!("Received Invalid JSON on websocket: {e}"),
                            ))
                        })?;
                    log_websocket_client_message_bytes(
                        client_message_size,
                        body.as_ref().to_string(),
                    );
                    log_websocket_message_in();
                    if client_tx.send((body, st.runtime.monotonic_now())).is_err() {
                        break;
                    }
                },
                Message::Pong(_) => {
                    log_websocket_pong(last_ping_sent.lock().elapsed());
                    continue;
                },
                Message::Ping(_) => {
                    // The browser sent us a Ping -- our websocket library internally handles
                    // sending a Pong back, so there's nothing more to do.
                    continue;
                },
                Message::Close(_) => break,
                _ => anyhow::bail!("Unexpected message type: {:?}", message),
            }
        }
        // Drop our channel to send to the sync worker, which will cause it to shutdown
        // cleanly.
        drop(client_tx);
        Ok(())
    };

    let (server_tx, mut server_rx) = measurable_unbounded_channel();
    let send_messages = async {
        let _send_message_drop_token = DebugSyncSocketDropToken::new("send_message");
        let mut ping_ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
        'top: loop {
            select_biased! {
                _ = ping_ticker.tick().fuse() => {
                    let now = Instant::now();
                    let last_received = *last_received.lock();
                    if now - last_received > CLIENT_TIMEOUT {
                        log_websocket_client_timeout();
                        return Err(anyhow::anyhow!(ErrorMetadata::client_disconnect()).context("Websocket ping/pong timeout"));
                    }
                    *last_ping_sent.lock() = Instant::now();
                    log_websocket_ping();
                    if tx.send(Message::Ping(Bytes::new())).await.is_err() {
                        break 'top;
                    }
                },
                maybe_message = server_rx.next().fuse() => {
                    let (mut message, send_time) = match maybe_message {
                        Some(m) => m,
                        None => break 'top,
                    };
                    let delay = st.runtime.monotonic_now() - send_time;
                    log_websocket_message_out(&message, delay);
                    message.inject_server_ts(st.runtime.generate_timestamp()?);
                    let serialized = serde_json::to_string(&JsonValue::from(message))?;
                    if tx.send(Message::Text(serialized.into())).await.is_err() {
                        break 'top;
                    }
                },
            }
        }
        Ok(())
    };
    let mut identity_version: Option<IdentityVersion> = None;
    let sync_worker_go = async {
        let _sync_worker_drop_token = DebugSyncSocketDropToken::new("sync_worker");
        // For segmenting metrics
        let partition_id = st.api.partition_id(&host).await?;
        let mut sync_worker = SyncWorker::new(
            st.api.clone(),
            st.runtime.clone(),
            host,
            config.clone(),
            client_rx,
            server_tx,
            on_connect,
            partition_id,
        );
        let r = sync_worker.go().await;
        identity_version = Some(sync_worker.identity_version());
        // Explicit drop for emphasis: dropping triggers send_messages to complete.
        drop(sync_worker);
        r
    };

    let result = try_join!(receive_messages, send_messages, sync_worker_go);

    // This should only fail if we accidentally pass the wrong receiver to
    // `reunite`.
    let mut socket = tx.reunite(rx).expect("Mixed up WebSocket halves?");

    let close_msg = match result {
        Ok(..) => None,
        Err(mut err) => {
            // Send a message on the WebSocket before closing it if the sync
            // worker failed with a "4xx" type error. In this case the client will
            // assume the error is its fault and not retry.
            let final_message = err.downcast_ref::<ErrorMetadata>().and_then(|em| {
                // Special case unauthenticated errors, which want to know the sync worker's
                // base version.
                if em.is_auth_update_failed() {
                    let message = ServerMessage::AuthError {
                        error_message: em.to_string(),
                        base_version: identity_version,
                        auth_update_attempted: Some(true),
                    };
                    Some(message)
                } else if em.is_unauthenticated() {
                    let message = ServerMessage::AuthError {
                        error_message: em.to_string(),
                        base_version: identity_version,
                        auth_update_attempted: Some(false),
                    };
                    Some(message)
                }
                // Otherwise, send a `FatalError` message if it's a user error (not to be retried)
                else if em.is_deterministic_user_error() {
                    Some(ServerMessage::FatalError {
                        error_message: em.to_string(),
                    })
                } else {
                    None
                }
            });
            // Only do a best-effort send of the final application message.
            if let Some(final_message) = final_message {
                let r: anyhow::Result<_> = try {
                    let serialized = serde_json::to_string(&JsonValue::from(final_message))?;
                    socket.send(Message::Text(serialized.into())).await?;
                };
                if let Err(mut e) = r {
                    if is_connection_closed_error(&*e) {
                        log_websocket_closed_error_not_reported()
                    } else {
                        report_error(&mut e).await;
                    }
                }
            }
            sentry::with_scope(|s| *s = sentry_scope, || report_error_sync(&mut err));
            if let Some(label) = err.metric_server_error_label() {
                log_websocket_server_error(label);
            }
            // Convert from tungstenite::Message to axum::Message
            let close_frame = err.close_frame().map(|cf| CloseFrame {
                code: cf.code.into(),
                reason: cf.reason.to_string().into(),
            });
            Some(Message::Close(close_frame))
        },
    };
    // Similarly, only do a best effort send of the close message.
    if let Some(close_msg) = close_msg {
        if let Err(e) = socket.send(close_msg).await {
            if is_connection_closed_error(&e) {
                log_websocket_closed_error_not_reported()
            } else {
                let msg = format!("Failed to gracefully close WebSocket: {e:?}");
                report_error(&mut anyhow::anyhow!(e).context(msg)).await;
            }
        }
    }

    // The close message reply to a client-initiated close handshake
    // automatically sent by Tungstenite (the underlying WebSocket library)
    // isn't actually sent until flush.
    // This is visible in Wireshark.
    if let Err(e) = socket.flush().await {
        if !is_connection_closed_error(&e) {
            let msg = format!("Failed to flush WebSocket: {e:?}");
            report_error(&mut anyhow::anyhow!(e).context(msg)).await;
        }
    }
    log_websocket_closed();
}

fn new_sync_worker_config(client_version: ClientVersion) -> anyhow::Result<SyncWorkerConfig> {
    Ok(SyncWorkerConfig { client_version })
}

pub async fn sync_handler(
    st: RouterState,
    host: ResolvedHostname,
    client_version: ClientVersion,
    ws: WebSocketUpgrade,
    on_connect: Box<dyn FnOnce(SessionId) + Send>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let config = new_sync_worker_config(client_version)?;
    // Make a copy of the Sentry scope, which contains the request metadata.
    let sentry_scope = sentry::configure_scope(move |s| s.clone());

    let upgrade_timer = websocket_upgrade_timer();
    let hub = sentry::Hub::current();
    Ok(ws.on_upgrade(move |ws: WebSocket| {
        upgrade_timer.finish();
        let monitor = ProdRuntime::task_monitor("sync_socket");
        monitor.instrument(
            run_sync_socket(st, host, config, ws, sentry_scope, on_connect).bind_hub(hub),
        )
    }))
}

pub async fn sync(
    State(st): State<RouterState>,
    ExtractResolvedHostname(host): ExtractResolvedHostname,
    ExtractClientVersion(client_version): ExtractClientVersion,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, HttpResponseError> {
    sync_handler(st, host, client_version, ws, Box::new(|_session_id| ())).await
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::{
            ws::{
                Message,
                WebSocket,
            },
            State,
            WebSocketUpgrade,
        },
        routing::get,
        Router,
    };
    use common::http::ConvexHttpService;
    use tokio::sync::{
        mpsc,
        oneshot,
    };
    use tokio_tungstenite::connect_async;
    use tungstenite::error::Error as TungsteniteError;

    use super::is_connection_closed_error;

    /// Test that the axum tungstenite matches the tungstenite we're using in
    /// backend in `is_connection_closed_error` to work around axum sloppiness.
    #[tokio::test]
    async fn test_ws_tungstenite_version_match() -> anyhow::Result<()> {
        let (ws_shutdown_tx, mut ws_shutdown_rx) = mpsc::channel(1);

        async fn ws_handler(
            ws: WebSocketUpgrade,
            st: State<mpsc::Sender<bool>>,
        ) -> axum::response::Response {
            ws.on_upgrade(move |mut ws: WebSocket| async move {
                let ws_shutdown_tx = st.0;
                assert_eq!(ws.recv().await.unwrap().unwrap(), Message::Close(None));
                let e = ws
                    .send(Message::Text("Hello".into()))
                    .await
                    .expect_err("Should not be able to send");

                if is_connection_closed_error(&e) {
                    ws_shutdown_tx.send(true).await.unwrap();
                    return;
                }

                ws_shutdown_tx.send(false).await.unwrap();
                panic!(
                    "Got {e:?}. Expected {:?}. Wrong tungstenite version?",
                    TungsteniteError::ConnectionClosed
                );
            })
        }

        let app = ConvexHttpService::new_for_test(
            Router::new()
                .route("/test", get(ws_handler))
                .with_state(ws_shutdown_tx),
        );
        let port = portpicker::pick_unused_port().expect("No ports free");
        let addr = format!("127.0.0.1:{port}").parse()?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let proxy_server = tokio::spawn(app.serve(addr, async move {
            shutdown_rx.await.unwrap();
        }));

        let (mut websocket, _) = loop {
            match connect_async(format!("ws://{addr}/test")).await {
                Ok(r) => break r,
                Err(e) => {
                    // Can take a moment after the server spawn to connect to it.
                    println!("Got error {e}. Retrying");
                    tokio::task::yield_now().await;
                },
            }
        };

        // close websocket - make sure server handles it ok
        websocket.close(None).await?;
        let closed = ws_shutdown_rx.recv().await.unwrap();
        assert!(closed);

        // server shutdown
        shutdown_tx.send(()).unwrap();
        proxy_server.await??;
        Ok(())
    }
}
