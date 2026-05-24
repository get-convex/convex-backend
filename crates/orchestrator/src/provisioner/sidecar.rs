//! Helpers for the v3 "sidecar" provisioning strategy. Each deployment
//! owns its own Postgres + MinIO container alongside the backend; this
//! module knows how to spawn them, wait for them to be ready, and create
//! the 5 buckets the backend's `--s3-storage` mode expects.
//!
//! All container/volume names are derived from the deployment name +
//! orchestrator's `container_prefix`, so callers can stay name-agnostic.

use rand::{distr::Alphanumeric, Rng};
use tokio::process::Command;
use tokio::time::{sleep, Duration};

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

const POSTGRES_UNBOUNDED_TUNING_MEMORY_MB: u32 = 32 * 1024;
const MIN_POSTGRES_MEMORY_MB: u32 = 512;
const MIN_POSTGRES_CPUS: f32 = 1.0;
const MIN_MINIO_MEMORY_MB: u32 = 256;
const MIN_MINIO_CPUS: f32 = 0.5;
const BYTES_PER_MB: u64 = 1024 * 1024;
const NANO_CPUS_PER_CPU: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidecarContainerResources {
    pub memory_mb: u32,
    pub cpus: f32,
    pub unbounded: bool,
}

impl SidecarContainerResources {
    fn desired_memory_bytes(self) -> u64 {
        if self.unbounded {
            0
        } else {
            self.memory_mb as u64 * BYTES_PER_MB
        }
    }

    fn desired_memory_swap_bytes(self) -> i64 {
        if self.unbounded {
            0
        } else {
            (self.desired_memory_bytes() * 2) as i64
        }
    }

