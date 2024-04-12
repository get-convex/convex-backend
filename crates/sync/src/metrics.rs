use std::time::Duration;

use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    register_convex_counter,
    register_convex_histogram,
    MetricLabel,
    StatusTimer,
    STATUS_LABEL,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::{
    types::ClientEvent,
    ClientMessage,
};
register_convex_histogram!(
    SYNC_CONNECT_SECONDS,
    "Time between SyncWorker creation and receiving Connect message",
    &STATUS_LABEL
);
/// Measuring time between SyncWorker creation and receiving Connect message.
pub fn connect_timer() -> StatusTimer {
    StatusTimer::new(&SYNC_CONNECT_SECONDS)
}

register_convex_histogram!(
    SYNC_HANDLE_MESSAGE_SECONDS,
    "Time to handle a websocket message",
    &["status", "endpoint"]
);
pub fn handle_message_timer(message: &ClientMessage) -> StatusTimer {
    let mut timer = StatusTimer::new(&SYNC_HANDLE_MESSAGE_SECONDS);
    let request_name = match message {
        ClientMessage::Authenticate { .. } => "Authenticate",
        ClientMessage::Connect { .. } => "Connect",
        ClientMessage::Action { .. } => "Action",
        ClientMessage::ModifyQuerySet { .. } => "ModifyQuerySet",
        ClientMessage::Mutation { .. } => "Mutation",
        ClientMessage::Event { .. } => "Event",
    };
    timer.add_label(MetricLabel::new("endpoint", request_name.to_owned()));
    timer
}

register_convex_histogram!(
    SYNC_UPDATE_QUERIES_SECONDS,
    "Time to update queries",
    &STATUS_LABEL
);
pub fn update_queries_timer() -> StatusTimer {
    StatusTimer::new(&SYNC_UPDATE_QUERIES_SECONDS)
}

register_convex_histogram!(
    SYNC_MUTATION_QUEUE_SECONDS,
    "Time between a mutation entering and exiting the single threaded sync worker queue",
    &STATUS_LABEL
);
pub fn mutation_queue_timer() -> StatusTimer {
    StatusTimer::new(&SYNC_MUTATION_QUEUE_SECONDS)
}

register_convex_counter!(SYNC_QUERY_FAILED_TOTAL, "Number of query failures");
pub fn log_query_failed() {
    log_counter(&SYNC_QUERY_FAILED_TOTAL, 1);
}

register_convex_histogram!(SYNC_QUERY_SET_TOTAL, "Size of query set");
pub fn log_query_set_size(num_queries: usize) {
    log_distribution(&SYNC_QUERY_SET_TOTAL, num_queries as f64);
}

register_convex_counter!(
    SYNC_QUERY_RESULT_DEDUP_TOTAL,
    "Number of deduplicated query results"
);
pub fn log_query_result_dedup(same_value: bool) {
    let sample = if same_value { 1 } else { 0 };
    log_counter(&SYNC_QUERY_RESULT_DEDUP_TOTAL, sample);
}

register_convex_counter!(SYNC_EMPTY_TRANSITION_TOTAL, "Number of empty transitions");
pub fn log_empty_transition() {
    log_counter(&SYNC_EMPTY_TRANSITION_TOTAL, 1);
}

register_convex_counter!(
    SYNC_CONNECT_TOTAL,
    "Number of new WS connections",
    &["reason"]
);
register_convex_histogram!(
    SYNC_RECONNECT_PREV_CONNECTIONS,
    "How many previous connections happened on a given reconnect",
);
pub fn log_connect(last_close_reason: String, connection_count: u32) {
    let labels = vec![MetricLabel::new("reason", last_close_reason)];
    log_counter_with_labels(&SYNC_CONNECT_TOTAL, 1, labels);
    log_distribution(&SYNC_RECONNECT_PREV_CONNECTIONS, connection_count.into());
}

register_convex_histogram!(
    SYNC_LINEARIZABILITY_DELAY_SECONDS,
    "How far behind the current backend is behind what the client has observed",
);
pub fn log_linearizability_violation(delay_secs: f64) {
    log_distribution(&SYNC_LINEARIZABILITY_DELAY_SECONDS, delay_secs);
}

