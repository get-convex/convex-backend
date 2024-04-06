#[cfg(any(test, feature = "testing"))]
use std::sync::Arc;

use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    errors::report_error,
    runtime::Runtime,
    types::UdfType,
};
use futures::FutureExt;
use isolate::{
    client::{
        IsolateWorker,
        Request,
        RequestType,
        SharedIsolateHeapStats,
        PAUSE_REQUEST,
    },
    environment::{
        action::ActionEnvironment,
        udf::DatabaseUdfEnvironment,
    },
    isolate::Isolate,
    metrics::{
        finish_service_request_timer,
        is_developer_ok,
        service_request_timer,
        RequestStatus,
    },
    IsolateConfig,
};
#[derive(Clone)]
pub(crate) struct FunctionRunnerIsolateWorker<RT: Runtime> {
    rt: RT,
    isolate_config: IsolateConfig,
    // This tokio Mutex is safe only because it's stripped out of production
    // builds. We shouldn't use tokio locks for prod code (see
    // https://github.com/rust-lang/rust/issues/104883 for background and
    // https://github.com/get-convex/convex/pull/19307 for an alternative).
    #[cfg(any(test, feature = "testing"))]
    pause_client: Option<Arc<tokio::sync::Mutex<PauseClient>>>,
}

impl<RT: Runtime> FunctionRunnerIsolateWorker<RT> {
    pub(crate) fn new(rt: RT, isolate_config: IsolateConfig) -> Self {
        Self {
            rt,
            isolate_config,
            #[cfg(any(test, feature = "testing"))]
            pause_client: None,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    #[allow(dead_code)]
    fn new_for_tests(rt: RT, isolate_config: IsolateConfig, pause_client: PauseClient) -> Self {
        Self {
            rt,
            isolate_config,
            pause_client: Some(Arc::new(tokio::sync::Mutex::new(pause_client))),
        }
    }
}

#[async_trait(?Send)]
impl<RT: Runtime> IsolateWorker<RT> for FunctionRunnerIsolateWorker<RT> {
    #[minitrace::trace]
    async fn handle_request(
        &self,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        Request {
            client_id: _,
            inner,
            mut pause_client,
            parent_trace: _,
        }: Request<RT>,
        heap_stats: SharedIsolateHeapStats,
    ) -> String {
        pause_client.wait(PAUSE_REQUEST).await;
        match inner {
            RequestType::Udf {
                request,
                environment_data,
                mut response,
                queue_timer,
            } => {
                drop(queue_timer);
                // TODO: Add metrics with funrun tagging
                let timer = service_request_timer(&request.udf_type);
                let udf_path = request.path_and_args.udf_path().to_owned();
                let environment = DatabaseUdfEnvironment::new(
                    self.rt.clone(),
                    environment_data,
                    heap_stats.clone(),
                    request,
                );
                let r = environment
                    .run(isolate, isolate_clean, response.cancellation().boxed())
                    .await;
                let status = match &r {
                    Ok((_tx, outcome)) => {
                        if is_developer_ok(outcome) {
                            RequestStatus::Success
                        } else {
                            RequestStatus::DeveloperError
                        }
                    },
                    Err(_) => RequestStatus::SystemError,
                };
                finish_service_request_timer(timer, status);
                let _ = response.send(r);
                format!("UDF: {udf_path:?}")
            },
            RequestType::Action {
                request,
                environment_data,
                response,
                queue_timer,
                action_callbacks,
                fetch_client,
                log_line_sender,
            } => {
                drop(queue_timer);
                let timer = service_request_timer(&UdfType::Action);
                let udf_path = request.params.path_and_args.udf_path().to_owned();
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    environment_data,
                    request.identity,
                    request.transaction,
                    action_callbacks,
                    fetch_client,
                    log_line_sender,
                    heap_stats.clone(),
                    request.context,
                );
                let r = environment
                    .run_action(isolate, isolate_clean, request.params.clone())
                    .await;

                let status = match &r {
                    Ok(outcome) => {
                        if outcome.result.is_ok() {
                            RequestStatus::Success
                        } else {
                            RequestStatus::DeveloperError
                        }
                    },
                    Err(_) => RequestStatus::SystemError,
                };
                finish_service_request_timer(timer, status);
                let _ = response.send(r);
                format!("Action: {udf_path:?}")
            },
            _ => {
                report_error(&mut anyhow::anyhow!(
                    "Unsupported request sent to funrun isolate",
                ));
                "Unsupported request".to_string()
            },
        }
    }

    fn config(&self) -> &IsolateConfig {
        &self.isolate_config
    }

    fn rt(&self) -> RT {
        self.rt.clone()
    }

    #[cfg(any(test, feature = "testing"))]
    fn pause_client(&self) -> Option<Arc<tokio::sync::Mutex<PauseClient>>> {
        self.pause_client.clone()
    }
}

#[cfg(test)]
mod tests {
    use common::pause::PauseController;
    use isolate::{
        client::PAUSE_RECREATE_CLIENT,
        test_helpers::{
            test_isolate_not_recreated_with_same_client,
            test_isolate_recreated_with_client_change,
        },
        IsolateConfig,
    };
    use runtime::testing::TestRuntime;

    use super::FunctionRunnerIsolateWorker;

    #[convex_macro::test_runtime]
    async fn test_isolate_recreated_with_client_change_function_runner(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let isolate_config = IsolateConfig::default();
        let (pause, pause_client) = PauseController::new([PAUSE_RECREATE_CLIENT]);
        let worker =
            FunctionRunnerIsolateWorker::new_for_tests(rt.clone(), isolate_config, pause_client);
        test_isolate_recreated_with_client_change(rt, worker, pause).await
    }

    #[convex_macro::test_runtime]
    async fn test_isolate_not_recreated_with_same_client_function_runner(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let isolate_config = IsolateConfig::default();
        let (pause, pause_client) = PauseController::new([PAUSE_RECREATE_CLIENT]);
        let worker =
            FunctionRunnerIsolateWorker::new_for_tests(rt.clone(), isolate_config, pause_client);
        test_isolate_not_recreated_with_same_client(rt, worker, pause).await
    }
}
