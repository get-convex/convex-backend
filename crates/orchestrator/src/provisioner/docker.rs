//! Docker provisioner — spawns one `convex-local-backend` container per
//! deployment via the host's docker CLI. Used when the orchestrator itself
//! runs in Docker and `/var/run/docker.sock` is mounted in.
//!
//! Each container:
//! - is named `<container_prefix><deployment_name>` so we can target it for
//!   teardown without tracking IDs in our storage layer
//! - records unique bookkeeping ports from `next_port` but does not publish
//!   them on the host; Traefik reaches labeled containers directly and the
//!   in-orchestrator proxy remains as a wildcard fallback
//! - gets a stable `INSTANCE_NAME` (= deployment name) and a fresh
//!   `INSTANCE_SECRET` so the orchestrator can mint matching admin keys
//! - uses `--restart unless-stopped` so it survives orchestrator restarts

use std::sync::atomic::{AtomicU16, Ordering};

use async_trait::async_trait;
use rand::{distr::Alphanumeric, Rng};
use tokio::process::Command;

use super::{ProvisionRequest, ProvisionResult, Provisioner};
use crate::auth::tokens::{sha256_hex, suffix_of};

pub struct DockerProvisioner {
    backend_image: String,
    container_prefix: String,
    /// Internal-only port counter, used to keep `backend_port` rows unique
    /// for bookkeeping. Containers no longer publish to host ports — the
    /// reverse proxy fronts them.
    next_port: AtomicU16,
    /// Docker network name. When set, spawned containers join this network
    /// so the orchestrator's reverse proxy can reach them via DNS hostname.
    network: Option<String>,
    /// Hostname suffix used in deployment URLs. e.g. `localhost` →
    /// `http://<deployment>.localhost:<router_public_port>`.
    router_host: String,
    /// Port the reverse proxy is exposed on from the browser's perspective.
    router_public_port: u16,
    /// `http` or `https`. Determines the scheme of the deployment URLs the
    /// orchestrator hands out — must be `https` when TLS terminates in
    /// front of the orchestrator (Traefik, etc.) so the browser doesn't
    /// hit mixed-content blocks on the Convex client's WebSocket.
    router_public_scheme: String,
    /// Enables exact-host Traefik labels on spawned backend containers so
    /// data-plane traffic bypasses the in-orchestrator wildcard proxy.
    direct_backend_routing: bool,
    /// Strategy decided at orchestrator startup. Each `provision()` branches
    /// on this — `VolumeSqlite` keeps v2 behavior (named `/convex/data`
    /// volume); `Sidecar { .. }` spawns pg+minio sidecars and layers their
    /// env into the backend container's docker-run flags.
    strategy: crate::provisioner::ProvisioningStrategy,
}

impl DockerProvisioner {
    pub fn new(
        backend_image: String,
        container_prefix: String,
        network: Option<String>,
        router_host: String,
        router_public_port: u16,
        router_public_scheme: String,
        direct_backend_routing: bool,
        strategy: crate::provisioner::ProvisioningStrategy,
    ) -> Self {
        Self {
            backend_image,
            container_prefix,
            next_port: AtomicU16::new(9100),
            network,
            router_host,
            router_public_port,
            router_public_scheme,
            direct_backend_routing,
            strategy,
        }
    }

