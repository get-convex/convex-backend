use std::{
    collections::BTreeMap,
    sync::Arc,
};

use application::{
    test_helpers::ApplicationTestExt,
    Application,
};
use common::{
    assert_obj,
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
        PublicFunctionPath,
    },
    http::{
        RequestDestination,
        ResolvedHostname,
    },
    runtime::{
        shutdown_and_join,
        Runtime,
        SpawnHandle,
    },
    types::{
        FunctionCaller,
        MemberId,
        SessionId,
        SessionRequestSeqNumber,
        Timestamp,
    },
    value::{
        assert_val,
        ConvexObject,
        ConvexValue,
    },
    version::ClientVersion,
    RequestId,
};
use errors::ErrorMetadataAnyhowExt;
use isolate::test_helpers::TEST_SOURCE_ISOLATE_ONLY;
use keybroker::{
    testing::TestUserIdentity,
    Identity,
    KeyBroker,
    DEV_SECRET,
};
use model::{
    config::{
        types::ConfigMetadata,
        ConfigModel,
    },
    udf_config::types::UdfConfig,
};
use must_let::must_let;
use parking_lot::Mutex;
use runtime::testing::TestRuntime;
use sync_types::{
    types::SerializedArgs,
    AuthenticationToken,
    ClientMessage,
    Query,
    QueryId,
    QuerySetModification,
    StateModification,
    UserIdentityAttributes,
};
use tokio::sync::mpsc;

use crate::{
    worker::{
        measurable_unbounded_channel,
        SingleFlightReceiver,
    },
    ServerMessage,
    SyncWorker,
    SyncWorkerConfig,
};

struct SyncTest<RT: Runtime> {
    pub rt: RT,
    pub kb: KeyBroker,
    application: Application<RT>,
}

impl<RT: Runtime> SyncTest<RT> {
    async fn new(rt: RT) -> anyhow::Result<Self> {
        let application = Application::new_for_tests(&rt).await?;
        let kb = application.key_broker().clone();
        let application_ = application.clone();
        // Populate UDFs.
        {
            // Only analyze isolate modules, so sync tests don't require prod
            // runtime. Remove the filter if you need to test node modules.
            let modules = TEST_SOURCE_ISOLATE_ONLY.clone();
            let source_package = application.upload_package(&modules, None, None).await?;
            let udf_config = UdfConfig::new_for_test(&rt, "1000.0.0".parse()?);
            let analyze_results = application
                .analyze(
                    udf_config.clone(),
                    modules.clone(),
                    source_package.clone(),
                    BTreeMap::new(),
                    BTreeMap::new(),
                )
                .await??;

            let mut tx = application_.begin(Identity::system()).await?;
            ConfigModel::new(&mut tx, ComponentId::test_user())
                .apply(
                    ConfigMetadata::new(),
                    modules,
                    udf_config,
                    Some(source_package),
                    analyze_results,
                    None,
                )
                .await?;
            application_.commit_test(tx).await?;
        }

        Ok(Self {
            rt,
            kb,
            application,
        })
    }

    fn new_worker(&self) -> anyhow::Result<TestSyncWorker<RT>> {
        let config = SyncWorkerConfig::default();
        self.new_worker_with_config(config, None)
    }

