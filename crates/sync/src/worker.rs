use std::{
    collections::BTreeMap,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering,
        },
        Arc,
    },
    time::Duration,
};

use ::metrics::StatusTimer;
use application::{
    api::{
        ApplicationApi,
        ExecuteQueryTimestamp,
        SubscriptionClient,
        SubscriptionTrait,
    },
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
    RedactedActionError,
    RedactedMutationError,
};
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        ExportPath,
    },
    fastrace_helpers::get_sampled_span,
    http::ResolvedHostname,
    knobs::{
        SEARCH_INDEXES_UNAVAILABLE_RETRY_DELAY,
        SYNC_MAX_SEND_TRANSITION_COUNT,
    },
    runtime::{
        try_join_buffer_unordered,
        Runtime,
        WithTimeout,
    },
    types::{
        FunctionCaller,
        UdfType,
    },
    value::JsonPackedValue,
    version::ClientVersion,
    RequestId,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use fastrace::prelude::*;
use futures::{
    future::{
        self,
        BoxFuture,
        Fuse,
    },
    select_biased,
    stream::{
        Buffered,
        FuturesUnordered,
    },
    Future,
    FutureExt,
    StreamExt,
};
use keybroker::Identity;
use maplit::btreemap;
use model::session_requests::types::SessionRequestIdentifier;
use sync_types::{
    ClientMessage,
    IdentityVersion,
    QueryId,
    QuerySetModification,
    QuerySetVersion,
    SerializedQueryJournal,
    SessionId,
    StateModification,
    StateVersion,
    Timestamp,
    UdfPath,
};
use tokio::sync::{
    mpsc,
    mpsc::error::{
        SendError,
        TrySendError,
    },
};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    metrics::{
        self,
        connect_timer,
        modify_query_to_transition_timer,
        mutation_queue_timer,
        TypedClientEvent,
    },
    state::SyncState,
    ServerMessage,
};

// Buffer up to a thousand function and mutations executions.
const OPERATION_QUEUE_BUFFER_SIZE: usize = 1000;
const SYNC_WORKER_PROCESS_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub struct SyncWorkerConfig {
    pub client_version: ClientVersion,
}

impl Default for SyncWorkerConfig {
    fn default() -> Self {
        Self {
            client_version: ClientVersion::unknown(),
        }
    }
}

/// Creates a channel which allows the sender to track the buffer size and
/// opt-in to slow down if the buffer becomes too large.
pub fn measurable_unbounded_channel() -> (SingleFlightSender, SingleFlightReceiver) {
    let buffer_size_bytes = Arc::new(AtomicUsize::new(0));
    // The channel is used to send/receive "size reduced" notifications.
    let (size_reduced_tx, size_reduced_rx) = mpsc::channel(1);
    let (tx, rx) = mpsc::unbounded_channel();
    (
        SingleFlightSender {
            inner: tx,
            transition_count: buffer_size_bytes.clone(),
            count_reduced_rx: size_reduced_rx,
        },
        SingleFlightReceiver {
            inner: rx,
            transition_count: buffer_size_bytes,
            size_reduced_tx,
        },
    )
}

/// Wrapper around UnboundedSender that counts Transition messages,
/// allowing single-flighting, i.e. skipping transitions if the client is
/// backlogged on receiving them.
pub struct SingleFlightSender {
    inner: mpsc::UnboundedSender<(ServerMessage, tokio::time::Instant)>,

    transition_count: Arc<AtomicUsize>,
    count_reduced_rx: mpsc::Receiver<()>,
}

impl SingleFlightSender {
    pub fn send(
        &mut self,
        msg: (ServerMessage, tokio::time::Instant),
    ) -> Result<(), SendError<(ServerMessage, tokio::time::Instant)>> {
        if matches!(&msg.0, ServerMessage::Transition { .. }) {
            self.transition_count.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.send(msg)
    }

    pub fn transition_count(&self) -> usize {
        self.transition_count.load(Ordering::SeqCst)
    }

    // Waits until a single message has been received implying the size of the
    // buffer have been reduced. Note that if multiple messages are received
    // between calls, this will fire only once.
    pub async fn message_consumed(&mut self) {
        self.count_reduced_rx.recv().await;
    }
}

pub struct SingleFlightReceiver {
    inner: mpsc::UnboundedReceiver<(ServerMessage, tokio::time::Instant)>,

