use async_trait::async_trait;
use common::{
    runtime::Runtime,
    types::UdfType,
};
use deno_core::v8;
use futures::FutureExt;
use sync_types::CanonicalizedUdfPath;
use tracing::Instrument;
use udf::{
    metrics::is_developer_ok,
    HttpActionResult,
};

use crate::{
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
        record_component_function_path,
        service_request_timer,
        RequestStatus,
    },
    IsolateConfig,
};
#[derive(Clone)]
pub(crate) struct FunctionRunnerIsolateWorker<RT: Runtime> {
    rt: RT,
    isolate_config: IsolateConfig,
}

impl<RT: Runtime> FunctionRunnerIsolateWorker<RT> {
    pub(crate) fn new(rt: RT, isolate_config: IsolateConfig) -> Self {
        Self { rt, isolate_config }
    }

    async fn handle_request_inner(
        &self,
        isolate: &mut Isolate<RT>,
        v8_context: v8::Global<v8::Context>,
        isolate_clean: &mut bool,
        Request {
            client_id,
            inner,
            parent_trace: _,
        }: Request<RT>,
        heap_stats: SharedIsolateHeapStats,
    ) -> String {
        match inner {
            RequestType::Udf {
                request,
                environment_data,
                mut response,
                queue_timer,
                rng_seed,
                reactor_depth,
                udf_callback,
                function_started_sender,
            } => {
                drop(queue_timer);
                // TODO: Add metrics with funrun tagging
                let timer = service_request_timer(&request.udf_type);
                record_component_function_path(request.path_and_args.path());
                let udf_path = request.path_and_args.path().udf_path.to_owned();
                let function_timestamp = request.unix_timestamp;
                let environment = DatabaseUdfEnvironment::new(
                    self.rt.clone(),
                    environment_data,
                    heap_stats.clone(),
                    request,
                    reactor_depth,
                    client_id.clone(),
                );
                let r = environment
                    .run(
                        client_id,
                        isolate,
                        v8_context,
                        isolate_clean,
                        response.closed().boxed(),
                        rng_seed,
                        function_timestamp,
                        function_started_sender,
                        udf_callback,
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
                function_started_sender,
            } => {
                drop(queue_timer);
                let timer = service_request_timer(&UdfType::Action);
                let path = request.params.path_and_args.path();
                record_component_function_path(path);
                let udf_path = path.udf_path.to_owned();
                let component = path.component.to_owned();
                let component_path = path.component_path.clone();
                let log_string = format!("Action: {udf_path:?}");
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    component,
                    udf_path,
                    component_path,
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
                        v8_context,
                        isolate_clean,
                        request.params.clone(),
                        response.closed().boxed(),
                        function_started_sender,
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
                log_string
            },
            RequestType::Analyze {
                udf_config,
                modules,
                environment_variables,
                response,
                max_user_heap_size: _,
            } => {
                let r = AnalyzeEnvironment::analyze::<RT>(
                    client_id,
                    isolate,
                    v8_context,
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
                response,
                queue_timer,
                action_callbacks,
                fetch_client,
                log_line_sender,
                http_response_streamer,
                function_started_sender,
            } => {
                drop(queue_timer);
                let timer = service_request_timer(&UdfType::HttpAction);
                let http_path = request.http_module_path.path();
                let udf_path: CanonicalizedUdfPath = http_path.udf_path.clone();
                let component_path = http_path.component_path.clone();
                let log_string = format!("Http: {udf_path:?}");
                record_component_function_path(http_path);
                let environment = ActionEnvironment::new(
                    self.rt.clone(),
                    http_path.component,
                    udf_path,
                    component_path,
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
                        v8_context,
                        isolate_clean,
                        request.http_module_path,
                        request.routed_path,
                        request.http_request,
                        function_started_sender,
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
                log_string
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
                    v8_context,
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
                    v8_context,
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
                user_environment_variables,
                system_env_vars,
                response,
            } => {
                // AppDefinitionEvaluator doesn't use the prewarmed V8 context
                // because it uses an arbitrary number of contexts. This call is
                // rare and not particularly latency-sensitive though
                drop(v8_context);
                let env = AppDefinitionEvaluator::new(
                    app_definition,
                    component_definitions,
                    dependency_graph,
                    user_environment_variables,
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
}

#[async_trait(?Send)]
impl<RT: Runtime> IsolateWorker<RT> for FunctionRunnerIsolateWorker<RT> {
    #[fastrace::trace]
    async fn handle_request(
        &self,
        isolate: &mut Isolate<RT>,
        v8_context: v8::Global<v8::Context>,
        isolate_clean: &mut bool,
        request: Request<RT>,
        heap_stats: SharedIsolateHeapStats,
    ) -> String {
        let pause_client = self.rt.pause_client();
        pause_client.wait(PAUSE_REQUEST).await;
        let client_id = &request.client_id;
        // Set the scope to be tagged with the client_id just for the duration of
        // handling the request. It would be nice to get sentry::with_scope to work, but
        // it uses a synchronous callback and we need `report_error` in the future to
        // have the client_id tag.
        sentry::configure_scope(|scope| scope.set_tag("client_id", client_id));
        // Also add the tag to tracing so it shows up in DataDog logs.
        let span = tracing::info_span!("isolate_worker_handle_request", instance_name = client_id);
        let result = self
            .handle_request_inner(isolate, v8_context, isolate_clean, request, heap_stats)
            .instrument(span)
            .await;
        sentry::configure_scope(|scope| scope.remove_tag("client_id"));
        result
    }

    fn config(&self) -> &IsolateConfig {
        &self.isolate_config
    }

    fn rt(&self) -> RT {
        self.rt.clone()
    }
}
