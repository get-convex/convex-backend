// #![feature(coroutines)]
// #![feature(exit_status_error)]
use std::{
    env,
    sync::LazyLock,
};

use aws_config::{
    default_provider::credentials::DefaultCredentialsChain,
    BehaviorVersion,
    ConfigLoader,
};
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_types::region::Region;

pub mod s3;

static S3_ENDPOINT_URL: LazyLock<Option<String>> =
    LazyLock::new(|| env::var("S3_ENDPOINT_URL").ok());

static AWS_REGION: LazyLock<Option<String>> = LazyLock::new(|| env::var("AWS_REGION").ok());

static AWS_S3_FORCE_PATH_STYLE: LazyLock<bool> = LazyLock::new(|| {
    env::var("AWS_S3_FORCE_PATH_STYLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default()
});

/// Similar aws_config::from_env but returns an error if credentials or
/// region is are not. It also doesn't spew out log lines every time
/// credentials are accessed.
pub async fn must_config_from_env() -> anyhow::Result<ConfigLoader> {
    let Some(region) = AWS_REGION.clone() else {
        anyhow::bail!("AWS_REGION env variable must be set");
    };
    let region = Region::new(region);
    
    // Check for credentials using the default provider chain
    let _creds = preflight_credentials().await?;
    
    Ok(aws_config::defaults(BehaviorVersion::v2025_01_17())
        .region(region))
}

pub async fn must_s3_config_from_env() -> anyhow::Result<S3ConfigBuilder> {
    let base_config = must_config_from_env().await?.load().await;
    let mut s3_config_builder = S3ConfigBuilder::from(&base_config);
    if let Some(s3_endpoint_url) = S3_ENDPOINT_URL.clone() {
        s3_config_builder = s3_config_builder.endpoint_url(s3_endpoint_url);
    }
    s3_config_builder = s3_config_builder.force_path_style(*AWS_S3_FORCE_PATH_STYLE);
    Ok(s3_config_builder)
}

/// Attempts to resolve credentials using the default chain:
/// env vars -> shared config/credentials (incl. SSO) -> web identity -> container creds -> EC2 IMDSv2.
/// Returns early with a helpful error if nothing is available.
pub async fn preflight_credentials() -> anyhow::Result<aws_credential_types::Credentials> {
    let chain = DefaultCredentialsChain::builder().build().await;
    
    match chain.provide_credentials().await {
        Ok(creds) => Ok(creds),
        Err(err) => {
            // Give actionable hints based on common setups.
            let profile = env::var("AWS_PROFILE").unwrap_or_else(|_| "default".to_string());
            let mut help = String::new();
            help.push_str("No AWS credentials were found by the default provider chain.\n\n");
            help.push_str("Tried in this order:\n");
            help.push_str("  1) Environment: AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY [/ AWS_SESSION_TOKEN]\n");
            help.push_str("  2) Shared config/credentials files (~/.aws/config, ~/.aws/credentials) ");
            help.push_str(&format!("(profile: {})\n", profile));
            help.push_str("     - If you use IAM Identity Center (SSO), run: aws sso login");
            if profile != "default" { 
                help.push_str(&format!(" --profile {}", profile)); 
            }
            help.push_str("\n");
            help.push_str("  3) Web identity (AssumeRoleWithWebIdentity; env/profiles with role_arn & web_identity_token_file)\n");
            help.push_str("  4) Container credentials (ECS/EKS env: AWS_CONTAINER_CREDENTIALS_* or Pod Identity)\n");
            help.push_str("  5) EC2 Instance Metadata (IMDSv2; instance role)\n\n");

            help.push_str("Fixes:\n");
            help.push_str("  • For access keys: set AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, and AWS_SESSION_TOKEN (optional)\n");
            help.push_str("  • For profiles: set AWS_PROFILE or add a [profile] with credentials in ~/.aws/credentials\n");
            help.push_str("  • For SSO: aws configure sso && aws sso login\n");
            help.push_str("  • For web identity: ensure web_identity_token_file and role_arn are set\n");
            help.push_str("  • For containers/EC2: attach the proper task/IRSA/instance role\n");

            anyhow::bail!("{}Underlying error: {}", help, err)
        }
    }
}
