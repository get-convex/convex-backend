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
            FromMtState,
            Json,
            MtState,
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
use serde_json::json;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    admin::must_be_admin_from_key,
    authentication::ExtractIdentity,
    public_api::{
        export_value,
        UdfResponse,
    },
    scheduling::{
        __path_delete_scheduled_functions_table,
        delete_scheduled_functions_table,
    },
    schema::IndexMetadataResponse,
    LocalAppState,
};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTableArgs {
    table_names: Vec<String>,
    component_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteComponentArgs {
    component_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShapesArgs {
    component: Option<String>,
}

/// Get table shapes
///
/// Returns the schema shapes for all tables in the specified component.
#[utoipa::path(
    get,
    path = "/shapes2",
    params(
        ("component" = Option<String>, Query, description = "Component ID to get shapes for")
    ),
    responses((status = 200, body = serde_json::Value)),
)]
pub async fn shapes2(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(ShapesArgs { component }): Query<ShapesArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let mut out = serde_json::Map::new();

    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
    let component = ComponentId::deserialize_from_string(component.as_deref())?;
    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot.table_mapping().namespace(component.into());

    for (namespace, table_name) in snapshot.table_registry.user_table_names() {
        if TableNamespace::from(component) != namespace {
            continue;
        }
        let table_summary = snapshot.table_summary(namespace, table_name);

        let shape = match table_summary {
            Some(table_summary) => ReducedShape::from_type(
                table_summary.inferred_type(),
                &mapping.table_number_exists(),
            ),
            // Table summaries are still bootstrapping, use `Unknown` in the meantime
            None => ReducedShape::Unknown,
        };
        let json = dashboard_shape_json(&shape, &mapping, virtual_system_mapping())?;
        out.insert(String::from(table_name.clone()), json);
    }
    Ok(Json(out))
}

/// Delete database tables
///
/// Deletes the specified tables from the database.
#[utoipa::path(
    post,
    path = "/delete_tables",
    request_body = DeleteTableArgs,
    responses((status = 200)),
)]
pub async fn delete_tables(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(DeleteTableArgs {
        table_names,
        component_id,
    }): Json<DeleteTableArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::WriteData)?;
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

/// Delete component
///
/// Deletes the specified component and all its associated data.
#[utoipa::path(
    post,
    path = "/delete_component",
    request_body = DeleteComponentArgs,
    responses((status = 200)),
)]
pub async fn delete_component(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(DeleteComponentArgs { component_id }): Json<DeleteComponentArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::WriteData)?;
    let component_id = ComponentId::deserialize_from_string(component_id.as_deref())?;
    st.application
        .delete_component(&identity, component_id)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetIndexesArgs {
    component_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct GetIndexesResponse {
    #[schema(value_type = Vec<Object>)]
    indexes: Vec<IndexMetadataResponse>,
}

/// Get database indexes
///
/// Returns metadata about database indexes for the specified component.
#[utoipa::path(
    get,
    path = "/get_indexes",
    params(
        ("component_id" = Option<String>, Query, description = "Component ID to get indexes for")
    ),
    responses((status = 200, body = GetIndexesResponse)),
)]
pub async fn get_indexes(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(GetIndexesArgs { component_id }): Query<GetIndexesArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
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

/// Check admin key validity
///
/// This endpoint checks if the admin key included in the header is valid for
/// this instance.
#[utoipa::path(
    get,
    path = "/check_admin_key",
    responses((status = 200, body = serde_json::Value)),
    tag = "public_api"
)]
#[debug_handler]
pub async fn check_admin_key(
    State(_st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
    Ok(Json(json!({ "success": true })))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunTestFunctionArgs {
    admin_key: String,
    #[schema(value_type = Object)]
    bundle: ModuleJson,
    #[schema(value_type = Object)]
    args: UdfArgsJson,
    format: String,
    component_id: Option<String>,
}

/// Run test function
///
/// Executes a test function with the provided arguments and bundle.
#[utoipa::path(
    post,
    path = "/run_test_function",
    request_body = RunTestFunctionArgs,
    responses((status = 200, body = serde_json::Value)),
)]
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
    identity.require_operation(keybroker::DeploymentOp::RunTestQuery)?;
    let args = req.args.into_serialized_args()?;
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
            value: export_value(result.value.unpack()?, value_format, client_version)?,
            log_lines: result.log_lines,
        },
        Err(error) => {
            UdfResponse::error(error.error, error.log_lines, value_format, client_version)?
        },
    };
    Ok(Json(response))
}

