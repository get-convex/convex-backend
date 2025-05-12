use std::{
    sync::Arc,
    time::Duration,
};

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
        JoinSet,
        Runtime,
        SpawnHandle,
    },
    types::FunctionCaller,
    value::ConvexObject,
    RequestId,
};
use keybroker::Identity;
use rand_distr::{
    Distribution,
    Geometric,
};
use runtime::testing::TestRuntime;
use sync::{
    worker::{
        measurable_unbounded_channel,
        SingleFlightSender,
    },
    ServerMessage,
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
    time::Instant,
};

pub enum ServerRequest {
    Subscribe {
        incoming: mpsc::UnboundedReceiver<(ClientMessage, Instant)>,
        outgoing: SingleFlightSender,
    },
    Mutation {
        udf_path: UdfPath,
        args: ConvexObject,
        result: oneshot::Sender<Result<RedactedMutationReturn, RedactedMutationError>>,
    },
    LatestTimestamp {
        result: oneshot::Sender<Timestamp>,
    },
}

#[derive(Clone)]
pub struct ServerThread {
    rt: TestRuntime,
    tx: mpsc::UnboundedSender<ServerRequest>,
    expected_delay_duration: Option<Duration>,
}

impl ServerThread {
    pub fn new(
        rt: TestRuntime,
        application: Arc<dyn ApplicationApi>,
        expected_delay_duration: Option<Duration>,
    ) -> (Self, Box<dyn SpawnHandle>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let rt_clone = rt.clone();
        let handle = rt.spawn("ServerThread", async move {
            Self::go(rt_clone, application, rx)
                .await
                .expect("Server thread crashed")
        });
        (
            Self {
                rt,
                tx,
                expected_delay_duration,
            },
            handle,
        )
    }

    pub fn connect(
        &self,
    ) -> anyhow::Result<(
        mpsc::UnboundedSender<(ClientMessage, Instant)>,
        mpsc::UnboundedReceiver<(ServerMessage, Instant)>,
    )> {
        let (client_tx, client_rx) = mpsc::unbounded_channel();
        let (server_tx, mut server_rx) = measurable_unbounded_channel();
        self.tx.send(ServerRequest::Subscribe {
            incoming: client_rx,
            outgoing: server_tx,
        })?;

        let (faulty_client_tx, mut faulty_client_rx) = mpsc::unbounded_channel();
        let (faulty_server_tx, faulty_server_rx) = mpsc::unbounded_channel();

        let delay_distribution = match self.expected_delay_duration {
            Some(duration) => Some(Geometric::new(1.0 / duration.as_secs_f64())?),
            None => None,
        };
        let rt = self.rt.clone();
        tokio::task::spawn(async move {
            while let Some(msg) = faulty_client_rx.recv().await {
                if let Some(delay_distribution) = delay_distribution {
                    let delay = delay_distribution.sample(&mut *rt.rng());
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                if client_tx.send(msg).is_err() {
                    tracing::debug!("Server receiver closed");
                    return;
                }
            }
            tracing::debug!("Client sender closed");
        });
        let rt = self.rt.clone();
        tokio::task::spawn(async move {
            while let Some(msg) = server_rx.next().await {
                if let Some(delay_distribution) = delay_distribution {
                    let delay = delay_distribution.sample(&mut *rt.rng());
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                if faulty_server_tx.send(msg).is_err() {
                    tracing::debug!("Client receiver closed");
                    return;
                }
            }
            tracing::debug!("Server sender closed");
        });
        Ok((faulty_client_tx, faulty_server_rx))
    }

    pub async fn mutation(
        &self,
        udf_path: UdfPath,
        args: ConvexObject,
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
                                Box::new(|_session_id| ()),
                            );
                            join_set.spawn("sync_worker", async move { w.go().await });
                        },
                        ServerRequest::Mutation { udf_path, args, result } => {
                            let res = api.execute_public_mutation(
                                &host,
                                RequestId::new(),
                                Identity::system(),
                                udf_path.canonicalize().into(),
                                vec![args.into()],
                                FunctionCaller::Test,
                                None,
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
