use std::{
    collections::BTreeMap,
    mem,
};

use anyhow::Context;
use common::{
    errors::JsError,
    value::ConvexValue,
};
use deno_core::{
    serde_v8,
    v8,
};
use futures::future;
use isolate::error::extract_source_mapped_error;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync::ServerMessage;
use sync_types::{
    ClientMessage,
    Timestamp,
};
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    time::Instant,
};

use super::{
    js_protocol::{
        AddSyncQueryArgs,
        JsOutgoingMessage,
        MutationResult,
        RequestSyncMutationArgs,
        RunMutationArgs,
        SyncMutationStatus,
        SyncQueryResult,
    },
    JsClientThreadRequest,
};
use crate::test_helpers::{
    js_client::{
        environment::TEST_SOURCE_MAP,
        js_protocol::{
            AddQueryArgs,
            JsIncomingMessage,
        },
    },
    server::ServerThread,
};

pub type WebsocketId = u32;

pub struct JsThreadState<'a> {
    add_query: v8::Local<'a, v8::Function>,
    query_result: v8::Local<'a, v8::Function>,
    remove_query: v8::Local<'a, v8::Function>,
    run_mutation: v8::Local<'a, v8::Function>,

    add_sync_query: v8::Local<'a, v8::Function>,
    sync_query_result: v8::Local<'a, v8::Function>,
    remove_sync_query: v8::Local<'a, v8::Function>,

    request_sync_mutation: v8::Local<'a, v8::Function>,
    get_sync_mutation_status: v8::Local<'a, v8::Function>,

    get_outgoing_messages: v8::Local<'a, v8::Function>,
    receive_incoming_messages: v8::Local<'a, v8::Function>,
    get_max_observed_timestamp: v8::Local<'a, v8::Function>,

    server: ServerThread,

    next_mutation_id: u32,
    in_progress_mutations: BTreeMap<u32, oneshot::Sender<Result<ConvexValue, JsError>>>,

    js_inbox: Vec<JsOutgoingMessage>,
    js_outbox: Vec<JsIncomingMessage>,

    network: NetworkState,

    pub replay_state: ReplayState,
}

impl<'a> JsThreadState<'a> {
    pub fn new(
        scope: &mut v8::HandleScope<'a>,
        server: ServerThread,
        module: v8::Local<v8::Module>,
    ) -> anyhow::Result<Self> {
        let namespace: v8::Local<v8::Object> = module.get_module_namespace().try_into()?;

        let add_query = get_function(scope, namespace, "addQuery")?;
        let query_result = get_function(scope, namespace, "queryResult")?;
        let remove_query = get_function(scope, namespace, "removeQuery")?;
        let run_mutation = get_function(scope, namespace, "runMutation")?;

        let add_sync_query = get_function(scope, namespace, "addSyncQuery")?;
        let sync_query_result = get_function(scope, namespace, "syncQueryResult")?;
        let remove_sync_query = get_function(scope, namespace, "removeSyncQuery")?;

        let request_sync_mutation = get_function(scope, namespace, "requestSyncMutation")?;
        let get_sync_mutation_status = get_function(scope, namespace, "getSyncMutationStatus")?;
        let get_outgoing_messages = get_function(scope, namespace, "getOutgoingMessages")?;
        let receive_incoming_messages = get_function(scope, namespace, "receiveIncomingMessages")?;
        let get_max_observed_timestamp = get_function(scope, namespace, "getMaxObservedTimestamp")?;

        Ok(JsThreadState {
            add_query,
            query_result,
            remove_query,
            run_mutation,
            add_sync_query,
            sync_query_result,
            remove_sync_query,
            request_sync_mutation,
            get_sync_mutation_status,
            get_outgoing_messages,
            receive_incoming_messages,
            get_max_observed_timestamp,
            js_inbox: vec![],
            js_outbox: vec![],
            network: NetworkState::Enabled {
                websockets: BTreeMap::new(),
            },
            server,
            next_mutation_id: 0,
            in_progress_mutations: BTreeMap::new(),
            replay_state: ReplayState::new(),
        })
    }

    pub fn get_max_observed_timestamp(
        &self,
        scope: &mut v8::HandleScope<'a>,
    ) -> anyhow::Result<Option<Timestamp>> {
        let Some(ts_str) =
            self.call::<_, Option<String>>(scope, self.get_max_observed_timestamp, ())?
        else {
            return Ok(None);
        };
        let ts_u64: u64 = ts_str.parse()?;
        Ok(Some(ts_u64.try_into()?))
    }

    pub fn is_outbox_empty(&self) -> bool {
        self.js_outbox.is_empty()
    }

    pub fn is_inbox_empty(&self) -> bool {
        self.js_inbox.is_empty()
    }

