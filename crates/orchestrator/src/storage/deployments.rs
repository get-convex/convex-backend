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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum DeploymentType {
    Prod,
    Dev,
    Preview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum DeploymentClass {
    Standard,
    Professional,
    Business,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum DeploymentState {
    Running,
    Paused,
    Disabled,
    Provisioning,
}

#[derive(Debug, Clone)]
pub struct DeploymentRecord {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub deployment_type: DeploymentType,
    pub deployment_class: DeploymentClass,
    pub region: Option<String>,
    pub url: String,
    pub site_url: String,
    pub backend_pid: Option<i64>,
    pub backend_port: i64,
    pub creator_id: Option<i64>,
    pub creation_time: i64,
    pub state: DeploymentState,
    pub preview_identifier: Option<String>,
    pub instance_secret: String,
    pub tier: String,
    pub knob_overrides: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct NewDeployment<'a> {
    pub project_id: i64,
    pub name: &'a str,
    pub deployment_type: DeploymentType,
    pub deployment_class: DeploymentClass,
    pub region: Option<&'a str>,
    pub url: &'a str,
    pub site_url: &'a str,
    pub backend_pid: Option<i64>,
    pub backend_port: i64,
    pub creator_id: Option<i64>,
    pub preview_identifier: Option<&'a str>,
    pub instance_secret: &'a str,
    pub tier: &'a str,
    pub knob_overrides: &'a serde_json::Value,
}

impl Storage {
    pub async fn create_deployment(
        &self,
        n: NewDeployment<'_>,
    ) -> anyhow::Result<DeploymentRecord> {
        let now = now_unix_ms();
        let dt = n.deployment_type.to_string();
        let dc = n.deployment_class.to_string();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO deployments (
                    project_id, name, deployment_type, deployment_class, region, url,
                    site_url, backend_pid, backend_port, creator_id, creation_time, state,
                    preview_identifier, instance_secret, tier, knob_overrides
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'running',$12,$13,$14,$15)
                RETURNING id",
                &[
                    &n.project_id,
                    &n.name,
                    &dt,
                    &dc,
                    &n.region,
                    &n.url,
                    &n.site_url,
                    &n.backend_pid,
                    &n.backend_port,
                    &n.creator_id,
                    &now,
                    &n.preview_identifier,
                    &n.instance_secret,
                    &n.tier,
                    &n.knob_overrides,
                ],
            )
            .await?;
        let id: i64 = row.get(0);
        Ok(DeploymentRecord {
            id,
            project_id: n.project_id,
            name: n.name.to_string(),
            deployment_type: n.deployment_type,
            deployment_class: n.deployment_class,
            region: n.region.map(str::to_string),
            url: n.url.to_string(),
            site_url: n.site_url.to_string(),
            backend_pid: n.backend_pid,
            backend_port: n.backend_port,
            creator_id: n.creator_id,
            creation_time: now,
            state: DeploymentState::Running,
            preview_identifier: n.preview_identifier.map(str::to_string),
            instance_secret: n.instance_secret.to_string(),
            tier: n.tier.to_string(),
            knob_overrides: n.knob_overrides.clone(),
        })
    }

    pub async fn get_deployment(&self, id: i64) -> anyhow::Result<Option<DeploymentRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(SELECT_DEPLOYMENT_BY_ID, &[&id])
            .await?;
        Ok(row.map(map_deployment))
    }

    pub async fn get_deployment_by_name(
        &self,
        name: &str,
    ) -> anyhow::Result<Option<DeploymentRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(SELECT_DEPLOYMENT_BY_NAME, &[&name])
            .await?;
        Ok(row.map(map_deployment))
    }

    /// Highest `backend_port` across all deployments. Used by the docker
    /// provisioner on startup to re-seed its port allocator past any ports
    /// already occupied by surviving containers from prior runs.
    pub async fn max_deployment_backend_port(&self) -> anyhow::Result<Option<i64>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one("SELECT MAX(backend_port) FROM deployments", &[])
            .await?;
        Ok(row.try_get::<_, Option<i64>>(0).ok().flatten())
    }

    pub async fn list_deployments(
        &self,
        project_id: i64,
    ) -> anyhow::Result<Vec<DeploymentRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(SELECT_DEPLOYMENTS_BY_PROJECT, &[&project_id])
            .await?;
        Ok(rows.into_iter().map(map_deployment).collect())
    }

    pub async fn list_deployments_for_team(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<DeploymentRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(SELECT_DEPLOYMENTS_BY_TEAM, &[&team_id])
            .await?;
        Ok(rows.into_iter().map(map_deployment).collect())
    }

    pub async fn find_default_deployment(
        &self,
        project_id: i64,
        deployment_type: DeploymentType,
    ) -> anyhow::Result<Option<DeploymentRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, project_id, name, deployment_type, deployment_class, region, url,
                        site_url, backend_pid, backend_port, creator_id, creation_time, state,
                        preview_identifier, instance_secret, tier, knob_overrides
                 FROM deployments
                 WHERE project_id = $1 AND deployment_type = $2
                   AND preview_identifier IS NULL
                 ORDER BY creation_time ASC LIMIT 1",
                &[&project_id, &deployment_type.to_string()],
            )
            .await?;
        Ok(row.map(map_deployment))
    }

    pub async fn update_deployment_state(
        &self,
        id: i64,
        state: DeploymentState,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE deployments SET state = $1 WHERE id = $2",
                &[&state.to_string(), &id],
            )
            .await?;
        Ok(())
    }

    pub async fn update_deployment_class(
        &self,
        id: i64,
        class: DeploymentClass,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE deployments SET deployment_class = $1 WHERE id = $2",
                &[&class.to_string(), &id],
            )
            .await?;
        Ok(())
    }

    pub async fn transfer_deployment(
        &self,
        id: i64,
        new_project_id: i64,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE deployments SET project_id = $1 WHERE id = $2",
                &[&new_project_id, &id],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_deployment(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute("DELETE FROM deployments WHERE id = $1", &[&id])
            .await?;
        Ok(())
    }

    /// Returns tier strings for all deployments. Caller joins against TIERS to
    /// sum memory/CPU.
    pub async fn list_deployment_tiers(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query("SELECT tier FROM deployments", &[])
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<_, String>(0)).collect())
    }
}

