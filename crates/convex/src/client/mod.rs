use std::{
    collections::BTreeMap,
    convert::Infallible,
    sync::Arc,
};

use convex_sync_types::{
    AuthenticationToken,
    UdfPath,
    UserIdentityAttributes,
};
#[cfg(doc)]
use futures::Stream;
use futures::{
    channel::mpsc,
    SinkExt,
    StreamExt,
};
use tokio::{
    sync::{
        broadcast,
        oneshot,
    },
    task::JoinHandle,
};
use tokio_stream::wrappers::BroadcastStream;
use url::Url;

use self::worker::AuthenticateRequest;
#[cfg(doc)]
use crate::SubscriberId;
use crate::{
    base_client::{
        BaseConvexClient,
        QueryResults,
    },
    client::{
        subscription::{
            QuerySetSubscription,
            QuerySubscription,
        },
        worker::{
            worker,
            ActionRequest,
            ClientRequest,
            MutationRequest,
            SubscribeRequest,
        },
    },
    sync::{
        web_socket_manager::WebSocketManager,
        SyncProtocol,
    },
    value::Value,
    FunctionResult,
};

pub mod subscription;
mod worker;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

/// An asynchronous client to interact with a specific project to perform
/// mutations and manage query subscriptions using [`tokio`].
///
/// The Convex client requires a deployment url,
/// which can be found in the [dashboard](https://dashboard.convex.dev/) settings tab.
///
/// ```no_run
/// use convex::ConvexClient;
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let mut client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
///     let mut sub = client.subscribe("listMessages", maplit::btreemap!{}).await?;
///     while let Some(result) = sub.next().await {
///         println!("{result:?}");
///     }
///     Ok(())
/// }
/// ```
///
/// The [`ConvexClient`] internally holds a connection and a [`tokio`]
/// background task to manage it. It is advised that you create one and
/// **reuse** it. You can safely clone with [`ConvexClient::clone()`] to share
/// the connection and outstanding subscriptions.
///
/// ## Examples
/// For example code, please refer to the examples directory.
pub struct ConvexClient {
    listen_handle: Option<Arc<JoinHandle<Infallible>>>,
    request_sender: mpsc::UnboundedSender<ClientRequest>,
    watch_receiver: broadcast::Receiver<QueryResults>,
}

/// Clone the [`ConvexClient`], sharing the connection and outstanding
/// subscriptions.
impl Clone for ConvexClient {
    fn clone(&self) -> Self {
        Self {
            listen_handle: self.listen_handle.clone(),
            request_sender: self.request_sender.clone(),
            watch_receiver: self.watch_receiver.resubscribe(),
        }
    }
}

/// Drop the [`ConvexClient`]. When the final reference to the [`ConvexClient`]
/// is dropped, the connection is cleaned up.
impl Drop for ConvexClient {
    fn drop(&mut self) {
        if let Ok(j_handle) = Arc::try_unwrap(
            self.listen_handle
                .take()
                .expect("INTERNAL BUG: listen handle should never be none"),
        ) {
            j_handle.abort()
        }
    }
}

