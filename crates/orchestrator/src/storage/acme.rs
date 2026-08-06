//! Storage for ACME state: the account key, issued certificates, and the DNS
//! provider credentials used for the dns-01 challenge.
//!
//! Sealed columns (`secrets`, `credentials`) are returned as raw bytes; only
//! `SecretSealer` can open them, and nothing here ever hands a decrypted
//! secret to an API response.

use super::Storage;
use crate::time::now_unix_ms;

#[derive(Debug, Clone)]
pub struct AcmeAccountRecord {
    pub account_url: String,
    pub credentials: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DnsCredentialRecord {
    pub id: i64,
    pub team_id: i64,
    pub name: String,
    pub provider: String,
    pub created_at: i64,
}

/// Separate from [`DnsCredentialRecord`] so the sealed blob is only loaded
/// when an issuance actually needs it — list endpoints can't leak what they
/// never fetch.
#[derive(Debug, Clone)]
pub struct DnsCredentialSecrets {
    pub provider: String,
    pub sealed: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StoredCertificate {
    pub domain: String,
    pub cert_pem: String,
    pub key_pem: String,
    pub issued_at: i64,
    pub renew_after: i64,
}

impl Storage {
    // ---------- ACME account ----------

    pub async fn get_acme_account(
        &self,
        directory_url: &str,
    ) -> anyhow::Result<Option<AcmeAccountRecord>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT account_url, credentials FROM acme_accounts WHERE directory_url = $1",
                &[&directory_url],
            )
            .await?;
        Ok(row.map(|r| AcmeAccountRecord {
            account_url: r.get(0),
            credentials: r.get(1),
        }))
    }

    pub async fn upsert_acme_account(
        &self,
        directory_url: &str,
        account_url: &str,
        credentials: &[u8],
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "INSERT INTO acme_accounts (directory_url, account_url, credentials, created_at)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (directory_url) DO UPDATE
                   SET account_url = EXCLUDED.account_url,
                       credentials = EXCLUDED.credentials",
                &[
                    &directory_url,
                    &account_url,
                    &credentials.to_vec(),
                    &now_unix_ms(),
                ],
            )
            .await?;
        Ok(())
    }

    // ---------- DNS provider credentials ----------

    pub async fn list_dns_credentials(
        &self,
        team_id: i64,
    ) -> anyhow::Result<Vec<DnsCredentialRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT id, team_id, name, provider, created_at
                 FROM dns_provider_credentials WHERE team_id = $1 ORDER BY name",
                &[&team_id],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| DnsCredentialRecord {
                id: r.get(0),
                team_id: r.get(1),
                name: r.get(2),
                provider: r.get(3),
                created_at: r.get(4),
            })
            .collect())
    }

    pub async fn create_dns_credential(
        &self,
        team_id: i64,
        name: &str,
        provider: &str,
        sealed: &[u8],
    ) -> anyhow::Result<DnsCredentialRecord> {
        let now = now_unix_ms();
        let conn = self.pool().acquire().await?;
        // Re-saving under the same name rotates the token rather than
        // erroring, which is what an operator pasting a fresh token expects.
        let row = conn
            .client()
            .query_one(
                "INSERT INTO dns_provider_credentials
                     (team_id, name, provider, secrets, created_at)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (team_id, name) DO UPDATE
                   SET provider = EXCLUDED.provider,
                       secrets = EXCLUDED.secrets
                 RETURNING id, created_at",
                &[&team_id, &name, &provider, &sealed.to_vec(), &now],
            )
            .await?;
        Ok(DnsCredentialRecord {
            id: row.get(0),
            team_id,
            name: name.to_string(),
            provider: provider.to_string(),
            created_at: row.get(1),
        })
    }

    pub async fn delete_dns_credential(&self, team_id: i64, id: i64) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "DELETE FROM dns_provider_credentials WHERE team_id = $1 AND id = $2",
                &[&team_id, &id],
            )
            .await?;
        Ok(())
    }

    pub async fn get_dns_credential_secrets(
        &self,
        id: i64,
    ) -> anyhow::Result<Option<DnsCredentialSecrets>> {
        let conn = self.pool().acquire().await?;
        let row = conn
            .client()
            .query_opt(
                "SELECT provider, secrets FROM dns_provider_credentials WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(|r| DnsCredentialSecrets {
            provider: r.get(0),
            sealed: r.get(1),
        }))
    }

    // ---------- Certificates ----------

    pub async fn upsert_certificate(
        &self,
        domain: &str,
        cert_pem: &str,
        key_pem: &str,
        issued_at: i64,
        renew_after: i64,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "INSERT INTO custom_domain_certs
                     (domain, cert_pem, key_pem, issued_at, renew_after)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (domain) DO UPDATE
                   SET cert_pem = EXCLUDED.cert_pem,
                       key_pem = EXCLUDED.key_pem,
                       issued_at = EXCLUDED.issued_at,
                       renew_after = EXCLUDED.renew_after",
                &[&domain, &cert_pem, &key_pem, &issued_at, &renew_after],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_certificate(&self, domain: &str) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "DELETE FROM custom_domain_certs WHERE domain = $1",
                &[&domain],
            )
            .await?;
        Ok(())
    }

    /// Every stored certificate. Used to rebuild the Traefik dynamic
    /// directory, which may be an empty volume on a fresh host.
    pub async fn list_certificates(&self) -> anyhow::Result<Vec<StoredCertificate>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT domain, cert_pem, key_pem, issued_at, renew_after
                 FROM custom_domain_certs ORDER BY domain",
                &[],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| StoredCertificate {
                domain: r.get(0),
                cert_pem: r.get(1),
                key_pem: r.get(2),
                issued_at: r.get(3),
                renew_after: r.get(4),
            })
            .collect())
    }

    /// Domains whose certificate is due for renewal, plus domains that have
    /// no certificate at all (a first issuance that failed, or one added
    /// while the ACME server was unreachable).
    pub async fn domains_needing_certificates(
        &self,
        now: i64,
    ) -> anyhow::Result<Vec<super::CustomDomainRecord>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT cd.id, cd.deployment_id, cd.domain, cd.cert_state, cd.created_at,
                        cd.challenge_type, cd.dns_credential_id, cd.last_error
                 FROM custom_domains cd
                 LEFT JOIN custom_domain_certs c ON c.domain = cd.domain
                 WHERE c.domain IS NULL OR c.renew_after <= $1
                 ORDER BY cd.domain",
                &[&now],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| super::CustomDomainRecord {
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
