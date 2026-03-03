use std::time::Duration;

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
    },
    http::{
        RequestDestination,
        ResolvedHostname,
    },
    knobs,
    log_streaming::StructuredLogEvent,
    runtime::Runtime,
    types::{
        BackendState,
        FunctionCaller,
    },
    RequestId,
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    stream,
    StreamExt,
};
use keybroker::Identity;
use log_streaming::sinks::mock_sink::MOCK_SINK_EVENTS_BUFFER;
use model::{
    backend_state::BackendStateModel,
    canonical_urls::types::CanonicalUrl,
    file_storage::FileStorageId,
    log_sinks::{
        types::SinkConfig,
        LogSinksModel,
    },
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::json;
use sync_types::types::SerializedArgs;
use value::ConvexValue;

use crate::{
    api::{
        ApplicationApi,
        ExecuteQueryTimestamp,
    },
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
pub(crate) async fn test_backend_not_running_cannot_store_file(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;

    let file_body = Box::pin(stream::once(async {
        Ok(bytes::Bytes::from(vec![55; 1024 + 1]))
    }));
    let ok_result = app
        .store_file(ComponentId::Root, None, None, None, file_body)
        .await;
    assert!(ok_result.is_ok());

    let mut tx = app.begin(Identity::system()).await?;
    BackendStateModel::new(&mut tx)
        .toggle_backend_state(BackendState::Disabled)
        .await?;
    app.commit_test(tx).await?;
    let file_body = Box::pin(stream::once(async {
        Ok(bytes::Bytes::from(vec![55; 1024 + 1]))
    }));
    let result = app
        .store_file(ComponentId::Root, None, None, None, file_body)
        .await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.is_bad_request());
    assert_eq!(error.short_msg(), "BackendIsNotRunning");
    Ok(())
}

// Test of successful ctx.storage.getUrl from query and action.
// The action uses a different codepath, going through action callbacks, but
// should have the same url.
// Also check that canonical url is respected.
#[convex_macro::test_runtime]
async fn test_storage_get_url(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    app.load_udf_tests_modules().await?;

    let file_body = Box::pin(stream::once(async {
        Ok(bytes::Bytes::from(vec![55; 1024 + 1]))
    }));
    let id = app
        .store_file(ComponentId::Root, None, None, None, file_body)
        .await?;

    // Call ctx.storage.getUrl from a query.
    let request_id = RequestId::new();
    let identity = Identity::system();
    let args = vec![json!({"id": id.to_string()})];
    let caller = FunctionCaller::Action {
        parent_scheduled_job: None,
        parent_execution_id: None,
    };
    let host = ResolvedHostname {
        instance_name: "carnitas".to_string(),
        destination: RequestDestination::ConvexCloud,
    };
    let ts = ExecuteQueryTimestamp::Latest;
    let query_result = app
        .execute_admin_query(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:getFileUrl".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
            ts.clone(),
            None,
        )
        .await?;
    must_let!(let ConvexValue::String(url) = query_result.result?.unpack()?);
    assert!(url.starts_with("http://127.0.0.1:8000/api/storage/"));
    // Call it from an action.
    let action_result = app
        .execute_admin_action(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:getFileUrlFromAction".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
        )
        .await??;
    must_let!(let ConvexValue::String(url) = action_result.value.unpack()?);
    assert!(url.starts_with("http://127.0.0.1:8000/api/storage/"));

    // Now set a canonical url and call the functions again.
    let mut tx = app.begin(Identity::system()).await?;
    app.set_canonical_url(
        &mut tx,
        CanonicalUrl {
            request_destination: RequestDestination::ConvexCloud,
            url: "https://carnitas.convex.cloud".to_string(),
        },
    )
    .await?;
    app.commit_test(tx).await?;

    let query_result = app
        .execute_admin_query(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:getFileUrl".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
            ts.clone(),
            None,
        )
        .await?;
    must_let!(let ConvexValue::String(url) = query_result.result?.unpack()?);
    assert!(url.starts_with("https://carnitas.convex.cloud/api/storage/"));
    // Call it from an action.
    let action_result = app
        .execute_admin_action(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:getFileUrlFromAction".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
        )
        .await??;
    must_let!(let ConvexValue::String(url) = action_result.value.unpack()?);
    assert!(url.starts_with("https://carnitas.convex.cloud/api/storage/"));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_storage_generate_upload_url(rt: TestRuntime) -> anyhow::Result<()> {
    let app = Application::new_for_tests(&rt).await?;
    app.load_udf_tests_modules().await?;

    // Call ctx.storage.generateUploadUrl from a mutation.
    let request_id = RequestId::new();
    let identity = Identity::system();
    let args = vec![json!({})];
    let caller = FunctionCaller::Action {
        parent_scheduled_job: None,
        parent_execution_id: None,
    };
    let host = ResolvedHostname {
        instance_name: "carnitas".to_string(),
        destination: RequestDestination::ConvexCloud,
    };
    let mutation_result = app
        .execute_admin_mutation(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:generateUploadUrl".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
            None,
            None,
        )
        .await??;
    must_let!(let ConvexValue::String(url) = mutation_result.value.unpack()?);
    assert!(url.starts_with("http://127.0.0.1:8000/api/storage/upload?token="));

    // Call it from an action.
    let action_result = app
        .execute_admin_action(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:generateUploadUrlFromAction".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
        )
        .await??;
    must_let!(let ConvexValue::String(url) = action_result.value.unpack()?);
    assert!(url.starts_with("http://127.0.0.1:8000/api/storage/upload?token="));

    // Now set a canonical url and call the functions again.
    let mut tx = app.begin(Identity::system()).await?;
    app.set_canonical_url(
        &mut tx,
        CanonicalUrl {
            request_destination: RequestDestination::ConvexCloud,
            url: "https://carnitas.convex.cloud".to_string(),
        },
    )
    .await?;
    app.commit_test(tx).await?;

    let mutation_result = app
        .execute_admin_mutation(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:generateUploadUrl".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
            None,
            None,
        )
        .await??;
    must_let!(let ConvexValue::String(url) = mutation_result.value.unpack()?);
    assert!(url.starts_with("https://carnitas.convex.cloud/api/storage/upload?token="));

    // Call it from an action.
    let action_result = app
        .execute_admin_action(
            &host,
            request_id.clone(),
            identity.clone(),
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::root(),
                udf_path: "storage:generateUploadUrlFromAction".parse()?,
            },
            SerializedArgs::from_args(args.clone())?,
            caller.clone(),
        )
        .await??;
    must_let!(let ConvexValue::String(url) = action_result.value.unpack()?);
    assert!(url.starts_with("https://carnitas.convex.cloud/api/storage/upload?token="));

    Ok(())
}

/// Wait for log aggregation and return storage bandwidth events from the mock
/// sink buffer.
async fn collect_storage_bandwidth_events(rt: &TestRuntime) -> Vec<(String, u64)> {
    rt.wait(Duration::from_millis(
        *knobs::LOG_MANAGER_AGGREGATION_INTERVAL_MILLIS,
    ))
    .await;
    MOCK_SINK_EVENTS_BUFFER
        .read()
        .iter()
        .filter_map(|e| match &e.event {
            StructuredLogEvent::StorageApiBandwidth {
                storage_id,
                egress_bytes,
            } => Some((storage_id.clone(), *egress_bytes)),
            _ => None,
        })
        .collect()
}

#[convex_macro::test_runtime]
async fn test_storage_api_bandwidth_log_events(rt: TestRuntime) -> anyhow::Result<()> {
    use std::ops::Bound;

    let app = Application::new_for_tests(&rt).await?;

    // Set up mock log sink.
    MOCK_SINK_EVENTS_BUFFER.write().clear();
    let mut tx = app.begin(Identity::system()).await?;
    let mut model = LogSinksModel::new(&mut tx);
    model.add_or_update(SinkConfig::Mock).await?;
    app.commit_test(tx).await?;
    rt.wait(Duration::from_secs(1)).await;

    // --- Test 1: get_file full read emits bandwidth event ---
    let file_body = Box::pin(stream::once(async {
        Ok(bytes::Bytes::from(vec![42u8; 2048]))
    }));
    let doc_id = app
        .store_file(ComponentId::Root, None, None, None, file_body)
        .await?;

    let mut file_stream = app
        .get_file(ComponentId::Root, FileStorageId::DocumentId(doc_id))
        .await?;
    let mut total = 0u64;
    while let Some(chunk) = file_stream.next().await {
        total += chunk?.len() as u64;
    }
    drop(file_stream);
    assert_eq!(total, 2048);

    let events = collect_storage_bandwidth_events(&rt).await;
    assert_eq!(events.len(), 1, "expected one event after full read");
    assert_eq!(
        events[0].0,
        doc_id.to_string(),
        "storage_id should be document ID"
    );
    assert_eq!(events[0].1, 2048);

    // --- Test 2: get_file partial read still emits event ---
    MOCK_SINK_EVENTS_BUFFER.write().clear();

    let file_body = Box::pin(stream::once(async {
        Ok(bytes::Bytes::from(vec![7u8; 4096]))
    }));
    let doc_id = app
        .store_file(ComponentId::Root, None, None, None, file_body)
        .await?;

    let mut file_stream = app
        .get_file(ComponentId::Root, FileStorageId::DocumentId(doc_id))
        .await?;
    let _first_chunk = file_stream.next().await;
    // Drop without fully consuming.
    drop(file_stream);

    let events = collect_storage_bandwidth_events(&rt).await;
    assert_eq!(events.len(), 1, "expected one event after partial read");
    assert_eq!(
        events[0].0,
        doc_id.to_string(),
        "storage_id should be document ID"
    );
    assert!(events[0].1 > 0, "should have streamed some bytes");

    // --- Test 3: get_file_range emits bandwidth event ---
    MOCK_SINK_EVENTS_BUFFER.write().clear();

    let file_body = Box::pin(stream::once(async {
        Ok(bytes::Bytes::from(vec![99u8; 4096]))
    }));
    let doc_id = app
        .store_file(ComponentId::Root, None, None, None, file_body)
        .await?;

    let range = (Bound::Included(0), Bound::Included(1023));
    let mut file_stream = app
        .get_file_range(ComponentId::Root, FileStorageId::DocumentId(doc_id), range)
        .await?;
    let mut total = 0u64;
    while let Some(chunk) = file_stream.next().await {
        total += chunk?.len() as u64;
    }
    drop(file_stream);

    let events = collect_storage_bandwidth_events(&rt).await;
    assert_eq!(events.len(), 1, "expected one event after range read");
    assert_eq!(
        events[0].0,
        doc_id.to_string(),
        "storage_id should be document ID"
    );
    assert_eq!(events[0].1, total);

    Ok(())
}
