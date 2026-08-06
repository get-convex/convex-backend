//! DNS providers for the ACME DNS-01 challenge.
//!
//! The orchestrator performs the challenge itself rather than delegating to
//! Traefik, because Traefik reads DNS credentials from static configuration
//! and would need a restart whenever they change. Doing it here means an
//! operator can paste an API token into the dashboard and have the next
//! certificate use it immediately.
//!
//! Each provider only has to create and delete a TXT record. Adding one is a
//! new `Provider` match arm plus a `DnsProvider` impl.

use std::{
    collections::BTreeMap,
    fmt,
};

use anyhow::Context;
use async_trait::async_trait;
use serde::{
    Deserialize,
    Serialize,
};

/// Providers the orchestrator can drive. The string form is what's stored in
/// `dns_provider_credentials.provider` and sent over the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Cloudflare,
    DigitalOcean,
    Hetzner,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Cloudflare => "cloudflare",
            Provider::DigitalOcean => "digitalocean",
            Provider::Hetzner => "hetzner",
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "cloudflare" => Ok(Provider::Cloudflare),
            "digitalocean" => Ok(Provider::DigitalOcean),
            "hetzner" => Ok(Provider::Hetzner),
            other => anyhow::bail!("unknown DNS provider {other:?}"),
        }
    }

    /// Secret fields this provider needs, in the order the dashboard should
    /// render them. Driving the form from here means adding a provider
    /// doesn't require a matching dashboard change.
    pub fn required_fields(self) -> &'static [ProviderField] {
        match self {
            Provider::Cloudflare => &[ProviderField {
                key: "apiToken",
                label: "API token",
                help: "Cloudflare token with Zone:DNS:Edit and Zone:Zone:Read on the target zone.",
            }],
            Provider::DigitalOcean => &[ProviderField {
                key: "apiToken",
                label: "API token",
                help: "DigitalOcean personal access token with write scope.",
            }],
            Provider::Hetzner => &[ProviderField {
                key: "apiToken",
                label: "API token",
                help: "Hetzner DNS Console API token.",
            }],
        }
    }

    pub fn all() -> &'static [Provider] {
        &[
            Provider::Cloudflare,
            Provider::DigitalOcean,
            Provider::Hetzner,
        ]
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderField {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
}

/// Decrypted provider secrets. Held only for the duration of a challenge —
/// at rest these live sealed in `dns_provider_credentials.secrets`.
pub type Secrets = BTreeMap<String, String>;

/// A TXT record the orchestrator created and must clean up afterwards.
pub struct TxtRecord {
    pub provider_id: String,
    pub zone_id: String,
}

#[async_trait]
pub trait DnsProvider: Send + Sync {
    /// Creates `_acme-challenge.<domain>` TXT = `value`.
    async fn create_txt(&self, domain: &str, value: &str) -> anyhow::Result<TxtRecord>;

    /// Best-effort cleanup. Failure here is logged, never fatal: a stale
    /// challenge record is untidy but harmless, whereas failing the whole
    /// issuance over cleanup would be worse.
    async fn delete_txt(&self, record: &TxtRecord) -> anyhow::Result<()>;
}

pub fn build(provider: Provider, secrets: &Secrets) -> anyhow::Result<Box<dyn DnsProvider>> {
    let token = secrets
        .get("apiToken")
        .filter(|t| !t.trim().is_empty())
        .context("credential is missing `apiToken`")?
        .clone();

    Ok(match provider {
        Provider::Cloudflare => Box::new(Cloudflare::new(token)),
        Provider::DigitalOcean => Box::new(DigitalOcean::new(token)),
        Provider::Hetzner => Box::new(Hetzner::new(token)),
    })
}

/// The record name ACME looks for.
pub fn challenge_record_name(domain: &str) -> String {
    format!("_acme-challenge.{}", domain.trim_start_matches("*."))
}

/// Walks `a.b.example.com` -> `b.example.com` -> `example.com` -> `com`.
/// Providers key their APIs by registrable zone, but a custom domain can be
/// any depth of subdomain, so the zone has to be discovered by trying each
/// suffix rather than assuming the last two labels (which breaks on
/// `example.co.uk`).
fn zone_candidates(domain: &str) -> Vec<String> {
    let base = domain.trim_start_matches("*.");
    let labels: Vec<&str> = base.split('.').collect();
    (0..labels.len().saturating_sub(1))
        .map(|i| labels[i..].join("."))
        .collect()
}

fn http() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("building DNS provider HTTP client")
}

