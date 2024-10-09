use std::sync::Arc;

use anyhow::Context;
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        Reference,
    },
    errors::JsError,
    execution_context::ExecutionContext,
    http::RoutedHttpPath,
    knobs::EXECUTE_HTTP_ACTIONS_IN_FUNRUN,
    log_lines::{
        run_function_and_collect_log_lines,
        LogLevel,
        LogLine,
        LogLines,
        SystemLogMetadata,
    },
    runtime::Runtime,
    types::{
        FunctionCaller,
        ModuleEnvironment,
        RoutableMethod,
    },
    RequestId,
};
use database::{
    BootstrapComponentsModel,
    Transaction,
};
use errors::ErrorMetadataAnyhowExt;
use function_runner::server::HttpActionMetadata;
use futures::{
    channel::mpsc,
    select_biased,
    FutureExt,
    StreamExt,
};
use http::StatusCode;
use isolate::{
    ActionCallbacks,
    HttpActionOutcome,
    HttpActionRequest,
    HttpActionRequestHead,
    HttpActionResponsePart,
    HttpActionResponseStreamer,
    HttpActionResult,
    ValidatedHttpPath,
};
use keybroker::Identity;
use model::modules::{
    ModuleModel,
    HTTP_MODULE_PATH,
};
use sync_types::{
    CanonicalizedUdfPath,
    FunctionName,
};
use usage_tracking::FunctionUsageTracker;

use super::ApplicationFunctionRunner;
use crate::function_log::HttpActionStatusCode;

