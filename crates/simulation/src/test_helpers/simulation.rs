use std::{
    future::Future,
    sync::Arc,
    time::Duration,
};

use application::{
    api::ApplicationApi,
    deploy_config::StartPushRequest,
    test_helpers::ApplicationTestExt,
    Application,
};
use common::runtime::shutdown_and_join;
use runtime::testing::TestRuntime;

use super::{
    js_client::JsClientThread,
    server::ServerThread,
};

pub struct SimulationTest {
    pub rt: TestRuntime,
    pub application: Arc<dyn ApplicationApi>,

    pub server: ServerThread,
    pub js_clients: Vec<JsClientThread>,
}

pub struct SimulationTestConfig {
    pub num_client_threads: usize,
    pub expected_delay_duration: Option<Duration>,
}

impl SimulationTest {
    pub async fn run<F, Fut>(
        rt: TestRuntime,
        config: SimulationTestConfig,
        f: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        let start = std::time::Instant::now();
        common::testing::init_test_logging();

        let application = Application::new_for_tests(&rt).await?;
        tracing::error!("create app: {:?}", start.elapsed());

        let start = std::time::Instant::now();
        let start_push_json =
            include_str!("../../../../npm-packages/simulation/dist/start_push.json");
        let output: StartPushRequest = serde_json::from_str(start_push_json)?;
        application.run_test_push(output).await?;
        tracing::error!("push: {:?}", start.elapsed());

        let application = Arc::new(application);

        let start = std::time::Instant::now();
        let mut handles = vec![];
        let (server, handle) = ServerThread::new(
            rt.clone(),
            application.clone(),
            config.expected_delay_duration,
        );
        handles.push(handle);
        let mut js_clients = vec![];
        for _ in 0..config.num_client_threads {
            let (js_client, handle) = JsClientThread::new(rt.clone(), server.clone());
            js_clients.push(js_client);
            handles.push(handle);
        }
        tracing::error!("create threads: {:?}", start.elapsed());

        let start = std::time::Instant::now();
        let test = Self {
            rt,
            application,
            server,
            js_clients,
        };
        let result = f(test).await;
        tracing::error!("run test: {:?}", start.elapsed());
        let start = std::time::Instant::now();
        for handle in handles {
            shutdown_and_join(handle).await?;
        }
        tracing::error!("shutdown: {:?}", start.elapsed());

        result
    }
}