pub fn local_only_dashboard_router() -> OpenApiRouter<crate::LocalAppState> {
    OpenApiRouter::new().routes(utoipa_axum::routes!(check_admin_key))
}

// Routes with the same handlers for the local backend + closed source backend
pub fn common_dashboard_api_router<S>() -> OpenApiRouter<S>
where
    LocalAppState: FromMtState<S>,
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(shapes2))
        .routes(utoipa_axum::routes!(get_indexes))
        .routes(utoipa_axum::routes!(delete_tables))
        .routes(utoipa_axum::routes!(delete_component))
        .routes(utoipa_axum::routes!(delete_scheduled_functions_table))
}

#[cfg(test)]
mod tests {
    use axum_extra::headers::{
        authorization::Credentials,
        Authorization,
    };
    use common::{
        http::HttpError,
        types::MemberId,
    };
    use http::Request;
    use runtime::prod::ProdRuntime;
    use serde_json::json;
    use sync_types::headers::ConvexAdminAuthorization;

    use crate::test_helpers::{
        setup_backend_for_test,
        TestLocalBackend,
    };

    fn run_test_function_request(admin_key: &str) -> anyhow::Result<Request<axum::body::Body>> {
        let json_body = json!({
            "adminKey": admin_key,
            "bundle": {
                "path": "test.js",
                "source": "export default function() { return 42; }",
                "sourceMap": null,
                "environment": "isolate",
            },
            "args": "[]",
            "format": "json",
        });
        let body = axum::body::Body::from(serde_json::to_vec(&json_body)?);
        Ok(Request::builder()
            .uri("/api/run_test_function")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Convex-Client", "actions-0.0.0")
            .body(body)?)
    }

    fn read_only_auth_header(
        backend: &TestLocalBackend,
    ) -> anyhow::Result<Authorization<ConvexAdminAuthorization>> {
        backend
            .config
            .key_broker()?
            .issue_read_only_admin_key(MemberId(2))
            .as_header()
    }

    fn get_request(
        uri: &str,
        auth: &Authorization<ConvexAdminAuthorization>,
    ) -> anyhow::Result<Request<axum::body::Body>> {
        Ok(Request::builder()
            .uri(uri)
            .method("GET")
            .header("Authorization", auth.0.encode())
            .body(axum::body::Body::empty())?)
    }