impl ConvexClient {
    /// Constructs a new client for communicating with `deployment_url`.
    ///
    /// ```no_run
    /// # use convex::ConvexClient;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(deployment_url: &str) -> anyhow::Result<Self> {
        let client_id = format!("rust-{}", VERSION.unwrap_or("unknown"));
        Self::new_with_client_id(deployment_url, &client_id).await
    }

    #[doc(hidden)]
    pub async fn new_with_client_id(deployment_url: &str, client_id: &str) -> anyhow::Result<Self> {
        let ws_url = deployment_to_ws_url(deployment_url.try_into()?)?;

        // Channels for the `listen` background thread
        let (response_sender, response_receiver) = mpsc::channel(1);
        let (request_sender, request_receiver) = mpsc::unbounded();

        // Listener for when each transaction completes
        let (watch_sender, watch_receiver) = broadcast::channel(1);

        let base_client = BaseConvexClient::new();

        let protocol = WebSocketManager::open(ws_url, response_sender, client_id).await?;

        let listen_handle = tokio::spawn(worker(
            response_receiver,
            request_receiver,
            watch_sender,
            base_client,
            protocol,
        ));
        let client = ConvexClient {
            listen_handle: Some(Arc::new(listen_handle)),
            request_sender,
            watch_receiver,
        };
        Ok(client)
    }

    /// Subscribe to the results of query `name` called with `args`.
    ///
    /// Returns a [`QuerySubscription`] which implements [`Stream`]<
    /// [`FunctionResult`]>. A new value appears on the stream each
    /// time the query function produces a new result.
    ///
    /// The subscription is automatically unsubscribed when it is dropped.
    ///
    /// ```no_run
    /// # use convex::ConvexClient;
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
    /// let mut sub = client.subscribe("listMessages", maplit::btreemap!{}).await?;
    /// while let Some(result) = sub.next().await {
    ///     println!("{result:?}");
    /// }
    /// # Ok(())
    /// # }
    pub async fn subscribe(
        &mut self,
        name: &str,
        args: BTreeMap<String, Value>,
    ) -> anyhow::Result<QuerySubscription> {
        let (tx, rx) = oneshot::channel();

        let udf_path = name.parse()?;
        let request = SubscribeRequest { udf_path, args };

        self.request_sender
            .send(ClientRequest::Subscribe(
                request,
                tx,
                self.request_sender.clone(),
            ))
            .await?;

        let res = rx.await?;
        Ok(res)
    }

    /// Make a oneshot request to a query `name` with `args`.
    ///
    /// Returns a [`FunctionResult`] representing the result of the query.
    ///
    /// This method is syntactic sugar for waiting for a single result on
    /// a subscription.
    /// It is equivalent to `client.subscribe(name,
    /// args).await?.next().unwrap()`
    ///
    /// ```no_run
    /// # use convex::ConvexClient;
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
    /// let result = client.query("listMessages", maplit::btreemap!{}).await?;
    /// println!("{result:?}");
    /// # Ok(())
    /// # }
    pub async fn query(
        &mut self,
        name: &str,
        args: BTreeMap<String, Value>,
    ) -> anyhow::Result<FunctionResult> {
        Ok(self
            .subscribe(name, args)
            .await?
            .next()
            .await
            .expect("INTERNAL BUG: Convex Client dropped prematurely."))
    }

    /// Perform a mutation `name` with `args` and return a future
    /// containing the return value of the mutation once it completes.
    ///
    /// ```no_run
    /// # use convex::ConvexClient;
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
    /// let result = client.mutation("sendMessage", maplit::btreemap!{
    ///     "body".into() => "Let it be.".into(),
    ///     "author".into() => "The Beatles".into(),
    /// }).await?;
    /// println!("{result:?}");
    /// # Ok(())
    /// # }
    pub async fn mutation(
        &mut self,
        name: &str,
        args: BTreeMap<String, Value>,
    ) -> anyhow::Result<FunctionResult> {
        let (tx, rx) = oneshot::channel();

        let udf_path: UdfPath = name.parse()?;
        let request = MutationRequest { udf_path, args };

        self.request_sender
            .send(ClientRequest::Mutation(request, tx))
            .await?;

        let res = rx.await?;
        Ok(res.await?)
    }

    /// Perform an action `name` with `args` and return a future
    /// containing the return value of the action once it completes.
    ///
    /// ```no_run
    /// # use convex::ConvexClient;
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
    /// let result = client.action("sendGif", maplit::btreemap!{
    ///     "body".into() => "Tatooine Sunrise.".into(),
    ///     "author".into() => "Luke Skywalker".into(),
    /// }).await?;
    /// println!("{result:?}");
    /// # Ok(())
    /// # }
    pub async fn action(
        &mut self,
        name: &str,
        args: BTreeMap<String, Value>,
    ) -> anyhow::Result<FunctionResult> {
        let (tx, rx) = oneshot::channel();

        let udf_path: UdfPath = name.parse()?;
        let request = ActionRequest { udf_path, args };

        self.request_sender
            .send(ClientRequest::Action(request, tx))
            .await?;

        let res = rx.await?;
        Ok(res.await?)
    }

    /// Get a consistent view of the results of multiple queries (query set).
    ///
    /// Returns a [`QuerySetSubscription`] which
    /// implements [`Stream`]<[`QueryResults`]>.
    /// Each item in the stream contains a consistent view
    /// of the results of all the queries in the query set.
    ///
    /// Queries can be added to the query set via [`ConvexClient::subscribe`].
    /// Queries can be removed from the query set via dropping the
    /// [`QuerySubscription`] token returned by [`ConvexClient::subscribe`].
    ///
    ///
    /// [`QueryResults`] is a copy-on-write mapping from [`SubscriberId`] to
    /// its latest result [`Value`].
    ///
    /// ```no_run
    /// # use convex::ConvexClient;
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut client = ConvexClient::new("https://cool-music-123.convex.cloud").await?;
    /// let mut watch = client.watch_all();
    /// let sub1 = client.subscribe("listMessages", maplit::btreemap!{
    ///     "channel".into() => 1.into(),
    /// }).await?;
    /// let sub2 = client.subscribe("listMessages", maplit::btreemap!{
    ///     "channel".into() => 1.into(),
    /// }).await?;
    /// # Ok(())
    /// # }
    pub fn watch_all(&self) -> QuerySetSubscription {
        QuerySetSubscription::new(BroadcastStream::new(self.watch_receiver.resubscribe()))
    }

    /// Set auth for use when calling Convex functions.
    ///
    /// Set it with a token that you get from your auth provider via their login
    /// flow. If `None` is passed as the token, then auth is unset (logging
    /// out).
    pub async fn set_auth(&mut self, token: Option<String>) {
        let req = AuthenticateRequest {
            token: match token {
                None => AuthenticationToken::None,
                Some(token) => AuthenticationToken::User(token),
            },
        };
        self.request_sender
            .send(ClientRequest::Authenticate(req))
            .await
            .expect("INTERNAL BUG: Worker has gone away");
    }

    /// Set admin auth for use when calling Convex functions as a deployment
    /// admin. Not typically required.
    ///
    /// You can get a deploy_key from the Convex dashboard's deployment settings
    /// page. Deployment admins can act as users as part of their
    /// development flow to see how a function would act.
    #[doc(hidden)]
    pub async fn set_admin_auth(
        &mut self,
        deploy_key: String,
        acting_as: Option<UserIdentityAttributes>,
    ) {
        let req = AuthenticateRequest {
            token: AuthenticationToken::Admin(deploy_key, acting_as),
        };
        self.request_sender
            .send(ClientRequest::Authenticate(req))
            .await
            .expect("INTERNAL BUG: Worker has gone away");
    }
}