impl<RT: Runtime> ApplicationFunctionRunner<RT> {
    #[minitrace::trace]
    pub async fn run_http_action(
        &self,
        request_id: RequestId,
        http_request: HttpActionRequest,
        mut response_streamer: HttpActionResponseStreamer,
        identity: Identity,
        caller: FunctionCaller,
        action_callbacks: Arc<dyn ActionCallbacks>,
    ) -> anyhow::Result<isolate::HttpActionResult> {
        let start = self.runtime.monotonic_now();
        let usage_tracker = FunctionUsageTracker::new();

        let mut tx = self
            .database
            .begin_with_usage(identity.clone(), usage_tracker.clone())
            .await?;

        let (component_path, routed_path) =
            match self.route_http_action(&mut tx, &http_request.head).await? {
                Some(r) => r,
                None => {
                    drop(tx);
                    let response_parts = isolate::HttpActionResponsePart::from_text(
                        StatusCode::NOT_FOUND,
                        "This Convex deployment does not have HTTP actions enabled.".to_string(),
                    );
                    for part in response_parts {
                        response_streamer.send_part(part)?;
                    }
                    return Ok(isolate::HttpActionResult::Streamed);
                },
            };
        let path = CanonicalizedComponentFunctionPath {
            component: component_path,
            udf_path: CanonicalizedUdfPath::new(
                HTTP_MODULE_PATH.clone(),
                FunctionName::default_export(),
            ),
        };
        let validated_path = match ValidatedHttpPath::new(&mut tx, path).await? {
            Ok(validated_path) => validated_path,
            Err(e) => return Ok(isolate::HttpActionResult::Error(e)),
        };
        let unix_timestamp = self.runtime.unix_timestamp();
        let context = ExecutionContext::new(request_id, &caller);

        let request_head = http_request.head.clone();
        let route = http_request.head.route_for_failure();
        let (log_line_sender, log_line_receiver) = mpsc::unbounded();
        // We want to intercept the response head so we can log it on function
        // completion, but still stream the response as it comes in, so we
        // create another channel here.
        let (isolate_response_sender, mut isolate_response_receiver) = mpsc::unbounded();
        let outcome_future = if *EXECUTE_HTTP_ACTIONS_IN_FUNRUN {
            self.isolate_functions
                .execute_http_action(
                    tx,
                    log_line_sender,
                    HttpActionMetadata {
                        http_response_streamer: HttpActionResponseStreamer::new(
                            isolate_response_sender,
                        ),
                        http_module_path: validated_path,
                        routed_path,
                        http_request,
                    },
                    context.clone(),
                )
                .boxed()
        } else {
            self.http_actions
                .execute_http_action(
                    validated_path,
                    routed_path,
                    http_request,
                    identity.clone(),
                    action_callbacks,
                    self.fetch_client.clone(),
                    log_line_sender,
                    HttpActionResponseStreamer::new(isolate_response_sender),
                    tx,
                    context.clone(),
                )
                .boxed()
        };

        let context_ = context.clone();
        let mut outcome_and_log_lines_fut = Box::pin(
            run_function_and_collect_log_lines(outcome_future, log_line_receiver, |log_line| {
                self.function_log.log_http_action_progress(
                    route.clone(),
                    unix_timestamp,
                    context_.clone(),
                    vec![log_line].into(),
                    // http actions are always run in Isolate
                    ModuleEnvironment::Isolate,
                )
            })
            .fuse(),
        );

        let mut result_for_logging = None;
        let (outcome_result, mut log_lines): (anyhow::Result<HttpActionOutcome>, LogLines) = loop {
            select_biased! {
                result = isolate_response_receiver.select_next_some() => {
                    match result {
                        HttpActionResponsePart::Head(h) => {
                            result_for_logging = Some(Ok(HttpActionStatusCode(h.status)));
                            response_streamer.send_part(HttpActionResponsePart::Head(h))?;
                        },
                        HttpActionResponsePart::BodyChunk(bytes) => {
                            response_streamer.send_part(HttpActionResponsePart::BodyChunk(bytes))?;
                        }
                    }
                },
                outcome_and_log_lines = outcome_and_log_lines_fut => {
                    break outcome_and_log_lines
                }
            }
        };

        while let Some(part) = isolate_response_receiver.next().await {
            match part {
                HttpActionResponsePart::Head(h) => {
                    result_for_logging = Some(Ok(HttpActionStatusCode(h.status)));
                    response_streamer.send_part(HttpActionResponsePart::Head(h))?;
                },
                HttpActionResponsePart::BodyChunk(bytes) => {
                    response_streamer.send_part(HttpActionResponsePart::BodyChunk(bytes))?;
                },
            }
        }

        let response_sha256 = response_streamer.complete();

        match outcome_result {
            Ok(outcome) => {
                let result = outcome.result.clone();
                let result_for_logging = match &result {
                    HttpActionResult::Error(e) => Err(e.clone()),
                    HttpActionResult::Streamed => result_for_logging.ok_or_else(|| {
                        anyhow::anyhow!(
                            "Result should be populated for successfully completed HTTP action"
                        )
                    })?,
                };
                self.function_log.log_http_action(
                    outcome,
                    result_for_logging,
                    log_lines,
                    start.elapsed(),
                    caller,
                    usage_tracker,
                    context,
                    response_sha256,
                );
                Ok(result)
            },
            Err(e) if e.is_deterministic_user_error() => {
                let js_err = JsError::from_error(e);
                match result_for_logging {
                    Some(r) => {
                        let outcome = HttpActionOutcome::new(
                            None,
                            request_head,
                            identity.into(),
                            unix_timestamp,
                            HttpActionResult::Streamed,
                            None,
                            None,
                        );
                        log_lines.push(LogLine::new_system_log_line(
                            LogLevel::Warn,
                            vec![js_err.to_string()],
                            outcome.unix_timestamp,
                            SystemLogMetadata {
                                code: "error:httpAction".to_string(),
                            },
                        ));
                        self.function_log.log_http_action(
                            outcome.clone(),
                            r,
                            log_lines,
                            start.elapsed(),
                            caller,
                            usage_tracker,
                            context,
                            response_sha256,
                        );
                        Ok(HttpActionResult::Streamed)
                    },
                    None => {
                        let result = isolate::HttpActionResult::Error(js_err.clone());
                        let outcome = HttpActionOutcome::new(
                            None,
                            request_head,
                            identity.into(),
                            unix_timestamp,
                            result.clone(),
                            None,
                            None,
                        );
                        self.function_log.log_http_action(
                            outcome.clone(),
                            Err(js_err),
                            log_lines,
                            start.elapsed(),
                            caller,
                            usage_tracker,
                            context,
                            response_sha256,
                        );
                        Ok(result)
                    },
                }
            },
            Err(e) => {
                self.function_log.log_http_action_system_error(
                    &e,
                    request_head,
                    identity.into(),
                    start,
                    caller,
                    log_lines,
                    context,
                    response_sha256,
                );
                Err(e)
            },
        }
    }