    fn new_worker_with_config(
        &self,
        config: SyncWorkerConfig,
        max_observed_timestamp: Option<Timestamp>,
    ) -> anyhow::Result<TestSyncWorker<RT>> {
        let worker_failed = Arc::new(Mutex::new(None));
        let (client_tx, client_rx) = mpsc::unbounded_channel();
        let (server_tx, server_rx) = measurable_unbounded_channel();

        let worker_failed_ = worker_failed.clone();
        let api = Arc::new(self.application.clone());
        let rt = self.rt.clone();
        let future = async move {
            // TODO(CX-597): The panic in this future currently gets swallowed by
            // `futures::RemoteHandle`.
            if let Err(e) = SyncWorker::new(
                api,
                rt,
                ResolvedHostname {
                    instance_name: String::new(),
                    destination: RequestDestination::ConvexCloud,
                },
                config,
                client_rx,
                server_tx,
                Box::new(|_session_id| ()),
                0,
            )
            .go()
            .await
            {
                worker_failed_.lock().replace(e);
            }
        };
        let worker_handle = self.rt.spawn("sync_test", future);

        client_tx.send((
            ClientMessage::Connect {
                session_id: SessionId::nil(),
                connection_count: 0,
                last_close_reason: "InitialConnect".to_string(),
                max_observed_timestamp,
                client_ts: None,
            },
            self.rt.monotonic_now(),
        ))?;

        Ok(TestSyncWorker {
            rt: self.rt.clone(),
            worker_handle,
            worker_failed,
            tx: client_tx,
            rx: server_rx,
        })
    }
}

struct TestSyncWorker<RT: Runtime> {
    rt: RT,

    tx: mpsc::UnboundedSender<(ClientMessage, tokio::time::Instant)>,
    rx: SingleFlightReceiver,

    worker_handle: Box<dyn SpawnHandle>,
    worker_failed: Arc<Mutex<Option<anyhow::Error>>>,
}

impl<RT: Runtime> TestSyncWorker<RT> {
    fn send(&self, message: ClientMessage) -> anyhow::Result<()> {
        self.tx.send((message, self.rt.monotonic_now()))?;
        Ok(())
    }

    async fn receive(&mut self) -> anyhow::Result<ServerMessage> {
        let message = self
            .rx
            .next()
            .await
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Sync worker unexpectedly hung up: {:?}",
                    self.worker_failed.lock()
                )
            })?
            .0;
        Ok(message)
    }

    async fn mutation(
        &mut self,
        path: &str,
        args: ConvexObject,
        request_id: SessionRequestSeqNumber,
    ) -> anyhow::Result<(ConvexValue, Timestamp)> {
        self.send(ClientMessage::Mutation {
            request_id,
            udf_path: path.parse()?,
            args: SerializedArgs::from_args(vec![args.into()])?,
            component_path: None,
        })?;

        must_let!(let ServerMessage::MutationResponse {
            request_id: outgoing_mutation_id,
            result,
            ts,
            ..
        } = self.receive().await?);
        // Check that the mutation ID on the outgoing message matches to
        // confirm we intercepted the right message
        assert_eq!(request_id, outgoing_mutation_id);
        Ok((result.unwrap().unpack()?, ts.unwrap()))
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        shutdown_and_join(self.worker_handle).await?;
        self.worker_failed.lock().take().map_or(Ok(()), Err)
    }

    fn with_worker_error<T>(&self, f: impl FnOnce(&Option<anyhow::Error>) -> T) -> T {
        let worker_failed = self.worker_failed.lock();
        f(&worker_failed)
    }
}