fn deployment_to_ws_url(mut deployment_url: Url) -> anyhow::Result<Url> {
    let ws_scheme = match deployment_url.scheme() {
        "http" | "ws" => "ws",
        "https" | "wss" => "wss",
        scheme => anyhow::bail!("Unknown scheme {scheme}. Expected http or https."),
    };
    deployment_url
        .set_scheme(ws_scheme)
        .expect("Scheme not supported");
    deployment_url.set_path("api/sync");
    Ok(deployment_url)
}

#[cfg(test)]
pub mod tests {
    use std::{
        str::FromStr,
        sync::Arc,
        time::Duration,
    };

    use convex_sync_types::{
        AuthenticationToken,
        ClientMessage,
        LogLinesMessage,
        Query,
        QueryId,
        QuerySetModification,
        SessionId,
        StateModification,
        StateVersion,
        UdfPath,
        UserIdentityAttributes,
    };
    use futures::{
        channel::mpsc,
        StreamExt,
    };
    use maplit::btreemap;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tokio::sync::broadcast;

    use super::ConvexClient;
    use crate::{
        base_client::FunctionResult,
        client::{
            deployment_to_ws_url,
            worker::worker,
            BaseConvexClient,
        },
        sync::{
            testing::TestProtocolManager,
            ServerMessage,
            SyncProtocol,
        },
        value::Value,
    };

