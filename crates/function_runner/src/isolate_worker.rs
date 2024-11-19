#[cfg(any(test, feature = "testing"))]
use std::sync::Arc;

use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    runtime::Runtime,
    sync::oneshot_receiver_closed,
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
        analyze::AnalyzeEnvironment,
        auth_config::AuthConfigEnvironment,
        component_definitions::{
            AppDefinitionEvaluator,
            ComponentInitializerEvaluator,
        },
        schema::SchemaEnvironment,
        udf::DatabaseUdfEnvironment,
    },
    isolate::Isolate,
    metrics::{
        finish_service_request_timer,
        is_developer_ok,
        service_request_timer,
        RequestStatus,
    },
    HttpActionResult,
    IsolateConfig,
};
use sync_types::CanonicalizedUdfPath;
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
    #[cfg_attr(not(test), expect(dead_code))]
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
            client_id,
            inner,
            pause_client,
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
                reactor_depth,
                udf_callback,
            } => {
                drop(queue_timer);
                // TODO: Add metrics with funrun tagging
                let timer = service_request_timer(&request.udf_type);
                let udf_path = request.path_and_args.path().udf_path.to_owned();
                let environment = DatabaseUdfEnvironment::new(
                    self.rt.clone(),
                    environment_data,
                    heap_stats.clone(),
                    request,
                    reactor_depth,
                    udf_callback,
                );
                let r = environment
                    .run(
                        client_id,
                        isolate,
                        isolate_clean,
                        oneshot_receiver_closed(&mut response).boxed(),
                    )
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
                mut response,
                queue_timer,
                action_callbacks,
                fetch_client,
                log_line_sender,
            } => {
                drop(queue_timer);
                let timer = service_request_timer(&UdfType::Action);
                let path = request.params.path_and_args.path();
                let udf_path = path.udf_path.to_owned();
                let component = path.component.to_owned();
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    component,
                    environment_data,
                    request.identity,
                    request.transaction,
                    action_callbacks,
                    fetch_client,
                    log_line_sender,
                    None,
                    heap_stats.clone(),
                    request.context,
                );
                let r = environment
                    .run_action(
                        client_id,
                        isolate,
                        isolate_clean,
                        request.params.clone(),
                        oneshot_receiver_closed(&mut response).boxed(),
                    )
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
            RequestType::Analyze {
                udf_config,
                modules,
                environment_variables,
                response,
            } => {
                let r = AnalyzeEnvironment::analyze::<RT>(
                    client_id,
                    isolate,
                    isolate_clean,
                    udf_config,
                    modules,
                    environment_variables,
                )
                .await;
                let _ = response.send(r);
                "Analyze".to_string()
            },
            RequestType::HttpAction {
                request,
                environment_data,
                mut response,
                queue_timer,
                action_callbacks,
                fetch_client,
                log_line_sender,
                http_response_streamer,
            } => {
                drop(queue_timer);
                let timer = service_request_timer(&UdfType::HttpAction);
                let udf_path: CanonicalizedUdfPath =
                    request.http_module_path.path().udf_path.clone();
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    request.http_module_path.path().component,
                    environment_data,
                    request.identity,
                    request.transaction,
                    action_callbacks,
                    fetch_client,
                    log_line_sender,
                    Some(http_response_streamer),
                    heap_stats.clone(),
                    request.context,
                );
                let r = environment
                    .run_http_action(
                        client_id,
                        isolate,
                        isolate_clean,
                        request.http_module_path,
                        request.routed_path,
                        request.http_request,
                        oneshot_receiver_closed(&mut response).boxed(),
                    )
                    .await;
                let status = match &r {
                    Ok(outcome) => match outcome.result {
                        // Note that the stream could potentially encounter errors later
                        HttpActionResult::Streamed => RequestStatus::Success,
                        HttpActionResult::Error(_) => RequestStatus::DeveloperError,
                    },
                    Err(_) => RequestStatus::SystemError,
                };
                finish_service_request_timer(timer, status);
                let _ = response.send(r);
                format!("Http: {udf_path:?}")
            },
            RequestType::EvaluateSchema {
                schema_bundle,
                source_map,
                rng_seed,
                unix_timestamp,
                response,
            } => {
                let r = SchemaEnvironment::evaluate_schema(
                    client_id,
                    isolate,
                    schema_bundle,
                    source_map,
                    rng_seed,
                    unix_timestamp,
                )
                .await;

                let _ = response.send(r);
                "EvaluateSchema".to_string()
            },
            RequestType::EvaluateAuthConfig {
                auth_config_bundle,
                source_map,
                environment_variables,
                response,
            } => {
                let r = AuthConfigEnvironment::evaluate_auth_config(
                    client_id,
                    isolate,
                    auth_config_bundle,
                    source_map,
                    environment_variables,
                )
                .await;
                let _ = response.send(r);
                "EvaluateAuthConfig".to_string()
            },
            RequestType::EvaluateAppDefinitions {
                app_definition,
                component_definitions,
                dependency_graph,
                environment_variables,
                system_env_vars,
                response,
            } => {
                let env = AppDefinitionEvaluator::new(
                    app_definition,
                    component_definitions,
                    dependency_graph,
                    environment_variables,
                    system_env_vars,
                );
                let r = env.evaluate(client_id, isolate).await;
                let _ = response.send(r);
                "EvaluateAppDefinitions".to_string()
            },
            RequestType::EvaluateComponentInitializer {
                evaluated_definitions,
                path,
                definition,
                args,
                name,
                response,
            } => {
                let env = ComponentInitializerEvaluator::new(
                    evaluated_definitions,
                    path,
                    definition,
                    args,
                    name,
                );
                let r = env.evaluate(client_id, isolate).await;
                let _ = response.send(r);
                "EvaluateComponentInitializer".to_string()
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
