use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct OptInRecord {
    pub member_id: i64,
    pub name: String,
    pub accepted_at: i64,
}

impl Storage {
    pub async fn list_opt_ins(&self, member_id: i64) -> anyhow::Result<Vec<OptInRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT member_id, name, accepted_at FROM opt_ins WHERE member_id = $1",
                &[&member_id],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| OptInRecord {
                member_id: r.get(0),
                name: r.get(1),
                accepted_at: r.get(2),
            })
            .collect())
    }

    pub async fn accept_opt_in(&self, member_id: i64, name: &str) -> anyhow::Result<()> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "INSERT INTO opt_ins (member_id, name, accepted_at) VALUES ($1, $2, $3)
                 ON CONFLICT (member_id, name) DO UPDATE SET accepted_at = EXCLUDED.accepted_at",
                &[&member_id, &name, &now],
            )
            .await?;
        Ok(())
    }
}
