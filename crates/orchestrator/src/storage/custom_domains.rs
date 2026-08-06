use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct CustomDomainRecord {
    pub id: i64,
    pub deployment_id: i64,
    pub domain: String,
    pub cert_state: String,
    pub created_at: i64,
    /// `http-01` or `dns-01`. Chosen per domain in the dashboard.
    pub challenge_type: String,
    /// DNS provider credential used for `dns-01`. `None` for `http-01`,
    /// which needs no credentials at all.
    pub dns_credential_id: Option<i64>,
    /// Why the last issuance attempt failed, verbatim. Only the operator can
    /// fix the usual causes (DNS not pointed here, token lacks zone access).
    pub last_error: Option<String>,
}

/// A custom domain plus the name of the deployment it fronts. Rendering the
/// Traefik file-provider config needs the deployment name to derive the
/// upstream container host, so the join happens in SQL rather than N+1
/// lookups per domain.
#[derive(Debug, Clone)]
pub struct CustomDomainRoute {
    pub domain: String,
    pub deployment_name: String,
}

impl Storage {
    pub async fn create_custom_domain(
        &self,
        deployment_id: i64,
        domain: &str,
        challenge_type: &str,
        dns_credential_id: Option<i64>,
    ) -> anyhow::Result<CustomDomainRecord> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_one(
                "INSERT INTO custom_domains
                     (deployment_id, domain, cert_state, created_at, challenge_type,
                      dns_credential_id)
                 VALUES ($1, $2, 'pending', $3, $4, $5)
                 RETURNING id",
                &[
                    &deployment_id,
                    &domain,
                    &now,
                    &challenge_type,
                    &dns_credential_id,
                ],
            )
            .await?;
        Ok(CustomDomainRecord {
            id: row.get(0),
            deployment_id,
            domain: domain.to_string(),
            cert_state: "pending".to_string(),
            created_at: now,
            challenge_type: challenge_type.to_string(),
            dns_credential_id,
            last_error: None,
        })
    }

    /// Records the result of an issuance attempt. `cert_state` is only ever
    /// written from an observed outcome, never assumed.
    pub async fn set_custom_domain_status(
        &self,
        domain: &str,
        cert_state: &str,
        last_error: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE custom_domains SET cert_state = $2, last_error = $3 WHERE domain = $1",
                &[&domain, &cert_state, &last_error],
            )
            .await?;
        Ok(())
    }

    pub async fn get_custom_domain(
        &self,
        domain: &str,
    ) -> anyhow::Result<Option<CustomDomainRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT id, deployment_id, domain, cert_state, created_at, challenge_type,
                        dns_credential_id, last_error
                 FROM custom_domains WHERE domain = $1",
                &[&domain],
            )
            .await?;
        Ok(row.map(|r| CustomDomainRecord {
            id: r.get(0),
            deployment_id: r.get(1),
            domain: r.get(2),
            cert_state: r.get(3),
            created_at: r.get(4),
            challenge_type: r.get(5),
            dns_credential_id: r.get(6),
            last_error: r.get(7),
        }))
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

    /// Every custom domain across all deployments, joined to the deployment
    /// name. Used to re-render the Traefik dynamic config, which is written
    /// whole rather than patched per-domain.
    pub async fn list_all_custom_domain_routes(&self) -> anyhow::Result<Vec<CustomDomainRoute>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT cd.domain, d.name
                 FROM custom_domains cd
                 JOIN deployments d ON d.id = cd.deployment_id
                 ORDER BY cd.domain",
                &[],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| CustomDomainRoute {
                domain: r.get(0),
                deployment_name: r.get(1),
            })
            .collect())
    }

    /// Records the outcome of a reachability probe. `cert_state` is the only
    /// signal the dashboard has for whether ACME actually issued a cert, so
    /// it is set from an observed HTTPS response rather than optimistically
    /// on insert.
    pub async fn set_custom_domain_cert_state(
        &self,
        deployment_id: i64,
        domain: &str,
        cert_state: &str,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "UPDATE custom_domains SET cert_state = $3
                 WHERE deployment_id = $1 AND domain = $2",
                &[&deployment_id, &domain, &cert_state],
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
                "SELECT id, deployment_id, domain, cert_state, created_at, challenge_type,
                        dns_credential_id, last_error
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
                challenge_type: r.get(5),
                dns_credential_id: r.get(6),
                last_error: r.get(7),
            })
            .collect())
    }
}
