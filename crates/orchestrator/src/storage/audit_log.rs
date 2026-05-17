use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: i64,
    pub team_id: i64,
    pub member_id: Option<i64>,
    pub action: String,
    pub metadata: serde_json::Value,
    pub creation_time: i64,
}

#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub team_id: i64,
    pub member_id: Option<i64>,
    pub action: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
}

impl Storage {
    pub async fn append_audit(
        &self,
        team_id: i64,
        member_id: Option<i64>,
        action: &str,
        metadata: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "INSERT INTO audit_log_events (team_id, member_id, action, metadata, creation_time)
                 VALUES ($1, $2, $3, $4, $5)",
                &[&team_id, &member_id, &action, metadata, &now],
            )
            .await?;
        Ok(())
    }

    pub async fn query_audit(&self, q: &AuditQuery) -> anyhow::Result<Vec<AuditEntry>> {
        let limit = q.limit.unwrap_or(100).min(1000);
        let mut sql = String::from(
            "SELECT id, team_id, member_id, action, metadata, creation_time \
             FROM audit_log_events WHERE team_id = $1",
        );
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            vec![Box::new(q.team_id)];
        if let Some(m) = q.member_id {
            params.push(Box::new(m));
            sql.push_str(&format!(" AND member_id = ${}", params.len()));
        }
        if let Some(action) = &q.action {
            params.push(Box::new(action.clone()));
            sql.push_str(&format!(" AND action = ${}", params.len()));
        }
        if let Some(from) = q.from {
            params.push(Box::new(from));
            sql.push_str(&format!(" AND creation_time >= ${}", params.len()));
        }
        if let Some(to) = q.to {
            params.push(Box::new(to));
            sql.push_str(&format!(" AND creation_time <= ${}", params.len()));
        }
        sql.push_str(" ORDER BY creation_time DESC LIMIT ");
        sql.push_str(&limit.to_string());

        let conn = self.pool().acquire().await?;
        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|b| b.as_ref() as _).collect();
        let rows = conn.client().query(&sql, &params_refs[..]).await?;
        Ok(rows
            .into_iter()
            .map(|r| AuditEntry {
                id: r.get(0),
                team_id: r.get(1),
                member_id: r.get(2),
                action: r.get(3),
                metadata: r.get(4),
                creation_time: r.get(5),
            })
            .collect())
    }
}
