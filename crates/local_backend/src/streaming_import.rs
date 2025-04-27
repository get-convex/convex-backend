use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use application::{
    airbyte_import::{
        AirbyteRecordMessage,
        AirbyteStream,
        PrimaryKey,
    },
    valid_identifier::ValidIdentifier,
};
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    self,
    components::ComponentPath,
    http::{
        extract::Json,
        HttpResponseError,
    },
    schemas::json::DatabaseSchemaJson,
};
use convex_fivetran_destination::api_types::{
    BatchWriteRow,
    CreateTableArgs,
    DeleteType,
    TruncateTableArgs,
};
use errors::ErrorMetadata;
use http::StatusCode;
use serde::{
    Deserialize,
    Serialize,
};
use value::{
    identifier::IDENTIFIER_REQUIREMENTS,
    TableName,
    TableNamespace,
};

use crate::{
    admin::{
        must_be_admin,
        must_be_admin_with_write_access,
    },
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirbyteImportArgs {
    tables: BTreeMap<String, AirbyteStream>,
    messages: Vec<AirbyteRecordMessage>,
}

#[debug_handler]
pub async fn import_airbyte_records(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(AirbyteImportArgs { tables, messages }): Json<AirbyteImportArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let records = messages
        .into_iter()
        .map(|msg| msg.try_into())
        .collect::<anyhow::Result<_>>()?;
    let tables = tables
        .into_iter()
        .map(|(k, v)| Ok((k.parse::<ValidIdentifier<TableName>>()?.0, v.try_into()?)))
        .collect::<anyhow::Result<_>>()?;
    st.application
        .import_airbyte_records(&identity, records, tables)
        .await?;
    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn apply_fivetran_operations(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(rows): Json<Vec<BatchWriteRow>>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;

    st.application
        .apply_fivetran_operations(&identity, rows)
        .await?;

    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn get_schema(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let schema = st
        .application
        .get_schema(TableNamespace::root_component(), &identity)
        .await?;
    Ok(Json(match schema {
        None => None,
        Some(schema) => Some(DatabaseSchemaJson::try_from(schema)?),
    }))
}

#[debug_handler]
pub async fn fivetran_create_table(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(CreateTableArgs { table_definition }): Json<CreateTableArgs>,
) -> Result<StatusCode, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let table_definition = table_definition.try_into()?;
    st.application
        .fivetran_create_table(&identity, table_definition)
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearTableArgs {
    table_names: Vec<String>,
}

#[debug_handler]
pub async fn clear_tables(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(ClearTableArgs { table_names }): Json<ClearTableArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let table_names = table_names
        .into_iter()
        .map(|t| {
            Ok((
                ComponentPath::root(),
                t.parse::<ValidIdentifier<TableName>>()?.0,
            ))
        })
        .collect::<anyhow::Result<_>>()?;
    st.application.clear_tables(&identity, table_names).await?;
    Ok(StatusCode::OK)
}

/// Truncates the given table.
///
/// This function call corresponds to the `TruncateRequest` request sent by
/// the Fivetran destination connector.
///
/// It supports truncate requests for all rows (delete_before == None) and
/// for the rows that have a fivetran.synced value smaller than
/// `delete_before`. The deletion can either be a hard delete, or a soft
/// delete, in which case the `fivetran.deleted` attribute is set to true.
///
/// In any case, we use the index that the user created on `fivetran.synced`
/// (and `fivetran.deleted` when using soft deletes) to efficiently find the
/// rows to delete.
#[debug_handler]
pub async fn fivetran_truncate_table(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(TruncateTableArgs {
        table_name,
        delete_before,
        delete_type,
    }): Json<TruncateTableArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;

    let table_name = table_name.parse::<ValidIdentifier<TableName>>()?;

    // Full table hard deletes can be done through the optimized implementation
    if delete_before.is_none() && delete_type == DeleteType::HardDelete {
        st.application
            .clear_tables(&identity, vec![(ComponentPath::root(), table_name.0)])
            .await?;
        return Ok(StatusCode::OK);
    }

    st.application
        .fivetran_truncate(&identity, table_name.0, delete_before, delete_type)
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceTableArgs {
    _table_names: BTreeMap<String, String>,
}

#[debug_handler]
pub async fn replace_tables(
    State(_st): State<LocalAppState>,
    ExtractIdentity(_identity): ExtractIdentity,
    Json(ReplaceTableArgs { _table_names: _ }): Json<ReplaceTableArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    // Disabled until we figure out how to make schema validation and indexes
    // work with overwrites.
    Err::<(), _>(
        anyhow::anyhow!(ErrorMetadata::bad_request(
            "OverwriteNotSupported",
            "Overwrite sync mode is not supported. Please use a different sync mode."
        ))
        .into(),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIndexesArgs {
    indexes: BTreeMap<String, Vec<Vec<String>>>,
}

#[debug_handler]
pub async fn add_primary_key_indexes(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(AddIndexesArgs { indexes }): Json<AddIndexesArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let indexes: BTreeMap<TableName, PrimaryKey> = indexes
        .into_iter()
        .map(|(stream, primary_key)| {
            let table_name = stream.parse::<ValidIdentifier<TableName>>()?.0;
            let primary_key =
                PrimaryKey::try_from(primary_key.clone()).context(ErrorMetadata::bad_request(
                    "InvalidPrimaryKey",
                    format!("Invalid primary key: {IDENTIFIER_REQUIREMENTS} {primary_key:?}."),
                ))?;
            Ok((table_name, primary_key))
        })
        .collect::<anyhow::Result<_>>()?;

    st.application
        .add_primary_key_indexes(&identity, indexes)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexesReadyArgs {
    tables: BTreeSet<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexesReadyResponse {
    indexes_ready: bool,
}

#[debug_handler]
pub async fn primary_key_indexes_ready(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(IndexesReadyArgs { tables }): Json<IndexesReadyArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;
    let table_names = tables
        .into_iter()
        .map(|t| Ok(t.parse::<ValidIdentifier<TableName>>()?.0))
        .collect::<anyhow::Result<BTreeSet<_>>>()?;
    let indexes_ready = st
        .application
        .primary_key_indexes_ready(identity, table_names)
        .await?;
    Ok(Json(IndexesReadyResponse { indexes_ready }))
}