    pub fn process_js_inbox(&mut self, scope: &mut v8::HandleScope<'a>) -> anyhow::Result<()> {
        let messages =
            self.call::<(), Vec<JsOutgoingMessage>>(scope, self.get_outgoing_messages, ())?;
        self.js_inbox.extend(messages);

        for msg in self.js_inbox.drain(..) {
            match msg {
                JsOutgoingMessage::Connect { web_socket_id } => {
                    let pair = self.server.connect()?;
                    match self.network {
                        NetworkState::Enabled { ref mut websockets } => {
                            anyhow::ensure!(websockets.insert(web_socket_id, pair).is_none());
                            self.js_outbox
                                .push(JsIncomingMessage::Connected { web_socket_id });
                        },
                        NetworkState::Disabled {
                            ref mut waiting_connects,
                        } => {
                            waiting_connects.push(web_socket_id);
                        },
                    }
                },
                JsOutgoingMessage::Send {
                    web_socket_id,
                    data,
                } => {
                    let tx = self.network.get_sender(web_socket_id)?;
                    let msg: serde_json::Value = serde_json::from_str(&data)?;

                    // TODO: Pass disconnects down to JS.
                    tx.send((msg.try_into()?, Instant::now()))?;
                },
                JsOutgoingMessage::Close { web_socket_id } => {
                    self.network.close(web_socket_id);
                    self.js_outbox
                        .push(JsIncomingMessage::Closed { web_socket_id });
                },
                JsOutgoingMessage::PersistMutation {
                    persist_id,
                    mutation_info,
                } => {
                    tracing::info!("PersistMutation: {persist_id:?} {mutation_info:?}");
                    self.js_outbox.push(JsIncomingMessage::PersistenceDone {
                        persist_id,
                        error: None,
                    });
                },
                JsOutgoingMessage::PersistPages { persist_id, pages } => {
                    tracing::info!("PersistPages: {persist_id:?} {pages:?}");
                    self.js_outbox.push(JsIncomingMessage::PersistenceDone {
                        persist_id,
                        error: None,
                    });
                },
                JsOutgoingMessage::MutationDone {
                    mutation_id,
                    result,
                } => {
                    let sender = self
                        .in_progress_mutations
                        .remove(&mutation_id)
                        .context("Missing completion for mutation ID")?;
                    let result = match result {
                        MutationResult::Success { value } => Ok(value.try_into()?),
                        MutationResult::Failure { error } => Err(JsError::from_message(error)),
                    };
                    let _ = sender.send(result);
                },
            }
        }
        Ok(())
    }

    pub fn process_js_outbox(&mut self, scope: &mut v8::HandleScope<'a>) -> anyhow::Result<()> {
        if self.js_outbox.is_empty() {
            return Ok(());
        }
        let messages = mem::take(&mut self.js_outbox);
        for message in &messages {
            tracing::info!("js outbox: {message:?}");
        }
        self.call::<_, ()>(scope, self.receive_incoming_messages, messages)?;
        Ok(())
    }