const SELECT_DEPLOYMENT_BY_ID: &str = "SELECT id, project_id, name, deployment_type, deployment_class, region, url, site_url, backend_pid, backend_port, creator_id, creation_time, state, preview_identifier, instance_secret, tier, knob_overrides FROM deployments WHERE id = $1";
const SELECT_DEPLOYMENT_BY_NAME: &str = "SELECT id, project_id, name, deployment_type, deployment_class, region, url, site_url, backend_pid, backend_port, creator_id, creation_time, state, preview_identifier, instance_secret, tier, knob_overrides FROM deployments WHERE name = $1";
const SELECT_DEPLOYMENTS_BY_PROJECT: &str = "SELECT id, project_id, name, deployment_type, deployment_class, region, url, site_url, backend_pid, backend_port, creator_id, creation_time, state, preview_identifier, instance_secret, tier, knob_overrides FROM deployments WHERE project_id = $1 ORDER BY creation_time ASC";
const SELECT_DEPLOYMENTS_BY_TEAM: &str = "SELECT d.id, d.project_id, d.name, d.deployment_type, d.deployment_class, d.region, d.url, d.site_url, d.backend_pid, d.backend_port, d.creator_id, d.creation_time, d.state, d.preview_identifier, d.instance_secret, d.tier, d.knob_overrides FROM deployments d INNER JOIN projects p ON p.id = d.project_id WHERE p.team_id = $1 ORDER BY d.creation_time ASC";

fn map_deployment(row: Row) -> DeploymentRecord {
    DeploymentRecord {
        id: row.get(0),
        project_id: row.get(1),
        name: row.get(2),
        deployment_type: row
            .get::<_, String>(3)
            .parse()
            .unwrap_or(DeploymentType::Prod),
        deployment_class: row
            .get::<_, String>(4)
            .parse()
            .unwrap_or(DeploymentClass::Standard),
        region: row.get(5),
        url: row.get(6),
        site_url: row.get(7),
        backend_pid: row.get(8),
        backend_port: row.get(9),
        creator_id: row.get(10),
        creation_time: row.get(11),
        state: row
            .get::<_, String>(12)
            .parse()
            .unwrap_or(DeploymentState::Running),
        preview_identifier: row.get(13),
        instance_secret: row.get(14),
        tier: row.get(15),
        knob_overrides: row.get(16),
    }
}
