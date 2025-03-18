use common::{
    pause::PauseController,
    runtime::tokio_spawn,
    types::FunctionCaller,
};
use futures::StreamExt;
use headers::HeaderMap;
use http::{
    Method,
    StatusCode,
};
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use tokio::{
    select,
    sync::mpsc,
};
use udf::{
    HttpActionRequest,
    HttpActionRequestHead,
    HttpActionResponsePart,
    HttpActionResponseStreamer,
};
use url::Url;

use crate::{
    function_log::{
        FunctionExecution,
        FunctionExecutionPart,
        FunctionExecutionProgress,
        HttpActionStatusCode,
        UdfParams,
    },
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_http_action_basic(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_actions")
        .await?;

    // Create a basic HTTP request
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/echo")?,
            method: Method::POST,
        },
        body: Some(futures::stream::once(async move { Ok("test body".into()) }).boxed()),
    };

    // Create channels for response streaming
    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    // Run the HTTP action
    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    // Collect response parts
    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    // Verify response parts
    assert_eq!(response_parts.len(), 2); // Head and body
    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
            assert!(head.headers.contains_key("content-type"));
        },
        _ => panic!("Expected head part first"),
    }

    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "test body");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_action_error(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_actions")
        .await?;

    // Create a request to an endpoint that will error
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/errorInEndpoint")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    // Run the HTTP action
    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    // Collect response parts
    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    // Verify error response
    assert_eq!(response_parts.len(), 2); // Head and body
    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::INTERNAL_SERVER_ERROR);
        },
        _ => panic!("Expected head part first"),
    }

    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            let error_msg = std::str::from_utf8(body)?;
            assert!(error_msg.contains("Custom error"), "error_msg: {error_msg}");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_action_not_found(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_actions")
        .await?;

    // Create a request to a non-existent endpoint
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/nonexistent")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    // Run the HTTP action
    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    // Collect response parts
    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    // Verify 404 response
    assert_eq!(response_parts.len(), 2); // Head and body
    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_action_disconnect_before_head(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_actions")
        .await?;

    let hold = pause_controller.hold("begin_run_sleep");
    let hold_end = pause_controller.hold("end_run_http_action");

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/pausableHello")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    let mut http_action_fut = Box::pin(application.http_action_udf(
        common::RequestId::new(),
        http_request,
        Identity::system(),
        FunctionCaller::HttpEndpoint,
        response_streamer,
    ));
    select! {
        _ = &mut http_action_fut => {
            anyhow::bail!("HTTP action should pause");
        },
        paused = hold.wait_for_blocked() => {
            // The HTTP action hit the sleep
            let paused = paused.expect("HTTP action should pause");
            // Simulate disconnecting the client by dropping the future.
            drop(response_receiver);
            drop(http_action_fut);
            paused.unpause();
        },
    }
    // The client has disconnected. The HTTP action should still run to completion
    // and log to the execution log.
    let paused = hold_end.wait_for_blocked().await;
    paused.expect("HTTP action should pause").unpause();

    let (function_log, _) = application.function_log().stream(0.0).await;
    let last_log_entry = function_log.last().unwrap();
    must_let!(let UdfParams::Http { result, .. } = &last_log_entry.params);
    must_let!(let Err(e) = &result);
    // It fails to write the response HEAD because the channel closed,
    // so it gets logged as a 500.
    assert!(e.to_string().contains("Client disconnected"), "{e}");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_action_disconnect_while_streaming(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_actions")
        .await?;

    let hold = pause_controller.hold("begin_run_sleep");
    let hold_end = pause_controller.hold("end_run_http_action");

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/pausableHelloBody")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    let application_ = application.clone();
    let http_action_fut = tokio_spawn("test_http_action", async move {
        application_
            .http_action_udf(
                common::RequestId::new(),
                http_request,
                Identity::system(),
                FunctionCaller::HttpEndpoint,
                response_streamer,
            )
            .await
    });
    must_let!(let HttpActionResponsePart::Head(head) = response_receiver.recv().await.expect("should receive head"));
    assert_eq!(head.status, StatusCode::OK);
    must_let!(let HttpActionResponsePart::BodyChunk(first_body_chunk) = response_receiver.recv().await.expect("should receive first body chunk"));
    assert_eq!(std::str::from_utf8(&first_body_chunk)?, "Hello, ");
    let paused = hold.wait_for_blocked().await;
    let paused = paused.expect("HTTP action should pause, actually died (maybe deadlock)");
    // The HTTP action hit the sleep
    // Simulate disconnecting the client by dropping the receiver and the future.
    drop(response_receiver);
    drop(http_action_fut);
    paused.unpause();
    // The client has disconnected. The HTTP action should still run to completion
    // and log to the execution log.
    let paused = hold_end.wait_for_blocked().await;
    paused.expect("HTTP action should pause").unpause();

    let (mut function_log, _) = application.function_log().stream_parts(0.0).await;
    let execution_entry = function_log.pop().unwrap();
    let log_entry = function_log.pop().unwrap();
    must_let!(let FunctionExecutionPart::Completion(
        FunctionExecution { params: UdfParams::Http { result, .. }, .. }
    ) = &execution_entry);
    // The HEAD was a 200, not an error
    must_let!(let Ok(HttpActionStatusCode(status)) = result);
    assert_eq!(*status, StatusCode::OK);

    must_let!(let FunctionExecutionPart::Progress(
        FunctionExecutionProgress { log_lines, .. }
    ) = &log_entry);
    assert_eq!(log_lines.len(), 1);
    assert_eq!(
        log_lines[0].clone().to_pretty_string_test_only(),
        "[WARN] Client disconnected\n"
    );

    Ok(())
}