    fn desired_nano_cpus(self) -> u64 {
        if self.unbounded {
            0
        } else {
            (self.cpus as f64 * NANO_CPUS_PER_CPU).round() as u64
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidecarResources {
    pub postgres: SidecarContainerResources,
    pub minio: SidecarContainerResources,
}

impl SidecarResources {
    pub fn for_tier(tier: &crate::provisioner::tiers::Tier) -> Self {
        if tier.unbounded {
            let unbounded = SidecarContainerResources {
                memory_mb: 0,
                cpus: 0.0,
                unbounded: true,
            };
            return Self {
                postgres: unbounded,
                minio: unbounded,
            };
        }

        // Sidecars share the host with the backend, but the orchestrator allows
        // overprovisioning. Give Postgres enough headroom to avoid becoming the
        // first limiter on larger tiers; Docker caps are limits, not reserved
        // capacity.
        Self {
            postgres: SidecarContainerResources {
                memory_mb: tier.memory_mb.max(MIN_POSTGRES_MEMORY_MB),
                cpus: tier.cpus.max(MIN_POSTGRES_CPUS),
                unbounded: false,
            },
            minio: SidecarContainerResources {
                memory_mb: (tier.memory_mb / 2).max(MIN_MINIO_MEMORY_MB),
                cpus: (tier.cpus / 2.0).max(MIN_MINIO_CPUS),
                unbounded: false,
            },
        }
    }
}

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
pub async fn create_postgres_database(container_name: &str, db_name: &str) -> anyhow::Result<()> {
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
        // MinIO doesn't have KMS configured by default; if the backend
        // emits the `x-amz-server-side-encryption: aws:kms` header MinIO
        // rejects the request with `NotImplemented`. Disabling SSE makes
        // the backend skip the encryption headers (aws_utils::is_sse_disabled).
        ("AWS_S3_DISABLE_SSE", "true".into()),
        // MinIO is conservative about S3 checksum trailers; disabling
        // them avoids signature mismatches on multipart uploads.
        ("AWS_S3_DISABLE_CHECKSUMS", "true".into()),
        ("AWS_ACCESS_KEY_ID", root_user.to_string()),
        ("AWS_SECRET_ACCESS_KEY", root_password.to_string()),
        ("S3_STORAGE_EXPORTS_BUCKET", BUCKET_NAMES[0].into()),
        ("S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET", BUCKET_NAMES[1].into()),
        ("S3_STORAGE_MODULES_BUCKET", BUCKET_NAMES[2].into()),
        ("S3_STORAGE_FILES_BUCKET", BUCKET_NAMES[3].into()),
        ("S3_STORAGE_SEARCH_BUCKET", BUCKET_NAMES[4].into()),
    ]
}

fn push_resource_args(args: &mut Vec<String>, resources: SidecarContainerResources) {
    if resources.unbounded {
        return;
    }
    args.push("--memory".into());
    args.push(format!("{}m", resources.memory_mb));
    args.push("--memory-swap".into());
    args.push(resources.desired_memory_swap_bytes().to_string());
    args.push("--cpus".into());
    args.push(format!("{:.2}", resources.cpus));
}

fn format_pg_memory(mb: u32) -> String {
    if mb >= 1024 && mb % 1024 == 0 {
        format!("{}GB", mb / 1024)
    } else {
        format!("{mb}MB")
    }
}

fn postgres_tuning_memory_mb(resources: SidecarContainerResources) -> u32 {
    if resources.unbounded || resources.memory_mb == 0 {
        POSTGRES_UNBOUNDED_TUNING_MEMORY_MB
    } else {
        resources.memory_mb
    }
}

fn postgres_settings(
    max_connections: u32,
    resources: SidecarContainerResources,
) -> Vec<(&'static str, String)> {
    let memory_mb = postgres_tuning_memory_mb(resources);
    let shared_buffers_mb = (memory_mb / 4).clamp(128, 8192);
    let effective_cache_min_mb = memory_mb.min(1024);
    let effective_cache_size_mb = ((memory_mb * 3) / 4).clamp(effective_cache_min_mb, 24576);
    let per_connection_work_mem_mb =
        (memory_mb as u64 / max_connections.max(1) as u64 / 16) as u32;
    let work_mem_mb = per_connection_work_mem_mb.clamp(4, 64);
    let maintenance_work_mem_mb = (memory_mb / 16).clamp(64, 2048);
    let wal_buffers_mb = (shared_buffers_mb / 32).clamp(16, 256);
    let max_wal_size_mb = (memory_mb / 4).clamp(1024, 8192);
    let min_wal_size_mb = (max_wal_size_mb / 16).clamp(80, 1024);

    vec![
        ("max_connections", max_connections.to_string()),
        ("shared_buffers", format_pg_memory(shared_buffers_mb)),
        (
            "effective_cache_size",
            format_pg_memory(effective_cache_size_mb),
        ),
        ("work_mem", format_pg_memory(work_mem_mb)),
        (
            "maintenance_work_mem",
            format_pg_memory(maintenance_work_mem_mb),
        ),
        ("random_page_cost", "1.1".into()),
        ("effective_io_concurrency", "200".into()),
        ("wal_buffers", format_pg_memory(wal_buffers_mb)),
        ("checkpoint_completion_target", "0.9".into()),
        ("max_wal_size", format_pg_memory(max_wal_size_mb)),
        ("min_wal_size", format_pg_memory(min_wal_size_mb)),
    ]
}

fn postgres_start_command(
    max_connections: u32,
    resources: SidecarContainerResources,
) -> Vec<String> {
    let mut command = Vec::new();
    for (key, value) in postgres_settings(max_connections, resources) {
        command.push("-c".into());
        command.push(format!("{key}={value}"));
    }
    command
}

fn minio_start_command() -> Vec<String> {
    vec!["server".into(), "/data".into()]
}

fn build_postgres_run_args(
    container_name: &str,
    volume_name: &str,
    network: Option<&str>,
    image: &str,
    password: &str,
    max_connections: u32,
    resources: SidecarContainerResources,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--restart".into(),
        "unless-stopped".into(),
        // Postgres needs extra fds for backends + workers + WAL. Default
        // 1024 nofile is too tight once large tiers open big pools.
        "--ulimit".into(),
        "nofile=1048576:1048576".into(),
    ];
    push_resource_args(&mut args, resources);
    args.extend([
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
    ]);
    if let Some(net) = network
        && !net.is_empty()
    {
        args.push("--network".into());
        args.push(net.into());
    }
    args.push(image.into());
    // Server-side max_connections must match the backend's pool ceiling.
    args.extend(postgres_start_command(max_connections, resources));
    args
}

fn build_minio_run_args(
    container_name: &str,
    volume_name: &str,
    network: Option<&str>,
    image: &str,
    root_user: &str,
    root_password: &str,
    resources: SidecarContainerResources,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--restart".into(),
        "unless-stopped".into(),
        // MinIO opens an fd per concurrent multipart upload + per
        // connection from the backend's S3 client. Match the rest of
        // the stack so MinIO isn't the next bottleneck.
        "--ulimit".into(),
        "nofile=1048576:1048576".into(),
    ];
    push_resource_args(&mut args, resources);
    args.extend([
        "--name".into(),
        container_name.into(),
        "-v".into(),
        format!("{volume_name}:/data"),
        "-e".into(),
        format!("MINIO_ROOT_USER={root_user}"),
        "-e".into(),
        format!("MINIO_ROOT_PASSWORD={root_password}"),
    ]);
    if let Some(net) = network
        && !net.is_empty()
    {
        args.push("--network".into());
        args.push(net.into());
    }
    args.push(image.into());
    args.extend(minio_start_command());
    args
}

