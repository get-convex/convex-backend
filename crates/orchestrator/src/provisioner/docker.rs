//! Docker provisioner — spawns one `convex-local-backend` container per
//! deployment via the host's docker CLI. Used when the orchestrator itself
//! runs in Docker and `/var/run/docker.sock` is mounted in.
//!
//! Each container:
//! - is named `<container_prefix><deployment_name>` so we can target it for
//!   teardown without tracking IDs in our storage layer
//! - publishes its API + HTTP-actions ports on host ports allocated from
//!   the `next_port` atomic (v1 starts at 9100; restarts re-seed from the
//!   max `backend_port` already in the orchestrator's `deployments` table)
//! - gets a stable `INSTANCE_NAME` (= deployment name) and a fresh
//!   `INSTANCE_SECRET` so the orchestrator can mint matching admin keys
//! - uses `--restart unless-stopped` so it survives orchestrator restarts

use std::sync::atomic::{
    AtomicU16,
    Ordering,
};

use async_trait::async_trait;
use rand::{
    distr::Alphanumeric,
    Rng,
};
use tokio::process::Command;

use super::{
    ProvisionRequest,
    ProvisionResult,
    Provisioner,
};
use crate::auth::tokens::{
    sha256_hex,
    suffix_of,
};

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
}

impl DockerProvisioner {
    pub fn new(
        backend_image: String,
        container_prefix: String,
        network: Option<String>,
        router_host: String,
        router_public_port: u16,
        router_public_scheme: String,
    ) -> Self {
        Self {
            backend_image,
            container_prefix,
            next_port: AtomicU16::new(9100),
            network,
            router_host,
            router_public_port,
            router_public_scheme,
        }
    }

    /// Format a browser-facing deployment URL. Omits the port when it's
    /// the default for the scheme so URLs stay clean behind TLS.
    fn deployment_url(&self, host_prefix: &str) -> String {
        let default_port = match self.router_public_scheme.as_str() {
            "https" => 443,
            _ => 80,
        };
        if self.router_public_port == default_port {
            format!(
                "{}://{}.{}",
                self.router_public_scheme, host_prefix, self.router_host
            )
        } else {
            format!(
                "{}://{}.{}:{}",
                self.router_public_scheme,
                host_prefix,
                self.router_host,
                self.router_public_port
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
}

#[async_trait]
impl Provisioner for DockerProvisioner {
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        let api_port = self.next_port.fetch_add(2, Ordering::SeqCst);
        let site_port = api_port + 1;
        // Browser-facing URLs go through the reverse proxy. `*.localhost`
        // resolves to the loopback in modern browsers; the proxy parses
        // `Host` and forwards to the docker-DNS hostname.
        let url = self.deployment_url(&req.deployment_name);
        let site_url = self.deployment_url(&format!("{}-site", req.deployment_name));

        let secret: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(|c| c as char)
            .collect();
        let admin_key = format!(
            "{}:{}|{}",
            req.deployment_type, req.deployment_name, secret
        );
        let hash = sha256_hex(&secret);
        let admin_key_suffix = suffix_of(&secret);

        // INSTANCE_SECRET is 64 hex chars — convex-local-backend uses it to
        // derive its admin-key cipher; mismatching it makes admin auth fail.
        let instance_secret: String = (0..64)
            .map(|_| {
                let n = rand::rng().random_range(0..16);
                std::char::from_digit(n, 16).unwrap()
            })
            .collect();

        let tier = crate::provisioner::tiers::lookup(&req.tier).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown tier {} (known: {:?})",
                req.tier,
                crate::provisioner::tiers::all_tier_names().collect::<Vec<_>>()
            )
        })?;

        let base_env: Vec<(&str, String)> = vec![
            ("INSTANCE_NAME", req.deployment_name.clone()),
            ("INSTANCE_SECRET", instance_secret.clone()),
            ("CONVEX_CLOUD_ORIGIN", url.clone()),
            ("CONVEX_SITE_ORIGIN", site_url.clone()),
            ("DISABLE_BEACON", "true".into()),
            ("DO_NOT_REQUIRE_SSL", "true".into()),
            ("DOCUMENT_RETENTION_DELAY", "172800".into()),
        ];
        let env = crate::provisioner::env::compose_env(
            base_env.iter().map(|(k, v)| (*k, v.as_str())),
            tier.knob_defaults,
            req.knob_overrides.iter().map(|(k, v)| (k.clone(), v.clone())),
        );

        let container_name = self.container_name(&req.deployment_name);

        // Note: no `-p` mappings. The proxy reaches each backend over the
        // shared docker network via DNS hostname. Removing host ports lets
        // us spawn hundreds of deployments without exhausting the host's
        // ephemeral port range.
        let mut args: Vec<String> = vec![
            "run".into(),
            "-d".into(),
            "--restart".into(),
            "unless-stopped".into(),
        ];
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
            format!("orchestrator.tier={}", tier.name),
        ]);
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
            backend_pid: None,
            backend_port: api_port as i64,
            resolved_env: env,
        })
    }

    async fn teardown(&self, deployment_name: &str) -> anyhow::Result<()> {
        let container_name = self.container_name(deployment_name);
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
        Ok(())
    }
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
    anyhow::bail!(
        "backend never produced an admin key (last error: {last_err})"
    )
}
