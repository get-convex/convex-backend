#![feature(coroutines)]
#![feature(exit_status_error)]
use std::{
    env,
    sync::LazyLock,
};

use aws_config::{
    environment::credentials::EnvironmentVariableCredentialsProvider,
    BehaviorVersion,
    ConfigLoader,
};
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_types::region::Region;

pub mod s3;

static S3_ENDPOINT_URL: LazyLock<Option<String>> =
    LazyLock::new(|| env::var("S3_ENDPOINT_URL").ok());

static AWS_ACCESS_KEY_ID: LazyLock<Option<String>> =
    LazyLock::new(|| env::var("AWS_ACCESS_KEY_ID").ok());

static AWS_SECRET_ACCESS_KEY: LazyLock<Option<String>> =
    LazyLock::new(|| env::var("AWS_SECRET_ACCESS_KEY").ok());

static AWS_REGION: LazyLock<Option<String>> = LazyLock::new(|| env::var("AWS_REGION").ok());

static AWS_S3_FORCE_PATH_STYLE: LazyLock<bool> = LazyLock::new(|| {
    env::var("AWS_S3_FORCE_PATH_STYLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default()
});

static AWS_S3_DISABLE_SSE: LazyLock<bool> = LazyLock::new(|| {
    env::var("AWS_S3_DISABLE_SSE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default()
});

static AWS_S3_DISABLE_CHECKSUMS: LazyLock<bool> = LazyLock::new(|| {
    env::var("AWS_S3_DISABLE_CHECKSUMS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default()
});

/// Similar aws_config::from_env but returns an error if credentials or
/// region is are not. It also doesn't spew out log lines every time
/// credentials are accessed.
pub fn must_config_from_env() -> anyhow::Result<ConfigLoader> {
    let Some(region) = AWS_REGION.clone() else {
        anyhow::bail!("AWS_REGION env variable must be set");
    };
    let region = Region::new(region);
    let Some(_) = AWS_ACCESS_KEY_ID.clone() else {
        anyhow::bail!("AWS_ACCESS_KEY_ID env variable must be set");
    };
    let Some(_) = AWS_SECRET_ACCESS_KEY.clone() else {
        anyhow::bail!("AWS_SECRET_ACCESS_KEY env variable must be set");
    };
    let credentials = EnvironmentVariableCredentialsProvider::new();
    Ok(aws_config::defaults(BehaviorVersion::v2025_01_17())
        .region(region)
        .credentials_provider(credentials))
}

pub async fn must_s3_config_from_env() -> anyhow::Result<S3ConfigBuilder> {
    let base_config = must_config_from_env()?.load().await;
    let mut s3_config_builder = S3ConfigBuilder::from(&base_config);
    if let Some(s3_endpoint_url) = S3_ENDPOINT_URL.clone() {
        s3_config_builder = s3_config_builder.endpoint_url(s3_endpoint_url);
    }
    s3_config_builder = s3_config_builder.force_path_style(*AWS_S3_FORCE_PATH_STYLE);
    Ok(s3_config_builder)
}

/// Returns true if server-side encryption headers should be disabled
pub fn is_sse_disabled() -> bool {
    *AWS_S3_DISABLE_SSE
}

/// Returns true if checksum headers should be disabled
pub fn are_checksums_disabled() -> bool {
    *AWS_S3_DISABLE_CHECKSUMS
}
