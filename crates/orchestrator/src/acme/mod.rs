//! ACME certificate issuance for custom domains.
//!
//! # Why the orchestrator does this instead of Traefik
//!
//! Traefik can absolutely run ACME itself — but `certificatesResolvers` is
//! *static* configuration. Adding a provider or rotating a DNS API token
//! means editing the compose file and restarting Traefik. That rules it out
//! for a dashboard-driven workflow.
//!
//! Traefik's *dynamic* (file) provider, by contrast, hot-reloads routers and
//! `tls.certificates`. So the orchestrator owns the ACME conversation and
//! hands Traefik finished certificates. Consequences:
//!
//! - DNS credentials live in Postgres (sealed), entered in the dashboard, and
//!   take effect on the next issuance — no restart, no SSH.
//! - Adding a DNS provider is a code change here, not an operator-facing
//!   configuration change.
//! - Custom domains need *zero* static Traefik configuration beyond pointing
//!   the file provider at a directory.
//!
//! # Challenges
//!
//! - `http-01` needs no credentials at all. Traefik routes
//!   `/.well-known/acme-challenge/` for the domain to the orchestrator (via a
//!   dynamic router it writes itself), which serves the token from
//!   [`ChallengeStore`].
//! - `dns-01` works when port 80 isn't reachable, and is the only way to get
//!   a wildcard. It needs a provider token — see [`dns_providers`].

pub mod dns_providers;
pub mod renewal;

use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use instant_acme::{
    Account,
    AccountCredentials,
    AuthorizationStatus,
    ChallengeType,
    Identifier,
    LetsEncrypt,
    NewAccount,
    NewOrder,
    OrderStatus,
};
use parking_lot::Mutex;

use crate::{
    acme::dns_providers::{
        DnsProvider,
        Provider,
        Secrets,
    },
    state::OrchestratorState,
    time::now_unix_ms,
};

/// Certificates are renewed well before the 90-day Let's Encrypt lifetime so
/// a few failed attempts don't turn into an outage.
const RENEW_AFTER_MS: i64 = 60 * 24 * 60 * 60 * 1000;

/// How long to wait for the ACME server to validate a challenge before
/// giving up. DNS propagation is the slow part.
const VALIDATION_TIMEOUT: Duration = Duration::from_secs(180);
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// Gap between publishing a TXT record and telling the ACME server to check
/// it. Without this the server frequently reads a stale negative answer from
/// its resolver cache and fails the authorization outright.
const DNS_PROPAGATION_DELAY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeKind {
    Http01,
    Dns01,
}

impl ChallengeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChallengeKind::Http01 => "http-01",
            ChallengeKind::Dns01 => "dns-01",
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "http-01" => Ok(ChallengeKind::Http01),
            "dns-01" => Ok(ChallengeKind::Dns01),
            other => anyhow::bail!("unknown challenge type {other:?}"),
        }
    }
}

/// In-flight HTTP-01 tokens, keyed by challenge token. Populated for the
/// duration of an issuance and served by the orchestrator's
/// `/.well-known/acme-challenge/{token}` route.
#[derive(Clone, Default)]
pub struct ChallengeStore(Arc<Mutex<HashMap<String, String>>>);

impl ChallengeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, token: String, key_authorization: String) {
        self.0.lock().insert(token, key_authorization);
    }

    pub fn get(&self, token: &str) -> Option<String> {
        self.0.lock().get(token).cloned()
    }

    pub fn remove(&self, token: &str) {
        self.0.lock().remove(token);
    }
}

pub struct IssuedCertificate {
    pub cert_pem: String,
    pub key_pem: String,
    pub issued_at: i64,
    pub renew_after: i64,
}

