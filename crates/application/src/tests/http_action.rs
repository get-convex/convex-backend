use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    http::{
        RequestDestination,
        ResolvedHostname,
    },
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
use keybroker::{
    testing::TestUserIdentity,
    Identity,
    UserIdentity,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::json;
use sync_types::types::SerializedArgs;
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
use value::val;

use crate::{
    api::{
        ApplicationApi,
        ExecuteQueryTimestamp,
    },
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

    let (function_log, _) = application.function_log.stream(0.0).await;
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

    let (mut function_log, _) = application.function_log.stream_parts(0.0).await;
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
        "[INFO] Client disconnected\n"
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_mount_routes_to_child_component(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/hello")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "hello from component");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_mount_no_route_outside_mount(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/hello")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_mount_site_url_prefixed(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/site-url")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "http://127.0.0.1:8001/api");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_prefix_and_mount_app_route(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_prefix_and_mount_routing")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/echo")?,
            method: Method::POST,
        },
        body: Some(futures::stream::once(async move { Ok("hello app".into()) }).boxed()),
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "hello app");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_prefix_and_mount_child_route(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_prefix_and_mount_routing")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/hello")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "hello from component");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_prefix_and_mount_child_site_url(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_prefix_and_mount_routing")
        .await?;

    // Child component is mounted at "/", so its CONVEX_SITE_URL should match
    // the base site URL (no path appended).
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/site-url")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "http://127.0.0.1:8001");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_legacy_get_route(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_legacy_routes")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/greet")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(
                std::str::from_utf8(body)?,
                "hello from legacy component route"
            );
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_legacy_post_echo_route(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_legacy_routes")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/echo")?,
            method: Method::POST,
        },
        body: Some(futures::stream::once(async move { Ok("test payload".into()) }).boxed()),
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "test payload");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_legacy_unregistered_route(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_legacy_routes")
        .await?;

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

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_no_prefix_component_routes_not_accessible(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_no_prefix_mounting")
        .await?;

    // Component defines a GET /hello route, but since it was mounted without
    // httpPrefix, it should not be accessible.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/hello")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_grandchild_routes_not_mounted_outside_of_tree(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // Grandchild is mounted at /gc/ within the parent (which is at /api/),
    // so /api/grandchild-hello and /grandchild-hello should 404 — the correct
    // path is /api/gc/grandchild-hello.
    for url in [
        "http://127.0.0.1:8001/api/grandchild-hello",
        "http://127.0.0.1:8001/grandchild-hello",
    ] {
        let http_request = HttpActionRequest {
            head: HttpActionRequestHead {
                headers: HeaderMap::new(),
                url: Url::parse(url)?,
                method: Method::GET,
            },
            body: None,
        };

        let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
        let response_streamer = HttpActionResponseStreamer::new(response_sender);

        application
            .http_action_udf(
                common::RequestId::new(),
                http_request,
                Identity::system(),
                FunctionCaller::HttpEndpoint,
                response_streamer,
            )
            .await?;

        let mut response_parts = Vec::new();
        while let Some(part) = response_receiver.recv().await {
            response_parts.push(part);
        }

        match &response_parts[0] {
            HttpActionResponsePart::Head(head) => {
                assert_eq!(head.status, StatusCode::NOT_FOUND, "Expected 404 for {url}");
            },
            _ => panic!("Expected head part first"),
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_grandchild_explicitly_exposed_via_parent(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // The parent component explicitly exposes grandchild functionality through
    // its own /grandchild-greeting route, which calls the grandchild's query.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/grandchild-greeting")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "hello from grandchild query");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_action_continues_after_client_disconnects(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_actions")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/writeAfterDisconnect")?,
            method: Method::GET,
        },
        body: None,
    };

    let hold = pause_controller.hold("begin_run_sleep");
    let hold_end = pause_controller.hold("end_run_http_action");

    let (response_sender, response_receiver) = mpsc::unbounded_channel();
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
    let paused = hold.wait_for_blocked().await;
    let paused = paused.expect("HTTP action should pause, actually died (maybe deadlock)");
    // The HTTP action hit the sleep
    // Simulate disconnecting the client by dropping the receiver and the future.
    drop(response_receiver);
    drop(http_action_fut);
    paused.unpause();
    let paused = hold_end.wait_for_blocked().await;
    paused.expect("HTTP action should pause").unpause();

    let host = ResolvedHostname {
        instance_name: "carnitas".to_string(),
        destination: RequestDestination::ConvexCloud,
    };

    // The HTTP action should have continued to run after the client disconnected.
    // It ran a mutation, so we run a query to check it.
    let query_result = application
        .execute_admin_query(
            &host,
            common::RequestId::new(),
            Identity::system(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "functions:didWrite".parse()?,
            },
            SerializedArgs::from_args(vec![json!({})])?,
            FunctionCaller::HttpEndpoint,
            ExecuteQueryTimestamp::Latest,
            None,
        )
        .await?;
    assert_eq!(
        query_result.result.map(|v| v.unpack().unwrap()),
        Ok(val!(true))
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_nested_grandchild_route(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // Grandchild is mounted at /gc/ within the parent (which is at /api/),
    // so the full path is /api/gc/grandchild-hello.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/gc/grandchild-hello")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(
                std::str::from_utf8(body)?,
                "hello from grandchild component"
            );
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_grandchild_site_url(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // Grandchild is mounted at /gc/ within the parent (at /api/), so its
    // CONVEX_SITE_URL should be the base URL + /api/gc.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/gc/site-url")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "http://127.0.0.1:8001/api/gc");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_app_catch_all_handles_unmatched_routes(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

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

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "app custom 404");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_grandchild_catch_all_handles_unmatched_routes(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/gc/nonexistent")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }
    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            assert_eq!(std::str::from_utf8(body)?, "grandchild custom 404");
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_mounted_component_does_not_fall_through_to_app_catch_all(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // Request to /api/nonexistent: the /api/ mount prefix matches httpComponent,
    // so routing descends into it. httpComponent has no catch-all, so this should
    // return a generic 404, NOT the app's "app custom 404" catch-all.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/nonexistent")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::system(),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::NOT_FOUND);
        },
        _ => panic!("Expected head part first"),
    }
    // Ensure the body does NOT contain the app's catch-all response.
    // The response should be the generic 404, not the app-level catch-all.
    if let Some(HttpActionResponsePart::BodyChunk(body)) = response_parts.get(1) {
        let body_str = std::str::from_utf8(body)?;
        assert!(
            !body_str.contains("app custom 404"),
            "Expected generic 404, but got app catch-all response: {body_str}"
        );
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_no_prefix_grandchild_routes_not_accessible(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_no_prefix_mounting")
        .await?;

    // httpComponent is used in the app WITHOUT httpPrefix, so even though it
    // internally mounts httpGrandchild with httpPrefix "/gc/", the whole chain
    // is broken.
    for url in [
        "http://127.0.0.1:8001/gc/grandchild-hello",
        "http://127.0.0.1:8001/api/gc/grandchild-hello",
    ] {
        let http_request = HttpActionRequest {
            head: HttpActionRequestHead {
                headers: HeaderMap::new(),
                url: Url::parse(url)?,
                method: Method::GET,
            },
            body: None,
        };

        let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
        let response_streamer = HttpActionResponseStreamer::new(response_sender);

        application
            .http_action_udf(
                common::RequestId::new(),
                http_request,
                Identity::system(),
                FunctionCaller::HttpEndpoint,
                response_streamer,
            )
            .await?;

        let mut response_parts = Vec::new();
        while let Some(part) = response_receiver.recv().await {
            response_parts.push(part);
        }

        match &response_parts[0] {
            HttpActionResponsePart::Head(head) => {
                assert_eq!(head.status, StatusCode::NOT_FOUND, "Expected 404 for {url}");
            },
            _ => panic!("Expected head part first"),
        }
    }

    Ok(())
}

// --- Auth scenario tests ---

#[convex_macro::test_runtime]
async fn test_http_app_action_with_auth_identity(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // App-level /whoami with an authenticated user identity should return 200
    // with the identity data.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/whoami")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::user(UserIdentity::test()),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::OK);
        },
        _ => panic!("Expected head part first"),
    }

    match &response_parts[1] {
        HttpActionResponsePart::BodyChunk(body) => {
            let body_str = std::str::from_utf8(body)?;
            let body_json: serde_json::Value = serde_json::from_str(body_str)?;
            // The identity should include subject and issuer from UserIdentity::test()
            assert!(
                body_json.get("subject").is_some(),
                "Expected identity to have subject, got: {body_str}"
            );
        },
        _ => panic!("Expected body part second"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_app_action_without_auth_identity(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // App-level /whoami without auth should return 401.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/whoami")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::Unknown(None),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(head.status, StatusCode::UNAUTHORIZED);
        },
        _ => panic!("Expected head part first"),
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_http_component_action_cannot_access_auth(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    application
        .load_component_tests_modules("http_mount_routing")
        .await?;

    // Component-level /auth/inspect with an authenticated user identity should
    // return 401 because components should not be able to access auth data.
    let http_request = HttpActionRequest {
        head: HttpActionRequestHead {
            headers: HeaderMap::new(),
            url: Url::parse("http://127.0.0.1:8001/api/whoami")?,
            method: Method::GET,
        },
        body: None,
    };

    let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
    let response_streamer = HttpActionResponseStreamer::new(response_sender);

    application
        .http_action_udf(
            common::RequestId::new(),
            http_request,
            Identity::user(UserIdentity::test()),
            FunctionCaller::HttpEndpoint,
            response_streamer,
        )
        .await?;

    let mut response_parts = Vec::new();
    while let Some(part) = response_receiver.recv().await {
        response_parts.push(part);
    }

    match &response_parts[0] {
        HttpActionResponsePart::Head(head) => {
            assert_eq!(
                head.status,
                StatusCode::UNAUTHORIZED,
                "Component HTTP action should not have access to auth identity"
            );
        },
        _ => panic!("Expected head part first"),
    }

    Ok(())
}
