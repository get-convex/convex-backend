//! Helpers for the v3 "sidecar" provisioning strategy. Each deployment
//! owns its own Postgres + MinIO container alongside the backend; this
//! module knows how to spawn them, wait for them to be ready, and create
//! the 5 buckets the backend's `--s3-storage` mode expects.
//!
//! All container/volume names are derived from the deployment name +
//! orchestrator's `container_prefix`, so callers can stay name-agnostic.

use rand::{
    distr::Alphanumeric,
    Rng,
};
use tokio::process::Command;
use tokio::time::{
    sleep,
    Duration,
};

/// 5 buckets per deployment, matching the backend's `--s3-storage` env
/// var names. Kept as constants so `compose_s3_env` and `create_buckets`
/// stay in lockstep.
pub const BUCKET_NAMES: [&str; 5] = [
    "convex-exports",
    "convex-snapshot-imports",
    "convex-modules",
    "convex-files",
    "convex-search",
];

pub fn pg_container_name(container_prefix: &str, deployment_name: &str) -> String {
    format!("{container_prefix}pg-{deployment_name}")
}

pub fn pg_volume_name(container_prefix: &str, deployment_name: &str) -> String {
    format!("{container_prefix}pgdata-{deployment_name}")
}

pub fn minio_container_name(container_prefix: &str, deployment_name: &str) -> String {
    format!("{container_prefix}minio-{deployment_name}")
}

pub fn minio_volume_name(container_prefix: &str, deployment_name: &str) -> String {
    format!("{container_prefix}miniodata-{deployment_name}")
}

pub fn generate_password() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(|c| c as char)
        .collect()
}

/// Build the POSTGRES_URL the backend uses to reach its sidecar Postgres.
/// Always SSL-disabled — the sidecar is reachable only via the orchestrator's
/// docker network, never over a public path.
///
/// The URL deliberately has no database in the path. The backend's
/// `--db postgres-v5` driver (see `crates/clusters/src/lib.rs`) appends the
/// per-deployment database name itself, derived from the deployment name
/// with hyphens replaced by underscores. We create that database via
/// [`create_postgres_database`] right after the sidecar comes up.
pub fn compose_postgres_url(pg_container: &str, password: &str) -> String {
    format!("postgres://convex:{password}@{pg_container}:5432/?sslmode=disable")
}

/// Derive the per-deployment database name from the deployment name.
/// Mirrors the backend's `deployment_name.replace('-', "_")` convention
/// in `crates/clusters/src/lib.rs`.
pub fn postgres_db_name(deployment_name: &str) -> String {
    deployment_name.replace('-', "_")
}