    async fn route_http_action(
        &self,
        tx: &mut Transaction<RT>,
        head: &HttpActionRequestHead,
    ) -> anyhow::Result<Option<(ComponentPath, RoutedHttpPath)>> {
        let mut model = BootstrapComponentsModel::new(tx);
        let mut current_component_path = ComponentPath::root();
        let mut routed_path = RoutedHttpPath(head.url.path().to_string());
        let method = RoutableMethod::try_from(head.method.clone())?;
        loop {
            let (definition_id, current_id) =
                model.must_component_path_to_ids(&current_component_path)?;
            let definition = model.load_definition_metadata(definition_id).await?;
            let http_routes = ModuleModel::new(model.tx)
                .get_http(current_id)
                .await?
                .map(|m| {
                    m.into_value()
                        .analyze_result
                        .context("Missing analyze result for http module")?
                        .http_routes
                        .context("Missing http routes")
                })
                .transpose()?;

            if http_routes.is_none() && definition.http_mounts.is_empty() {
                return Ok(None);
            }

            // First, try matching an exact path from `http.js`, which will always
            // be the most specific match.
            if let Some(ref http_routes) = http_routes {
                if http_routes.route_exact(&routed_path[..], method) {
                    return Ok(Some((current_component_path, routed_path)));
                }
            }

            // Next, try finding the most specific prefix match from both `http.js`
            // and the component-level mounts.
            enum CurrentMatch<'a> {
                CurrentHttpJs,
                MountedComponent(&'a Reference),
            }
            let mut longest_match = None;

            if let Some(ref http_routes) = http_routes {
                if let Some(match_suffix) = http_routes.route_prefix(&routed_path, method) {
                    longest_match = Some((match_suffix, CurrentMatch::CurrentHttpJs));
                }
            }
            for (mount_path, reference) in &definition.http_mounts {
                let Some(match_suffix) = routed_path.strip_prefix(&mount_path[..]) else {
                    continue;
                };
                let new_match = RoutedHttpPath(format!("/{match_suffix}"));
                if let Some((ref existing_suffix, _)) = longest_match {
                    // If the existing longest match has a shorter suffix, then it
                    // matches a longer prefix.
                    if existing_suffix.len() < match_suffix.len() {
                        continue;
                    }
                }
                longest_match = Some((new_match, CurrentMatch::MountedComponent(reference)));
            }
            match longest_match {
                None => {
                    // If we couldn't match the route, forward the request to the current
                    // component's `http.js` if present. This lets the JS layer uniformly handle
                    // 404s when defined.
                    if http_routes.is_some() {
                        return Ok(Some((
                            current_component_path,
                            RoutedHttpPath(routed_path.to_string()),
                        )));
                    } else {
                        return Ok(None);
                    }
                },
                Some((_, CurrentMatch::CurrentHttpJs)) => {
                    return Ok(Some((
                        current_component_path,
                        RoutedHttpPath(routed_path.to_string()),
                    )));
                },
                Some((match_suffix, CurrentMatch::MountedComponent(reference))) => {
                    let Reference::ChildComponent {
                        component: name,
                        attributes,
                    } = reference
                    else {
                        anyhow::bail!("Invalid reference in component definition: {reference:?}");
                    };
                    anyhow::ensure!(attributes.is_empty());

                    current_component_path = current_component_path.join(name.clone());
                    routed_path = match_suffix;
                    continue;
                },
            }
        }
    }
}