    transition_count: Arc<AtomicUsize>,
    size_reduced_tx: mpsc::Sender<()>,
}

impl SingleFlightReceiver {
    pub async fn next(&mut self) -> Option<(ServerMessage, tokio::time::Instant)> {
        let result = self.inner.recv().await;
        if let Some(msg) = &result {
            if matches!(msg.0, ServerMessage::Transition { .. }) {
                self.transition_count.fetch_sub(1, Ordering::SeqCst);
            }
            // Don't block if channel is full.
            _ = self.size_reduced_tx.try_send(());
        }
        result
    }

    pub fn try_next(&mut self) -> Option<(ServerMessage, tokio::time::Instant)> {
        let result = self.inner.try_recv().ok();
        if let Some(msg) = &result {
            if matches!(msg.0, ServerMessage::Transition { .. }) {
                self.transition_count.fetch_sub(1, Ordering::SeqCst);
            }
            // Don't block if channel is full.
            _ = self.size_reduced_tx.try_send(());
        }
        result
    }
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

pub struct SyncWorker<RT: Runtime> {
    api: Arc<dyn ApplicationApi>,
    config: SyncWorkerConfig,
    rt: RT,
    state: SyncState,
    host: ResolvedHostname,

    rx: mpsc::UnboundedReceiver<(ClientMessage, tokio::time::Instant)>,
    tx: SingleFlightSender,

    // Queue of pending functions or mutations. For time being, we only execute
    // a single one since this is less error prone model for the developer.
    mutation_futures: Buffered<ReceiverStream<BoxFuture<'static, anyhow::Result<ServerMessage>>>>,
    mutation_sender: mpsc::Sender<BoxFuture<'static, anyhow::Result<ServerMessage>>>,

    action_futures: FuturesUnordered<BoxFuture<'static, anyhow::Result<ServerMessage>>>,

    transition_future: Option<Fuse<BoxFuture<'static, anyhow::Result<TransitionState>>>>,

    // Has an update been scheduled for the future?
    update_scheduled: bool,

    /// If we've seen a SearchIndexesUnavailable error, wait for this Future to
    /// resolve before retrying
    search_query_retry_future: Option<Fuse<BoxFuture<'static, ()>>>,

    /// Timers to track time between handling ModifyQuerySet message and sending
    /// the Transition with the update
    modify_query_to_transition_timers: BTreeMap<QuerySetVersion, StatusTimer>,

    on_connect: Option<(StatusTimer, Box<dyn FnOnce(SessionId) + Send>)>,
}

enum QueryResult {
    Rerun {
        result: Result<JsonPackedValue, RedactedJsError>,
        log_lines: RedactedLogLines,
        journal: SerializedQueryJournal,
    },
    /// Skip returning results of this query because search indexes are
    /// unavailable
    SearchIndexesUnavailable,
    Refresh,
}

struct TransitionState {
    udf_results: Vec<(QueryId, QueryResult, Box<dyn SubscriptionTrait>)>,
    state_modifications: BTreeMap<QueryId, StateModification<JsonPackedValue>>,
    current_version: StateVersion,
    new_version: StateVersion,
    timer: StatusTimer,
    search_indexes_unavailable: bool,
}

impl<RT: Runtime> SyncWorker<RT> {
    pub fn new(
        api: Arc<dyn ApplicationApi>,
        rt: RT,
        host: ResolvedHostname,
        config: SyncWorkerConfig,
        rx: mpsc::UnboundedReceiver<(ClientMessage, tokio::time::Instant)>,
        tx: SingleFlightSender,
        on_connect: Box<dyn FnOnce(SessionId) + Send>,
    ) -> Self {
        let (mutation_sender, receiver) = mpsc::channel(OPERATION_QUEUE_BUFFER_SIZE);
        let mutation_futures = ReceiverStream::new(receiver).buffered(1); // Execute at most one operation at a time.
        SyncWorker {
            api,
            config,
            rt,
            state: SyncState::new(),
            host,
            rx,
            tx,
            mutation_futures,
            mutation_sender,
            action_futures: FuturesUnordered::new(),
            transition_future: None,
            update_scheduled: false,
            search_query_retry_future: None,
            modify_query_to_transition_timers: BTreeMap::new(),
            on_connect: Some((connect_timer(), on_connect)),
        }
    }

