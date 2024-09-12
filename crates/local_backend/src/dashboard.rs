use anyhow::Context;
use application::{
    deploy_config::ModuleJson,
    valid_identifier::ValidIdentifier,
};
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    components::ComponentId,
    http::{
        extract::{
            Json,
            Query,
        },
        ExtractClientVersion,
        ExtractRequestId,
        HttpResponseError,
    },
    shapes::{
        dashboard_shape_json,
        reduced::ReducedShape,
    },
    types::FunctionCaller,
};
use database::IndexModel;
use errors::ErrorMetadata;
use http::StatusCode;
use isolate::UdfArgsJson;
use model::{
    config::types::ModuleConfig,
    virtual_system_mapping,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    admin::{
        must_be_admin_from_key,
        must_be_admin_member,
        must_be_admin_member_with_write_access,
    },
    authentication::ExtractIdentity,
    public_api::{
        export_value,
        UdfResponse,
    },
    schema::IndexMetadataResponse,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTableArgs {
    table_names: Vec<String>,
    component_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteComponentArgs {
    component_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapesArgs {
    component: Option<String>,
}

#[debug_handler]
pub async fn shapes2(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(ShapesArgs { component }): Query<ShapesArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let mut out = serde_json::Map::new();

    must_be_admin_member(&identity)?;
    let component = ComponentId::deserialize_from_string(component.as_deref())?;
    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot.table_mapping().namespace(component.into());

    for (namespace, table_name) in snapshot.table_registry.user_table_names() {
        if TableNamespace::from(component) != namespace {
            continue;
        }
        let table_summary = snapshot.table_summary(namespace, table_name);
        let shape = ReducedShape::from_type(
            table_summary.inferred_type(),
            &mapping.table_number_exists(),
        );
        let json = dashboard_shape_json(&shape, &mapping, &virtual_system_mapping())?;
        out.insert(String::from(table_name.clone()), json);
    }
    Ok(Json(out))
}

#[debug_handler]
pub async fn delete_tables(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(DeleteTableArgs {
        table_names,
        component_id,
    }): Json<DeleteTableArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member_with_write_access(&identity)?;
    let table_names = table_names
        .into_iter()
        .map(|t| Ok(t.parse::<ValidIdentifier<TableName>>()?.0))
        .collect::<anyhow::Result<_>>()?;
    let table_namespace = TableNamespace::from(ComponentId::deserialize_from_string(
        component_id.as_deref(),
    )?);
    st.application
        .delete_tables(&identity, table_names, table_namespace)
        .await?;
    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn delete_component(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(DeleteComponentArgs { component_id }): Json<DeleteComponentArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member_with_write_access(&identity)?;
    let component_id = ComponentId::deserialize_from_string(component_id.as_deref())?;
    st.application
        .delete_component(&identity, component_id)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetIndexesArgs {
    component_id: Option<String>,
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
    Query(GetIndexesArgs { component_id }): Query<GetIndexesArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member(&identity)?;
    let component_id = ComponentId::deserialize_from_string(component_id.as_deref())?;
    let mut tx = st.application.begin(identity.clone()).await?;
    let indexes = IndexModel::new(&mut tx)
        .get_application_indexes(TableNamespace::from(component_id))
        .await?;
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
    component: Option<String>,
}

#[debug_handler]
pub async fn get_source_code(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(GetSourceCodeArgs { path, component }): Query<GetSourceCodeArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_member(&identity)?;
    let component = ComponentId::deserialize_from_string(component.as_deref())?;
    let path = path.parse().context(ErrorMetadata::bad_request(
        "InvalidModulePath",
        "Invalid module path",
    ))?;
    let source_code = st
        .application
        .get_source_code(identity, path, component)
        .await?;
    Ok(Json(source_code))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTestFunctionArgs {
    admin_key: String,
    bundle: ModuleJson,
    args: UdfArgsJson,
    format: String,
    component_id: Option<String>,
}

#[debug_handler]
pub async fn run_test_function(
    State(st): State<LocalAppState>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractClientVersion(client_version): ExtractClientVersion,
    Json(req): Json<RunTestFunctionArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let identity = must_be_admin_from_key(
        st.application.app_auth(),
        st.instance_name.clone(),
        req.admin_key.clone(),
    )
    .await?;
    let args = req.args.into_arg_vec();
    let module: ModuleConfig = req.bundle.try_into()?;
    let component_id = ComponentId::deserialize_from_string(req.component_id.as_deref())?;
    let udf_return = st
        .application
        .execute_standalone_module(
            request_id,
            module,
            args,
            identity,
            FunctionCaller::Tester(client_version.clone()),
            component_id,
        )
        .await?;
    let value_format = Some(req.format.parse()?);
    let response = match udf_return {
        Ok(result) => UdfResponse::Success {
            value: export_value(result.value, value_format, client_version)?,
            log_lines: result.log_lines,
        },
        Err(error) => {
            UdfResponse::error(error.error, error.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}
