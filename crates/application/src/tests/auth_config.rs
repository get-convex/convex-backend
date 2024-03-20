use common::types::ModuleEnvironment;
use keybroker::Identity;
use model::config::types::{
    ConfigFile,
    ModuleConfig,
};
use runtime::testing::TestRuntime;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_evaluate_auth_config_has_system_env_var(rt: TestRuntime) -> anyhow::Result<()> {
    let source = r#"
export default {
    providers: [
        {
            domain: process.env.CONVEX_SITE_URL,
            applicationID: "convex",
        },
    ],
    };
"#;
    let application = Application::new_for_tests(&rt).await?;
    let mut tx = application.begin(Identity::system()).await?;
    let error = Application::get_evaluated_auth_config(
        application.runner(),
        &mut tx,
        Some(ModuleConfig {
            path: "auth.config.js".parse().unwrap(),
            source: source.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        }),
        &ConfigFile {
            functions: "convex".to_owned(),
            auth_info: None,
        },
    )
    .await
    .unwrap_err();
    // The config will fail because the CONVEX_SITE_URL will be an http url,
    // but this is ok outside of tests because it will be https there.
    assert!(format!("{}", error).contains("Invalid provider domain URL"),);
    assert!(format!("{}", error).contains("must use HTTPS"),);
    Ok(())
}