register_convex_histogram!(
    SYNC_PROCESS_CLIENT_MESSAGE_SECONDS,
    "Delay between receiving a client message over the web socket and processing it",
);
pub fn log_process_client_message_delay(delay: Duration) {
    log_distribution(&SYNC_PROCESS_CLIENT_MESSAGE_SECONDS, delay.as_secs_f64());
}

register_convex_histogram!(
    SYNC_CLIENT_CONSTRUCT_TO_FIRST_MESSAGE_SECONDS,
    "Time from client construction to first message"
);
pub fn log_client_construct_to_first_message_millis(ms: f64) {
    log_distribution(&SYNC_CLIENT_CONSTRUCT_TO_FIRST_MESSAGE_SECONDS, ms / 1000.0);
}

register_convex_histogram!(
    SYNC_CLIENT_CONSTRUCT_TO_WEBSOCKET_OPENED_SECONDS,
    "Time from client construction to websocket open"
);
pub fn log_client_construct_to_websocket_opened_millis(ms: f64) {
    log_distribution(
        &SYNC_CLIENT_CONSTRUCT_TO_WEBSOCKET_OPENED_SECONDS,
        ms / 1000.0,
    );
}

register_convex_counter!(
    SYNC_CLIENT_METRICS_INCOMPLETE_TOTAL,
    "Number of incomplete metric reports from the client"
);
pub fn log_client_metrics_incomplete() {
    log_counter(&SYNC_CLIENT_METRICS_INCOMPLETE_TOTAL, 1);
}

pub fn log_client_connect_timings(marks: Vec<ClientMark>) {
    let mut client_constructed: Option<f64> = None;
    let mut websocket_opened: Option<f64> = None;
    let mut first_message_received: Option<f64> = None;
    for mark in marks {
        match mark.name {
            ClientMarkName::ClientConstructed => client_constructed = Some(mark.start_time),
            ClientMarkName::WebSocketOpen => websocket_opened = Some(mark.start_time),
            ClientMarkName::FirstMessageReceived => first_message_received = Some(mark.start_time),
        }
    }
    match (client_constructed, websocket_opened, first_message_received) {
        (Some(construct), Some(opened), Some(first)) => {
            log_client_construct_to_first_message_millis(first - construct);
            log_client_construct_to_websocket_opened_millis(opened - construct);
        },
        _ => {
            log_client_metrics_incomplete();
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum ClientMarkName {
    /// When a client is constructed (the browser client, not the React client)
    ClientConstructed,
    /// When the browser/runtime runs `WebSocket.onopen`
    WebSocketOpen,
    /// When the client receives its first WebSocket message from the server
    FirstMessageReceived,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ClientMark {
    pub name: ClientMarkName,
    pub start_time: f64,
}

impl TryFrom<JsonValue> for ClientMark {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let v: ClientMarkJson = serde_json::from_value(value)?;
        let client_mark = match v.name {
            ClientMarkNameJson::ClientConstructed => ClientMark {
                name: ClientMarkName::ClientConstructed,
                start_time: v.start_time,
            },
            ClientMarkNameJson::WebSocketOpen => ClientMark {
                name: ClientMarkName::WebSocketOpen,
                start_time: v.start_time,
            },
            ClientMarkNameJson::FirstMessageReceived => ClientMark {
                name: ClientMarkName::FirstMessageReceived,
                start_time: v.start_time,
            },
        };
        Ok(client_mark)
    }
}

#[derive(Clone, Debug)]
pub enum TypedClientEvent {
    ClientConnect { marks: Vec<ClientMark> },
}

impl TryFrom<ClientEvent> for TypedClientEvent {
    type Error = anyhow::Error;

    fn try_from(value: ClientEvent) -> Result<Self, Self::Error> {
        match value.event_type.as_str() {
            "ClientConnect" => {
                let parsed_marks = if let JsonValue::Array(marks) = value.event {
                    marks
                        .into_iter()
                        .map(ClientMark::try_from)
                        .collect::<anyhow::Result<Vec<_>>>()
                } else {
                    Err(anyhow::anyhow!("Client marks JSON is not an array"))
                }?;
                Ok(TypedClientEvent::ClientConnect {
                    marks: parsed_marks,
                })
            },
            _ => Err(anyhow::anyhow!("Unknown ClientEvent type")),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ClientMarkNameJson {
    ClientConstructed,
    WebSocketOpen,
    FirstMessageReceived,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientMarkJson {
    name: ClientMarkNameJson,
    start_time: f64,
}
