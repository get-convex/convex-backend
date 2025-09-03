use std::env;

use anyhow::{
    Context,
    Result,
};
use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use aws_utils::preflight_credentials;

#[tokio::main]
async fn main() -> Result<()> {
    // 1) Preflight: try to resolve credentials using the standard chain.
    let _creds = preflight_credentials().await?;

    println!(
        "✅ Credentials resolved{}",
        match env::var("AWS_PROFILE") {
            Ok(p) => format!(" (AWS_PROFILE={p})"),
            Err(_) => String::new(),
        }
    );

    // 2) Load full config explicitly setting profile if available
    let mut config_loader = aws_config::defaults(BehaviorVersion::latest());
    if let Ok(profile) = env::var("AWS_PROFILE") {
        config_loader = config_loader.profile_name(&profile);
    }
    let conf = config_loader.load().await;

    // 3) Use S3 client safely now that we know creds exist.
    let client = s3::Client::new(&conf);

    // Example: list buckets
    println!("Testing S3 access by listing buckets...");
    let resp = client.list_buckets().send().await.context(
        "S3 call failed (credentials may be invalid/expired or region/network misconfigured)",
    )?;

    println!("✅ S3 access successful!");
    println!("Buckets:");
    let buckets = resp.buckets();
    if buckets.is_empty() {
        println!(" (no buckets found)");
    } else {
        for b in buckets {
            println!(" - {}", b.name().unwrap_or("<unnamed>"));
        }
    }

    Ok(())
}