/// Spawn or update the Postgres sidecar while preserving its data volume.
pub async fn spawn_postgres_sidecar(
    container_name: &str,
    volume_name: &str,
    network: Option<&str>,
    image: &str,
    password: &str,
    max_connections: u32,
    resources: SidecarContainerResources,
) -> anyhow::Result<()> {
    if container_exists(container_name).await? {
        let desired_command = postgres_start_command(max_connections, resources);
        if container_image_matches(container_name, image).await?
            && container_command_matches(container_name, &desired_command).await?
        {
            if container_resources_match(container_name, resources).await? {
                return Ok(());
            }
            if !resources.unbounded {
                update_container_resources(container_name, resources).await?;
                return Ok(());
            }
            remove_container(container_name, "replace postgres sidecar resources").await?;
        } else {
            remove_container(container_name, "replace postgres sidecar").await?;
        }
    }
    ensure_volume(volume_name).await?;
    let args = build_postgres_run_args(
        container_name,
        volume_name,
        network,
        image,
        password,
        max_connections,
        resources,
    );
    run_docker(&args, "spawn postgres sidecar").await?;
    Ok(())
}

/// Spawn or update the MinIO sidecar while preserving its data volume.
pub async fn spawn_minio_sidecar(
    container_name: &str,
    volume_name: &str,
    network: Option<&str>,
    image: &str,
    root_user: &str,
    root_password: &str,
    resources: SidecarContainerResources,
) -> anyhow::Result<()> {
    if container_exists(container_name).await? {
        let desired_command = minio_start_command();
        if container_image_matches(container_name, image).await?
            && container_command_matches(container_name, &desired_command).await?
        {
            if container_resources_match(container_name, resources).await? {
                return Ok(());
            }
            if !resources.unbounded {
                update_container_resources(container_name, resources).await?;
                return Ok(());
            }
            remove_container(container_name, "replace minio sidecar resources").await?;
        } else {
            remove_container(container_name, "replace minio sidecar").await?;
        }
    }
    ensure_volume(volume_name).await?;
    let args = build_minio_run_args(
        container_name,
        volume_name,
        network,
        image,
        root_user,
        root_password,
        resources,
    );
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

async fn container_image_matches(container_name: &str, image: &str) -> anyhow::Result<bool> {
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.Config.Image}}", container_name])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker inspect image {container_name}: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "docker inspect image {container_name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim() == image)
}

