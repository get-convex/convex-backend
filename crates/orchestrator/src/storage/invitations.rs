use tokio_postgres::Row;

use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct InvitationRecord {
    pub id: i64,
    pub team_id: i64,
    pub email: String,
    pub role: String,
    pub code: String,
    pub invited_by: Option<i64>,
    pub created_at: i64,
    pub accepted_at: Option<i64>,
}

impl Storage {
    pub async fn create_invitation(
        &self,
        team_id: i64,
        email: &str,
        role: &str,
        code: &str,
        invited_by: Option<i64>,
    ) -> anyhow::Result<InvitationRecord> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO invitations (team_id, email, role, code, invited_by, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING id",
                &[&team_id, &email, &role, &code, &invited_by, &now],
            )
            .await?;
        Ok(InvitationRecord {
            id: row.get(0),
            team_id,
            email: email.to_string(),
            role: role.to_string(),
            code: code.to_string(),
            invited_by,
            created_at: now,
            accepted_at: None,
        })
    }

    pub async fn list_invitations(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<InvitationRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT id, team_id, email, role, code, invited_by, created_at, accepted_at
                 FROM invitations WHERE team_id = $1 AND accepted_at IS NULL
                 ORDER BY created_at DESC",
                &[&team_id],
            )
            .await?;
        Ok(rows.into_iter().map(map_invitation).collect())
    }

    pub async fn get_invitation_by_code(
        &self,
        code: &str,
    ) -> anyhow::Result<Option<InvitationRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, team_id, email, role, code, invited_by, created_at, accepted_at
                 FROM invitations WHERE code = $1",
                &[&code],
            )
            .await?;
        Ok(row.map(map_invitation))
    }

    pub async fn accept_invitation(&self, code: &str) -> anyhow::Result<()> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE invitations SET accepted_at = $1 WHERE code = $2",
                &[&now, &code],
            )
            .await?;
        Ok(())
    }

    pub async fn cancel_invitation(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute("DELETE FROM invitations WHERE id = $1", &[&id])
            .await?;
        Ok(())
    }
}

fn map_invitation(row: Row) -> InvitationRecord {
    InvitationRecord {
        id: row.get(0),
        team_id: row.get(1),
        email: row.get(2),
        role: row.get(3),
        code: row.get(4),
        invited_by: row.get(5),
        created_at: row.get(6),
        accepted_at: row.get(7),
    }
}
