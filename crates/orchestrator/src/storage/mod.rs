//! Postgres-backed storage for the orchestrator.

pub mod access_tokens;
mod acme;
mod audit_log;
mod custom_domains;
pub mod deployments;
mod env_vars;
mod invitations;
mod members;
mod migrations;
mod opt_ins;
mod pool;
mod projects;
mod schema;
mod teams;

use anyhow::Context;

pub use self::{
    access_tokens::{
        AccessToken,
        AccessTokenKind,
    },
    audit_log::{
        AuditEntry,
        AuditQuery,
    },
    acme::{
        AcmeAccountRecord,
        DnsCredentialRecord,
        DnsCredentialSecrets,
        StoredCertificate,
    },
    custom_domains::{
        CustomDomainRecord,
        CustomDomainRoute,
    },
    deployments::{
        DeploymentClass,
        DeploymentRecord,
        DeploymentState,
        DeploymentType,
    },
    env_vars::DefaultEnvVar,
    invitations::InvitationRecord,
    members::MemberRecord,
    opt_ins::OptInRecord,
    pool::PgPool,
    projects::ProjectRecord,
    teams::{
        TeamMemberRecord,
        TeamRecord,
        TeamRole,
    },
};

/// Cheaply-clonable handle to the orchestrator's storage.
#[derive(Clone)]
pub struct Storage {
    pool: PgPool,
}

impl Storage {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(database_url)
            .await
            .with_context(|| format!("connecting to postgres at {database_url}"))?;
        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    async fn run_migrations(&self) -> anyhow::Result<()> {
        migrations::run(&self.pool).await
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