    impl ConvexClient {
        pub async fn with_test_protocol() -> anyhow::Result<(Self, TestProtocolManager)> {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .try_init();

            // Channels for the `listen` background thread
            let (response_sender, response_receiver) = mpsc::channel(1);
            let (request_sender, request_receiver) = mpsc::unbounded();

            // Listener for when each transaction completes
            let (watch_sender, watch_receiver) = broadcast::channel(1);

            let test_protocol =
                TestProtocolManager::open("ws://test.com".parse()?, response_sender, "rust-0.0.1")
                    .await?;
            let base_client = BaseConvexClient::new();

            let listen_handle = tokio::spawn(worker(
                response_receiver,
                request_receiver,
                watch_sender,
                base_client,
                test_protocol.clone(),
            ));

            let client = ConvexClient {
                listen_handle: Some(Arc::new(listen_handle)),
                request_sender,
                watch_receiver,
            };
            Ok((client, test_protocol))
        }
    }

    fn fake_mutation_response(result: FunctionResult) -> (ServerMessage, ServerMessage) {
        let (transition_response, new_version) = fake_transition(StateVersion::initial(), vec![]);
        let mutation_response = ServerMessage::MutationResponse {
            request_id: 0,
            result: result.into(),
            ts: Some(new_version.ts),
            log_lines: LogLinesMessage(vec![]),
        };
        (mutation_response, transition_response)
    }

    fn fake_action_response(result: FunctionResult) -> ServerMessage {
        ServerMessage::ActionResponse {
            request_id: 0,
            result: result.into(),
            log_lines: LogLinesMessage(vec![]),
        }
    }

    fn fake_transition(
        start_version: StateVersion,
        modifications: Vec<(QueryId, Value)>,
    ) -> (ServerMessage, StateVersion) {
        let end_version = StateVersion {
            ts: start_version.ts.succ().expect("Succ failed"),
            ..start_version
        };
        (
            ServerMessage::Transition {
                start_version,
                end_version,
                modifications: modifications
                    .into_iter()
                    .map(|(query_id, value)| StateModification::QueryUpdated {
                        query_id,
                        value,
                        journal: None,
                        log_lines: LogLinesMessage(vec![]),
                    })
                    .collect(),
            },
            end_version,
        )
    }

