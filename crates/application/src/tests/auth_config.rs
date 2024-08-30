use common::types::ModuleEnvironment;
use keybroker::Identity;
use model::{
    config::types::{
        ConfigFile,
        ModuleConfig,
    },
    environment_variables::EnvironmentVariablesModel,
};
use openidconnect::IssuerUrl;
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
    let environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
    let config = Application::get_evaluated_auth_config(
        application.runner(),
        environment_variables,
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
    .unwrap();
    assert_eq!(
        config[0].domain,
        IssuerUrl::new("http://127.0.0.1:8001".to_string())?
    );
    Ok(())
}
