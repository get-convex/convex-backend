use tokio_postgres::Row;

use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct MemberRecord {
    pub id: i64,
    pub auth_user_id: String,
    pub primary_email: String,
    pub name: Option<String>,
    pub creation_time: i64,
    pub deleted: bool,
}

impl Storage {
    /// Find-or-create a member by their BetterAuth `auth_user_id`. Email and
    /// name are updated if they've changed upstream.
    pub async fn upsert_member(
        &self,
        auth_user_id: &str,
        email: &str,
        name: Option<&str>,
    ) -> anyhow::Result<MemberRecord> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO members (auth_user_id, primary_email, name, creation_time)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (auth_user_id) DO UPDATE SET
                     primary_email = EXCLUDED.primary_email,
                     name = COALESCE(EXCLUDED.name, members.name)
                 RETURNING id, auth_user_id, primary_email, name, creation_time, deleted",
                &[&auth_user_id, &email, &name, &now],
            )
            .await?;
        Ok(map_member(row))
    }

    pub async fn get_member(&self, id: i64) -> anyhow::Result<Option<MemberRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, auth_user_id, primary_email, name, creation_time, deleted
                 FROM members WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(map_member))
    }

    pub async fn get_member_by_auth_user_id(
        &self,
        auth_user_id: &str,
    ) -> anyhow::Result<Option<MemberRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, auth_user_id, primary_email, name, creation_time, deleted
                 FROM members WHERE auth_user_id = $1 AND deleted = FALSE",
                &[&auth_user_id],
            )
            .await?;
        Ok(row.map(map_member))
    }

    pub async fn get_member_by_email(
        &self,
        email: &str,
    ) -> anyhow::Result<Option<MemberRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, auth_user_id, primary_email, name, creation_time, deleted
                 FROM members WHERE primary_email = $1 AND deleted = FALSE",
                &[&email],
            )
            .await?;
        Ok(row.map(map_member))
    }

    pub async fn count_members(&self) -> anyhow::Result<i64> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "SELECT COUNT(*)::BIGINT FROM members WHERE deleted = FALSE",
                &[],
            )
            .await?;
        Ok(row.get(0))
    }

    pub async fn update_member_name(&self, id: i64, name: &str) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE members SET name = $1 WHERE id = $2",
                &[&name, &id],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_member(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE members SET deleted = TRUE WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(())
    }
}

fn map_member(row: Row) -> MemberRecord {
    MemberRecord {
        id: row.get(0),
        auth_user_id: row.get(1),
        primary_email: row.get(2),
        name: row.get(3),
        creation_time: row.get(4),
        deleted: row.get(5),
    }
}