    #[tokio::test]
    async fn test_mutation() -> anyhow::Result<()> {
        let (mut client, mut test_protocol) = ConvexClient::with_test_protocol().await?;
        test_protocol.take_sent().await;

        let mut res =
            tokio::spawn(async move { client.mutation("incrementCounter", btreemap! {}).await });
        test_protocol.wait_until_n_messages_sent(1).await;

        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::Mutation {
                request_id: 0,
                udf_path: UdfPath::from_str("incrementCounter")?,
                args: vec![json!({})],
                component_path: None,
            }]
        );

        let mutation_result = FunctionResult::Value(Value::Null);
        let (mut_resp, transition) = fake_mutation_response(mutation_result.clone());
        test_protocol.fake_server_response(mut_resp).await?;
        // Should not be ready until transition completes.
        tokio::time::timeout(Duration::from_millis(50), &mut res)
            .await
            .unwrap_err();

        // Once transition is sent, it is ready.
        test_protocol.fake_server_response(transition).await?;
        assert_eq!(res.await??, mutation_result);
        Ok(())
    }

    #[tokio::test]
    async fn test_mutation_error() -> anyhow::Result<()> {
        let (mut client, mut test_protocol) = ConvexClient::with_test_protocol().await?;
        test_protocol.take_sent().await;

        let res =
            tokio::spawn(async move { client.mutation("incrementCounter", btreemap! {}).await });
        test_protocol.wait_until_n_messages_sent(1).await;
        test_protocol.take_sent().await;

        let mutation_result = FunctionResult::ErrorMessage("JEEPERS".into());
        let (mut_resp, _transition) = fake_mutation_response(mutation_result.clone());
        test_protocol.fake_server_response(mut_resp).await?;
        // Errors should be ready immediately (no transition needed)
        assert_eq!(res.await??, mutation_result);

        Ok(())
    }

    #[tokio::test]
    async fn test_action() -> anyhow::Result<()> {
        let (mut client, mut test_protocol) = ConvexClient::with_test_protocol().await?;
        test_protocol.take_sent().await;

        let action_result = FunctionResult::Value(Value::Null);
        let server_message = fake_action_response(action_result.clone());

        let res = tokio::spawn(async move { client.action("runAction:hello", btreemap! {}).await });
        test_protocol.wait_until_n_messages_sent(1).await;

        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::Action {
                request_id: 0,
                udf_path: UdfPath::from_str("runAction:hello")?,
                args: vec![json!({})],
                component_path: None,
            }]
        );

        test_protocol.fake_server_response(server_message).await?;
        assert_eq!(res.await??, action_result);
        Ok(())
    }

    #[tokio::test]
    async fn test_auth() -> anyhow::Result<()> {
        let (mut client, test_protocol) = ConvexClient::with_test_protocol().await?;
        test_protocol.take_sent().await;

        // Set token
        client.set_auth(Some("myauthtoken".into())).await;
        test_protocol.wait_until_n_messages_sent(1).await;
        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::Authenticate {
                base_version: 0,
                token: AuthenticationToken::User("myauthtoken".into()),
            }]
        );

        // Unset token
        client.set_auth(None).await;
        test_protocol.wait_until_n_messages_sent(1).await;
        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::Authenticate {
                base_version: 1,
                token: AuthenticationToken::None,
            }]
        );

        // Set admin auth
        client.set_admin_auth("myadminauth".into(), None).await;
        test_protocol.wait_until_n_messages_sent(1).await;
        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::Authenticate {
                base_version: 2,
                token: AuthenticationToken::Admin("myadminauth".into(), None),
            }]
        );

        // Set admin auth acting as user
        let acting_as = UserIdentityAttributes {
            name: Some("Barbara Liskov".into()),
            ..Default::default()
        };
        client
            .set_admin_auth("myadminauth".into(), Some(acting_as.clone()))
            .await;
        test_protocol.wait_until_n_messages_sent(1).await;
        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::Authenticate {
                base_version: 3,
                token: AuthenticationToken::Admin("myadminauth".into(), Some(acting_as)),
            }]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_client_single_subscription() -> anyhow::Result<()> {
        let (mut client, mut test_protocol) = ConvexClient::with_test_protocol().await?;

        let mut subscription1 = client.subscribe("getValue1", btreemap! {}).await?;
        let query_id = subscription1.query_id();
        assert_eq!(
            test_protocol.take_sent().await,
            vec![
                ClientMessage::Connect {
                    session_id: SessionId::nil(),
                    connection_count: 0,
                    last_close_reason: "InitialConnect".to_string(),
                    max_observed_timestamp: None,
                },
                ClientMessage::ModifyQuerySet {
                    base_version: 0,
                    new_version: 1,
                    modifications: vec![QuerySetModification::Add(Query {
                        query_id,
                        udf_path: "getValue1".parse()?,
                        args: vec![json!({})],
                        journal: None,
                        component_path: None,
                    })]
                },
            ]
        );

        test_protocol
            .fake_server_response(
                fake_transition(
                    StateVersion::initial(),
                    vec![(subscription1.query_id(), 10.into())],
                )
                .0,
            )
            .await?;
        assert_eq!(
            subscription1.next().await,
            Some(FunctionResult::Value(10.into()))
        );
        assert_eq!(
            client.query("getValue1", btreemap! {}).await?,
            FunctionResult::Value(10.into())
        );

        drop(subscription1);
        test_protocol.wait_until_n_messages_sent(1).await;
        assert_eq!(
            test_protocol.take_sent().await,
            vec![ClientMessage::ModifyQuerySet {
                base_version: 1,
                new_version: 2,
                modifications: vec![QuerySetModification::Remove { query_id }],
            }]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_client_consistent_view_watch() -> anyhow::Result<()> {
        let (mut client, mut test_protocol) = ConvexClient::with_test_protocol().await?;
        let subscription1 = client.subscribe("getValue1", btreemap! {}).await?;
        let subscription2a = client.subscribe("getValue2", btreemap! {}).await?;
        let subscription2b = client.subscribe("getValue2", btreemap! {}).await?;
        let subscription3 = client.subscribe("getValue3", btreemap! {}).await?;
        test_protocol.take_sent().await;
        let mut watch = client.watch_all();

        test_protocol
            .fake_server_response(
                fake_transition(
                    StateVersion::initial(),
                    vec![(QueryId::new(0), 10.into()), (QueryId::new(1), 20.into())],
                )
                .0,
            )
            .await?;

        let results = watch.next().await.expect("Watch should have results");
        assert_eq!(
            results.get(&subscription1),
            Some(&FunctionResult::Value(10.into()))
        );
        assert_eq!(
            results.get(&subscription2a),
            Some(&FunctionResult::Value(20.into()))
        );
        assert_eq!(
            results.get(&subscription2b),
            Some(&FunctionResult::Value(20.into()))
        );
        assert_eq!(results.get(&subscription3), None);
        assert_eq!(
            results.iter().collect::<Vec<_>>(),
            vec![
                (subscription1.id(), Some(&FunctionResult::Value(10.into()))),
                (subscription2a.id(), Some(&FunctionResult::Value(20.into()))),
                (subscription2b.id(), Some(&FunctionResult::Value(20.into()))),
                (subscription3.id(), None,),
            ]
        );

        // Ideally a new watch should immediately give you results, but we don't have
        // that yet. Need to replace tokio::broadcast with something that buffers 1
        // item.
        //let mut watch2 = client.watch();
        //let results = watch.next().await.expect("Watch should have results");
        //assert_eq!(results.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_drop_client() -> anyhow::Result<()> {
        let (mut client, _test_protocol) = ConvexClient::with_test_protocol().await?;
        let mut subscription1 = client.subscribe("getValue1", btreemap! {}).await?;
        drop(client);
        tokio::task::yield_now().await;
        assert!(subscription1.next().await.is_none());
        drop(subscription1);
        Ok(())
    }

    #[tokio::test]
    async fn test_client_separate_queries() -> anyhow::Result<()> {
        let (mut client, test_protocol) = ConvexClient::with_test_protocol().await?;

        // All three of these should be considered separate
        let subscription1 = client.subscribe("getValue1", btreemap! {}).await?;
        let subscription2 = client.subscribe("getValue2", btreemap! {}).await?;
        let subscription3 = client
            .subscribe("getValue2", btreemap! {"hello".into() => "world".into()})
            .await?;
        assert_ne!(subscription1.query_id(), subscription2.query_id());
        assert_ne!(subscription2.query_id(), subscription3.query_id());

        assert_eq!(
            test_protocol.take_sent().await,
            vec![
                ClientMessage::Connect {
                    session_id: SessionId::nil(),
                    connection_count: 0,
                    last_close_reason: "InitialConnect".to_string(),
                    max_observed_timestamp: None,
                },
                ClientMessage::ModifyQuerySet {
                    base_version: 0,
                    new_version: 1,
                    modifications: vec![QuerySetModification::Add(Query {
                        query_id: subscription1.query_id(),
                        udf_path: "getValue1".parse()?,
                        args: vec![json!({})],
                        journal: None,
                        component_path: None,
                    })]
                },
                ClientMessage::ModifyQuerySet {
                    base_version: 1,
                    new_version: 2,
                    modifications: vec![QuerySetModification::Add(Query {
                        query_id: subscription2.query_id(),
                        udf_path: "getValue2".parse()?,
                        args: vec![json!({})],
                        journal: None,
                        component_path: None,
                    })]
                },
                ClientMessage::ModifyQuerySet {
                    base_version: 2,
                    new_version: 3,
                    modifications: vec![QuerySetModification::Add(Query {
                        query_id: subscription3.query_id(),
                        udf_path: "getValue2".parse()?,
                        args: vec![json!({"hello": "world"})],
                        journal: None,
                        component_path: None,
                    })]
                },
            ]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_client_two_identical_queries() -> anyhow::Result<()> {
        let (mut client, mut test_protocol) = ConvexClient::with_test_protocol().await?;

        // These two should be considered the same query.
        let mut subscription1 = client.subscribe("getValue", btreemap! {}).await?;
        let mut subscription2 = client.subscribe("getValue", btreemap! {}).await?;

        assert_ne!(subscription1.subscriber_id, subscription2.subscriber_id);
        assert_eq!(subscription1.query_id(), subscription2.query_id());
        let query_id = subscription1.query_id();

        assert_eq!(
            test_protocol.take_sent().await,
            vec![
                ClientMessage::Connect {
                    session_id: SessionId::nil(),
                    connection_count: 0,
                    last_close_reason: "InitialConnect".to_string(),
                    max_observed_timestamp: None,
                },
                ClientMessage::ModifyQuerySet {
                    base_version: 0,
                    new_version: 1,
                    modifications: vec![QuerySetModification::Add(Query {
                        query_id,
                        udf_path: "getValue".parse()?,
                        args: vec![json!({})],
                        journal: None,
                        component_path: None,
                    })]
                },
            ]
        );

        let mut version = StateVersion::initial();
        for i in 1..5 {
            let (transition, new_version) = fake_transition(version, vec![(query_id, i.into())]);
            test_protocol.fake_server_response(transition).await?;
            version = new_version;

            assert_eq!(
                subscription1.next().await,
                Some(FunctionResult::Value(i.into()))
            );
            assert_eq!(
                subscription2.next().await,
                Some(FunctionResult::Value(i.into()))
            );
        }

        // A new subscription should auto-initialize with the value if available
        let mut subscription3 = client.subscribe("getValue", btreemap! {}).await?;
        assert_eq!(
            subscription3.next().await,
            Some(FunctionResult::Value(4.into())),
        );

        // Dropping sub1 and sub2 should still maintain subscription
        drop(subscription1);
        drop(subscription2);
        let (transition, _new_version) = fake_transition(version, vec![(query_id, 5.into())]);
        test_protocol.fake_server_response(transition).await?;
        assert_eq!(
            subscription3.next().await,
            Some(FunctionResult::Value(5.into())),
        );

        Ok(())
    }

    #[test]
    fn test_deployment_url() -> anyhow::Result<()> {
        assert_eq!(
            deployment_to_ws_url("http://flying-shark-123.convex.cloud".parse()?)?.to_string(),
            "ws://flying-shark-123.convex.cloud/api/sync",
        );
        assert_eq!(
            deployment_to_ws_url("https://flying-shark-123.convex.cloud".parse()?)?.to_string(),
            "wss://flying-shark-123.convex.cloud/api/sync",
        );
        assert_eq!(
            deployment_to_ws_url("ws://flying-shark-123.convex.cloud".parse()?)?.to_string(),
            "ws://flying-shark-123.convex.cloud/api/sync",
        );
        assert_eq!(
            deployment_to_ws_url("wss://flying-shark-123.convex.cloud".parse()?)?.to_string(),
            "wss://flying-shark-123.convex.cloud/api/sync",
        );
        assert_eq!(
            deployment_to_ws_url("ftp://flying-shark-123.convex.cloud".parse()?)
                .unwrap_err()
                .to_string(),
            "Unknown scheme ftp. Expected http or https.",
        );
        Ok(())
    }
}