    pub fn handle_thread_request(
        &mut self,
        scope: &mut v8::HandleScope<'a>,
        req: JsClientThreadRequest,
    ) -> anyhow::Result<()> {
        match req {
            JsClientThreadRequest::AddQuery {
                udf_path,
                args,
                sender,
            } => {
                let args = AddQueryArgs {
                    udf_path: String::from(udf_path),
                    udf_args_json: serde_json::to_string(&serde_json::Value::from(args))?,
                };
                let token: String = self.call(scope, self.add_query, args)?;
                let _ = sender.send(token);
            },
            JsClientThreadRequest::QueryResult { token, sender } => {
                let maybe_result: Option<String> = self.call(scope, self.query_result, token)?;
                let result = match maybe_result {
                    Some(result) => {
                        let value: serde_json::Value = serde_json::from_str(&result)?;
                        Some(value.try_into()?)
                    },
                    None => None,
                };
                let _ = sender.send(result);
            },
            JsClientThreadRequest::RemoveQuery { token, sender } => {
                self.call::<_, ()>(scope, self.remove_query, token)?;
                let _ = sender.send(());
            },
            JsClientThreadRequest::RunMutation {
                udf_path,
                args,
                sender,
            } => {
                let mutation_id = self.next_mutation_id;
                self.next_mutation_id += 1;
                let args = RunMutationArgs {
                    mutation_id,
                    udf_path: String::from(udf_path),
                    udf_args_json: serde_json::to_string(&serde_json::Value::from(args))?,
                };
                self.call(scope, self.run_mutation, args)?;
                self.in_progress_mutations.insert(mutation_id, sender);
            },
            JsClientThreadRequest::AddSyncQuery {
                id,
                name,
                args,
                sender,
            } => {
                let args_json = serde_json::Value::from(args);
                let func_args = AddSyncQueryArgs {
                    id: id.to_string(),
                    name: name.to_string(),
                    udf_args_json: serde_json::to_string(&args_json)?,
                };
                self.call(scope, self.add_sync_query, func_args)?;
                self.replay_state.add_message(ReplayMessage::AddSyncQuery {
                    id: id.to_string(),
                    name: name.to_string(),
                    args: args_json,
                });
                let _ = sender.send(());
            },
            JsClientThreadRequest::SyncQueryResult { id, sender } => {
                let query_result: Option<SyncQueryResult> =
                    self.call(scope, self.sync_query_result, id.clone())?;
                self.replay_state
                    .add_message(ReplayMessage::CheckSyncQueryResult {
                        id: id.to_string(),
                        expected_result: query_result.clone(),
                    });
                let result = match query_result {
                    Some(SyncQueryResult::Loading) => None,
                    Some(SyncQueryResult::Success { value }) => Some(Ok(value.try_into()?)),
                    Some(SyncQueryResult::Error { error }) => {
                        Some(Err(JsError::from_message(error)))
                    },
                    None => None,
                };

                let _ = sender.send(result);
            },
            JsClientThreadRequest::RemoveSyncQuery { id, sender } => {
                self.call::<_, ()>(scope, self.remove_sync_query, id.clone())?;
                self.replay_state
                    .add_message(ReplayMessage::RemoveSyncQuery { id: id.to_string() });
                let _ = sender.send(());
            },
            JsClientThreadRequest::RequestSyncMutation {
                id,
                mutation_info,
                sender,
            } => {
                let value: serde_json::Value = serde_json::to_value(mutation_info)?;
                let mutation_info_json = serde_json::to_string(&value)?;
                let args = RequestSyncMutationArgs {
                    id: id.to_string(),
                    mutation_info_json,
                };
                self.call(scope, self.request_sync_mutation, args)?;
                self.replay_state.add_message(ReplayMessage::Mutate {
                    id: id.to_string(),
                    mutation_info: value,
                });
                let _ = sender.send(());
            },
            JsClientThreadRequest::GetSyncMutationStatus { id, sender } => {
                let status: Option<SyncMutationStatus> =
                    self.call(scope, self.get_sync_mutation_status, id.clone())?;
                self.replay_state
                    .add_message(ReplayMessage::CheckMutationStatus {
                        id: id.to_string(),
                        expected_status: status.clone(),
                    });
                let _ = sender.send(status);
            },
            JsClientThreadRequest::MaxObservedTimestamp { sender } => {
                let ts = self.get_max_observed_timestamp(scope)?;
                let _ = sender.send(ts);
            },
            JsClientThreadRequest::DisconnectNetwork { sender } => {
                let result = match self.network {
                    NetworkState::Enabled { ref mut websockets } => {
                        for (websocket_id, _) in mem::take(websockets) {
                            self.js_outbox.push(JsIncomingMessage::Closed {
                                web_socket_id: websocket_id,
                            });
                        }
                        self.network = NetworkState::Disabled {
                            waiting_connects: vec![],
                        };
                        true
                    },
                    NetworkState::Disabled { .. } => false,
                };
                let _ = sender.send(result);
            },
            JsClientThreadRequest::ReconnectNetwork { sender } => {
                let result = match self.network {
                    NetworkState::Disabled {
                        ref mut waiting_connects,
                    } => {
                        let mut websockets = BTreeMap::new();
                        for websocket_id in waiting_connects.drain(..) {
                            let pair = self.server.connect()?;
                            websockets.insert(websocket_id, pair);
                            self.js_outbox.push(JsIncomingMessage::Connected {
                                web_socket_id: websocket_id,
                            });
                        }
                        self.network = NetworkState::Enabled { websockets };
                        true
                    },
                    NetworkState::Enabled { .. } => false,
                };
                let _ = sender.send(result);
            },
        }
        Ok(())
    }

    pub fn handle_websocket_message(
        &mut self,
        websocket_id: WebsocketId,
        msg: Option<ServerMessage>,
    ) -> anyhow::Result<()> {
        match msg {
            Some(msg) => {
                let msg_string = serde_json::to_string(&serde_json::Value::from(msg))?;
                self.replay_state.add_message(ReplayMessage::WsMessage {
                    message: msg_string.clone(),
                });
                self.js_outbox.push(JsIncomingMessage::Message {
                    web_socket_id: websocket_id,
                    data: msg_string,
                });
            },
            None => {
                self.network.close(websocket_id);
                self.js_outbox.push(JsIncomingMessage::Closed {
                    web_socket_id: websocket_id,
                });
            },
        }
        Ok(())
    }

