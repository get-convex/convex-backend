use common::runtime::Runtime;
use errors::ErrorMetadataAnyhowExt;
use runtime::testing::TestRuntime;
use sync_types::AuthenticationToken;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_auth_with_invalid_admin_key(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;

    let bad_admin_key = "convex-self-hosted|01a87dc926f9d4f8ee91242ef59ccc2b2a768bde199fab5e3ffe7c83e30265c1f6f54a0ec4f1257cfe741a06dd8413a44d";
    let bad_token = AuthenticationToken::Admin(bad_admin_key.to_string(), None);

    let authenticate_result = application.authenticate(bad_token, rt.system_time()).await;
    let error = authenticate_result.unwrap_err();
    assert!(error.is_unauthenticated());
    assert!(error
        .to_string()
        .contains("The provided admin key was invalid for this instance"));

    Ok(())
}
