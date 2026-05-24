use std::sync::Arc;

use anyhow::Context;
use axum::extract::FromRef;

use crate::{
    auth::tokens::{
        sha256_hex,
        suffix_of,
    },
    config::OrchestratorConfig,
    ids::random_id,
    provisioner::Provisioner,
    storage::{
        access_tokens::NewAccessToken,
        AccessTokenKind,
        Storage,
    },
};

/// `auth_user_id` value used for the synthetic system member that owns
/// bootstrap-token-issued PATs. Clearly namespaced so it can never collide
/// with a BetterAuth-issued ID.
pub const SYSTEM_AUTH_USER_ID: &str = "system:bootstrap";

#[derive(Clone)]
pub struct OrchestratorState {
    pub storage: Storage,
    pub config: Arc<OrchestratorConfig>,
    pub provisioner: Arc<dyn Provisioner>,
    pub host_capacity: Arc<crate::host_capacity::HostCapacityReader>,
}

impl OrchestratorState {
    pub async fn new(config: OrchestratorConfig) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&config.data_root)
            .with_context(|| format!("creating data root {:?}", config.data_root))?;

        let storage = Storage::connect(&config.database_url).await?;
        let provisioner: Arc<dyn Provisioner> = match config.provisioner_mode {
            crate::config::ProvisionerMode::External => {
                Arc::new(crate::provisioner::ExternalProvisioner::new())
            },
            crate::config::ProvisionerMode::Process => Arc::new(
                crate::provisioner::ProcessProvisioner::new(config.data_root.clone()),
            ),
            crate::config::ProvisionerMode::Docker => {
                let strategy = config.provisioning_strategy();
                let dp = crate::provisioner::DockerProvisioner::new(
                    config.backend_image.clone(),
                    config.backend_container_prefix.clone(),
                    config.backend_network.clone(),
                    config.router_host.clone(),
                    config.router_public_port,
                    config.router_public_scheme.clone(),
                    config.direct_backend_routing,
                    strategy,
                );
                // Re-seed port allocator from the highest backend_port already
                // recorded so we don't collide with surviving containers.
                let max_port = storage
                    .max_deployment_backend_port()
                    .await
                    .unwrap_or(None)
                    .unwrap_or(0);
                if max_port > 0 {
                    dp.seed_port(max_port as u16);
                }
                Arc::new(dp)
            },
        };

        let state = Self {
            storage,
            config: Arc::new(config),
            provisioner,
            host_capacity: Arc::new(crate::host_capacity::HostCapacityReader::new()),
        };

        state.bootstrap_if_empty().await?;

        Ok(state)
    }

    /// On first startup with a configured bootstrap token, mint a synthetic
    /// "system" member and register the bootstrap token as their PAT. This
    /// keeps the CLI auth path working without requiring an interactive
    /// dashboard sign-in.
    ///
    /// Real human members are created lazily by the dashboard via
    /// `POST /api/internal/exchange_session`, after BetterAuth authenticates
    /// them.
    async fn bootstrap_if_empty(&self) -> anyhow::Result<()> {
        if self.config.bootstrap_token.is_none() {
            return Ok(());
        }
        if self
            .storage
            .get_member_by_auth_user_id(SYSTEM_AUTH_USER_ID)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let bootstrap_token = self.config.bootstrap_token.as_deref().unwrap();
        let member = self
            .storage
            .upsert_member(
                SYSTEM_AUTH_USER_ID,
                "bootstrap@self-hosted.local",
                Some("System (bootstrap)"),
            )
            .await?;
        let team = match self.storage.get_team_by_slug("self-hosted").await? {
            Some(t) => t,
            None => {
                self.storage
                    .create_team(
                        self.config.default_team_display_name(),
                        "self-hosted",
                        Some(member.id),
                    )
                    .await?
            },
        };
        self.storage
            .add_team_member(team.id, member.id, crate::storage::TeamRole::Admin)
            .await?;
        let public_id = random_id();
        let suffix = suffix_of(bootstrap_token);
        let hash = sha256_hex(bootstrap_token);
        self.storage
            .create_access_token(NewAccessToken {
                public_id: &public_id,
                kind: AccessTokenKind::Pat,
                member_id: Some(member.id),
                team_id: Some(team.id),
                project_id: None,
                deployment_id: None,
                name: "bootstrap",
                secret_hash: &hash,
                secret_suffix: &suffix,
                expiry: None,
            })
            .await?;
        tracing::info!(
            member_id = member.id,
            team_id = team.id,
            "bootstrap token registered as system PAT (real users register via the dashboard)"
        );
        Ok(())
    }
}

impl FromRef<OrchestratorState> for Storage {
    fn from_ref(state: &OrchestratorState) -> Self {
        state.storage.clone()
    }
}

impl FromRef<OrchestratorState> for Arc<OrchestratorConfig> {
    fn from_ref(state: &OrchestratorState) -> Self {
        state.config.clone()
    }
}