    pub fn call<Args, Returns>(
        &self,
        scope: &mut v8::HandleScope<'a>,
        f: v8::Local<'a, v8::Function>,
        args: Args,
    ) -> anyhow::Result<Returns>
    where
        Args: Serialize,
        Returns: Deserialize<'static>,
    {
        let args_v8 = serde_v8::to_v8(scope, args)?;
        let mut tc_scope = v8::TryCatch::new(scope);
        let result = f.call(&mut tc_scope, f.into(), &[args_v8]);
        if let Some(e) = tc_scope.exception() {
            drop(tc_scope);
            let err = extract_error(scope, e)?;
            anyhow::bail!(err);
        }
        drop(tc_scope);
        let result = result.ok_or_else(|| anyhow::anyhow!("No result"))?;
        let result: Returns = serde_v8::from_v8(scope, result)?;
        Ok(result)
    }

    pub async fn next_message(&mut self) -> (WebsocketId, Option<ServerMessage>) {
        match self.network {
            NetworkState::Disabled { .. } => future::pending().await,
            NetworkState::Enabled { ref mut websockets } => {
                assert!(websockets.len() <= 1);
                match websockets.iter_mut().next() {
                    Some((web_socket_id, (_, rx))) => {
                        let maybe_msg = rx.recv().await;
                        (*web_socket_id, maybe_msg.map(|(msg, _)| msg))
                    },
                    None => futures::future::pending().await,
                }
            },
        }
    }

    #[allow(unused)]
    pub fn print_replay_state(&self) {
        tracing::info!("Replay state: {:?}", self.replay_state.messages);
    }
}

enum NetworkState {
    Disabled {
        waiting_connects: Vec<WebsocketId>,
    },
    Enabled {
        websockets: BTreeMap<
            WebsocketId,
            (
                mpsc::UnboundedSender<(ClientMessage, Instant)>,
                mpsc::UnboundedReceiver<(ServerMessage, Instant)>,
            ),
        >,
    },
}

impl NetworkState {
    pub fn get_sender(
        &self,
        websocket_id: WebsocketId,
    ) -> anyhow::Result<&mpsc::UnboundedSender<(ClientMessage, Instant)>> {
        match self {
            NetworkState::Enabled { websockets } => {
                let (tx, _) = websockets
                    .get(&websocket_id)
                    .ok_or_else(|| anyhow::anyhow!("Unknown websocket id: {websocket_id}"))?;
                Ok(tx)
            },
            NetworkState::Disabled { .. } => anyhow::bail!("Network is disabled"),
        }
    }

    pub fn close(&mut self, websocket_id: WebsocketId) {
        match self {
            NetworkState::Enabled { websockets } => {
                websockets.remove(&websocket_id);
            },
            NetworkState::Disabled { .. } => (),
        }
    }
}

pub fn get_function<'a>(
    scope: &mut v8::HandleScope<'a>,
    namespace: v8::Local<v8::Object>,
    name: &str,
) -> anyhow::Result<v8::Local<'a, v8::Function>> {
    let v8_name =
        v8::String::new(scope, name).ok_or_else(|| anyhow::anyhow!("Failed to create {name}"))?;
    let v8_function = namespace
        .get(scope, v8_name.into())
        .ok_or_else(|| anyhow::anyhow!("Missing {name}"))?
        .try_into()?;
    Ok(v8_function)
}

pub fn extract_error<'a>(
    scope: &mut v8::HandleScope<'a>,
    err: v8::Local<'a, v8::Value>,
) -> anyhow::Result<JsError> {
    let (message, frame_data, custom_data) = extract_source_mapped_error(scope, err)?;
    let err = JsError::from_frames(message, frame_data, custom_data, |url| {
        if url.as_str() == "convex:/test.js" {
            return Ok(Some(TEST_SOURCE_MAP.clone()));
        }
        tracing::error!("Unknown source URL: {url}");
        Ok(None)
    });
    Ok(err)
}

#[allow(unused)]
#[derive(Debug)]
pub enum ReplayMessage {
    AddSyncQuery {
        id: String,
        name: String,
        args: JsonValue,
    },
    RemoveSyncQuery {
        id: String,
    },
    Mutate {
        id: String,
        mutation_info: JsonValue,
    },
    WsMessage {
        message: String,
    },
    CheckSyncQueryResult {
        id: String,
        expected_result: Option<SyncQueryResult>,
    },
    CheckMutationStatus {
        id: String,
        expected_status: Option<SyncMutationStatus>,
    },
    WaitForAllMessagesProcessed,
}

#[derive(Debug)]
pub struct ReplayState {
    messages: Vec<ReplayMessage>,
}

impl ReplayState {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    pub fn add_message(&mut self, message: ReplayMessage) {
        self.messages.push(message);
    }
}