async fn container_command_matches(
    container_name: &str,
    desired_command: &[String],
) -> anyhow::Result<bool> {
    let output = Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{json .Config.Cmd}}",
            container_name,
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker inspect command {container_name}: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "docker inspect command {container_name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let current: Vec<String> = serde_json::from_slice(&output.stdout).map_err(|e| {
        anyhow::anyhow!(
            "parsing docker command for {container_name} failed: {e}; stdout: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        )
    })?;
    Ok(current == desired_command)
}

async fn container_resources_match(
    container_name: &str,
    resources: SidecarContainerResources,
) -> anyhow::Result<bool> {
    let output = Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{.HostConfig.Memory}} {{.HostConfig.MemorySwap}} {{.HostConfig.NanoCpus}}",
            container_name,
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker inspect resources {container_name}: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "docker inspect resources {container_name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut fields = stdout.split_whitespace();
    let memory = fields
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| anyhow::anyhow!("docker inspect resources missing memory"))?;
    let memory_swap = fields
        .next()
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or_else(|| anyhow::anyhow!("docker inspect resources missing memory swap"))?;
    let nano_cpus = fields
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| anyhow::anyhow!("docker inspect resources missing nano cpus"))?;
    Ok(memory == resources.desired_memory_bytes()
        && memory_swap == resources.desired_memory_swap_bytes()
        && nano_cpus == resources.desired_nano_cpus())
}

fn build_update_container_resource_args(
    container_name: &str,
    resources: SidecarContainerResources,
) -> Vec<String> {
    let mut args = vec!["update".into()];
    args.push("--memory".into());
    args.push(resources.desired_memory_bytes().to_string());
    args.push("--memory-swap".into());
    args.push(resources.desired_memory_swap_bytes().to_string());
    args.push("--cpus".into());
    args.push(format!("{:.2}", resources.desired_nano_cpus() as f64 / NANO_CPUS_PER_CPU));
    args.push(container_name.into());
    args
}

async fn update_container_resources(
    container_name: &str,
    resources: SidecarContainerResources,
) -> anyhow::Result<()> {
    let args = build_update_container_resource_args(container_name, resources);
    run_docker(&args, "update sidecar resources").await
}

