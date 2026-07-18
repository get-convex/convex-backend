//! HTTP routes for managing the self-hosted periodic-backup config.
//!
//! Routes:
//!   POST /api/periodic_backup/configure  body: { cronspec, includeStorage }
//!   POST /api/periodic_backup/disable
//!
//! The dashboard reads the current config via `udfs.periodicBackup.get`
//! (system query), so there's no GET HTTP endpoint here.

use axum::response::IntoResponse;
use common::http::{
    extract::Json,
    ExtractRequestMetadata,
    HttpResponseError,
};
use errors::ErrorMetadata;
use http::StatusCode;
use keybroker::Identity;
use model::{
    deployment_audit_log::types::DeploymentAuditLogEvent,
    periodic_backup::PeriodicBackupModel,
};
use serde::Deserialize;

use crate::{
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureRequest {
    pub cronspec: String,
    pub include_storage: bool,
}

fn ensure_admin(identity: &Identity) -> Result<(), HttpResponseError> {
    if identity.is_admin() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(ErrorMetadata::forbidden(
            "NotAdmin",
            "Admin key required to manage periodic backups.",
        ))
        .into())
    }
}

pub async fn configure_periodic_backup(
    axum::extract::State(st): axum::extract::State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Json(body): Json<ConfigureRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    ensure_admin(&identity)?;

    let mut tx = st.application.begin(Identity::system()).await?;
    let stored = PeriodicBackupModel::new(&mut tx)
        .set(body.cronspec.clone(), body.include_storage)
        .await
        .map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidCronspec",
                format!("{e:#}"),
            ))
        })?;
    st.application
        .commit_with_audit_log_events(
            tx,
            vec![DeploymentAuditLogEvent::PeriodicBackupConfigured {
                cronspec: stored.cronspec,
                include_storage: stored.include_storage,
            }],
            request_metadata,
            "configure_periodic_backup",
        )
        .await?;
    Ok(StatusCode::OK)
}

pub async fn disable_periodic_backup(
    axum::extract::State(st): axum::extract::State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
) -> Result<impl IntoResponse, HttpResponseError> {
    ensure_admin(&identity)?;

    let mut tx = st.application.begin(Identity::system()).await?;
    let removed = PeriodicBackupModel::new(&mut tx).disable().await?;
    if removed {
        st.application
            .commit_with_audit_log_events(
                tx,
                vec![DeploymentAuditLogEvent::PeriodicBackupDisabled],
                request_metadata,
                "disable_periodic_backup",
            )
            .await?;
    }
    Ok(StatusCode::OK)
}
