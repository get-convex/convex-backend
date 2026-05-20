use tokio_postgres::Row;

use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct ProjectRecord {
    pub id: i64,
    pub team_id: i64,
    pub name: String,
    pub slug: String,
    pub is_demo: bool,
    pub creation_time: i64,
    pub deleted: bool,
    pub tier: String,
    pub knob_overrides: serde_json::Value,
}

impl Storage {
    pub async fn create_project(
        &self,
        team_id: i64,
        name: &str,
        slug: &str,
        is_demo: bool,
    ) -> anyhow::Result<ProjectRecord> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO projects (team_id, name, slug, is_demo, creation_time)
                 VALUES ($1, $2, $3, $4, $5)
                 RETURNING id",
                &[&team_id, &name, &slug, &is_demo, &now],
            )
            .await?;
        let id: i64 = row.get(0);
        Ok(ProjectRecord {
            id,
            team_id,
            name: name.to_string(),
            slug: slug.to_string(),
            is_demo,
            creation_time: now,
            deleted: false,
            // Mirrors the `tier DEFAULT 'S16'` and `knob_overrides DEFAULT '{}'::jsonb`
            // schema defaults — keep these in sync if the schema changes.
            tier: crate::provisioner::tiers::DEFAULT_TIER.to_string(),
            knob_overrides: serde_json::Value::Object(Default::default()),
        })
    }

    pub async fn get_project(&self, id: i64) -> anyhow::Result<Option<ProjectRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, team_id, name, slug, is_demo, creation_time, deleted, tier, knob_overrides
                 FROM projects WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(map_project))
    }

    pub async fn get_project_by_slug(
        &self,
        team_id: i64,
        slug: &str,
    ) -> anyhow::Result<Option<ProjectRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, team_id, name, slug, is_demo, creation_time, deleted, tier, knob_overrides
                 FROM projects WHERE team_id = $1 AND slug = $2 AND deleted = FALSE",
                &[&team_id, &slug],
            )
            .await?;
        Ok(row.map(map_project))
    }

    pub async fn list_projects(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<ProjectRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT id, team_id, name, slug, is_demo, creation_time, deleted, tier, knob_overrides
                 FROM projects WHERE team_id = $1 AND deleted = FALSE
                 ORDER BY creation_time ASC",
                &[&team_id],
            )
            .await?;
        Ok(rows.into_iter().map(map_project).collect())
    }

    pub async fn count_projects(&self) -> anyhow::Result<i64> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "SELECT COUNT(*)::BIGINT FROM projects WHERE deleted = FALSE",
                &[],
            )
            .await?;
        Ok(row.get(0))
    }

    pub async fn update_project(
        &self,
        id: i64,
        name: Option<&str>,
        slug: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        if let Some(name) = name {
            conn.client()
                .execute(
                    "UPDATE projects SET name = $1 WHERE id = $2",
                    &[&name, &id],
                )
                .await?;
        }
        if let Some(slug) = slug {
            conn.client()
                .execute(
                    "UPDATE projects SET slug = $1 WHERE id = $2",
                    &[&slug, &id],
                )
                .await?;
        }
        Ok(())
    }

    pub async fn delete_project(&self, id: i64) -> anyhow::Result<()> {
        // Hard delete so `UNIQUE(team_id, slug)` releases the slot —
        // soft-deleted rows keep blocking new projects with the same slug
        // and there's no restore UI to justify keeping the row around. The
        // schema's `ON DELETE CASCADE` chain cleans up access_tokens,
        // project_admins, default_env_vars; deployments are torn down
        // separately in `cascade_delete_project` before this runs.
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute("DELETE FROM projects WHERE id = $1", &[&id])
            .await?;
        Ok(())
    }

    /// Project-level admin grants. Returns `(project_id, member_id)` pairs;
    /// the caller joins against the team membership to render names/emails.
    pub async fn list_project_admins_for_team(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<(i64, i64)>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT pa.project_id, pa.member_id
                 FROM project_admins pa
                 JOIN projects p ON p.id = pa.project_id
                 WHERE p.team_id = $1 AND p.deleted = FALSE",
                &[&team_id],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<_, i64>(0), r.get::<_, i64>(1)))
            .collect())
    }

    pub async fn list_project_admins(
        &self,
        project_id: i64,
    ) -> anyhow::Result<Vec<i64>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT member_id FROM project_admins WHERE project_id = $1",
                &[&project_id],
            )
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<_, i64>(0)).collect())
    }

    /// Replace the full admin set for a project. Inserts missing pairs and
    /// deletes the ones that aren't in `member_ids`. `granted_by` is who
    /// performed the change (audit-log purposes).
    pub async fn set_project_admins(
        &self,
        project_id: i64,
        member_ids: &[i64],
        granted_by: Option<i64>,
    ) -> anyhow::Result<()> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        // Wipe-and-replace keeps the code simple at the cost of an extra
        // round trip; admin sets are tiny (single-digit) so this is fine.
        conn.client()
            .execute(
                "DELETE FROM project_admins WHERE project_id = $1",
                &[&project_id],
            )
            .await?;
        for member_id in member_ids {
            conn.client()
                .execute(
                    "INSERT INTO project_admins (project_id, member_id, granted_at, granted_by)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (project_id, member_id) DO NOTHING",
                    &[&project_id, member_id, &now, &granted_by],
                )
                .await?;
        }
        Ok(())
    }

    /// Replace the project's tier + knob_overrides atomically. Either field
    /// can be `None` to leave it unchanged.
    pub async fn update_project_settings(
        &self,
        project_id: i64,
        tier: Option<&str>,
        knob_overrides: Option<&serde_json::Value>,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        if let Some(tier) = tier {
            conn.client()
                .execute(
                    "UPDATE projects SET tier = $1 WHERE id = $2",
                    &[&tier, &project_id],
                )
                .await?;
        }
        if let Some(overrides) = knob_overrides {
            conn.client()
                .execute(
                    "UPDATE projects SET knob_overrides = $1 WHERE id = $2",
                    &[&overrides, &project_id],
                )
                .await?;
        }
        Ok(())
    }
}

fn map_project(row: Row) -> ProjectRecord {
    ProjectRecord {
        id: row.get(0),
        team_id: row.get(1),
        name: row.get(2),
        slug: row.get(3),
        is_demo: row.get(4),
        creation_time: row.get(5),
        deleted: row.get(6),
        tier: row.get(7),
        knob_overrides: row.get(8),
    }
}
