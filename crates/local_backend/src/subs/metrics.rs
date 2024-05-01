use std::time::Duration;

use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_distribution_with_labels,
    log_gauge,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
    STATUS_LABEL,
};
use sync::ServerMessage;

register_convex_histogram!(
    BACKEND_WS_UPGRADE_SECONDS,
    "Time taken to upgrade the Websocket connection",
    &STATUS_LABEL
);
pub fn websocket_upgrade_timer() -> StatusTimer {
    StatusTimer::new(&BACKEND_WS_UPGRADE_SECONDS)
}

register_convex_counter!(BACKEND_WS_IN_TOTAL, "Count of received websocket messages");
pub fn log_websocket_message_in() {
    log_counter(&BACKEND_WS_IN_TOTAL, 1);
}

register_convex_counter!(BACKEND_PING_TOTAL, "Number of websocket pings sent");
pub fn log_websocket_ping() {
    log_counter(&BACKEND_PING_TOTAL, 1);
}

register_convex_counter!(BACKEND_PONG_TOTAL, "Number of websocket pongs received");
register_convex_histogram!(
    BACKEND_PING_PONG_SECONDS,
    "When websocket Pong received, duration since latest Ping was sent",
);
/// backend_ping_pong_seconds approximates round trip time on the websocket.
/// We don't have a way to match up a pong with its ping, so the latency metric
/// might skew low, if two pings are sent before either pong is received.
pub fn log_websocket_pong(latency_since_ping: Duration) {
    log_counter(&BACKEND_PONG_TOTAL, 1);
    log_distribution(&BACKEND_PING_PONG_SECONDS, latency_since_ping.as_secs_f64());
}

register_convex_counter!(
    BACKEND_CLIENT_TIMEOUT_TOTAL,
    "Number of websocket ping/pong timeouts"
);
pub fn log_websocket_client_timeout() {
    log_counter(&BACKEND_CLIENT_TIMEOUT_TOTAL, 1);
}

register_convex_histogram!(
    BACKEND_WS_SEND_DELAY_SECONDS,
    "Delay between generating a message in the sync worker and sending it over the web socket.",
    &["endpoint"]
);
register_convex_counter!(
    BACKEND_WS_OUT_TOTAL,
    "Count of outgoing websocket messages",
    &["endpoint"]
);
pub fn log_websocket_message_out(message: &ServerMessage, delay: Duration) {
    let endpoint = match message {
        ServerMessage::Transition { .. } => "Transition",
        ServerMessage::MutationResponse { .. } => "MutationResponse",
        ServerMessage::ActionResponse { .. } => "ActionResponse",
        ServerMessage::AuthError { .. } => "AuthError",
        ServerMessage::FatalError { .. } => "FatalError",
        ServerMessage::Ping { .. } => "Ping",
    };
    let labels = vec![StaticMetricLabel::new("endpoint", endpoint)];
    log_distribution_with_labels(
        &BACKEND_WS_SEND_DELAY_SECONDS,
        delay.as_secs_f64(),
        labels.clone(),
    );
    log_counter_with_labels(&BACKEND_WS_OUT_TOTAL, 1, labels);
}

register_convex_counter!(
    BACKEND_WS_CLOSED_TOTAL,
    "Number of times the websocket was closed"
);
pub fn log_websocket_closed() {
    log_counter(&BACKEND_WS_CLOSED_TOTAL, 1);
}

register_convex_counter!(
    BACKEND_WS_SERVER_ERROR_TOTAL,
    "Count of websocket server errors",
    &["type"]
);
pub fn log_websocket_server_error(tag: StaticMetricLabel) {
    log_counter_with_labels(&BACKEND_WS_SERVER_ERROR_TOTAL, 1, vec![tag]);
}

register_convex_counter!(
    BACKEND_WS_CONNECTION_CLOSED_ERROR_NOT_REPORTED_TOTAL,
    "Count of connection closed errors not reported"
);
pub fn log_websocket_closed_error_not_reported() {
    log_counter(&BACKEND_WS_CONNECTION_CLOSED_ERROR_NOT_REPORTED_TOTAL, 1);
}

register_convex_gauge!(
    SYNC_PROTOCOL_WEBSOCKETS_TOTAL,
    "Number of WebSocket connected to a backend",
);
pub fn log_sync_protocol_websockets_total(count: u64) {
    log_gauge(&SYNC_PROTOCOL_WEBSOCKETS_TOTAL, count as f64);
}

register_convex_counter!(pub WEBSOCKET_CONNECTION_RESET_TOTAL, "Number of websocket connection resets");
pub fn log_websocket_connection_reset() {
    log_counter(&WEBSOCKET_CONNECTION_RESET_TOTAL, 1)
}
