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
    types::{
        BackendState,
        FunctionCaller,
    },
    RequestId,
};
use errors::ErrorMetadataAnyhowExt;
use futures::stream;
use keybroker::Identity;
use model::{
    backend_state::BackendStateModel,
    canonical_urls::types::CanonicalUrl,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use serde_json::json;
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
            args.clone(),
            caller.clone(),
            ts.clone(),
            None,
        )
        .await?;
    must_let!(let ConvexValue::String(url) = query_result.result?);
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
            args.clone(),
            caller.clone(),
        )
        .await??;
    must_let!(let ConvexValue::String(url) = action_result.value);
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
            args.clone(),
            caller.clone(),
            ts.clone(),
            None,
        )
        .await?;
    must_let!(let ConvexValue::String(url) = query_result.result?);
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
            args.clone(),
            caller.clone(),
        )
        .await??;
    must_let!(let ConvexValue::String(url) = action_result.value);
    assert!(url.starts_with("https://carnitas.convex.cloud/api/storage/"));

    Ok(())
}
