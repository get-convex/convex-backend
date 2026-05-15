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

#[derive(Debug, Clone)]
pub struct TeamRecord {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub creator_id: Option<i64>,
    pub creation_time: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum TeamRole {
    Admin,
    Developer,
}

#[derive(Debug, Clone)]
pub struct TeamMemberRecord {
    pub team_id: i64,
    pub member_id: i64,
    pub role: TeamRole,
}

impl Storage {
    pub async fn create_team(
        &self,
        name: &str,
        slug: &str,
        creator_id: Option<i64>,
    ) -> anyhow::Result<TeamRecord> {
        let now = now_unix_ms();
        let mut conn = self.pool().acquire().await?;
        let tx = conn.client_mut().transaction().await?;
        let row = tx
            .query_one(
                "INSERT INTO teams (name, slug, creator_id, creation_time)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id",
                &[&name, &slug, &creator_id, &now],
            )
            .await?;
        let id: i64 = row.get(0);
        if let Some(cid) = creator_id {
            tx.execute(
                "INSERT INTO team_members (team_id, member_id, role) VALUES ($1, $2, 'admin')",
                &[&id, &cid],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(TeamRecord {
            id,
            name: name.to_string(),
            slug: slug.to_string(),
            creator_id,
            creation_time: now,
        })
    }

    pub async fn get_team(&self, id: i64) -> anyhow::Result<Option<TeamRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, name, slug, creator_id, creation_time
                 FROM teams WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(map_team))
    }

    pub async fn get_team_by_slug(
        &self,
        slug: &str,
    ) -> anyhow::Result<Option<TeamRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, name, slug, creator_id, creation_time
                 FROM teams WHERE slug = $1",
                &[&slug],
            )
            .await?;
        Ok(row.map(map_team))
    }

    pub async fn list_teams_for_member(
        &self,
        member_id: i64,
    ) -> anyhow::Result<Vec<TeamRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT t.id, t.name, t.slug, t.creator_id, t.creation_time
                 FROM teams t
                 INNER JOIN team_members tm ON tm.team_id = t.id
                 WHERE tm.member_id = $1
                 ORDER BY t.creation_time ASC",
                &[&member_id],
            )
            .await?;
        Ok(rows.into_iter().map(map_team).collect())
    }

    pub async fn update_team(
        &self,
        id: i64,
        name: Option<&str>,
        slug: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        if let Some(name) = name {
            conn.client()
                .execute(
                    "UPDATE teams SET name = $1 WHERE id = $2",
                    &[&name, &id],
                )
                .await?;
        }
        if let Some(slug) = slug {
            conn.client()
                .execute(
                    "UPDATE teams SET slug = $1 WHERE id = $2",
                    &[&slug, &id],
                )
                .await?;
        }
        Ok(())
    }

    pub async fn delete_team(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute("DELETE FROM teams WHERE id = $1", &[&id])
            .await?;
        Ok(())
    }

    pub async fn list_team_members(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<TeamMemberRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT team_id, member_id, role FROM team_members WHERE team_id = $1",
                &[&team_id],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| TeamMemberRecord {
                team_id: r.get(0),
                member_id: r.get(1),
                role: r
                    .get::<_, String>(2)
                    .parse()
                    .unwrap_or(TeamRole::Developer),
            })
            .collect())
    }

    pub async fn add_team_member(
        &self,
        team_id: i64,
        member_id: i64,
        role: TeamRole,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "INSERT INTO team_members (team_id, member_id, role) VALUES ($1, $2, $3)
                 ON CONFLICT (team_id, member_id) DO UPDATE SET role = EXCLUDED.role",
                &[&team_id, &member_id, &role.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn remove_team_member(
        &self,
        team_id: i64,
        member_id: i64,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "DELETE FROM team_members WHERE team_id = $1 AND member_id = $2",
                &[&team_id, &member_id],
            )
            .await?;
        Ok(())
    }

    pub async fn get_team_role(
        &self,
        team_id: i64,
        member_id: i64,
    ) -> anyhow::Result<Option<TeamRole>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT role FROM team_members WHERE team_id = $1 AND member_id = $2",
                &[&team_id, &member_id],
            )
            .await?;
        Ok(row
            .and_then(|r| r.get::<_, String>(0).parse().ok()))
    }
}

fn map_team(row: Row) -> TeamRecord {
    TeamRecord {
        id: row.get(0),
        name: row.get(1),
        slug: row.get(2),
        creator_id: row.get(3),
        creation_time: row.get(4),
    }
}
