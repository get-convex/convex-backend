use common::{
    http::RequestDestination,
    types::ModuleEnvironment,
};
use keybroker::Identity;
use model::{
    canonical_urls::CanonicalUrlsModel,
    config::types::{
        ConfigFile,
        ModuleConfig,
    },
    environment_variables::EnvironmentVariablesModel,
};
use openidconnect::IssuerUrl;
use runtime::testing::TestRuntime;
use udf::environment::system_env_var_overrides;

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
    let user_environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
    let system_env_var_overrides = system_env_var_overrides(&mut tx).await?;
    let config = Application::get_evaluated_auth_config(
        application.runner(),
        user_environment_variables,
        system_env_var_overrides,
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

#[convex_macro::test_runtime]
async fn test_evaluate_auth_config_has_custom_system_env_var(
    rt: TestRuntime,
) -> anyhow::Result<()> {
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
    CanonicalUrlsModel::new(&mut tx)
        .set_canonical_url(
            RequestDestination::ConvexSite,
            "https://xkcd.example.com".to_string(),
        )
        .await?;
    let user_environment_variables = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
    let system_env_var_overrides = system_env_var_overrides(&mut tx).await?;
    let config = Application::get_evaluated_auth_config(
        application.runner(),
        user_environment_variables,
        system_env_var_overrides,
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
        IssuerUrl::new("https://xkcd.example.com".to_string())?
    );
    Ok(())
}
