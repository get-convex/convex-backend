use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use axum_extra::headers::Authorization;
use common::{
    http::{
        ConvexHttpService,
        HttpError,
        NoopRouteMapper,
    },
    shutdown::ShutdownSignal,
    testing::TestPersistence,
    types::MemberId,
};
use http::{
    Request,
    StatusCode,
};
use http_body_util::BodyExt;
use metrics::SERVER_VERSION_STR;
use runtime::prod::ProdRuntime;
use serde::de::DeserializeOwned;
use sync_types::headers::ConvexAdminAuthorization;
use tower::ServiceExt;

use crate::{
    config::LocalConfig,
    make_app,
    router::router,
    LocalAppState,
    MAX_CONCURRENT_REQUESTS,
};

pub struct TestLocalBackend {
    app: ConvexHttpService,
    pub st: LocalAppState,
    pub admin_auth_header: Authorization<ConvexAdminAuthorization>,
}

pub async fn setup_backend_for_test(runtime: ProdRuntime) -> anyhow::Result<TestLocalBackend> {
    let (preempt_tx, _preempt_rx) = async_broadcast::broadcast(1);
    let (_shutdown_tx, shutdown_rx) = async_broadcast::broadcast(1);
    let persistence = TestPersistence::new();
    let config = LocalConfig::new_for_test()?;
    let st = make_app(
        runtime,
        config.clone(),
        Arc::new(persistence),
        shutdown_rx,
        ShutdownSignal::new(preempt_tx, config.name()),
    )
    .await?;
    let router = router(st.clone());
    let app = ConvexHttpService::new(
        router,
        "backend_test",
        SERVER_VERSION_STR.to_string(),
        MAX_CONCURRENT_REQUESTS,
        Duration::from_secs(125),
        NoopRouteMapper,
    );
    let admin_auth_header = config
        .key_broker()?
        .issue_admin_key(MemberId(2))
        .as_header()?;
    Ok(TestLocalBackend {
        app,
        st,
        admin_auth_header,
    })
}

impl TestLocalBackend {
    pub async fn expect_success<T: DeserializeOwned>(
        &self,
        req: Request<axum::body::Body>,
    ) -> anyhow::Result<T> {
        tracing::info!("Sending req {req:?}");
        let (parts, body) = self.app.router().clone().oneshot(req).await?.into_parts();
        let bytes = body
            .collect()
            .await
            .context("Couldn't convert to bytes")?
            .to_bytes();
        let msg = format!("Got response: {}", String::from_utf8_lossy(&bytes));
        tracing::info!("{msg}");
        assert_eq!(parts.status, StatusCode::OK, "{msg}");
        serde_json::from_slice(if bytes.is_empty() { b"null" } else { &bytes })
            .context(format!("Couldn't deserialize as json: {bytes:?}"))
    }

    pub async fn expect_error(
        &self,
        req: Request<axum::body::Body>,
        expected_code: StatusCode,
        expected_short_msg: &str,
    ) -> anyhow::Result<()> {
        tracing::info!("Sending req {req:?}");
        let response = self.app.router().clone().oneshot(req).await?;
        let error = HttpError::from_response(response).await?;
        tracing::info!("Got {error:?}");
        assert_eq!(error.status_code(), expected_code);
        assert_eq!(error.error_code(), expected_short_msg);
        Ok(())
    }
}