async fn remove_container(container_name: &str, desc: &str) -> anyhow::Result<()> {
    let args = vec!["rm".into(), "-f".into(), container_name.into()];
    run_docker(&args, desc).await
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
        assert_eq!(map["AWS_S3_DISABLE_SSE"], "true");
        assert_eq!(map["AWS_S3_DISABLE_CHECKSUMS"], "true");
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

    #[test]
    fn sidecar_resources_scale_with_backend_tier() {
        let s4 = SidecarResources::for_tier(&crate::provisioner::tiers::lookup("S4").unwrap());
        let s16 = SidecarResources::for_tier(&crate::provisioner::tiers::lookup("S16").unwrap());
        let s64 = SidecarResources::for_tier(&crate::provisioner::tiers::lookup("S64").unwrap());

        assert!(s4.postgres.memory_mb < s16.postgres.memory_mb);
        assert!(s16.postgres.memory_mb < s64.postgres.memory_mb);
        assert!(s4.postgres.cpus < s16.postgres.cpus);
        assert!(s16.postgres.cpus < s64.postgres.cpus);

        assert!(s4.minio.memory_mb < s16.minio.memory_mb);
        assert!(s16.minio.memory_mb < s64.minio.memory_mb);
        assert!(s4.minio.cpus < s16.minio.cpus);
        assert!(s16.minio.cpus < s64.minio.cpus);
    }

    #[test]
    fn custom_sidecars_are_aggressively_overprovisioned() {
        let custom = crate::provisioner::tiers::resolve("custom:125760:32").unwrap();
        let resources = SidecarResources::for_tier(&custom);

        assert_eq!(resources.postgres.memory_mb, 125760);
        assert_eq!(resources.postgres.cpus, 32.0);
        assert_eq!(resources.minio.memory_mb, 62880);
        assert_eq!(resources.minio.cpus, 16.0);
    }

    #[test]
    fn sidecar_docker_args_include_tier_limits_for_bounded_tiers() {
        let resources =
            SidecarResources::for_tier(&crate::provisioner::tiers::lookup("S16").unwrap());

        let pg_args = build_postgres_run_args(
            "pg-dep",
            "pgdata-dep",
            Some("orch-net"),
            "postgres:17",
            "secret",
            128,
            resources.postgres,
        );
        assert!(pg_args.windows(2).any(|w| w == ["--memory", "4096m"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["--memory-swap", "8589934592"]));
        assert!(pg_args.windows(2).any(|w| w == ["--cpus", "2.00"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "max_connections=128"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "shared_buffers=1GB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "effective_cache_size=3GB"]));
        assert!(pg_args.windows(2).any(|w| w == ["-c", "work_mem=4MB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "maintenance_work_mem=256MB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "random_page_cost=1.1"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "effective_io_concurrency=200"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "wal_buffers=32MB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "checkpoint_completion_target=0.9"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "max_wal_size=1GB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "min_wal_size=80MB"]));

        let minio_args = build_minio_run_args(
            "minio-dep",
            "miniodata-dep",
            Some("orch-net"),
            "minio/minio:latest",
            "root",
            "secret",
            resources.minio,
        );
        assert!(minio_args.windows(2).any(|w| w == ["--memory", "2048m"]));
        assert!(minio_args
            .windows(2)
            .any(|w| w == ["--memory-swap", "4294967296"]));
        assert!(minio_args.windows(2).any(|w| w == ["--cpus", "1.00"]));
    }

    #[test]
    fn sidecar_resource_updates_include_memory_swap_with_memory() {
        let resources =
            SidecarResources::for_tier(&crate::provisioner::tiers::lookup("S16").unwrap());

        let args = build_update_container_resource_args("pg-dep", resources.postgres);

        assert!(args.windows(2).any(|w| w == ["--memory", "4294967296"]));
        assert!(args
            .windows(2)
            .any(|w| w == ["--memory-swap", "8589934592"]));
        assert!(args.windows(2).any(|w| w == ["--cpus", "2.00"]));
    }

    #[test]
    fn postgres_tuning_scales_with_sidecar_resources() {
        let resources =
            SidecarResources::for_tier(&crate::provisioner::tiers::lookup("S128").unwrap());

        let pg_args = build_postgres_run_args(
            "pg-dep",
            "pgdata-dep",
            Some("orch-net"),
            "postgres:17",
            "secret",
            1152,
            resources.postgres,
        );
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "shared_buffers=8GB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "effective_cache_size=24GB"]));
        assert!(pg_args.windows(2).any(|w| w == ["-c", "work_mem=4MB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "maintenance_work_mem=2GB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "wal_buffers=256MB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "max_wal_size=8GB"]));
        assert!(pg_args
            .windows(2)
            .any(|w| w == ["-c", "min_wal_size=512MB"]));
    }

    #[test]
    fn sidecar_docker_args_omit_limits_for_unbounded_tier() {
        let resources =
            SidecarResources::for_tier(&crate::provisioner::tiers::lookup("max").unwrap());

        let pg_args = build_postgres_run_args(
            "pg-dep",
            "pgdata-dep",
            None,
            "postgres:17",
            "secret",
            4096,
            resources.postgres,
        );
        assert!(!pg_args.iter().any(|arg| arg == "--memory"));
        assert!(!pg_args.iter().any(|arg| arg == "--cpus"));

        let minio_args = build_minio_run_args(
            "minio-dep",
            "miniodata-dep",
            None,
            "minio/minio:latest",
            "root",
            "secret",
            resources.minio,
        );
        assert!(!minio_args.iter().any(|arg| arg == "--memory"));
        assert!(!minio_args.iter().any(|arg| arg == "--cpus"));
    }
}
