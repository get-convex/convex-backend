use common::{
    components::ComponentId,
    types::BackendState,
};
use errors::ErrorMetadataAnyhowExt;
use futures::stream;
use keybroker::Identity;
use model::backend_state::BackendStateModel;
use runtime::testing::TestRuntime;

use crate::{
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