// ---------- Cloudflare ----------

struct Cloudflare {
    token: String,
}

impl Cloudflare {
    fn new(token: String) -> Self {
        Self { token }
    }

    async fn zone_id(&self, domain: &str) -> anyhow::Result<String> {
        let client = http()?;
        for candidate in zone_candidates(domain) {
            let res: serde_json::Value = client
                .get("https://api.cloudflare.com/client/v4/zones")
                .bearer_auth(&self.token)
                .query(&[("name", candidate.as_str())])
                .send()
                .await
                .context("querying Cloudflare zones")?
                .json()
                .await
                .context("parsing Cloudflare zone response")?;

            // A bad token fails identically for every candidate, so surface
            // Cloudflare's own message instead of the generic "no zone" below.
            if res.get("success").and_then(|v| v.as_bool()) == Some(false) {
                anyhow::bail!("Cloudflare rejected the token: {}", errors_of(&res));
            }
            if let Some(id) = res
                .get("result")
                .and_then(|r| r.as_array())
                .and_then(|a| a.first())
                .and_then(|z| z.get("id"))
                .and_then(|v| v.as_str())
            {
                return Ok(id.to_string());
            }
        }
        anyhow::bail!("no Cloudflare zone found for {domain} — is the token scoped to this zone?")
    }
}

fn errors_of(res: &serde_json::Value) -> String {
    res.get("errors")
        .and_then(|e| e.as_array())
        .map(|errs| {
            errs.iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown error".to_string())
}

#[async_trait]
impl DnsProvider for Cloudflare {
    async fn create_txt(&self, domain: &str, value: &str) -> anyhow::Result<TxtRecord> {
        let zone_id = self.zone_id(domain).await?;
        let res: serde_json::Value = http()?
            .post(format!(
                "https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records"
            ))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "type": "TXT",
                "name": challenge_record_name(domain),
                "content": value,
                "ttl": 60,
            }))
            .send()
            .await
            .context("creating Cloudflare TXT record")?
            .json()
            .await
            .context("parsing Cloudflare create response")?;

        if res.get("success").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("Cloudflare rejected the TXT record: {}", errors_of(&res));
        }

        let id = res
            .get("result")
            .and_then(|r| r.get("id"))
            .and_then(|v| v.as_str())
            .context("Cloudflare create response had no record id")?
            .to_string();

        Ok(TxtRecord {
            provider_id: id,
            zone_id,
        })
    }

    async fn delete_txt(&self, record: &TxtRecord) -> anyhow::Result<()> {
        http()?
            .delete(format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                record.zone_id, record.provider_id
            ))
            .bearer_auth(&self.token)
            .send()
            .await
            .context("deleting Cloudflare TXT record")?;
        Ok(())
    }
}

// ---------- DigitalOcean ----------

struct DigitalOcean {
    token: String,
}

impl DigitalOcean {
    fn new(token: String) -> Self {
        Self { token }
    }

    async fn zone(&self, domain: &str) -> anyhow::Result<String> {
        let client = http()?;
        for candidate in zone_candidates(domain) {
            let res = client
                .get(format!(
                    "https://api.digitalocean.com/v2/domains/{candidate}"
                ))
                .bearer_auth(&self.token)
                .send()
                .await
                .context("querying DigitalOcean domains")?;
            if res.status().is_success() {
                return Ok(candidate);
            }
            if res.status() == reqwest::StatusCode::UNAUTHORIZED {
                anyhow::bail!("DigitalOcean rejected the token");
            }
        }
        anyhow::bail!("no DigitalOcean domain found for {domain}")
    }
}

#[async_trait]
impl DnsProvider for DigitalOcean {
    async fn create_txt(&self, domain: &str, value: &str) -> anyhow::Result<TxtRecord> {
        let zone = self.zone(domain).await?;
        // DigitalOcean wants the record name relative to the zone.
        let full = challenge_record_name(domain);
        let name = full
            .strip_suffix(&format!(".{zone}"))
            .unwrap_or(&full)
            .to_string();

        let res: serde_json::Value = http()?
            .post(format!(
                "https://api.digitalocean.com/v2/domains/{zone}/records"
            ))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "type": "TXT",
                "name": name,
                "data": value,
                "ttl": 60,
            }))
            .send()
            .await
            .context("creating DigitalOcean TXT record")?
            .json()
            .await
            .context("parsing DigitalOcean create response")?;

        let id = res
            .get("domain_record")
            .and_then(|r| r.get("id"))
            .and_then(|v| v.as_i64())
            .context("DigitalOcean create response had no record id")?;

        Ok(TxtRecord {
            provider_id: id.to_string(),
            zone_id: zone,
        })
    }

    async fn delete_txt(&self, record: &TxtRecord) -> anyhow::Result<()> {
        http()?
            .delete(format!(
                "https://api.digitalocean.com/v2/domains/{}/records/{}",
                record.zone_id, record.provider_id
            ))
            .bearer_auth(&self.token)
            .send()
            .await
            .context("deleting DigitalOcean TXT record")?;
        Ok(())
    }
}