/// Loads the stored ACME account, creating and persisting one on first use.
///
/// The account key is what proves to Let's Encrypt that we're the same
/// client across renewals, so it's sealed with the same secretbox the DNS
/// credentials use.
async fn account(state: &OrchestratorState) -> anyhow::Result<Account> {
    let directory = directory_url(state);

    if let Some(record) = state
        .storage
        .get_acme_account(&directory)
        .await
        .context("loading ACME account")?
    {
        let plaintext = state
            .secrets
            .open(&record.credentials)
            .context("decrypting ACME account credentials")?;
        let credentials: AccountCredentials = serde_json::from_slice(&plaintext)
            .context("parsing stored ACME account credentials")?;
        return Account::from_credentials(credentials)
            .await
            .context("restoring ACME account");
    }

    let contact = state
        .config
        .acme_contact_email
        .as_ref()
        .map(|e| format!("mailto:{e}"));
    let contacts: Vec<&str> = contact.iter().map(|s| s.as_str()).collect();

    let (acct, credentials) = Account::create(
        &NewAccount {
            contact: &contacts,
            terms_of_service_agreed: true,
            only_return_existing: false,
        },
        &directory,
        None,
    )
    .await
    .context("creating ACME account")?;

    let sealed = state
        .secrets
        .seal(&serde_json::to_vec(&credentials).context("serializing ACME credentials")?);
    state
        .storage
        .upsert_acme_account(&directory, acct.id(), &sealed)
        .await
        .context("storing ACME account")?;

    Ok(acct)
}

fn directory_url(state: &OrchestratorState) -> String {
    state
        .config
        .acme_directory_url
        .clone()
        .unwrap_or_else(|| LetsEncrypt::Production.url().to_string())
}