    fn schedule_update(&mut self) {
        self.update_scheduled = true;
    }

    fn schedule_search_query_retry(&mut self) {
        if self.search_query_retry_future.is_none() {
            let rt = self.rt.clone();
            self.search_query_retry_future = Some(
                async move {
                    rt.wait(*SEARCH_INDEXES_UNAVAILABLE_RETRY_DELAY).await;
                }
                .boxed()
                .fuse(),
            );
        }
    }

    /// Run the sync protocol worker, returning `Ok(())` on clean exit and `Err`
    /// if there's an exceptional protocol condition that should shutdown
    /// the WebSocket.
    pub async fn go(&mut self) -> anyhow::Result<()> {
        let mut ping_timeout = self.rt.wait(HEARTBEAT_INTERVAL);
        let mut pending = future::pending().boxed().fuse();
        let mut search_retry_pending = future::pending().boxed().fuse();

        // Create a new subscription client for every sync socket. Thus we don't require
        // the subscription client to auto-recover on connection failures.
        let subscription_client: Arc<dyn SubscriptionClient> =
            self.api.subscription_client(&self.host).await?.into();

        // Starts off as a future that is never ready, as there's no identity that may
        // expire.
        'top: loop {
            let rt = self.rt.clone();
            self.state.validate()?;
            let maybe_response = select_biased! {
                message = self.rx.recv().fuse() => {
                    let (message, received_time) = match message {
                        Some(m) => m,
                        None => break 'top,
                    };
                    self.handle_message(message).await?;
                    let delay = self.rt.monotonic_now() - received_time;
                    metrics::log_process_client_message_delay(delay);
                    None
                },
                // TODO(presley): If I swap this with futures below, tests break.
                // We need to provide a guarantee that we can't transition to a
                // timestamp past a pending mutation or otherwise optimistic updates
                // might be flaky. To do that, we need to behave differently if we
                // have pending operation future or not.
                result = self.mutation_futures.next().fuse() => {
                    let message = match result {
                        Some(m) => m?,
                        None => panic!("mutation_futures sender dropped prematurely"),
                    };
                    self.schedule_update();
                    Some(message)
                },
                result = self.action_futures.select_next_some() => {
                    self.schedule_update();
                    Some(result?)
                },
                result = self.state.next_invalidated_query().fuse() => {
                    let _ = result?;
                    self.schedule_update();
                    None
                },
                transition_state = self.transition_future.as_mut().unwrap_or(&mut pending) => {
                    self.transition_future = None;
                    Some(self.finish_update_queries(transition_state?)?)
                },
                _ = self.search_query_retry_future
                        .as_mut()
                        .unwrap_or(&mut search_retry_pending) => {
                    self.search_query_retry_future = None;
                    self.schedule_update();
                    None
                },
                _ = self.tx.message_consumed().fuse() => {
                    // Wake up if any message is consumed from the send buffer
                    // in case update_scheduled is True.
                    None
                }
                _ = ping_timeout => Some(ServerMessage::Ping {}),
            };
            // If there is a message to return to the client, send it.
            if let Some(response) = maybe_response {
                assert!(
                    !matches!(response, ServerMessage::FatalError { .. })
                        && !matches!(response, ServerMessage::AuthError { .. }),
                    "fatal errors are returned above when handling special error types",
                );
                // Break and exit cleanly if the websocket is dead.
                ping_timeout = self.rt.wait(HEARTBEAT_INTERVAL);
                if self.tx.send((response, self.rt.monotonic_now())).is_err() {
                    break 'top;
                }
            }
            // Send update unless the send channel already contains enough transitions,
            // and unless we are already computing an update.
            if self.update_scheduled
                && self.tx.transition_count() < *SYNC_MAX_SEND_TRANSITION_COUNT
                && self.transition_future.is_none()
            {
                // Always transition to the latest timestamp. In the future,
                // when we have Sync Worker running on the edge, we can remove this
                // call by making self.update_scheduled to be a Option<Timestamp>,
                // and set it accordingly based on the operation that triggered the
                // Transition. We would choose the latest timestamp available at
                // the edge for the initial sync.
                let target_ts = *self
                    .api
                    .latest_timestamp(&self.host, RequestId::new())
                    .await?;
                let new_transition_future =
                    self.begin_update_queries(target_ts, subscription_client.clone())?;
                self.transition_future = Some(
                    async move {
                        rt.with_timeout(
                            "update_queries",
                            SYNC_WORKER_PROCESS_TIMEOUT,
                            new_transition_future,
                        )
                        .await
                    }
                    .boxed()
                    .fuse(),
                );
                self.update_scheduled = false;
            }
        }
        Ok(())
    }