    /// Spawn (idempotent) pg + minio sidecars and create their buckets.
    /// Returns:
    ///   - the env vars to layer into the backend's docker run flags
    ///     (POSTGRES_URL + AWS_* + S3_*)
    ///   - the SidecarCredentials used (so the caller can snapshot them on
    ///     the deployment row)
    ///
    /// When `creds` is Some(...), reuses those credentials (restart path).
    /// When None, mints fresh credentials (initial provision path).
    async fn bring_up_sidecars(
        &self,
        deployment_name: &str,
        pg_max_connections: u32,
        sidecar_resources: crate::provisioner::sidecar::SidecarResources,
        postgres_image: &str,
        minio_image: &str,
        creds: Option<&crate::provisioner::SidecarCredentials>,
    ) -> anyhow::Result<(
        Vec<(&'static str, String)>,
        crate::provisioner::SidecarCredentials,
    )> {
        use crate::provisioner::sidecar as sc;

        let pg_container = sc::pg_container_name(&self.container_prefix, deployment_name);
        let pg_volume = sc::pg_volume_name(&self.container_prefix, deployment_name);
        let minio_container = sc::minio_container_name(&self.container_prefix, deployment_name);
        let minio_volume = sc::minio_volume_name(&self.container_prefix, deployment_name);

        let credentials =
            creds
                .cloned()
                .unwrap_or_else(|| crate::provisioner::SidecarCredentials {
                    pg_password: sc::generate_password(),
                    minio_root_user: sc::generate_password(),
                    minio_root_password: sc::generate_password(),
                });

        sc::spawn_postgres_sidecar(
            &pg_container,
            &pg_volume,
            self.network.as_deref(),
            postgres_image,
            &credentials.pg_password,
            pg_max_connections,
            sidecar_resources.postgres,
        )
        .await?;
        sc::spawn_minio_sidecar(
            &minio_container,
            &minio_volume,
            self.network.as_deref(),
            minio_image,
            &credentials.minio_root_user,
            &credentials.minio_root_password,
            sidecar_resources.minio,
        )
        .await?;
        sc::wait_for_postgres(&pg_container).await?;
        sc::create_postgres_database(&pg_container, &sc::postgres_db_name(deployment_name)).await?;
        sc::wait_for_minio(&minio_container).await?;
        sc::create_minio_buckets(
            &minio_container,
            self.network.as_deref(),
            &credentials.minio_root_user,
            &credentials.minio_root_password,
        )
        .await?;

        let mut env = Vec::new();
        env.push((
            "POSTGRES_URL",
            sc::compose_postgres_url(&pg_container, &credentials.pg_password),
        ));
        env.extend(sc::compose_s3_env(
            &minio_container,
            &credentials.minio_root_user,
            &credentials.minio_root_password,
        ));
        Ok((env, credentials))
    }

    /// Format a browser-facing deployment URL. Omits the port when it's
    /// the default for the scheme so URLs stay clean behind TLS.
    fn deployment_url(&self, host_prefix: &str) -> String {
        let scheme = if self.router_public_scheme == "http" && self.router_public_port == 443 {
            "https"
        } else {
            self.router_public_scheme.as_str()
        };
        let default_port = match scheme {
            "https" => 443,
            _ => 80,
        };
        if self.router_public_port == default_port {
            format!("{}://{}.{}", scheme, host_prefix, self.router_host)
        } else {
            format!(
                "{}://{}.{}:{}",
                scheme, host_prefix, self.router_host, self.router_public_port
            )
        }
    }

    /// Re-seed the bookkeeping counter from the highest `backend_port`
    /// already recorded so freshly minted ones don't collide with rows
    /// inserted by older builds (when host port mapping was real).
    pub fn seed_port(&self, max_existing_port: u16) {
        let want = max_existing_port.saturating_add(2).max(9100);
        self.next_port.store(want, Ordering::SeqCst);
    }

    fn container_name(&self, deployment_name: &str) -> String {
        format!("{}{}", self.container_prefix, deployment_name)
    }

    fn volume_name(&self, deployment_name: &str) -> String {
        format!("{}{}", self.container_prefix, deployment_name)
    }

    fn push_backend_routing_labels(&self, args: &mut Vec<String>, deployment_name: &str) {
        if self.direct_backend_routing {
            push_direct_backend_routing_labels(
                args,
                deployment_name,
                &self.router_host,
                self.network.as_deref(),
            );
        }
    }
}

#[async_trait]
impl Provisioner for DockerProvisioner {
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        let api_port = self.next_port.fetch_add(2, Ordering::SeqCst);
        let site_port = api_port + 1;
        // Browser-facing URLs are stable regardless of whether Traefik
        // routes directly to the backend container or falls back through the
        // in-orchestrator wildcard proxy.
        let url = self.deployment_url(&req.deployment_name);
        let site_url = self.deployment_url(&format!("{}-site", req.deployment_name));