/// Create the per-deployment Postgres database via `docker exec psql`.
/// Idempotent: tolerates "already exists" (Postgres error code 42P04).
/// Connects to the bootstrap `convex` database (created by the
/// `POSTGRES_DB=convex` env var on the sidecar container) and issues a
/// `CREATE DATABASE` against it.
pub async fn create_postgres_database(
    container_name: &str,
    db_name: &str,
) -> anyhow::Result<()> {
    // db_name is derived from the deployment slug (alphanumeric + `_`
    // after `replace('-', "_")`) so identifier-quoting is safe here.
    let sql = format!(r#"CREATE DATABASE "{db_name}""#);
    let output = Command::new("docker")
        .args([
            "exec",
            container_name,
            "psql",
            "-U",
            "convex",
            "-d",
            "convex",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &sql,
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker exec psql failed: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("already exists") {
        return Ok(());
    }
    anyhow::bail!(
        "create database {db_name} on {container_name} failed: {}",
        stderr.trim()
    );
}

/// The 5 backend env vars that point file storage at the sidecar MinIO,
/// plus AWS_REGION + endpoint + creds. Returned as `(KEY, VALUE)` pairs
/// in the order the docker-run flags get emitted.
pub fn compose_s3_env(
    minio_container: &str,
    root_user: &str,
    root_password: &str,
) -> Vec<(&'static str, String)> {
    let endpoint = format!("http://{minio_container}:9000");
    vec![
        ("AWS_REGION", "us-east-1".into()),
        ("AWS_ENDPOINT_URL_S3", endpoint),
        // MinIO doesn't do virtual-hosted-style addressing — the AWS SDK
        // would otherwise resolve `<bucket>.<minio-container>:9000`, which
        // has no docker DNS entry. The backend's aws_utils crate honors
        // this env var (see crates/aws_utils/src/lib.rs).
        ("AWS_S3_FORCE_PATH_STYLE", "true".into()),
        ("AWS_ACCESS_KEY_ID", root_user.to_string()),
        ("AWS_SECRET_ACCESS_KEY", root_password.to_string()),
        ("S3_STORAGE_EXPORTS_BUCKET", BUCKET_NAMES[0].into()),
        ("S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET", BUCKET_NAMES[1].into()),
        ("S3_STORAGE_MODULES_BUCKET", BUCKET_NAMES[2].into()),
        ("S3_STORAGE_FILES_BUCKET", BUCKET_NAMES[3].into()),
        ("S3_STORAGE_SEARCH_BUCKET", BUCKET_NAMES[4].into()),
    ]
}

/// Spawn the Postgres sidecar. Idempotent on the container — if a
/// container with this name already exists, this is a no-op success.
pub async fn spawn_postgres_sidecar(
    container_name: &str,
    volume_name: &str,
    network: Option<&str>,
    image: &str,
    password: &str,
    max_connections: u32,
) -> anyhow::Result<()> {
    if container_exists(container_name).await? {
        return Ok(());
    }
    ensure_volume(volume_name).await?;
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--restart".into(),
        "unless-stopped".into(),
        "--name".into(),
        container_name.into(),
        "-v".into(),
        format!("{volume_name}:/var/lib/postgresql/data"),
        "-e".into(),
        "POSTGRES_USER=convex".into(),
        "-e".into(),
        "POSTGRES_DB=convex".into(),
        "-e".into(),
        format!("POSTGRES_PASSWORD={password}"),
    ];
    if let Some(net) = network
        && !net.is_empty()
    {
        args.push("--network".into());
        args.push(net.into());
    }
    args.push(image.into());
    // Server-side max_connections must match the backend's pool ceiling.
    args.push("-c".into());
    args.push(format!("max_connections={max_connections}"));
    run_docker(&args, "spawn postgres sidecar").await?;
    Ok(())
}

/// Spawn the MinIO sidecar. Idempotent on the container.
pub async fn spawn_minio_sidecar(
    container_name: &str,
    volume_name: &str,
    network: Option<&str>,
    image: &str,
    root_user: &str,
    root_password: &str,
) -> anyhow::Result<()> {
    if container_exists(container_name).await? {
        return Ok(());
    }
    ensure_volume(volume_name).await?;
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--restart".into(),
        "unless-stopped".into(),
        "--name".into(),
        container_name.into(),
        "-v".into(),
        format!("{volume_name}:/data"),
        "-e".into(),
        format!("MINIO_ROOT_USER={root_user}"),
        "-e".into(),
        format!("MINIO_ROOT_PASSWORD={root_password}"),
    ];
    if let Some(net) = network
        && !net.is_empty()
    {
        args.push("--network".into());
        args.push(net.into());
    }
    args.push(image.into());
    args.push("server".into());
    args.push("/data".into());
    run_docker(&args, "spawn minio sidecar").await?;
    Ok(())
}

/// Poll the Postgres sidecar via `docker exec pg_isready` until ready
/// or budget exhausted (~60s — `initdb` on slow disks can take 20s
/// before pg_isready returns OK).
pub async fn wait_for_postgres(container_name: &str) -> anyhow::Result<()> {
    let mut last_stderr = String::new();
    for attempt in 0..60 {
        if attempt > 0 {
            sleep(Duration::from_secs(1)).await;
        }
        let output = Command::new("docker")
            .args([
                "exec",
                container_name,
                "pg_isready",
                "-U",
                "convex",
                "-d",
                "convex",
            ])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("docker exec pg_isready failed: {e}"))?;
        if output.status.success() {
            return Ok(());
        }
        last_stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    anyhow::bail!(
        "postgres sidecar {container_name} never became ready after 60s; last stderr: \
         {last_stderr}"
    );
}

/// Poll MinIO's `/minio/health/live` endpoint via `docker exec curl`.
pub async fn wait_for_minio(container_name: &str) -> anyhow::Result<()> {
    let mut last_stderr = String::new();
    for attempt in 0..60 {
        if attempt > 0 {
            sleep(Duration::from_secs(1)).await;
        }
        let output = Command::new("docker")
            .args([
                "exec",
                container_name,
                "curl",
                "-fsS",
                "-o",
                "/dev/null",
                "http://127.0.0.1:9000/minio/health/live",
            ])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("docker exec curl failed: {e}"))?;
        if output.status.success() {
            return Ok(());
        }
        last_stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    anyhow::bail!(
        "minio sidecar {container_name} never became ready after 60s; last stderr: {last_stderr}"
    );
}

/// Create the 5 buckets via a one-shot `minio/mc` container.
pub async fn create_minio_buckets(
    minio_container: &str,
    network: Option<&str>,
    root_user: &str,
    root_password: &str,
) -> anyhow::Result<()> {
    let alias = "orch";
    let mut args: Vec<String> = vec![
        "run".into(),
        "--rm".into(),
        "-e".into(),
        format!("MC_HOST_{alias}=http://{root_user}:{root_password}@{minio_container}:9000"),
    ];
    if let Some(net) = network
        && !net.is_empty()
    {
        args.push("--network".into());
        args.push(net.into());
    }
    args.push("minio/mc:latest".into());
    args.push("mb".into());
    args.push("--ignore-existing".into());
    for bucket in BUCKET_NAMES {
        args.push(format!("{alias}/{bucket}"));
    }
    run_docker(&args, "create minio buckets").await?;
    Ok(())
}

async fn ensure_volume(name: &str) -> anyhow::Result<()> {
    let output = Command::new("docker")
        .args(["volume", "create", name])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker volume create {name}: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "docker volume create {name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

async fn container_exists(name: &str) -> anyhow::Result<bool> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name=^{name}$"),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker ps: {e}"))?;
    if !output.status.success() {
        return Ok(false);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|l| l == name))
}

