use std::sync::Arc;

use application::{
    api::ApplicationApi,
    RedactedMutationError,
    RedactedMutationReturn,
};
use common::{
    http::{
        RequestDestination,
        ResolvedHostname,
    },
    runtime::{
        Runtime,
        SpawnHandle,
    },
    types::FunctionCaller,
    RequestId,
};
use keybroker::Identity;
use runtime::testing::TestRuntime;
use serde_json::Value as JsonValue;
use sync::{
    worker::{
        measurable_unbounded_channel,
        SingleFlightReceiver,
        SingleFlightSender,
    },
    SyncWorker,
    SyncWorkerConfig,
};
use sync_types::{
    ClientMessage,
    Timestamp,
    UdfPath,
};
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinSet,
    time::Instant,
};

pub enum ServerRequest {
    Subscribe {
        incoming: mpsc::UnboundedReceiver<(ClientMessage, Instant)>,
        outgoing: SingleFlightSender,
    },
    Mutation {
        udf_path: UdfPath,
        args: Vec<JsonValue>,
        result: oneshot::Sender<Result<RedactedMutationReturn, RedactedMutationError>>,
    },
    LatestTimestamp {
        result: oneshot::Sender<Timestamp>,
    },
}

#[derive(Clone)]
pub struct ServerThread {
    tx: mpsc::UnboundedSender<ServerRequest>,
}

impl ServerThread {
    pub fn new(
        rt: TestRuntime,
        application: Arc<dyn ApplicationApi>,
    ) -> (Self, Box<dyn SpawnHandle>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = rt.clone().spawn("ServerThread", async move {
            Self::go(rt, application, rx)
                .await
                .expect("Server thread crashed")
        });
        (Self { tx }, handle)
    }

    pub fn connect(
        &self,
    ) -> anyhow::Result<(
        mpsc::UnboundedSender<(ClientMessage, Instant)>,
        SingleFlightReceiver,
    )> {
        let (client_tx, client_rx) = mpsc::unbounded_channel();
        let (server_tx, server_rx) = measurable_unbounded_channel();
        self.tx.send(ServerRequest::Subscribe {
            incoming: client_rx,
            outgoing: server_tx,
        })?;
        Ok((client_tx, server_rx))
    }

    pub async fn mutation(
        &self,
        udf_path: UdfPath,
        args: Vec<JsonValue>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(ServerRequest::Mutation {
            udf_path,
            args,
            result: tx,
        })?;
        Ok(rx.await?)
    }

    pub async fn latest_timestamp(&self) -> anyhow::Result<Timestamp> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ServerRequest::LatestTimestamp { result: tx })?;
        Ok(rx.await?)
    }

    async fn go(
        rt: TestRuntime,
        api: Arc<dyn ApplicationApi>,
        mut rx: mpsc::UnboundedReceiver<ServerRequest>,
    ) -> anyhow::Result<()> {
        let host = ResolvedHostname {
            instance_name: "suadero".to_string(),
            destination: RequestDestination::ConvexCloud,
        };
        let mut join_set = JoinSet::new();
        loop {
            tokio::select! {
                Some(next) = join_set.join_next() => {
                    if let Err(e) = next {
                        tracing::error!("Sync worker failed: {e:?}");
                    }
                }
                request = rx.recv() => {
                    let Some(request) = request else {
                        tracing::info!("Server thread shutting down...");
                        return Ok(());
                    };
                    match request {
                        ServerRequest::Subscribe { incoming, outgoing } => {
                            tracing::info!("Received subscribe...");
                            let mut w = SyncWorker::new(
                                api.clone(),
                                rt.clone(),
                                host.clone(),
                                SyncWorkerConfig::default(),
                                incoming,
                                outgoing,
                            );
                            join_set.spawn(async move { w.go().await });
                        },
                        ServerRequest::Mutation { udf_path, args, result } => {
                            let res = api.execute_public_mutation(
                                &host,
                                RequestId::new(),
                                Identity::system(),
                                udf_path.canonicalize().into(),
                                args,
                                FunctionCaller::Test,
                                None,
                            ).await?;
                            let _ = result.send(res);
                        },
                        ServerRequest::LatestTimestamp { result } => {
                            let res = api.latest_timestamp(&host, RequestId::new()).await?;
                            let _ = result.send(*res);
                        },
                    }
                }
            }
        }
    }
}