        let secret: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(|c| c as char)
            .collect();
        let admin_key = format!("{}:{}|{}", req.deployment_type, req.deployment_name, secret);
        let hash = sha256_hex(&secret);
        let admin_key_suffix = suffix_of(&secret);

        // INSTANCE_SECRET is 64 hex chars — convex-local-backend uses it to
        // derive its admin-key cipher; mismatching it makes admin auth fail.
        let instance_secret: String = if let Some(existing) = req.existing_instance_secret.clone() {
            existing
        } else {
            (0..64)
                .map(|_| {
                    let n = rand::rng().random_range(0..16);
                    std::char::from_digit(n, 16).unwrap()
                })
                .collect()
        };

        let tier = crate::provisioner::tiers::resolve(&req.tier).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown tier {} (known: {:?})",
                req.tier,
                crate::provisioner::tiers::all_tier_names().collect::<Vec<_>>()
            )
        })?;

        // Branch on the configured strategy. In `Sidecar` mode we spawn
        // pg+minio sidecars now, updating or replacing the sidecar
        // containers when the tier/image/command changes while preserving
        // their volumes, and layer their connection env into the backend's
        // docker-run flags. `VolumeSqlite` is a no-op here and keeps v2
        // behavior.
        let (strategy_env, sidecar_credentials_for_result): (
            Vec<(&'static str, String)>,
            Option<crate::provisioner::SidecarCredentials>,
        ) = match &self.strategy {
            crate::provisioner::ProvisioningStrategy::VolumeSqlite => (Vec::new(), None),
            crate::provisioner::ProvisioningStrategy::Sidecar {
                postgres_image,
                minio_image,
            } => {
                let pg_max_connections = tier
                    .knob_defaults
                    .iter()
                    .find(|(k, _)| *k == "POSTGRES_MAX_CONNECTIONS")
                    .and_then(|(_, v)| v.parse::<u32>().ok())
                    .unwrap_or(128);
                let sidecar_resources =
                    crate::provisioner::sidecar::SidecarResources::for_tier(&tier);
                let (env, creds) = self
                    .bring_up_sidecars(
                        &req.deployment_name,
                        pg_max_connections,
                        sidecar_resources,
                        postgres_image,
                        minio_image,
                        req.sidecar_credentials.as_ref(),
                    )
                    .await?;
                (env, Some(creds))
            },
        };

        let base_env: Vec<(&str, String)> = vec![
            ("INSTANCE_NAME", req.deployment_name.clone()),
            ("INSTANCE_SECRET", instance_secret.clone()),
            ("CONVEX_CLOUD_ORIGIN", url.clone()),
            ("CONVEX_SITE_ORIGIN", site_url.clone()),
            ("DISABLE_BEACON", "true".into()),
            ("DO_NOT_REQUIRE_SSL", "true".into()),
            ("DOCUMENT_RETENTION_DELAY", "172800".into()),
        ];
        let tier_defaults: Vec<_> = tier
            .knob_defaults
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let env = crate::provisioner::env::compose_env(
            base_env
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .chain(strategy_env.iter().map(|(k, v)| (*k, v.as_str()))),
            &tier_defaults,
            req.knob_overrides
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );

        let container_name = self.container_name(&req.deployment_name);
        let volume_name = self.volume_name(&req.deployment_name);

        // Ensure a named volume exists for this deployment's persistent data.
        // `docker volume create` is idempotent for the same name + driver, so
        // "already exists" is not an error. We bail hard on any other failure
        // to avoid stranding a container without its volume.
        //
        // In `Sidecar` mode the backend container holds no persistent state
        // (pg owns the DB, minio owns file storage) so we skip the volume
        // entirely.
        match &self.strategy {
            crate::provisioner::ProvisioningStrategy::VolumeSqlite => {
                let vol_output = Command::new("docker")
                    .args(["volume", "create", &volume_name])
                    .output()
                    .await
                    .map_err(|e| anyhow::anyhow!("docker volume create failed: {e}"))?;
                if !vol_output.status.success() {
                    let stderr = String::from_utf8_lossy(&vol_output.stderr);
                    anyhow::bail!(
                        "docker volume create failed (exit {:?}): {}",
                        vol_output.status.code(),
                        stderr.trim()
                    );
                }
            },
            crate::provisioner::ProvisioningStrategy::Sidecar { .. } => {
                // No backend-side volume — pg/minio own their own storage in
                // their own named volumes (managed by sidecar.rs spawn helpers).
            },
        }

        // Note: no `-p` mappings. Traefik and the fallback proxy reach each
        // backend over the shared docker network via DNS hostname. Removing
        // host ports lets us spawn hundreds of deployments without
        // exhausting the host's ephemeral port range.
        let mut args: Vec<String> = vec![
            "run".into(),
            "-d".into(),
        ];
        push_backend_lifecycle_args(&mut args);
        args.extend([
            // Each backend's Postgres pool + WebSocket client connections
            // + action subprocess fds routinely exceed docker's default
            // 1024 nofile cap. Match the orchestrator/traefik ceiling so
            // backends aren't the weakest-link bottleneck.
            "--ulimit".into(),
            "nofile=1048576:1048576".into(),
        ]);
        // Unbounded tiers (e.g. `max`) run without resource caps — the host
        // owns all of its memory/CPU budget. Bounded tiers get explicit limits
        // so noisy neighbours can't crowd each other out.
        if !tier.unbounded {
            args.push("--memory".into());
            args.push(format!("{}m", tier.memory_mb));
            args.push("--cpus".into());
            args.push(format!("{:.2}", tier.cpus));
        }
        args.extend([
            "--name".into(),
            container_name.clone(),
            "--label".into(),
            format!("orchestrator.deployment={}", req.deployment_name),
            "--label".into(),
            format!("orchestrator.project_id={}", req.project_id),
            "--label".into(),
            format!("orchestrator.tier={}", tier.name.as_ref()),
        ]);
        self.push_backend_routing_labels(&mut args, &req.deployment_name);
        match &self.strategy {
            crate::provisioner::ProvisioningStrategy::VolumeSqlite => {
                args.push("-v".into());
                args.push(format!("{}:/convex/data", volume_name));
            },
            crate::provisioner::ProvisioningStrategy::Sidecar { .. } => {
                // No /convex/data mount — pg holds the DB, minio holds storage.
            },
        }
        for (k, v) in &env {
            args.push("-e".into());
            args.push(format!("{k}={v}"));
        }
        if let Some(net) = self.network.as_deref()
            && !net.is_empty()
        {
            args.push("--network".into());
            args.push(net.into());
        }
        args.push(self.backend_image.clone());

        tracing::info!(
            deployment_name = %req.deployment_name,
            container_name = %container_name,
            api_port,
            site_port,
            "spawning convex-local-backend container",
        );

        let output = Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("docker invocation failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "docker run failed (exit {:?}): {}",
                output.status.code(),
                stderr.trim()
            );
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!(
            deployment_name = %req.deployment_name,
            container_id = %container_id,
            "convex-local-backend container started",
        );

        // Wait for the backend to come up, then ask it (via the bundled
        // `generate_admin_key.sh`) for an admin key derived from
        // INSTANCE_NAME + INSTANCE_SECRET. We store that as `instance_secret`
        // because the dashboard's deployment shell expects an admin-key in
        // `<deployment_name>|<secret>` form there — `ephemeral_admin_key`
        // returns it verbatim when the format matches.
        let admin_key_from_backend = wait_for_admin_key(&container_name).await?;
        // Sanity-check: backend's admin key must start with the deployment
        // name; otherwise `looksLikeRealAdminKey` in the dashboard rejects
        // it and the page renders "No backend connected" anyway.
        if !admin_key_from_backend.starts_with(&format!("{}|", req.deployment_name)) {
            anyhow::bail!(
                "backend produced unexpected admin-key format {:?} (expected `{}|...`)",
                admin_key_from_backend,
                req.deployment_name
            );
        }

        Ok(ProvisionResult {
            url,
            site_url,
            admin_key,
            admin_key_hash: hash,
            admin_key_suffix,
            // We store the backend-derived admin key here so
            // `ephemeral_admin_key` returns it as-is for the dashboard.
            instance_secret: admin_key_from_backend,
            // The 64-hex secret we actually fed to the backend; the
            // caller persists this so restart can reuse it (without it,
            // restart would round-trip the admin key as INSTANCE_SECRET
            // and the backend would fail to hex-decode it).
            backend_instance_secret: instance_secret.clone(),
            backend_pid: None,
            backend_port: api_port as i64,
            resolved_env: env,
            sidecar_credentials: sidecar_credentials_for_result,
        })
    }

    async fn respawn(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        // Remove only the container — the named volume survives because
        // `docker volume create` in `provision` is idempotent (no-op if
        // the volume already exists). The new container reattaches to the
        // same volume and picks up the new INSTANCE_SECRET env var.
        let container_name = self.container_name(&req.deployment_name);
        let _ = stop_and_remove_container(&container_name, "respawn backend").await;
        self.provision(req).await
    }

    async fn teardown(&self, deployment_name: &str, storage_mode: &str) -> anyhow::Result<()> {
        let container_name = self.container_name(deployment_name);
        // Backend container — present in both modes.
        let output = Command::new("docker")
            .args(["rm", "-f", &container_name])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("docker invocation failed: {e}"))?;
        if !output.status.success() {
            // Container may not exist (already removed); log and continue.
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                deployment_name,
                stderr = %stderr.trim(),
                "docker rm reported a non-zero exit; treating as best-effort teardown",
            );
        }

        // Branch on the deployment's snapshotted storage_mode. We deliberately
        // trust the row's value (not `self.strategy`) so a deployment created
        // in one mode is always torn down in that same mode, even if the
        // orchestrator was later restarted under the other strategy.
        match storage_mode {
            "sidecar" => {
                crate::provisioner::sidecar::teardown_sidecars(
                    &self.container_prefix,
                    deployment_name,
                )
                .await;
            },
            _ => {},
        }

        // Best-effort volume removal. Mixed infrastructure modes can have
        // sidecars and a backend volume at the same time, while older rows may
        // not have a named volume at all.
        let volume_name = self.volume_name(deployment_name);
        let vol_output = Command::new("docker")
            .args(["volume", "rm", &volume_name])
            .output()
            .await;
        match vol_output {
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                tracing::warn!(
                    deployment_name,
                    stderr = %stderr.trim(),
                    "docker volume rm reported a non-zero exit; treating as best-effort",
                );
            },
            Err(e) => {
                tracing::warn!(
                    deployment_name,
                    error = %e,
                    "docker volume rm invocation failed; treating as best-effort",
                );
            },
            Ok(_) => {},
        }
        Ok(())
    }
}