async fn run_docker(args: &[String], desc: &str) -> anyhow::Result<()> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker invocation failed during {desc}: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "docker {desc} failed (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Best-effort sidecar teardown for a deployment. Removes pg+minio
/// containers and their data volumes. Non-zero exits / invocation failures
/// are logged via `tracing::warn!` (matching `DockerProvisioner::teardown`)
/// but never bubbled up — teardown is best-effort.
pub async fn teardown_sidecars(container_prefix: &str, deployment_name: &str) {
    for container in [
        pg_container_name(container_prefix, deployment_name),
        minio_container_name(container_prefix, deployment_name),
    ] {
        match Command::new("docker")
            .args(["rm", "-f", &container])
            .output()
            .await
        {
            Ok(output) if !output.status.success() => {
                tracing::warn!(
                    container = %container,
                    stderr = %String::from_utf8_lossy(&output.stderr).trim(),
                    "docker rm reported non-zero exit during sidecar teardown; continuing",
                );
            },
            Err(e) => {
                tracing::warn!(
                    container = %container,
                    error = %e,
                    "docker rm invocation failed during sidecar teardown; continuing",
                );
            },
            _ => {},
        }
    }
    for volume in [
        pg_volume_name(container_prefix, deployment_name),
        minio_volume_name(container_prefix, deployment_name),
    ] {
        match Command::new("docker")
            .args(["volume", "rm", &volume])
            .output()
            .await
        {
            Ok(output) if !output.status.success() => {
                tracing::warn!(
                    volume = %volume,
                    stderr = %String::from_utf8_lossy(&output.stderr).trim(),
                    "docker volume rm reported non-zero exit during sidecar teardown; continuing",
                );
            },
            Err(e) => {
                tracing::warn!(
                    volume = %volume,
                    error = %e,
                    "docker volume rm invocation failed during sidecar teardown; continuing",
                );
            },
            _ => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn pg_url_has_no_db_in_path_and_no_sslmode_require() {
        // Backend's --db postgres-v5 driver appends the per-deployment DB
        // name itself (deployment_name with `-` → `_`) and rejects URLs
        // whose path is anything but "" or "/" — so the URL must end at
        // `/?sslmode=disable`, not `/convex?sslmode=disable`.
        let url = compose_postgres_url("orchestratorpg-foo", "abc123");
        assert_eq!(
            url,
            "postgres://convex:abc123@orchestratorpg-foo:5432/?sslmode=disable"
        );
    }

    #[test]
    fn postgres_db_name_matches_backend_convention() {
        // Mirrors `deployment_name.replace('-', "_")` in
        // crates/clusters/src/lib.rs so the DB we create is the one the
        // backend tries to connect to.
        assert_eq!(postgres_db_name("spry-fox"), "spry_fox");
        assert_eq!(postgres_db_name("noslug"), "noslug");
        assert_eq!(postgres_db_name("a-b-c"), "a_b_c");
    }

    #[test]
    fn s3_env_keys_match_backend_expectations() {
        let env = compose_s3_env("minio-foo", "user", "pass");
        let map: HashMap<&str, String> = env.into_iter().collect();
        assert_eq!(map["AWS_REGION"], "us-east-1");
        assert_eq!(map["AWS_ENDPOINT_URL_S3"], "http://minio-foo:9000");
        assert_eq!(map["AWS_S3_FORCE_PATH_STYLE"], "true");
        assert_eq!(map["AWS_ACCESS_KEY_ID"], "user");
        assert_eq!(map["AWS_SECRET_ACCESS_KEY"], "pass");
        assert_eq!(map["S3_STORAGE_EXPORTS_BUCKET"], "convex-exports");
        assert_eq!(
            map["S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET"],
            "convex-snapshot-imports"
        );
        assert_eq!(map["S3_STORAGE_MODULES_BUCKET"], "convex-modules");
        assert_eq!(map["S3_STORAGE_FILES_BUCKET"], "convex-files");
        assert_eq!(map["S3_STORAGE_SEARCH_BUCKET"], "convex-search");
    }

    #[test]
    fn names_are_namespaced_by_container_prefix() {
        assert_eq!(pg_container_name("orch-", "spry-fox"), "orch-pg-spry-fox");
        assert_eq!(pg_volume_name("orch-", "spry-fox"), "orch-pgdata-spry-fox");
        assert_eq!(
            minio_container_name("orch-", "spry-fox"),
            "orch-minio-spry-fox"
        );
        assert_eq!(
            minio_volume_name("orch-", "spry-fox"),
            "orch-miniodata-spry-fox"
        );
    }

    #[test]
    fn generate_password_is_32_alphanumeric() {
        let p = generate_password();
        assert_eq!(p.len(), 32);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
        let p2 = generate_password();
        assert_ne!(
            p, p2,
            "two generate_password() calls produced the same value — RNG may be seeded"
        );
    }
}
