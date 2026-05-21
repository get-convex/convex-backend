//! Process provisioner skeleton.
//!
//! v1 reserves a unique port, picks an instance secret, and returns a
//! pre-computed URL. Actual `convex-local-backend` spawning is left for
//! follow-up work since it requires lifecycle plumbing (signal handling,
//! readiness probes, log capture). The current implementation mints the
//! credentials and records intent in storage — the operator can then start
//! a backend with the printed CLI.

use std::{
    path::PathBuf,
    sync::atomic::{
        AtomicU16,
        Ordering,
    },
};

use async_trait::async_trait;
use rand::{
    distr::Alphanumeric,
    Rng,
};

use super::{
    ProvisionRequest,
    ProvisionResult,
    Provisioner,
};
use crate::auth::tokens::{
    sha256_hex,
    suffix_of,
};

pub struct ProcessProvisioner {
    data_root: PathBuf,
    next_port: AtomicU16,
}

impl ProcessProvisioner {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            next_port: AtomicU16::new(8100),
        }
    }
}

#[async_trait]
impl Provisioner for ProcessProvisioner {
    async fn provision(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        let port = self.next_port.fetch_add(2, Ordering::SeqCst);
        let site_port = port + 1;
        let url = format!("http://127.0.0.1:{port}");
        let site_url = format!("http://127.0.0.1:{site_port}");

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

        let instance_secret: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(|c| c as char)
            .collect();

        // Reserve a directory for the deployment's data.
        let dir = self.data_root.join("deployments").join(&req.deployment_name);
        std::fs::create_dir_all(&dir)?;

        tracing::info!(
            deployment_name = %req.deployment_name,
            port,
            data_dir = %dir.display(),
            "provisioned backend (process-mode skeleton — start convex-local-backend manually)"
        );

        Ok(ProvisionResult {
            url,
            site_url,
            admin_key,
            admin_key_hash: hash,
            admin_key_suffix,
            instance_secret,
            backend_pid: None,
            backend_port: port as i64,
            resolved_env: std::collections::BTreeMap::new(),
            sidecar_credentials: None,
        })
    }

    async fn teardown(&self, deployment_name: &str, _storage_mode: &str) -> anyhow::Result<()> {
        tracing::info!(
            deployment_name,
            "process-mode teardown is a no-op in v1; stop the backend process manually"
        );
        Ok(())
    }
}