fn push_backend_lifecycle_args(args: &mut Vec<String>) {
    args.extend([
        "--restart".into(),
        "unless-stopped".into(),
        "--stop-signal".into(),
        "SIGINT".into(),
        "--stop-timeout".into(),
        "10".into(),
    ]);
}

fn push_direct_backend_routing_labels(
    args: &mut Vec<String>,
    deployment_name: &str,
    router_host: &str,
    docker_network: Option<&str>,
) {
    push_label(args, "traefik.enable=true");
    if let Some(network) = docker_network
        && !network.trim().is_empty()
    {
        push_label(args, &format!("traefik.docker.network={network}"));
    }

    let safe_name = deployment_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    push_traefik_route(
        args,
        &format!("convex-backend-{safe_name}-api"),
        &format!("{deployment_name}.{router_host}"),
        3210,
    );
    push_traefik_route(
        args,
        &format!("convex-backend-{safe_name}-site"),
        &format!("{deployment_name}-site.{router_host}"),
        3211,
    );
}

fn push_traefik_route(args: &mut Vec<String>, name: &str, host: &str, port: u16) {
    let service = name;
    push_label(
        args,
        &format!("traefik.http.routers.{name}.rule=Host(`{host}`)"),
    );
    push_label(
        args,
        &format!("traefik.http.routers.{name}.priority=50"),
    );
    push_label(
        args,
        &format!("traefik.http.routers.{name}.entrypoints=websecure"),
    );
    push_label(args, &format!("traefik.http.routers.{name}.tls=true"));
    push_label(
        args,
        &format!("traefik.http.routers.{name}.service={service}"),
    );
    push_label(
        args,
        &format!("traefik.http.services.{service}.loadbalancer.server.port={port}"),
    );
}