    fn post_request(
        uri: &str,
        auth: &Authorization<ConvexAdminAuthorization>,
        body: serde_json::Value,
    ) -> anyhow::Result<Request<axum::body::Body>> {
        Ok(Request::builder()
            .uri(uri)
            .method("POST")
            .header("Authorization", auth.0.encode())
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(serde_json::to_vec(&body)?))?)
    }

    // run_test_function tests

    #[convex_macro::prod_rt_test]
    async fn test_run_test_function_full_admin_key_passes_operation_check(
        rt: ProdRuntime,
    ) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let admin_key = backend.config.key_broker()?.issue_admin_key(MemberId(2));
        let req = run_test_function_request(admin_key.as_str())?;
        // A full admin key (empty allowed_ops) should pass the operation
        // check. The request may fail later during module execution, but it
        // should NOT fail with an Unauthorized error.
        let response = backend.send_request(req).await?;
        let status = response.status();
        if status == http::StatusCode::FORBIDDEN {
            let error = HttpError::from_response(response).await?;
            assert_ne!(
                error.error_code(),
                "Unauthorized",
                "Full admin key should pass the RunTestQuery operation check"
            );
        }
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_run_test_function_read_only_key_passes_operation_check(
        rt: ProdRuntime,
    ) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let read_only_key = backend
            .config
            .key_broker()?
            .issue_read_only_admin_key(MemberId(2));
        let req = run_test_function_request(read_only_key.as_str())?;
        // A read-only admin key includes RunTestQuery in its allowed_ops,
        // so it should pass the operation check.
        let response = backend.send_request(req).await?;
        let status = response.status();
        if status == http::StatusCode::FORBIDDEN {
            let error = HttpError::from_response(response).await?;
            assert_ne!(
                error.error_code(),
                "Unauthorized",
                "Read-only key should pass the RunTestQuery operation check"
            );
        }
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_run_test_function_invalid_key_rejected(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = run_test_function_request("invalid-key")?;
        let response = backend.send_request(req).await?;
        let status = response.status();
        assert!(
            status == http::StatusCode::BAD_REQUEST
                || status == http::StatusCode::UNAUTHORIZED
                || status == http::StatusCode::FORBIDDEN,
            "Expected auth-related error, got {status}"
        );
        Ok(())
    }

    // shapes2 tests

    #[convex_macro::prod_rt_test]
    async fn test_shapes2_full_admin_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = get_request("/api/shapes2", &backend.admin_auth_header)?;
        let _: serde_json::Value = backend.expect_success(req).await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_shapes2_read_only_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let auth = read_only_auth_header(&backend)?;
        let req = get_request("/api/shapes2", &auth)?;
        // ViewData is a read-only operation, so this should succeed.
        let _: serde_json::Value = backend.expect_success(req).await?;
        Ok(())
    }

    // get_indexes tests

    #[convex_macro::prod_rt_test]
    async fn test_get_indexes_full_admin_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = get_request("/api/get_indexes", &backend.admin_auth_header)?;
        let _: serde_json::Value = backend.expect_success(req).await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_indexes_read_only_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let auth = read_only_auth_header(&backend)?;
        let req = get_request("/api/get_indexes", &auth)?;
        let _: serde_json::Value = backend.expect_success(req).await?;
        Ok(())
    }

    // delete_tables tests

    #[convex_macro::prod_rt_test]
    async fn test_delete_tables_full_admin_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = post_request(
            "/api/delete_tables",
            &backend.admin_auth_header,
            json!({ "tableNames": [] }),
        )?;
        let _: () = backend.expect_success(req).await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_delete_tables_read_only_key_rejected(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let auth = read_only_auth_header(&backend)?;
        let req = post_request("/api/delete_tables", &auth, json!({ "tableNames": [] }))?;
        backend
            .expect_error(req, http::StatusCode::FORBIDDEN, "Unauthorized")
            .await?;
        Ok(())
    }

    // delete_component tests

    #[convex_macro::prod_rt_test]
    async fn test_delete_component_read_only_key_rejected(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let auth = read_only_auth_header(&backend)?;
        let req = post_request("/api/delete_component", &auth, json!({}))?;
        backend
            .expect_error(req, http::StatusCode::FORBIDDEN, "Unauthorized")
            .await?;
        Ok(())
    }

    // check_admin_key tests

    #[convex_macro::prod_rt_test]
    async fn test_check_admin_key_full_admin_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = get_request("/api/check_admin_key", &backend.admin_auth_header)?;
        let _: serde_json::Value = backend.expect_success(req).await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_check_admin_key_read_only_key(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let auth = read_only_auth_header(&backend)?;
        let req = get_request("/api/check_admin_key", &auth)?;
        // check_admin_key should accept read-only keys.
        let _: serde_json::Value = backend.expect_success(req).await?;
        Ok(())
    }
}
