use serde::{
    Deserialize,
    Serialize,
};
use strum::{
    Display,
    EnumString,
};
use tokio_postgres::Row;

use super::Storage;
use crate::time::now_unix_ms;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display, PartialEq, Eq,
)]
#[strum(serialize_all = "snake_case")]
pub enum AccessTokenKind {
    Pat,
    Team,
    DeployProd,
    DeployDev,
    DeployPreview,
    ProjectDeploy,
    Session,
    App,
    Admin,
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub id: i64,
    pub public_id: String,
    pub kind: AccessTokenKind,
    pub member_id: Option<i64>,
    pub team_id: Option<i64>,
    pub project_id: Option<i64>,
    pub deployment_id: Option<i64>,
    pub name: String,
    pub secret_hash: String,
    pub secret_suffix: String,
    pub creation_time: i64,
    pub expiry: Option<i64>,
    pub revoked_time: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NewAccessToken<'a> {
    pub public_id: &'a str,
    pub kind: AccessTokenKind,
    pub member_id: Option<i64>,
    pub team_id: Option<i64>,
    pub project_id: Option<i64>,
    pub deployment_id: Option<i64>,
    pub name: &'a str,
    pub secret_hash: &'a str,
    pub secret_suffix: &'a str,
    pub expiry: Option<i64>,
}

impl Storage {
    pub async fn create_access_token(
        &self,
        n: NewAccessToken<'_>,
    ) -> anyhow::Result<AccessToken> {
        let now = now_unix_ms();
        let kind_s = n.kind.to_string();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO access_tokens (
                    public_id, kind, member_id, team_id, project_id, deployment_id,
                    name, secret_hash, secret_suffix, creation_time, expiry
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                RETURNING id",
                &[
                    &n.public_id,
                    &kind_s,
                    &n.member_id,
                    &n.team_id,
                    &n.project_id,
                    &n.deployment_id,
                    &n.name,
                    &n.secret_hash,
                    &n.secret_suffix,
                    &now,
                    &n.expiry,
                ],
            )
            .await?;
        let id: i64 = row.get(0);
        Ok(AccessToken {
            id,
            public_id: n.public_id.to_string(),
            kind: n.kind,
            member_id: n.member_id,
            team_id: n.team_id,
            project_id: n.project_id,
            deployment_id: n.deployment_id,
            name: n.name.to_string(),
            secret_hash: n.secret_hash.to_string(),
            secret_suffix: n.secret_suffix.to_string(),
            creation_time: now,
            expiry: n.expiry,
            revoked_time: None,
        })
    }

    pub async fn get_access_token_by_hash(
        &self,
        secret_hash: &str,
    ) -> anyhow::Result<Option<AccessToken>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(SELECT_TOKEN_BY_HASH, &[&secret_hash])
            .await?;
        Ok(row.map(map_token))
    }

    pub async fn list_access_tokens_by_member(
        &self,
        member_id: i64,
    ) -> anyhow::Result<Vec<AccessToken>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(SELECT_TOKENS_BY_MEMBER, &[&member_id])
            .await?;
        Ok(rows.into_iter().map(map_token).collect())
    }

    pub async fn list_access_tokens_by_team(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<AccessToken>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(SELECT_TOKENS_BY_TEAM, &[&team_id])
            .await?;
        Ok(rows.into_iter().map(map_token).collect())
    }

    pub async fn list_access_tokens_by_deployment(
        &self,
        deployment_id: i64,
    ) -> anyhow::Result<Vec<AccessToken>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(SELECT_TOKENS_BY_DEPLOYMENT, &[&deployment_id])
            .await?;
        Ok(rows.into_iter().map(map_token).collect())
    }

    pub async fn list_access_tokens_by_project(
        &self,
        project_id: i64,
    ) -> anyhow::Result<Vec<AccessToken>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(SELECT_TOKENS_BY_PROJECT, &[&project_id])
            .await?;
        Ok(rows.into_iter().map(map_token).collect())
    }

    pub async fn revoke_access_token(&self, public_id: &str) -> anyhow::Result<()> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE access_tokens SET revoked_time = $1 WHERE public_id = $2",
                &[&now, &public_id],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_access_token(&self, public_id: &str) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "DELETE FROM access_tokens WHERE public_id = $1",
                &[&public_id],
            )
            .await?;
        Ok(())
    }
}

// SELECT_TOKEN_BY_HASH intentionally returns revoked tokens too — the auth
// path still needs to recognize them so it can return a clean 401 instead of
// silently treating a revoked key as unknown.
const SELECT_TOKEN_BY_HASH: &str = "SELECT id, public_id, kind, member_id, team_id, project_id, deployment_id, name, secret_hash, secret_suffix, creation_time, expiry, revoked_time FROM access_tokens WHERE secret_hash = $1";
// All "list" queries hide revoked rows so the dashboard's deploy-key UI
// reflects current state. Without this, hitting Revoke just sets
// revoked_time but leaves the row visible — the bug users saw as "Revoke
// doesn't work."
const SELECT_TOKENS_BY_MEMBER: &str = "SELECT id, public_id, kind, member_id, team_id, project_id, deployment_id, name, secret_hash, secret_suffix, creation_time, expiry, revoked_time FROM access_tokens WHERE member_id = $1 AND revoked_time IS NULL ORDER BY creation_time DESC";
const SELECT_TOKENS_BY_TEAM: &str = "SELECT id, public_id, kind, member_id, team_id, project_id, deployment_id, name, secret_hash, secret_suffix, creation_time, expiry, revoked_time FROM access_tokens WHERE team_id = $1 AND revoked_time IS NULL ORDER BY creation_time DESC";
const SELECT_TOKENS_BY_DEPLOYMENT: &str = "SELECT id, public_id, kind, member_id, team_id, project_id, deployment_id, name, secret_hash, secret_suffix, creation_time, expiry, revoked_time FROM access_tokens WHERE deployment_id = $1 AND revoked_time IS NULL ORDER BY creation_time DESC";
const SELECT_TOKENS_BY_PROJECT: &str = "SELECT id, public_id, kind, member_id, team_id, project_id, deployment_id, name, secret_hash, secret_suffix, creation_time, expiry, revoked_time FROM access_tokens WHERE project_id = $1 AND revoked_time IS NULL ORDER BY creation_time DESC";

fn map_token(row: Row) -> AccessToken {
    AccessToken {
        id: row.get(0),
        public_id: row.get(1),
        kind: row
            .get::<_, String>(2)
            .parse()
            .unwrap_or(AccessTokenKind::Pat),
        member_id: row.get(3),
        team_id: row.get(4),
        project_id: row.get(5),
        deployment_id: row.get(6),
        name: row.get(7),
        secret_hash: row.get(8),
        secret_suffix: row.get(9),
        creation_time: row.get(10),
        expiry: row.get(11),
        revoked_time: row.get(12),
    }
}