#[convex_macro::test_runtime]
async fn test_basic_account(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    let name1 = assert_val!("orinoco");
    let name2 = assert_val!("tizoncito");

    // 1. Initialize the two account balances.
    let (result, _) = sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name1.clone(), "balance" => 100.0),
            0,
        )
        .await?;
    // initialize doesn't explicitly return a result, so the result is Null
    assert_eq!(result, ConvexValue::Null);
    must_let!(let ServerMessage::Transition { .. } = sync_worker.receive().await?);

    let (result, initialization_ts) = sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name2.clone(), "balance" => 50.0),
            1,
        )
        .await?;
    assert_eq!(result, ConvexValue::Null);
    must_let!(let ServerMessage::Transition { .. } = sync_worker.receive().await?);

    // 2. Start a new subscription.
    let query = Query {
        query_id: QueryId::new(0),
        udf_path: "sync:accountBalance".parse()?,
        args: SerializedArgs::from_args(vec![assert_obj!("name" => name1.clone()).into()])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 0,
        new_version: 1,
        modifications: vec![QuerySetModification::Add(query)],
    };
    sync_worker.send(msg)?;
    must_let!(let ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start.query_set, 0);
    assert_eq!(end.query_set, 1);
    let ts0 = end.ts;
    assert!(Timestamp::MIN < ts0);
    assert!(ts0 >= initialization_ts);
    assert_eq!(modifications.len(), 1);
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));
    assert_eq!(value.unpack()?, ConvexValue::from(100.0));

    // 3. Mutate a single query and see that it gets updated.
    let (result, _) = sync_worker
        .mutation(
            "sync:deposit",
            assert_obj!("name" => name1.clone(), "balance" => 50.0),
            2,
        )
        .await?;
    assert_eq!(result, assert_val!("orinoco's balance is now 150"));

    must_let!(let ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start.query_set, 1);
    assert_eq!(end.query_set, 1);
    let ts1 = end.ts;
    assert!(ts0 < ts1);
    assert_eq!(modifications.len(), 1);
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));
    assert_eq!(value.unpack()?, ConvexValue::from(150.0));

    // 4. Add a new query.
    let query = Query {
        query_id: QueryId::new(1),
        udf_path: "sync:accountBalance".parse()?,
        args: SerializedArgs::from_args(vec![assert_obj!("name" => name2.clone()).into()])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 1,
        new_version: 2,
        modifications: vec![QuerySetModification::Add(query)],
    };
    sync_worker.send(msg)?;
    must_let!(let ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start.query_set, 1);
    assert_eq!(end.query_set, 2);
    let ts2 = end.ts;
    assert!(ts1 <= ts2);
    assert_eq!(modifications.len(), 1);
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(1));
    assert_eq!(value.unpack()?, ConvexValue::from(50.0));

    // 5. Do a transfer and see that the two queries get updated atomically.
    let (result, _) = sync_worker
        .mutation(
            "sync:transfer",
            assert_obj!("from" => name1, "to" => name2, "amount" => 25.0),
            3,
        )
        .await?;
    assert_eq!(result, ConvexValue::Null);

    must_let!(let ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start.query_set, 2);
    assert_eq!(end.query_set, 2);
    let ts3 = end.ts;
    assert!(ts2 < ts3);
    assert_eq!(modifications.len(), 2);
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));
    assert_eq!(value.unpack()?, ConvexValue::from(125.0));
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[1]);
    assert_eq!(*query_id, QueryId::new(1));
    assert_eq!(value.unpack()?, ConvexValue::from(75.0));

    // 5. Remove a query.
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 2,
        new_version: 3,
        modifications: vec![QuerySetModification::Remove {
            query_id: QueryId::new(0),
        }],
    };
    sync_worker.send(msg)?;
    must_let!(let ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start.query_set, 2);
    assert_eq!(end.query_set, 3);
    assert!(ts3 <= end.ts);
    assert_eq!(modifications.len(), 1);
    must_let!(let StateModification::QueryRemoved { query_id } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));

    sync_worker.shutdown().await?;

    Ok(())
}