fn push_label(args: &mut Vec<String>, label: &str) {
    args.extend(["--label".into(), label.into()]);
}

async fn stop_and_remove_container(container_name: &str, desc: &str) -> anyhow::Result<()> {
    let stop_output = Command::new("docker")
        .args(["stop", "--time", "10", container_name])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker stop failed during {desc}: {e}"))?;
    if !stop_output.status.success() {
        let stderr = String::from_utf8_lossy(&stop_output.stderr);
        tracing::debug!(
            container = %container_name,
            stderr = %stderr.trim(),
            "docker stop reported non-zero during {desc}; falling back to remove",
        );
    }

    let rm_output = Command::new("docker")
        .args(["rm", container_name])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker rm failed during {desc}: {e}"))?;
    if rm_output.status.success() {
        return Ok(());
    }

    let rm_force_output = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("docker rm -f failed during {desc}: {e}"))?;
    if rm_force_output.status.success() {
        return Ok(());
    }

    anyhow::bail!(
        "docker remove failed during {desc}: {}",
        String::from_utf8_lossy(&rm_force_output.stderr).trim()
    )
}

/// Poll the spawned backend container's `generate_admin_key.sh` until it
/// produces an admin key. The script reads INSTANCE_NAME/INSTANCE_SECRET
/// (which we set as env vars on `docker run`) and shells out to the
/// `generate_key` binary baked into the image. We retry for ~30s while the
/// container's filesystem + binary settle.
async fn wait_for_admin_key(container_name: &str) -> anyhow::Result<String> {
    let mut last_err = String::new();
    for attempt in 0..30 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let output = Command::new("docker")
            .args(["exec", container_name, "./generate_admin_key.sh"])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("docker exec failed: {e}"))?;
        if output.status.success() {
            // Last non-empty stdout line is the key; the script may emit a
            // log preamble (read_credentials.sh) before printing it.
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(key) = stdout.lines().rev().find(|l| !l.trim().is_empty()) {
                return Ok(key.trim().to_string());
            }
            last_err = "generate_admin_key.sh produced empty stdout".into();
        } else {
            last_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        }
    }
    anyhow::bail!("backend never produced an admin key (last error: {last_err})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_run_args_include_graceful_stop_contract() {
        let mut args = Vec::new();
        push_backend_lifecycle_args(&mut args);

        assert!(args.windows(2).any(|w| w == ["--restart", "unless-stopped"]));
        assert!(args.windows(2).any(|w| w == ["--stop-signal", "SIGINT"]));
        assert!(args.windows(2).any(|w| w == ["--stop-timeout", "10"]));
    }
}
