//! Tiny tokio-postgres connection pool for the orchestrator.
//!
//! PlanetScale Postgres requires TLS. We use rustls with the OS root store.
//! The pool is intentionally minimal — orchestrator workload is admin-grade
//! traffic, not high-throughput query serving.

use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use parking_lot::Mutex;
use rustls::{
    pki_types::CertificateDer,
    RootCertStore,
};
use tokio::sync::{
    Semaphore,
    SemaphorePermit,
};
use tokio_postgres::{
    Client,
    Config,
    NoTls,
};
use tokio_postgres_rustls::MakeRustlsConnect;

/// Maximum concurrent connections held by the pool.
const DEFAULT_MAX_CONNECTIONS: usize = 8;

#[derive(Clone)]
pub struct PgPool {
    inner: Arc<PgPoolInner>,
}

struct PgPoolInner {
    config: Config,
    use_tls: bool,
    tls: Option<MakeRustlsConnect>,
    semaphore: Semaphore,
    idle: Mutex<Vec<Client>>,
}

impl PgPool {
    /// Connect to the database described by the given URL. URL formats:
    ///   postgres://user:pass@host:5432/dbname
    ///   postgresql://user:pass@host:5432/dbname?sslmode=require
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let config: Config = database_url
            .parse()
            .with_context(|| format!("parsing database URL {database_url:?}"))?;

        // Detect TLS preference from sslmode query param. Default off for
        // local development, on for anything containing `sslmode=require`.
        let use_tls = database_url.contains("sslmode=require")
            || database_url.contains("sslmode=verify-ca")
            || database_url.contains("sslmode=verify-full");

        let tls = if use_tls { Some(make_tls()?) } else { None };

        let pool = Self {
            inner: Arc::new(PgPoolInner {
                config,
                use_tls,
                tls,
                semaphore: Semaphore::new(DEFAULT_MAX_CONNECTIONS),
                idle: Mutex::new(Vec::new()),
            }),
        };

        // Probe connectivity once so misconfiguration fails fast at startup.
        // Drop the conn explicitly so it's returned to the pool before we
        // move `pool` out of this function.
        {
            let _conn = pool.acquire().await?;
        }
        Ok(pool)
    }

    pub async fn acquire(&self) -> anyhow::Result<PgConn<'_>> {
        let permit = self
            .inner
            .semaphore
            .acquire()
            .await
            .context("acquiring postgres pool permit")?;

        // Reuse an idle client if available.
        if let Some(client) = self.inner.idle.lock().pop()
            && !client.is_closed()
        {
            return Ok(PgConn {
                pool: self.inner.clone(),
                client: Some(client),
                _permit: permit,
            });
        }

        // Otherwise open a fresh connection.
        let client = self.connect_one().await?;
        Ok(PgConn {
            pool: self.inner.clone(),
            client: Some(client),
            _permit: permit,
        })
    }

    async fn connect_one(&self) -> anyhow::Result<Client> {
        if self.inner.use_tls {
            let tls = self.inner.tls.as_ref().expect("tls configured");
            let (client, conn) = self
                .inner
                .config
                .connect(tls.clone())
                .await
                .context("connecting to postgres (tls)")?;
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::warn!(error = %e, "postgres connection task ended");
                }
            });
            Ok(client)
        } else {
            let (client, conn) = self
                .inner
                .config
                .connect(NoTls)
                .await
                .context("connecting to postgres (notls)")?;
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::warn!(error = %e, "postgres connection task ended");
                }
            });
            Ok(client)
        }
    }
}

pub struct PgConn<'a> {
    pool: Arc<PgPoolInner>,
    client: Option<Client>,
    _permit: SemaphorePermit<'a>,
}

impl PgConn<'_> {
    pub fn client(&self) -> &Client {
        self.client.as_ref().expect("client present")
    }

    pub fn client_mut(&mut self) -> &mut Client {
        self.client.as_mut().expect("client present")
    }
}

impl Drop for PgConn<'_> {
    fn drop(&mut self) {
        if let Some(client) = self.client.take()
            && !client.is_closed()
        {
            self.pool.idle.lock().push(client);
        }
    }
}

fn make_tls() -> anyhow::Result<MakeRustlsConnect> {
    let mut roots = RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs();
    if !certs.errors.is_empty() {
        tracing::warn!(
            "rustls-native-certs reported {} errors loading roots",
            certs.errors.len()
        );
    }
    for cert in certs.certs {
        let cert: CertificateDer<'static> = cert.into_owned();
        let _ = roots.add(cert);
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(MakeRustlsConnect::new(config))
}

/// Sleep helper for retry loops that don't have access to tokio::time
/// directly; not used in v1 but exposed for future migrations module.
#[allow(dead_code)]
pub async fn small_sleep() {
    tokio::time::sleep(Duration::from_millis(50)).await;
}
