//! HTTP routes for managing dashboard-issued admin keys.
//!
//! These endpoints back the self-hosted dashboard's admin-key UI:
//!   GET    /api/admin_keys
//!   POST   /api/admin_keys
//!   POST   /api/admin_keys/:id/revoke
//!   PATCH  /api/admin_keys/:id

use anyhow::Context;
use application::admin_keys_cache::CachedAdminKey;
use axum::response::IntoResponse;
use common::{
    document::ParsedDocument,
    http::{
        extract::{
            Json,
            Path,
        },
        ExtractRequestMetadata,
        HttpResponseError,
    },
    types::MemberId,
};
use errors::ErrorMetadata;
use http::HeaderMap;
use keybroker::{
    admin_key_hash,
    admin_key_suffix,
    AdminKeyHash,
    Identity,
    ADMIN_KEY_SUFFIX_LEN,
};
use model::{
    admin_keys::{
        types::AdminKeyMetadata,
        AdminKeysModel,
        ADMIN_KEYS_TABLE,
    },
    deployment_audit_log::types::DeploymentAuditLogEvent,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::TableNamespace;

use crate::{
    authentication::ExtractIdentity,
    parse::parse_document_id,
    LocalAppState,
};

const MAX_ADMIN_KEY_NAME_LEN: usize = 128;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminKeyRow {
    pub id: String,
    pub name: String,
    /// Document creation time, in milliseconds since the unix epoch.
    pub creation_time: f64,
    /// `revoked_time` is `None` for active keys; otherwise the revocation
    /// timestamp in milliseconds since the unix epoch.
    pub revoked_time: Option<f64>,
    /// `true` if this row represents the same admin key the caller used to
    /// authenticate this request.
    pub is_current: bool,
    /// Last few characters of the normalized admin key (captured at insert /
    /// auto-adopt time). Surfaced so the UI can show users which key they are
    /// about to revoke without leaking the full secret. `None` for rows that
    /// pre-date this field.
    pub key_suffix: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResponse {
    pub id: String,
    pub name: String,
    pub creation_time: f64,
    /// The freshly minted admin key, in the same wire format as the keys
    /// produced by `cargo run -p keybroker --bin generate_key`. Returned only
    /// at creation time; never persisted in plaintext.
    pub admin_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeResponse {
    pub id: String,
    /// Revocation timestamp in milliseconds since the unix epoch.
    pub revoked_time: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRequest {
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResponse {
    pub id: String,
    pub name: String,
}

fn ts_to_ms(t: Timestamp) -> f64 {
    let nanos: i64 = t.into();
    (nanos as f64) / 1_000_000.0
}

fn to_row(doc: &ParsedDocument<AdminKeyMetadata>, is_current: bool) -> AdminKeyRow {
    let creation_time_ms: f64 = doc.creation_time().into();
    AdminKeyRow {
        id: doc.id().to_string(),
        name: doc.name.clone(),
        creation_time: creation_time_ms,
        revoked_time: doc.revoked_time.map(ts_to_ms),
        is_current,
        key_suffix: doc.key_suffix.clone(),
    }
}

fn ensure_admin(identity: &Identity) -> Result<(), HttpResponseError> {
    if identity.is_admin() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(ErrorMetadata::forbidden(
            "NotAdmin",
            "Admin key required to manage admin keys.",
        ))
        .into())
    }
}

/// Compute the hash of the admin key the caller used to authenticate, by
/// re-parsing the `Authorization` header. Returns `None` when no header is
/// present, when it isn't a `Convex ` admin key, or when parsing fails — in
/// those cases the dashboard simply won't highlight a "current" row.
fn caller_admin_key_hash(headers: &HeaderMap, st: &LocalAppState) -> Option<AdminKeyHash> {
    let header_value = headers.get(http::header::AUTHORIZATION)?;
    let header_str = header_value.to_str().ok()?;
    if header_str.len() < 7 || !header_str[..7].eq_ignore_ascii_case("convex ") {
        return None;
    }
    let key = &header_str[7..];
    Some(admin_key_hash(
        key,
        st.application.key_broker().deployment_secret(),
    ))
}

fn parse_admin_key_id(
    st: &LocalAppState,
    id_str: &str,
) -> Result<value::ResolvedDocumentId, HttpResponseError> {
    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot
        .table_registry
        .table_mapping()
        .namespace(TableNamespace::Global);
    parse_document_id(id_str, &mapping, &ADMIN_KEYS_TABLE).map_err(Into::into)
}

pub async fn list_admin_keys(
    axum::extract::State(st): axum::extract::State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    headers: HeaderMap,
) -> Result<impl IntoResponse, HttpResponseError> {
    ensure_admin(&identity)?;
    let caller_hash = caller_admin_key_hash(&headers, &st);

    let mut tx = st.application.begin(Identity::system()).await?;
    let rows = AdminKeysModel::new(&mut tx).list().await?;
    let response: Vec<AdminKeyRow> = rows
        .iter()
        .map(|doc| to_row(doc, caller_hash.as_ref() == Some(&doc.key_hash)))
        .collect();
    Ok(Json(response))
}

pub async fn create_admin_key(
    axum::extract::State(st): axum::extract::State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Json(body): Json<CreateRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    ensure_admin(&identity)?;
    let name = validate_name(&body.name)?;

    let admin_key = st
        .application
        .key_broker()
        .issue_admin_key(MemberId(0))
        .as_str()
        .to_string();
    let hash = admin_key_hash(&admin_key, st.application.key_broker().deployment_secret());
    let key_suffix = Some(admin_key_suffix(&admin_key, ADMIN_KEY_SUFFIX_LEN));

    let mut tx = st.application.begin(Identity::system()).await?;
    let doc_id = AdminKeysModel::new(&mut tx)
        .insert(hash, name.clone(), key_suffix.clone())
        .await?;
    st.application
        .commit_with_audit_log_events(
            tx,
            vec![DeploymentAuditLogEvent::AdminKeyCreated {
                id: doc_id.to_string(),
                name: name.clone(),
            }],
            request_metadata,
            "create_admin_key",
        )
        .await?;
    st.application.admin_keys_cache().insert(
        hash,
        CachedAdminKey {
            doc_id: doc_id.to_string(),
            name: name.clone(),
            revoked_time: None,
            key_suffix,
        },
    );

    // Re-read so we can return the persisted creation_time.
    let mut tx = st.application.begin(Identity::system()).await?;
    let row = AdminKeysModel::new(&mut tx)
        .get_by_hash(&hash)
        .await?
        .context("admin key row missing immediately after insert")?;
    let creation_time_ms: f64 = row.creation_time().into();

    Ok(Json(CreateResponse {
        id: row.id().to_string(),
        name,
        creation_time: creation_time_ms,
        admin_key,
    }))
}

pub async fn revoke_admin_key(
    axum::extract::State(st): axum::extract::State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Path(id_str): Path<String>,
) -> Result<impl IntoResponse, HttpResponseError> {
    ensure_admin(&identity)?;
    let doc_id = parse_admin_key_id(&st, &id_str)?;

    let mut tx = st.application.begin(Identity::system()).await?;
    let (updated, was_revoked_now) = AdminKeysModel::new(&mut tx).revoke(doc_id).await?;
    if was_revoked_now {
        st.application
            .commit_with_audit_log_events(
                tx,
                vec![DeploymentAuditLogEvent::AdminKeyRevoked { id: id_str.clone() }],
                request_metadata,
                "revoke_admin_key",
            )
            .await?;
        if let Some(t) = updated.revoked_time {
            st.application
                .admin_keys_cache()
                .mark_revoked(&updated.key_hash, t);
        }
    }
    let revoked_time = updated
        .revoked_time
        .map(ts_to_ms)
        .ok_or_else(|| anyhow::anyhow!("admin key has no revoked_time after revoke()"))?;
    Ok(Json(RevokeResponse {
        id: id_str,
        revoked_time,
    }))
}

pub async fn rename_admin_key(
    axum::extract::State(st): axum::extract::State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Path(id_str): Path<String>,
    Json(body): Json<RenameRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    ensure_admin(&identity)?;
    let name = validate_name(&body.name)?;
    let doc_id = parse_admin_key_id(&st, &id_str)?;

    let mut tx = st.application.begin(Identity::system()).await?;
    let updated = AdminKeysModel::new(&mut tx)
        .rename(doc_id, name.clone())
        .await?;
    st.application
        .commit_with_audit_log_events(
            tx,
            vec![DeploymentAuditLogEvent::AdminKeyRenamed {
                id: id_str.clone(),
                new_name: name.clone(),
            }],
            request_metadata,
            "rename_admin_key",
        )
        .await?;
    st.application
        .admin_keys_cache()
        .rename(&updated.key_hash, name.clone());

    Ok(Json(RenameResponse { id: id_str, name }))
}

fn validate_name(raw: &str) -> Result<String, HttpResponseError> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "AdminKeyNameEmpty",
            "Name is required.",
        ))
        .into());
    }
    if name.chars().count() > MAX_ADMIN_KEY_NAME_LEN {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "AdminKeyNameTooLong",
            format!("Name must be {MAX_ADMIN_KEY_NAME_LEN} characters or fewer."),
        ))
        .into());
    }
    Ok(name)
}