// ---------- Hetzner ----------

struct Hetzner {
    token: String,
}

impl Hetzner {
    fn new(token: String) -> Self {
        Self { token }
    }

    async fn zone_id(&self, domain: &str) -> anyhow::Result<String> {
        let res: serde_json::Value = http()?
            .get("https://dns.hetzner.com/api/v1/zones")
            .header("Auth-API-Token", &self.token)
            .send()
            .await
            .context("querying Hetzner zones")?
            .json()
            .await
            .context("parsing Hetzner zone response")?;

        let zones = res
            .get("zones")
            .and_then(|z| z.as_array())
            .context("Hetzner returned no zone list — is the token valid?")?;

        // Prefer the longest matching zone so a delegated subdomain zone wins
        // over its parent.
        for candidate in zone_candidates(domain) {
            if let Some(id) = zones
                .iter()
                .find(|z| z.get("name").and_then(|n| n.as_str()) == Some(candidate.as_str()))
                .and_then(|z| z.get("id"))
                .and_then(|v| v.as_str())
            {
                return Ok(id.to_string());
            }
        }
        anyhow::bail!("no Hetzner zone found for {domain}")
    }
}

#[async_trait]
impl DnsProvider for Hetzner {
    async fn create_txt(&self, domain: &str, value: &str) -> anyhow::Result<TxtRecord> {
        let zone_id = self.zone_id(domain).await?;
        let res: serde_json::Value = http()?
            .post("https://dns.hetzner.com/api/v1/records")
            .header("Auth-API-Token", &self.token)
            .json(&serde_json::json!({
                "zone_id": zone_id,
                "type": "TXT",
                "name": challenge_record_name(domain),
                "value": value,
                "ttl": 60,
            }))
            .send()
            .await
            .context("creating Hetzner TXT record")?
            .json()
            .await
            .context("parsing Hetzner create response")?;

        let id = res
            .get("record")
            .and_then(|r| r.get("id"))
            .and_then(|v| v.as_str())
            .context("Hetzner create response had no record id")?
            .to_string();

        Ok(TxtRecord {
            provider_id: id,
            zone_id,
        })
    }

    async fn delete_txt(&self, record: &TxtRecord) -> anyhow::Result<()> {
        http()?
            .delete(format!(
                "https://dns.hetzner.com/api/v1/records/{}",
                record.provider_id
            ))
            .header("Auth-API-Token", &self.token)
            .send()
            .await
            .context("deleting Hetzner TXT record")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_candidates_walk_up_from_most_specific() {
        assert_eq!(
            zone_candidates("a.b.example.com"),
            vec!["a.b.example.com", "b.example.com", "example.com"]
        );
    }

    #[test]
    fn zone_candidates_cover_multi_part_tlds() {
        // `example.co.uk` is the registrable zone here; assuming the last two
        // labels would look for `co.uk` and never find it.
        assert!(zone_candidates("api.example.co.uk").contains(&"example.co.uk".to_string()));
    }

    #[test]
    fn challenge_record_is_prefixed_and_unwildcarded() {
        assert_eq!(
            challenge_record_name("api.example.com"),
            "_acme-challenge.api.example.com"
        );
        assert_eq!(
            challenge_record_name("*.example.com"),
            "_acme-challenge.example.com"
        );
    }

    #[test]
    fn providers_round_trip_through_their_string_form() {
        for p in Provider::all() {
            assert_eq!(Provider::parse(p.as_str()).unwrap(), *p);
            assert!(!p.required_fields().is_empty());
        }
        assert!(Provider::parse("route53").is_err());
    }

    #[test]
    fn build_rejects_blank_tokens() {
        let mut secrets = Secrets::new();
        secrets.insert("apiToken".into(), "   ".into());
        assert!(build(Provider::Cloudflare, &secrets).is_err());
        assert!(build(Provider::Cloudflare, &Secrets::new()).is_err());
    }
}
