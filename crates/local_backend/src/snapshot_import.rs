use std::str::FromStr;

use anyhow::Context;
use application::snapshot_import::{
    self,
    do_import,
};
use axum::{
    body::Body,
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    components::ComponentPath,
    http::{
        extract::{
            Json,
            Query,
        },
        HttpResponseError,
    },
};
use errors::ErrorMetadata;
use futures::{
    StreamExt,
    TryStreamExt,
};
use model::snapshot_imports::types::{
    ImportFormat,
    ImportMode,
};
use serde::{
    Deserialize,
    Serialize,
};
use storage::{
    ClientDrivenUploadPartToken,
    ClientDrivenUploadToken,
};
use value::{
    id_v6::DeveloperDocumentId,
    TableName,
};

use crate::{
    admin::must_be_admin_with_write_access,
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportQueryArgs {
    table_name: Option<String>,
    component_path: Option<String>,
    format: ImportFormatArg,
    #[serde(default)]
    mode: ImportMode,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUploadPartArgs {
    upload_token: String,
    part_number: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFinishUploadArgs {
    import: ImportQueryArgs,

    upload_token: String,
    part_tokens: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ImportFormatArg {
    Csv,
    JsonLines,
    JsonArray,
    Zip,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResponse {
    num_written: u64,
}

fn parse_format_arg(
    table_name: Option<String>,
    format: ImportFormatArg,
) -> anyhow::Result<ImportFormat> {
    let table_name = table_name
        .map(|table_name| {
            TableName::from_str(&table_name).map_err(|e| {
                ErrorMetadata::bad_request(
                    "ImportInvalidName",
                    format!("invalid table name {table_name}: {e}"),
                )
            })
        })
        .transpose()?;
    let inner_format = match format {
        ImportFormatArg::Zip => {
            if table_name.is_some() {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidName",
                    "ZIP import cannot have table name",
                ));
            }
            ImportFormat::Zip
        },
        ImportFormatArg::Csv => ImportFormat::Csv(table_name.context(
            ErrorMetadata::bad_request("InvalidName", "CSV import requires table name"),
        )?),
        ImportFormatArg::JsonArray => ImportFormat::JsonArray(table_name.context(
            ErrorMetadata::bad_request("InvalidName", "JSON import requires table name"),
        )?),
        ImportFormatArg::JsonLines => ImportFormat::JsonLines(table_name.context(
            ErrorMetadata::bad_request("InvalidName", "JSONL import requires table name"),
        )?),
    };
    Ok(inner_format)
}

pub async fn import(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(ImportQueryArgs {
        table_name,
        component_path,
        format,
        mode,
    }): Query<ImportQueryArgs>,
    stream: Body,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let format = parse_format_arg(table_name, format)?;
    let component_path = ComponentPath::deserialize(component_path.as_deref())?;
    let body_stream = stream
        .into_data_stream()
        .map_err(anyhow::Error::from)
        .boxed();
    let num_written = do_import(
        &st.application,
        identity,
        format,
        mode,
        component_path,
        body_stream,
    )
    .await?;
    Ok(Json(ImportResponse { num_written }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartUploadResponse {
    pub upload_token: String,
}

pub async fn import_start_upload(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let token = st
        .application
        .start_upload_for_snapshot_import(identity)
        .await?;
    Ok(Json(StartUploadResponse {
        upload_token: token.0,
    }))
}

#[debug_handler]
pub async fn import_upload_part(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(ImportUploadPartArgs {
        upload_token,
        part_number,
    }): Query<ImportUploadPartArgs>,
    body_stream: Body,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let body_bytes = body_stream
        .into_data_stream()
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await
        .context(ErrorMetadata::bad_request(
            "ImportFailed",
            "failed to read request body",
        ))?;
    let token = st
        .application
        .upload_part_for_snapshot_import(
            identity,
            ClientDrivenUploadToken(upload_token),
            part_number,
            body_bytes.into(),
        )
        .await?;
    Ok(Json(token.0))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFinishUploadResponse {
    pub import_id: String,
}

pub async fn import_finish_upload(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(ImportFinishUploadArgs {
        import:
            ImportQueryArgs {
                table_name,
                component_path,
                format,
                mode,
            },
        upload_token,
        part_tokens,
    }): Json<ImportFinishUploadArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let format = parse_format_arg(table_name, format)?;
    let component_path = ComponentPath::deserialize(component_path.as_deref())?;
    let import_id = st
        .application
        .import_finish_upload(
            identity,
            format,
            mode,
            component_path,
            ClientDrivenUploadToken(upload_token),
            part_tokens
                .into_iter()
                .map(ClientDrivenUploadPartToken)
                .collect(),
        )
        .await?;
    Ok(Json(ImportFinishUploadResponse {
        import_id: import_id.encode(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformImportArgs {
    pub import_id: String,
}

pub async fn perform_import(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(PerformImportArgs { import_id }): Json<PerformImportArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let import_id = DeveloperDocumentId::decode(&import_id).context(ErrorMetadata::bad_request(
        "InvalidImport",
        format!("invalid import id {import_id}"),
    ))?;
    snapshot_import::perform_import(&st.application, identity, import_id).await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelImportArgs {
    pub import_id: String,
}

pub async fn cancel_import(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(CancelImportArgs { import_id }): Json<CancelImportArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let import_id = DeveloperDocumentId::decode(&import_id).context(ErrorMetadata::bad_request(
        "InvalidImport",
        format!("invalid import id {import_id}"),
    ))?;
    snapshot_import::cancel_import(&st.application, identity, import_id).await?;
    Ok(())
}