/// Runs a full ACME order for `domain` and returns the issued certificate.
///
/// `dns` must be supplied for [`ChallengeKind::Dns01`]; it's ignored for
/// HTTP-01, which needs no credentials.
pub async fn issue(
    state: &OrchestratorState,
    domain: &str,
    challenge_kind: ChallengeKind,
    dns: Option<(Provider, Secrets)>,
) -> anyhow::Result<IssuedCertificate> {
    let account = account(state).await?;

    let identifiers = [Identifier::Dns(domain.to_string())];
    let mut order = account
        .new_order(&NewOrder {
            identifiers: &identifiers,
        })
        .await
        .context("creating ACME order")?;

    let authorizations = order
        .authorizations()
        .await
        .context("fetching ACME authorizations")?;

    // Tracks what we published so cleanup runs even on the error paths.
    let mut http_tokens: Vec<String> = Vec::new();
    let mut dns_records: Vec<(Box<dyn DnsProvider>, dns_providers::TxtRecord)> = Vec::new();

    let result = async {
        let mut needs_dns_delay = false;

        for authz in &authorizations {
            if authz.status == AuthorizationStatus::Valid {
                // Already validated from a previous order — nothing to do.
                continue;
            }

            let wanted = match challenge_kind {
                ChallengeKind::Http01 => ChallengeType::Http01,
                ChallengeKind::Dns01 => ChallengeType::Dns01,
            };
            let challenge = authz
                .challenges
                .iter()
                .find(|c| c.r#type == wanted)
                .with_context(|| {
                    format!(
                        "the ACME server offered no {} challenge for this domain",
                        challenge_kind.as_str()
                    )
                })?;

            let key_auth = order.key_authorization(challenge);
            let Identifier::Dns(authz_domain) = &authz.identifier;

            match challenge_kind {
                ChallengeKind::Http01 => {
                    state
                        .challenges
                        .insert(challenge.token.clone(), key_auth.as_str().to_string());
                    http_tokens.push(challenge.token.clone());
                },
                ChallengeKind::Dns01 => {
                    let (provider, secrets) = dns.as_ref().context(
                        "the dns-01 challenge needs DNS provider credentials, but none were \
                         selected",
                    )?;
                    let client = dns_providers::build(*provider, secrets)?;
                    let record = client
                        .create_txt(authz_domain, &key_auth.dns_value())
                        .await
                        .with_context(|| format!("publishing the DNS-01 record for {authz_domain}"))?;
                    dns_records.push((client, record));
                    needs_dns_delay = true;
                },
            }
        }

        // Give recursive resolvers a chance to see the new TXT record before
        // the ACME server queries them.
        if needs_dns_delay {
            tokio::time::sleep(DNS_PROPAGATION_DELAY).await;
        }

        for authz in &authorizations {
            if authz.status == AuthorizationStatus::Valid {
                continue;
            }
            let wanted = match challenge_kind {
                ChallengeKind::Http01 => ChallengeType::Http01,
                ChallengeKind::Dns01 => ChallengeType::Dns01,
            };
            if let Some(challenge) = authz.challenges.iter().find(|c| c.r#type == wanted) {
                let url = challenge.url.clone();
                order
                    .set_challenge_ready(&url)
                    .await
                    .context("telling the ACME server the challenge is ready")?;
            }
        }

        wait_until_ready(&mut order).await?;

        // Order is authorized; generate a fresh keypair and CSR for it.
        let params = rcgen::CertificateParams::new(vec![domain.to_string()])
            .context("building certificate params")?;
        let keypair = rcgen::KeyPair::generate().context("generating certificate key")?;
        let csr = params
            .serialize_request(&keypair)
            .context("building certificate signing request")?;

        order
            .finalize(csr.der())
            .await
            .context("finalizing the ACME order")?;

        let cert_pem = wait_for_certificate(&mut order).await?;
        let issued_at = now_unix_ms();

        Ok::<_, anyhow::Error>(IssuedCertificate {
            cert_pem,
            key_pem: keypair.serialize_pem(),
            issued_at,
            renew_after: issued_at + RENEW_AFTER_MS,
        })
    }
    .await;

    // Cleanup regardless of outcome. Neither failure is worth surfacing over
    // the actual issuance result: a leftover TXT record or in-memory token is
    // untidy, not broken.
    for token in http_tokens {
        state.challenges.remove(&token);
    }
    for (client, record) in dns_records {
        if let Err(e) = client.delete_txt(&record).await {
            tracing::warn!(error = %e, "failed to clean up DNS-01 challenge record");
        }
    }

    result
}

async fn wait_until_ready(order: &mut instant_acme::Order) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + VALIDATION_TIMEOUT;
    loop {
        let state = order.refresh().await.context("polling ACME order")?;
        match state.status {
            OrderStatus::Ready | OrderStatus::Valid => return Ok(()),
            OrderStatus::Invalid => {
                anyhow::bail!(
                    "the ACME server could not validate this domain{}",
                    state
                        .error
                        .as_ref()
                        .map(|e| format!(": {}", e.detail.as_deref().unwrap_or("no detail")))
                        .unwrap_or_default()
                )
            },
            OrderStatus::Pending | OrderStatus::Processing => {
                anyhow::ensure!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for the ACME server to validate this domain — check that \
                     DNS points here (http-01) or that the provider token can edit this zone \
                     (dns-01)"
                );
                tokio::time::sleep(POLL_INTERVAL).await;
            },
        }
    }
}

async fn wait_for_certificate(order: &mut instant_acme::Order) -> anyhow::Result<String> {
    let deadline = tokio::time::Instant::now() + VALIDATION_TIMEOUT;
    loop {
        if let Some(pem) = order.certificate().await.context("downloading certificate")? {
            return Ok(pem);
        }
        anyhow::ensure!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for the ACME server to sign the certificate"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_kinds_round_trip() {
        for kind in [ChallengeKind::Http01, ChallengeKind::Dns01] {
            assert_eq!(ChallengeKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(ChallengeKind::parse("tls-alpn-01").is_err());
    }

    #[test]
    fn challenge_store_serves_then_forgets_tokens() {
        let store = ChallengeStore::new();
        store.insert("tok".into(), "tok.thumbprint".into());
        assert_eq!(store.get("tok").as_deref(), Some("tok.thumbprint"));
        store.remove("tok");
        assert_eq!(store.get("tok"), None);
    }

    #[test]
    fn renewal_is_well_inside_the_certificate_lifetime() {
        // Let's Encrypt issues for 90 days; renewing at 60 leaves a month of
        // failed attempts before anything actually breaks.
        assert!(RENEW_AFTER_MS < 90 * 24 * 60 * 60 * 1000);
    }
}