// Regression test that makes sure queries can be removed even if they were just
// added. The query should be removed from the in_progress_queries.
#[convex_macro::test_runtime]
async fn test_remove_in_progress_query(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    let name1: ConvexValue = assert_val!("orinoco");

    // Start a new subscription, and then cancel it.
    let query_id = QueryId::new(0);
    let query = Query {
        query_id,
        udf_path: "sync:accountBalance".parse()?,
        args: SerializedArgs::from_args(vec![assert_obj!("name" => name1.clone()).into()])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 0,
        new_version: 1,
        modifications: vec![
            QuerySetModification::Add(query),
            QuerySetModification::Remove { query_id },
        ],
    };
    sync_worker.send(msg)?;
    must_let!(let ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        client_clock_skew: _,
        server_ts: _,
    } = sync_worker.receive().await?);
    assert_eq!(start.query_set, 0);
    assert_eq!(end.query_set, 1);
    assert!(Timestamp::MIN < end.ts);
    assert_eq!(modifications.len(), 1);
    must_let!(let StateModification::QueryRemoved { query_id } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));

    sync_worker.shutdown().await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_failure(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    let query1 = Query {
        query_id: QueryId::new(0),
        udf_path: "sync:fail".parse()?,
        args: SerializedArgs::from_args(vec![assert_obj!("i" => ConvexValue::from(0.0)).into()])?,
        journal: None,
        component_path: None,
    };
    let query2 = Query {
        query_id: QueryId::new(1),
        udf_path: "sync:fail".parse()?,
        args: SerializedArgs::from_args(vec![assert_obj!("i" => ConvexValue::from(3.0)).into()])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 0,
        new_version: 1,
        modifications: vec![
            QuerySetModification::Add(query1),
            QuerySetModification::Add(query2),
        ],
    };
    sync_worker.send(msg)?;

    // First, we should successfully transition to both of these queries failing.
    must_let!(let ServerMessage::Transition {
        end_version, modifications, ..
    } = sync_worker.receive().await?);
    assert_eq!(modifications.len(), 2);
    must_let!(let StateModification::QueryFailed {
        query_id, error_message, ..
    } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));
    assert!(error_message.contains("I can't go for that"));
    must_let!(let StateModification::QueryFailed {
        query_id, error_message, ..
    } = &modifications[1]);
    assert_eq!(*query_id, QueryId::new(1));
    assert!(error_message.contains("But I won't do that"));

    // Add a successful query to the query set.
    let query3: Query = Query {
        query_id: QueryId::new(2),
        udf_path: "sync:succeed".parse()?,
        args: SerializedArgs::from_args(vec![])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: end_version.query_set,
        new_version: end_version.query_set + 1,
        modifications: vec![QuerySetModification::Add(query3)],
    };
    sync_worker.send(msg)?;

    must_let!(let ServerMessage::Transition {
        end_version,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(modifications.len(), 1);
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(2));
    assert_eq!(value.unpack()?, ConvexValue::try_from("on my list")?);

    // Remove the two failing queries.
    let msg = ClientMessage::ModifyQuerySet {
        base_version: end_version.query_set,
        new_version: end_version.query_set + 1,
        modifications: vec![
            QuerySetModification::Remove {
                query_id: QueryId::new(0),
            },
            QuerySetModification::Remove {
                query_id: QueryId::new(1),
            },
        ],
    };
    sync_worker.send(msg)?;

    must_let!(let ServerMessage::Transition { modifications, .. } = sync_worker.receive().await?);
    assert_eq!(modifications.len(), 2);
    assert_eq!(
        modifications[0],
        StateModification::QueryRemoved {
            query_id: QueryId::new(0)
        }
    );
    assert_eq!(
        modifications[1],
        StateModification::QueryRemoved {
            query_id: QueryId::new(1)
        }
    );

    sync_worker.shutdown().await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_admin_auth(rt: TestRuntime) -> anyhow::Result<()> {
    // Normal path

    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;
    let admin_key = test.kb.issue_admin_key(MemberId(1));
    sync_worker.send(ClientMessage::Authenticate {
        token: AuthenticationToken::Admin(admin_key.as_string(), None),
        base_version: 0,
    })?;
    must_let!(let ServerMessage::Transition {
        start_version,
        end_version,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start_version.identity, 0);
    assert_eq!(end_version.identity, 1);
    sync_worker.shutdown().await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_admin_auth_bad_key(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;
    let bogus_admin_key =
        KeyBroker::new("bozo-fish-123", DEV_SECRET.try_into()?)?.issue_admin_key(MemberId(1));
    sync_worker.send(ClientMessage::Authenticate {
        token: AuthenticationToken::Admin(bogus_admin_key.as_string(), None),
        base_version: 0,
    })?;
    let err = sync_worker.receive().await.unwrap_err();
    assert!(
        err.to_string().contains(
            "Sync worker unexpectedly hung up: Some(The provided admin key was invalid for this \
             instance"
        ),
        "{err}"
    );
    sync_worker.with_worker_error(|e| assert!(e.as_ref().unwrap().is_auth_update_failed()));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_acting_auth_bad_key(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;
    let bogus_admin_key =
        KeyBroker::new("bozo-fish-123", DEV_SECRET.try_into()?)?.issue_admin_key(MemberId(1));
    sync_worker.send(ClientMessage::Authenticate {
        token: AuthenticationToken::Admin(
            bogus_admin_key.as_string(),
            Some(UserIdentityAttributes::test()),
        ),
        base_version: 0,
    })?;
    let err = sync_worker.receive().await.unwrap_err();
    assert!(
        err.to_string().contains(
            "Sync worker unexpectedly hung up: Some(The provided admin key was invalid for this \
             instance"
        ),
        "{err}"
    );
    sync_worker.with_worker_error(|e| assert!(e.as_ref().unwrap().is_auth_update_failed()));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_acting_auth(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;
    let admin_key = test.kb.issue_admin_key(MemberId(1));
    sync_worker.send(ClientMessage::Authenticate {
        token: AuthenticationToken::Admin(
            admin_key.as_string(),
            Some(UserIdentityAttributes::test()),
        ),
        base_version: 0,
    })?;
    must_let!(let ServerMessage::Transition {
        start_version,
        end_version,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(start_version.identity, 0);
    assert_eq!(end_version.identity, 1);
    sync_worker.shutdown().await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_idempotent_mutations(rt: TestRuntime) -> anyhow::Result<()> {
    // A test that confirms that sending the same mutation twice only causes
    // it to run once.

    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    // Set up a balance of $0 for Alice
    let name = ConvexValue::try_from("Alice")?;
    sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name.clone(), "balance" => 0.0),
            0,
        )
        .await?;
    must_let!(let ServerMessage::Transition {
        end_version: v1,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert!(modifications.is_empty());

    // Deposit $5 in a mutation
    let (result, _) = sync_worker
        .mutation(
            "sync:deposit",
            assert_obj!("name" => name.clone(), "balance" => 5.0),
            1,
        )
        .await?;
    assert_eq!(result, ConvexValue::try_from("Alice's balance is now 5")?);
    must_let!(let ServerMessage::Transition {
        start_version: v2,
        end_version: v3,
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert_eq!(v1, v2);
    assert!(v2 < v3);
    assert!(modifications.is_empty());

    // Rerun the same mutation (with the same mutation ID) and expect the same
    // result.
    let (result, _) = sync_worker
        .mutation(
            "sync:deposit",
            assert_obj!("name" => name.clone(), "balance" => 5.0),
            1,
        )
        .await?;
    assert_eq!(result, ConvexValue::try_from("Alice's balance is now 5")?);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_value_deduplication_success(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    let name = ConvexValue::try_from("Alice")?;
    let (result, _) = sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name.clone(), "balance" => 0.0),
            0,
        )
        .await?;
    assert_eq!(result, ConvexValue::Null);
    must_let!(let ServerMessage::Transition { .. } = sync_worker.receive().await?);

    let query = Query {
        query_id: QueryId::new(0),
        udf_path: "sync:discardQueryResults".parse()?,
        args: SerializedArgs::from_args(vec![
            assert_obj!("throwError" => ConvexValue::from(false)).into(),
        ])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 0,
        new_version: 1,
        modifications: vec![QuerySetModification::Add(query)],
    };
    sync_worker.send(msg)?;
    must_let!(let ServerMessage::Transition {
        modifications, ..
    } = sync_worker.receive().await?);
    assert_eq!(modifications.len(), 1, "{modifications:?}");
    must_let!(let StateModification::QueryUpdated { query_id, value, .. } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));
    assert_eq!(value.unpack()?, assert_val!("hi"));

    // Insert a new value into the "accounts" table, which will invalidate the query
    // but not change its result.
    let name = ConvexValue::try_from("Bob")?;
    sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name.clone(), "balance" => 0.0),
            1,
        )
        .await?;

    must_let!(let ServerMessage::Transition {
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert!(modifications.is_empty());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_value_deduplication_failure(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    let name = ConvexValue::try_from("Alice")?;
    let (result, _) = sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name.clone(), "balance" => 0.0),
            0,
        )
        .await?;
    assert_eq!(result, ConvexValue::Null);
    must_let!(let ServerMessage::Transition { .. } = sync_worker.receive().await?);

    let query = Query {
        query_id: QueryId::new(0),
        udf_path: "sync:discardQueryResults".parse()?,
        args: SerializedArgs::from_args(vec![
            assert_obj!("throwError" => ConvexValue::from(true)).into(),
        ])?,
        journal: None,
        component_path: None,
    };
    let msg = ClientMessage::ModifyQuerySet {
        base_version: 0,
        new_version: 1,
        modifications: vec![QuerySetModification::Add(query)],
    };
    sync_worker.send(msg)?;
    must_let!(let ServerMessage::Transition { modifications, .. } = sync_worker.receive().await?);
    assert_eq!(modifications.len(), 1, "{modifications:?}");
    must_let!(let StateModification::QueryFailed {
        query_id,
        error_message,
        ..
    } = &modifications[0]);
    assert_eq!(*query_id, QueryId::new(0));
    assert!(error_message.contains("Uncaught Error: bye"));

    // Insert a new value into the "accounts" table, which will invalidate the query
    // but not change its result.
    let name = ConvexValue::try_from("Bob")?;
    sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name.clone(), "balance" => 0.0),
            1,
        )
        .await?;

    must_let!(let ServerMessage::Transition {
        modifications,
        ..
    } = sync_worker.receive().await?);
    assert!(modifications.is_empty());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_udf_cache_out_of_order(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;
    let mut sync_worker = test.new_worker()?;

    let name = ConvexValue::try_from("Alice")?;
    let (result, _) = sync_worker
        .mutation(
            "sync:initialize",
            assert_obj!("name" => name.clone(), "balance" => 0.0),
            0,
        )
        .await?;
    assert_eq!(result, ConvexValue::Null);
    must_let!(let ServerMessage::Transition { .. } = sync_worker.receive().await?);

    let ts1 = *test.application.now_ts_for_reads();

    sync_worker
        .mutation(
            "sync:deposit",
            assert_obj!("name" => name.clone(), "balance" => 5.0),
            1,
        )
        .await?;

    let ts2 = *test.application.now_ts_for_reads();

    let path = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: "sync:accountBalance".parse()?,
    };
    let result1 = test
        .application
        .read_only_udf_at_ts(
            RequestId::new(),
            PublicFunctionPath::Component(path.clone()),
            vec![assert_obj!("name" => name.clone()).into()],
            Identity::Unknown(None),
            ts2,
            None,
            FunctionCaller::SyncWorker(ClientVersion::unknown()),
        )
        .await?;
    assert_eq!(result1.result?.unpack()?, ConvexValue::from(5.0));

    let result2 = test
        .application
        .read_only_udf_at_ts(
            RequestId::new(),
            PublicFunctionPath::Component(path),
            vec![assert_obj!("name" => name).into()],
            Identity::Unknown(None),
            ts1,
            None,
            FunctionCaller::SyncWorker(ClientVersion::unknown()),
        )
        .await?;
    assert_eq!(result2.result?.unpack()?, ConvexValue::from(0.0));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_max_observed_timestamp(rt: TestRuntime) -> anyhow::Result<()> {
    let test = SyncTest::new(rt).await?;

    let config = SyncWorkerConfig::default();
    let mut sync_worker = test.new_worker_with_config(config, Some(Timestamp::MAX))?;
    must_let!(let Err(err) = sync_worker.receive().await);
    assert!(
        format!("{err}")
            .contains("Sync worker unexpectedly hung up: Some(Client has observed a timestamp"),
        "{err}"
    );

    Ok(())
}
