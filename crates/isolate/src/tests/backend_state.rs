use common::{
    runtime::Runtime,
    testing::assert_contains,
};
use database::Database;
use errors::ErrorMetadata;
use keybroker::Identity;
use model::backend_state::{
    types::BackendState,
    BackendStateModel,
    DISABLED_ERROR_MESSAGE,
    PAUSED_ERROR_MESSAGE,
    SUSPENDED_ERROR_MESSAGE,
};
use runtime::testing::TestRuntime;
use tokio::sync::mpsc;
use value::assert_obj;

use crate::{
    test_helpers::UdfTest,
    tests::http_action::{
        http_action_udf_test,
        http_post_request,
    },
    HttpActionResponseStreamer,
};

#[convex_macro::test_runtime]
async fn test_query_while_paused(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_helper(rt, BackendState::Paused, PAUSED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_mutation_while_paused(rt: TestRuntime) -> anyhow::Result<()> {
    test_mutation_helper(rt, BackendState::Paused, PAUSED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_action_while_paused(rt: TestRuntime) -> anyhow::Result<()> {
    test_action_helper(rt, BackendState::Paused, PAUSED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_http_action_while_paused(rt: TestRuntime) -> anyhow::Result<()> {
    test_http_action_helper(rt, BackendState::Paused).await
}

#[convex_macro::test_runtime]
async fn test_query_while_disabled(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_helper(rt, BackendState::Disabled, DISABLED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_mutation_while_disabled(rt: TestRuntime) -> anyhow::Result<()> {
    test_mutation_helper(rt, BackendState::Disabled, DISABLED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_action_while_disabled(rt: TestRuntime) -> anyhow::Result<()> {
    test_action_helper(rt, BackendState::Disabled, DISABLED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_http_action_while_disabled(rt: TestRuntime) -> anyhow::Result<()> {
    test_http_action_helper(rt, BackendState::Disabled).await
}

#[convex_macro::test_runtime]
async fn test_query_while_suspended(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_helper(rt, BackendState::Suspended, SUSPENDED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_mutation_while_suspended(rt: TestRuntime) -> anyhow::Result<()> {
    test_mutation_helper(rt, BackendState::Suspended, SUSPENDED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_action_while_suspended(rt: TestRuntime) -> anyhow::Result<()> {
    test_action_helper(rt, BackendState::Suspended, SUSPENDED_ERROR_MESSAGE).await
}

#[convex_macro::test_runtime]
async fn test_http_action_while_suspended(rt: TestRuntime) -> anyhow::Result<()> {
    test_http_action_helper(rt, BackendState::Suspended).await
}

async fn test_query_helper(
    rt: TestRuntime,
    backend_state: BackendState,
    error_message: &str,
) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    toggle_backend_state(&t.database, backend_state).await?;
    let error = t.query_js_error("basic:count", assert_obj!()).await?;
    assert_contains(&error, error_message);
    Ok(())
}

async fn test_mutation_helper(
    rt: TestRuntime,
    backend_state: BackendState,
    error_message: &str,
) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    toggle_backend_state(&t.database, backend_state).await?;
    let error = t
        .mutation_js_error("basic:addOneInt", assert_obj!("x" => 1))
        .await?;
    assert_contains(&error, error_message);
    Ok(())
}

async fn test_action_helper(
    rt: TestRuntime,
    backend_state: BackendState,
    error_message: &str,
) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    toggle_backend_state(&t.database, backend_state).await?;
    let error = t
        .action_js_error("action:getCloudUrl", assert_obj!())
        .await?;
    assert_contains(&error, error_message);
    Ok(())
}

async fn test_http_action_helper(
    rt: TestRuntime,
    backend_state: BackendState,
) -> anyhow::Result<()> {
    let t = http_action_udf_test(rt).await?;
    toggle_backend_state(&t.database, backend_state.clone()).await?;
    let (http_response_sender, _http_response_receiver) = mpsc::unbounded_channel();
    let error = t
        .raw_http_action(
            "http_action",
            http_post_request("basic", "hi".as_bytes().to_vec()),
            Identity::system(),
            HttpActionResponseStreamer::new(http_response_sender),
        )
        .await
        .unwrap_err();
    assert_error(error, backend_state);
    Ok(())
}

fn assert_error(error: anyhow::Error, backend_state: BackendState) {
    let error_message = match backend_state {
        BackendState::Paused => "NoRunWhilePaused",
        BackendState::Disabled => "NoRunWhileDisabled",
        BackendState::Suspended => "NoRunWhileSuspended",
        BackendState::Running => return,
    };
    let error_metadata = error.downcast_ref::<ErrorMetadata>().unwrap();
    assert_eq!(error_metadata.short_msg, error_message);
}

async fn toggle_backend_state<RT: Runtime>(
    db: &Database<RT>,
    backend_state: BackendState,
) -> anyhow::Result<()> {
    let mut tx = db.begin_system().await?;
    BackendStateModel::new(&mut tx)
        .toggle_backend_state(backend_state)
        .await?;
    db.commit(tx).await?;
    Ok(())
}