    pub fn identity_version(&self) -> IdentityVersion {
        self.state.current_version().identity
    }

    pub fn parse_admin_component_path(
        component_path: &str,
        udf_path: &UdfPath,
        identity: &Identity,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath> {
        let path = ComponentPath::deserialize(Some(component_path))?;
        anyhow::ensure!(
            path.is_root() || identity.is_admin() || identity.is_system(),
            "Only admin or system users can call functions on non-root components directly"
        );
        let path = CanonicalizedComponentFunctionPath {
            component: path,
            udf_path: udf_path.clone().canonicalize(),
        };
        Ok(path)
    }

    async fn handle_message(&mut self, message: ClientMessage) -> anyhow::Result<()> {
        let timer = metrics::handle_message_timer(&message);
        match message {
            ClientMessage::Connect {
                session_id,
                last_close_reason,
                max_observed_timestamp,
                connection_count,
            } => {
                if let Some((timer, on_connect)) = self.on_connect.take() {
                    timer.finish();
                    on_connect(session_id);
                }
                self.state.set_session_id(session_id);
                if let Some(max_observed_timestamp) = max_observed_timestamp {
                    let latest_timestamp = *self
                        .api
                        .latest_timestamp(&self.host, RequestId::new())
                        .await?;
                    if max_observed_timestamp > latest_timestamp {
                        // Unless there is a bug, this means the client have communicated
                        // with a backend that have database writes we are not aware of. If
                        // we serve the request, we will get a linearizability violation.
                        // Instead error and report. It is possible we have to eventually turn
                        // into a client error if there are bogus custom client implementations
                        // but lets keep it as server one for now.
                        metrics::log_linearizability_violation(
                            max_observed_timestamp.secs_since_f64(latest_timestamp),
                        );
                        anyhow::bail!(
                            "Client has observed a timestamp {max_observed_timestamp:?} ahead of \
                             the backend latest known timestamp {latest_timestamp:?}",
                        );
                    }
                }
                metrics::log_connect(last_close_reason, connection_count)
            },
            ClientMessage::ModifyQuerySet {
                base_version,
                new_version,
                modifications,
            } => {
                self.state
                    .modify_query_set(base_version, new_version, modifications)?;
                self.schedule_update();
                self.modify_query_to_transition_timers
                    .insert(new_version, modify_query_to_transition_timer());
            },
            ClientMessage::Mutation {
                request_id,
                udf_path,
                args,
                component_path,
            } => {
                let identity = self.state.identity(self.rt.system_time())?;
                let mutation_identifier =
                    self.state.session_id().map(|id| SessionRequestIdentifier {
                        session_id: id,
                        request_id,
                    });
                let server_request_id = match self.state.session_id() {
                    Some(id) => RequestId::new_for_ws_session(id, request_id),
                    None => RequestId::new(),
                };
                let root = get_sampled_span(
                    &self.host.instance_name,
                    "sync-worker/mutation",
                    &mut self.rt.rng(),
                    btreemap! {
                       "udf_type".into() => UdfType::Mutation.to_lowercase_string().into(),
                       "udf_path".into() => udf_path.clone().into(),
                    },
                );
                let rt = self.rt.clone();
                let client_version = self.config.client_version.clone();
                let timer = mutation_queue_timer();
                let api = self.api.clone();
                let host = self.host.clone();
                let caller = FunctionCaller::SyncWorker(client_version);

                let mutation_queue_size =
                    self.mutation_sender.max_capacity() - self.mutation_sender.capacity();
                root.add_property(|| ("mutation_queue_size", mutation_queue_size.to_string()));

                let future = async move {
                    rt.with_timeout("mutation", SYNC_WORKER_PROCESS_TIMEOUT, async move {
                        timer.finish();
                        let result = match component_path {
                            None => {
                                api.execute_public_mutation(
                                    &host,
                                    server_request_id,
                                    identity,
                                    ExportPath::from(udf_path.canonicalize()),
                                    args,
                                    caller,
                                    mutation_identifier,
                                    Some(mutation_queue_size),
                                )
                                .in_span(root)
                                .await?
                            },
                            Some(ref p) => {
                                let path =
                                    Self::parse_admin_component_path(p, &udf_path, &identity)?;
                                api.execute_admin_mutation(
                                    &host,
                                    server_request_id,
                                    identity,
                                    path,
                                    args,
                                    caller,
                                    mutation_identifier,
                                    Some(mutation_queue_size),
                                )
                                .in_span(root)
                                .await?
                            },
                        };
                        let response = match result {
                            Ok(udf_return) => ServerMessage::MutationResponse {
                                request_id,
                                result: Ok(udf_return.value),
                                ts: Some(udf_return.ts),
                                log_lines: udf_return.log_lines.into(),
                            },
                            Err(RedactedMutationError { error, log_lines }) => {
                                ServerMessage::MutationResponse {
                                    request_id,
                                    result: Err(error.into_error_payload()),
                                    ts: None,
                                    log_lines: log_lines.into(),
                                }
                            },
                        };
                        Ok(response)
                    })
                    .await
                }
                .boxed();
                self.mutation_sender.try_send(future).map_err(|err| {
                    if matches!(err, TrySendError::Full(..)) {
                        anyhow::anyhow!(ErrorMetadata::rate_limited(
                            "TooManyConcurrentMutations",
                            format!(
                                "Too many concurrent mutations. Only up to \
                                 {OPERATION_QUEUE_BUFFER_SIZE} pending mutations allowed on a \
                                 single websocket."
                            ),
                        ))
                    } else {
                        anyhow::anyhow!("Failed to send to mutation channel: {err}")
                    }
                })?;
            },
            ClientMessage::Action {
                request_id,
                udf_path,
                args,
                component_path,
            } => {
                let identity = self.state.identity(self.rt.system_time())?;

                let api = self.api.clone();
                let host = self.host.clone();
                let client_version = self.config.client_version.clone();
                let server_request_id = match self.state.session_id() {
                    Some(id) => RequestId::new_for_ws_session(id, request_id),
                    None => RequestId::new(),
                };
                let root = get_sampled_span(
                    &self.host.instance_name,
                    "sync-worker/action",
                    &mut self.rt.rng(),
                    btreemap! {
                       "udf_type".into() => UdfType::Action.to_lowercase_string().into(),
                       "udf_path".into() => udf_path.clone().into(),
                    },
                );
                let future = async move {
                    let caller = FunctionCaller::SyncWorker(client_version);
                    let result = match component_path {
                        None => {
                            api.execute_public_action(
                                &host,
                                server_request_id,
                                identity,
                                ExportPath::from(udf_path.canonicalize()),
                                args,
                                caller,
                            )
                            .in_span(root)
                            .await?
                        },
                        Some(ref p) => {
                            let path = Self::parse_admin_component_path(p, &udf_path, &identity)?;
                            api.execute_admin_action(
                                &host,
                                server_request_id,
                                identity,
                                path,
                                args,
                                caller,
                            )
                            .in_span(root)
                            .await?
                        },
                    };
                    let response = match result {
                        Ok(udf_return) => ServerMessage::ActionResponse {
                            request_id,
                            result: Ok(udf_return.value),
                            log_lines: udf_return.log_lines.into(),
                        },
                        Err(RedactedActionError { error, log_lines }) => {
                            ServerMessage::ActionResponse {
                                request_id,
                                result: Err(error.into_error_payload()),
                                log_lines: log_lines.into(),
                            }
                        },
                    };
                    Ok(response)
                }
                .boxed();
                anyhow::ensure!(
                    self.action_futures.len() <= OPERATION_QUEUE_BUFFER_SIZE,
                    "Inflight actions overloaded, max concurrency: {OPERATION_QUEUE_BUFFER_SIZE}"
                );
                self.action_futures.push(future);
            },
            ClientMessage::Authenticate {
                token: auth_token,
                base_version,
            } => {
                let identity_result = self
                    .api
                    .authenticate(&self.host, RequestId::new(), auth_token)
                    .await;
                let identity = match identity_result {
                    Ok(identity) => identity,
                    Err(e) => {
                        let short_msg = e.short_msg().to_string();
                        let msg = e.msg().to_string();
                        // If the auth token is invalid, we want to signal the client
                        // that we tried to update the auth token but failed, which will
                        // prompt the client to not try the same token again.
                        return Err(ErrorMetadata::auth_update_failed(short_msg, msg).into());
                    },
                };
                self.state.modify_identity(identity, base_version)?;
                self.schedule_update();
            },
            ClientMessage::Event(client_event) => {
                tracing::info!(
                    "Event with type {}: {}",
                    client_event.event_type,
                    client_event.event
                );
                match TypedClientEvent::try_from(client_event) {
                    Ok(typed_client_event) => match typed_client_event {
                        TypedClientEvent::ClientConnect { marks } => {
                            metrics::log_client_connect_timings(marks)
                        },
                    },
                    Err(_) => (),
                }
            },
        };

        timer.finish();
        Ok(())
    }

    fn begin_update_queries(
        &mut self,
        new_ts: Timestamp,
        subscriptions_client: Arc<dyn SubscriptionClient>,
    ) -> anyhow::Result<impl Future<Output = anyhow::Result<TransitionState>>> {
        let root = get_sampled_span(
            &self.host.instance_name,
            "sync-worker/update-queries",
            &mut self.rt.rng(),
            btreemap! {
               "udf_type".into() => UdfType::Query.to_lowercase_string().into(),
            },
        );
        let _guard = root.set_local_parent();
        let timer = metrics::update_queries_timer();
        let current_version = self.state.current_version();

        let (modifications, new_query_version, pending_identity, new_identity_version) =
            self.state.take_modifications();

        let mut identity_version = current_version.identity;
        if let Some(new_identity) = pending_identity {
            // If the identity version has changed, invalidate all existing tokens.
            // TODO(CX-737): Don't invalidate queries that don't examine auth state.
            // TODO(CX-737): Don't invalidate the queries if the User the is the same
            // only with refreshed token. This is a bit tricky because:
            // - We need to prove that query does not depend on token issue/expiration time.
            // - We need to make rpc to backend to compare the properties since Usher can't
            // validate auth tokens. Alternatively, we make Usher be able to validate tokens
            // long term.
            self.state.take_subscriptions();
            self.state.insert_identity(new_identity);
            identity_version = new_identity_version;
        }
        let identity = self.state.identity(self.rt.system_time())?;

        // Step 1: Decide on a new target (query set version, identity version, ts) for
        // the system.
        let new_version = StateVersion {
            ts: new_ts,
            // We only bump the query set version when the client modifies the query set
            query_set: new_query_version,
            identity: identity_version,
        };

        // Step 2: Add or remove queries from our query set.
        let mut state_modifications = BTreeMap::new();
        for modification in modifications {
            match modification {
                QuerySetModification::Add(query) => {
                    self.state.insert(query)?;
                },
                QuerySetModification::Remove { query_id } => {
                    self.state.remove(query_id)?;
                    state_modifications
                        .insert(query_id, StateModification::QueryRemoved { query_id });
                },
            }
        }

        // Step 3: Take all remaining subscriptions.
        let mut remaining_subscriptions = self.state.take_subscriptions();

        // Step 4: Refresh subscriptions up to new_ts and run queries which
        // subscriptions are no longer current.
        let api = self.api.clone();
        let need_fetch: Vec<_> = self.state.need_fetch().collect();
        let host = self.host.clone();
        let client_version = self.config.client_version.clone();
        Ok(async move {
            let future_results: anyhow::Result<Vec<_>> = try_join_buffer_unordered(
                "update_query",
                need_fetch.into_iter().map(move |query| {
                    let api = api.clone();
                    let host = host.clone();
                    let identity_ = identity.clone();
                    let client_version = client_version.clone();
                    let current_subscription = remaining_subscriptions.remove(&query.query_id);
                    let subscriptions_client = subscriptions_client.clone();
                    async move {
                        LocalSpan::add_property(|| ("udf_path", query.udf_path.to_string()));
                        let new_subscription = match current_subscription {
                            Some(subscription) => {
                                if subscription.extend_validity(new_ts).await? {
                                    Some(subscription)
                                } else {
                                    None
                                }
                            },
                            None => None,
                        };
                        let (query_result, subscription) = match new_subscription {
                            Some(subscription) => (QueryResult::Refresh, Some(subscription)),
                            None => {
                                // We failed to refresh the subscription or it was invalid to start
                                // with. Rerun the query.
                                let caller = FunctionCaller::SyncWorker(client_version);
                                let ts = ExecuteQueryTimestamp::At(new_ts);

                                // This query run might have been triggered due to invalidation
                                // of a subscription. The sync worker is effectively the owner
                                // of the query so we do not want to re-use the original query
                                // request id.
                                let request_id = RequestId::new();
                                let udf_return_result = match query.component_path {
                                    None => {
                                        api.execute_public_query(
                                            &host,
                                            request_id,
                                            identity_,
                                            ExportPath::from(query.udf_path.canonicalize()),
                                            query.args,
                                            caller,
                                            ts,
                                            query.journal,
                                        )
                                        .await
                                    },
                                    Some(ref p) => {
                                        let path = Self::parse_admin_component_path(
                                            p,
                                            &query.udf_path,
                                            &identity_,
                                        )?;
                                        api.execute_admin_query(
                                            &host,
                                            request_id,
                                            identity_,
                                            path,
                                            query.args,
                                            caller,
                                            ts,
                                            query.journal,
                                        )
                                        .await
                                    },
                                };
                                match udf_return_result {
                                    Err(e) => {
                                        if let Some(error) = e.downcast_ref::<ErrorMetadata>()
                                            && error.short_msg == "SearchIndexesUnavailable"
                                        {
                                            (QueryResult::SearchIndexesUnavailable, None)
                                        } else {
                                            anyhow::bail!(e)
                                        }
                                    },
                                    Ok(udf_return) => {
                                        let subscription = subscriptions_client
                                            .subscribe(udf_return.token)
                                            .await?;
                                        (
                                            QueryResult::Rerun {
                                                result: udf_return.result,
                                                log_lines: udf_return.log_lines,
                                                journal: udf_return.journal,
                                            },
                                            Some(subscription),
                                        )
                                    },
                                }
                            },
                        };
                        Ok::<_, anyhow::Error>((query.query_id, query_result, subscription))
                    }
                }),
            )
            .await;

            let mut udf_results = vec![];
            let mut search_indexes_unavailable = false;
            for result in future_results? {
                let (query_id, result, maybe_subscription) = result;
                if matches!(result, QueryResult::SearchIndexesUnavailable) {
                    search_indexes_unavailable = true;
                }
                if let Some(subscription) = maybe_subscription {
                    udf_results.push((query_id, result, subscription));
                }
            }

            Ok(TransitionState {
                udf_results,
                state_modifications,
                current_version,
                new_version,
                timer,
                search_indexes_unavailable,
            })
        }
        .in_span(root))
    }

    fn finish_update_queries(
        &mut self,
        TransitionState {
            udf_results,
            mut state_modifications,
            current_version,
            new_version,
            timer,
            search_indexes_unavailable,
        }: TransitionState,
    ) -> anyhow::Result<ServerMessage> {
        for (query_id, result, subscription) in udf_results {
            match result {
                QueryResult::Rerun {
                    result,
                    log_lines,
                    journal,
                } => {
                    let modification = self.state.complete_fetch(
                        query_id,
                        result,
                        log_lines,
                        journal,
                        subscription,
                    )?;
                    let Some(modification) = modification else {
                        continue;
                    };
                    state_modifications.insert(query_id, modification);
                },
                QueryResult::Refresh => {
                    self.state.refill_subscription(query_id, subscription)?;
                },
                QueryResult::SearchIndexesUnavailable => {
                    anyhow::bail!(
                        "No QueryResult::SearchIndexesUnavailable should have a udf result and \
                         subscription"
                    )
                },
            }
        }

        if search_indexes_unavailable {
            self.schedule_search_query_retry();
        }

        // Resubscribe for queries that don't have an active invalidation
        // future.
        self.state.fill_invalidation_futures()?;

        // Step 6: Send our transition to the client and update our version.
        self.state.advance_version(new_version)?;
        let transition = ServerMessage::Transition {
            start_version: current_version,
            end_version: new_version,
            modifications: state_modifications.into_values().collect(),
        };
        timer.finish();
        metrics::log_query_set_size(self.state.num_queries());
        // Only retain timers for queries that haven't been updated yet. Finish the
        // timers for everything up through the new version.
        let finished_timers = self
            .modify_query_to_transition_timers
            .extract_if(|version, _| *version <= new_version.query_set);
        for (_, timer) in finished_timers {
            timer.finish();
        }
        Ok(transition)
    }
}
