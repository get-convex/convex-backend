use application::valid_identifier::ValidIdentifier;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    http::{
        extract::{
            Json,
            Query,
        },
        HttpResponseError,
    },
    shapes::{
        dashboard_shape_json,
        reduced::ReducedShape,
    },
};
use database::IndexModel;
use http::StatusCode;
use serde::{
    Deserialize,
    Serialize,
};
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    admin::must_be_admin_member,
    authentication::ExtractIdentity,
    schema::IndexMetadataResponse,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTableArgs {
    table_names: Vec<String>,
}

#[debug_handler]
pub async fn shapes2(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    let mut out = serde_json::Map::new();

    must_be_admin_member(&identity)?;
    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot
        .table_mapping()
        .namespace(TableNamespace::by_component_TODO());
    let virtual_mapping = snapshot.table_registry.virtual_table_mapping();

    for table_name in snapshot.table_registry.user_table_names() {
        let table_summary = snapshot.table_summary(table_name);
        let shape = ReducedShape::from_type(
            table_summary.inferred_type(),
            &mapping.table_number_exists(),
            &virtual_mapping.table_number_exists(),
        );
        let json = dashboard_shape_json(&shape, &mapping, virtual_mapping)?;
        out.insert(String::from(table_name.clone()), json);
    }
    Ok(Json(out))
}

#[debug_handler]
pub async fn delete_tables(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(DeleteTableArgs { table_names }): Json<DeleteTableArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member(&identity)?;
    let table_names = table_names
        .into_iter()
        .map(|t| Ok(t.parse::<ValidIdentifier<TableName>>()?.0))
        .collect::<anyhow::Result<_>>()?;
    st.application.delete_tables(&identity, table_names).await?;
    Ok(StatusCode::OK)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetIndexesResponse {
    indexes: Vec<IndexMetadataResponse>,
}

#[debug_handler]
pub async fn get_indexes(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member(&identity)?;
    let mut tx = st.application.begin(identity.clone()).await?;
    let indexes = IndexModel::new(&mut tx).get_application_indexes().await?;
    Ok(Json(GetIndexesResponse {
        indexes: indexes
            .into_iter()
            .map(|idx| idx.into_value().try_into())
            .collect::<anyhow::Result<_>>()?,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSourceCodeArgs {
    path: String,
}

#[debug_handler]
pub async fn get_source_code(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(GetSourceCodeArgs { path }): Query<GetSourceCodeArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member(&identity)?;
    let source_code = st
        .application
        .get_source_code(identity, path.parse()?)
        .await?;
    Ok(Json(source_code))
}
