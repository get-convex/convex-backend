use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct CustomDomainRecord {
    pub id: i64,
    pub deployment_id: i64,
    pub domain: String,
    pub cert_state: String,
    pub created_at: i64,
}

impl Storage {
    pub async fn create_custom_domain(
        &self,
        deployment_id: i64,
        domain: &str,
    ) -> anyhow::Result<CustomDomainRecord> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO custom_domains (deployment_id, domain, cert_state, created_at)
                 VALUES ($1, $2, 'pending', $3)
                 RETURNING id",
                &[&deployment_id, &domain, &now],
            )
            .await?;
        Ok(CustomDomainRecord {
            id: row.get(0),
            deployment_id,
            domain: domain.to_string(),
            cert_state: "pending".to_string(),
            created_at: now,
        })
    }

    pub async fn delete_custom_domain(
        &self,
        deployment_id: i64,
        domain: &str,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "DELETE FROM custom_domains WHERE deployment_id = $1 AND domain = $2",
                &[&deployment_id, &domain],
            )
            .await?;
        Ok(())
    }

    pub async fn list_custom_domains(
        &self,
        deployment_id: i64,
    ) -> anyhow::Result<Vec<CustomDomainRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT id, deployment_id, domain, cert_state, created_at
                 FROM custom_domains WHERE deployment_id = $1",
                &[&deployment_id],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| CustomDomainRecord {
                id: r.get(0),
                deployment_id: r.get(1),
                domain: r.get(2),
                cert_state: r.get(3),
                created_at: r.get(4),
            })
            .collect())
    }
}
