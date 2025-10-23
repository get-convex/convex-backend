use std::time::{
    Duration,
    Instant,
};

use reqwest::Url;

pub async fn health_check(service_url: &Url) -> anyhow::Result<Option<String>> {
    health_check_with_retries(
        service_url,
        None,
        None,
        "instance_version",
        1,
        Duration::ZERO,
    )
    .await
}

pub async fn wait_for_http_health(
    service_url: &Url,
    expected_version: Option<&str>,
    expected_instance_name: Option<&str>,
    num_retries: usize,
    sleep: Duration,
) -> anyhow::Result<String> {
    wait_for_http_health_inner(
        service_url,
        None,
        expected_version,
        expected_instance_name,
        "instance_version",
        num_retries,
        sleep,
    )
    .await
}

pub async fn wait_for_conductor_http_health(
    service_name: &str,
    service_url: &Url,
    expected_version: Option<&str>,
    num_retries: usize,
    sleep: Duration,
) -> anyhow::Result<String> {
    let version = wait_for_http_health_inner(
        service_url,
        Some(service_name),
        expected_version,
        None,
        "version",
        num_retries,
        sleep,
    )
    .await?;

    // Provide a better error message than parse if we hit a common default version.
    if version == "unknown" {
        anyhow::bail!(
            "Health check on {service_name} on url {service_url} failed with invalid version: \
             {version}"
        )
    }

    Ok(version)
}

async fn wait_for_http_health_inner(
    service_url: &Url,
    service_name: Option<&str>,
    expected_version: Option<&str>,
    expected_instance_name: Option<&str>,
    health_check_endpoint: &str,
    num_retries: usize,
    sleep: Duration,
) -> anyhow::Result<String> {
    let start = Instant::now();
    let service_name = service_name
        .map(|n| format!(" ({n}) "))
        .unwrap_or_else(|| "".into());

    tracing::info!("Waiting for health to {service_url}{service_name}");
    match health_check_with_retries(
        service_url,
        expected_version,
        expected_instance_name,
        health_check_endpoint,
        num_retries,
        sleep,
    )
    .await?
    {
        Some(version) => {
            tracing::info!(
                "{service_url} healthy at version:{version} after {:?}",
                start.elapsed()
            );
            Ok(version)
        },
        None => anyhow::bail!(
            "Timed out waiting for service health: {service_url}{service_name} after {:?}",
            start.elapsed()
        ),
    }
}

async fn health_check_with_retries(
    service_url: &Url,
    expected_version: Option<&str>,
    expected_instance_name: Option<&str>,
    health_check_endpoint: &str,
    num_retries: usize,
    sleep: Duration,
) -> anyhow::Result<Option<String>> {
    let mut last_error = None;
    for i in 0..=num_retries {
        if i != 0 {
            tokio::time::sleep(sleep).await;
        }
        match health_check_once(service_url, expected_version, health_check_endpoint).await {
            Ok(version) => {
                if let Some(expected_instance_name) = expected_instance_name
                    && let Err(e) = verify_instance_name(service_url, expected_instance_name).await
                {
                    last_error = Some(e);
                    continue;
                }
                return Ok(Some(version));
            },
            Err(e) => last_error = Some(e),
        }
    }
    tracing::error!("{:?}", last_error.unwrap());
    Ok(None)
}

async fn health_check_once(
    service_url: &Url,
    expected_version: Option<&str>,
    health_check_endpoint: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let health_url = service_url.join(health_check_endpoint)?;
    let response = client
        .get(health_url.clone())
        .timeout(Duration::from_secs(15))
        .send()
        .await;

    match response {
        Ok(r) => {
            if r.status().is_success() {
                let version = r.text().await?;
                if let Some(expected_version) = expected_version {
                    anyhow::ensure!(expected_version == &version);
                }

                Ok(version)
            } else {
                anyhow::bail!("Health check on {service_url} failed: {r:?}");
            }
        },
        Err(e) => anyhow::bail!("Health check on {service_url} failed: {e:?}"),
    }
}

pub async fn verify_instance_name(service_url: &Url, instance_name: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let instance_name_url = service_url.join("instance_name")?;
    let response = client
        .get(instance_name_url)
        .header("x-convex-instance", instance_name)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    anyhow::ensure!(
        response.status().is_success(),
        "Health check on {service_url} failed: {response:?}"
    );
    let response_instance_name = response.text().await?;
    anyhow::ensure!(
        response_instance_name == instance_name,
        "Health checking {service_url}. Expected {instance_name}. Got {response_instance_name}."
    );
    Ok(())
}
