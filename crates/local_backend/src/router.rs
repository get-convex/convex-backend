use std::{
    convert::Infallible,
    sync::Arc,
    time::Duration,
};

use axum::{
    error_handling::HandleErrorLayer,
    extract::{
        DefaultBodyLimit,
        FromRef,
        State,
    },
    routing::{
        delete,
        get,
        post,
        put,
    },
    Router,
};
use common::{
    http::cli_cors,
    knobs::{
        AIRBYTE_STREAMING_IMPORT_REQUEST_SIZE_LIMIT,
        MAX_BACKEND_RPC_REQUEST_SIZE,
        MAX_ECHO_BYTES,
        MAX_PUSH_BYTES,
    },
};
use http::{
    Method,
    StatusCode,
};
use metrics::SERVER_VERSION_STR;
use tower::ServiceBuilder;
use tower_http::{
    cors::{
        AllowHeaders,
        AllowOrigin,
        CorsLayer,
    },
    decompression::RequestDecompressionLayer,
};
use udf::HTTP_ACTION_BODY_LIMIT;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::{
    app_metrics::{
        cache_hit_percentage,
        cache_hit_percentage_top_k,
        failure_percentage_top_k,
        latency_percentiles,
        scheduled_job_lag,
        table_rate,
        udf_rate,
    },
    canonical_urls::update_canonical_url,
    dashboard::{
        common_dashboard_api_router,
        local_only_dashboard_router,
        run_test_function,
    },
    deploy_config::{
        get_config,
        get_config_hashes,
        push_config,
    },
    deploy_config2,
    environment_variables::{
        list_environment_variables,
        platform_router,
        update_environment_variables,
    },
    http_actions::http_action_handler,
    log_sinks::{
        add_axiom_sink,
        add_datadog_sink,
        add_sentry_sink,
        add_webhook_sink,
        delete_log_sink,
    },
    logs::{
        stream_function_logs,
        stream_udf_execution,
    },
    node_action_callbacks::{
        action_callbacks_middleware,
        cancel_developer_job,
        create_function_handle,
        internal_action_post,
        internal_mutation_post,
        internal_query_post,
        schedule_job,
        storage_delete,
        storage_generate_upload_url,
        storage_get_metadata,
        storage_get_url,
        vector_search,
    },
    public_api::public_api_router,
    scheduling::{
        cancel_all_jobs,
        cancel_job,
    },
    schema::{
        prepare_schema,
        schema_state,
    },
    snapshot_export::{
        cancel_export,
        get_zip_export,
        request_zip_export,
        set_export_expiration,
    },
    snapshot_import::{
        cancel_import,
        import,
        import_finish_upload,
        import_start_upload,
        import_upload_part,
        perform_import,
    },
    storage::{
        storage_get,
        storage_upload,
    },
    streaming_export::{
        document_deltas_get,
        document_deltas_post,
        get_table_column_names,
        get_tables_and_columns,
        json_schemas,
        list_snapshot_get,
        list_snapshot_post,
        test_streaming_export_connection,
    },
    streaming_import::{
        add_primary_key_indexes,
        apply_fivetran_operations,
        clear_tables,
        fivetran_create_table,
        fivetran_truncate_table,
        get_schema,
        import_airbyte_records,
        primary_key_indexes_ready,
        replace_tables,
    },
    subs::sync,
    LocalAppState,
    RouterState,
};

// TODO security per endpoint

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Convex Deployment API",
        version = "1.0.0",
        description = "Admin API for interacting with deployments",
    ),
    servers(
        (url = "/api/v1", description = "Deployment API")
    )
)]
struct PlatformApiDoc;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Convex Public HTTP routes",
        version = "1.0.0",
        description = "Endpoints that require no authentication"
    ),
    servers(
        (url = "/api", description = "Deployment API")
    )
)]
struct PublicApiDoc;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Convex Dashboard HTTP routes",
        version = "1.0.0",
        description = "Endpoints intended for dashboard use"
    ),
    servers(
        (url = "/api", description = "Deployment API")
    )
)]
struct DashboardApiDoc;

pub async fn add_extension<S, B>(
    State(st): State<S>,
    mut request: http::Request<B>,
) -> http::Request<B>
where
    S: Clone + Send + Sync + 'static,
{
    request.extensions_mut().insert(st);
    request
}

pub fn router(st: LocalAppState) -> Router {
    let browser_routes = Router::new()
        // Called by the browser (and optionally authenticated by a cookie or `Authorization`
        // header). Passes version in the URL because websockets can't do it in header.
        .route("/{client_version}/sync", get(sync));

    // routes are added by common_dashboard_routes below
    let (_, common_dashboard_openapi_spec) =
        OpenApiRouter::with_openapi(DashboardApiDoc::openapi())
            .merge(common_dashboard_api_router())
            .split_for_parts();
    let (local_only_dashboard_routes, local_only_openapi_spec) =
        OpenApiRouter::with_openapi(DashboardApiDoc::openapi())
            .merge(local_only_dashboard_router())
            .split_for_parts();

    let mut dashboard_openapi_spec = common_dashboard_openapi_spec;
    dashboard_openapi_spec.merge(local_only_openapi_spec);
    let dashboard_openapi_json = dashboard_openapi_spec.to_pretty_json().unwrap();
    let dashboard_routes = common_dashboard_routes()
        .merge(local_only_dashboard_routes)
        // Environment variable routes
        .route("/update_environment_variables", post(update_environment_variables))
        .route("/list_environment_variables", get(list_environment_variables))
        // Canonical URL routes
        .route("/update_canonical_url", post(update_canonical_url))
        // Scheduled jobs routes
        .route("/cancel_all_jobs", post(cancel_all_jobs))
        .route("/cancel_job", post(cancel_job))
        .route("/dashboard_openapi.json", axum::routing::get({
            move || async { dashboard_openapi_json }
        }))
        .layer(ServiceBuilder::new());

    let cli_routes = Router::new()
        .route("/push_config", post(push_config))
        .route("/prepare_schema", post(prepare_schema))
        .route("/deploy2/start_push", post(deploy_config2::start_push))
        .route("/run_test_function", post(run_test_function))
        .route(
            "/deploy2/wait_for_schema",
            post(deploy_config2::wait_for_schema),
        )
        .route("/deploy2/finish_push", post(deploy_config2::finish_push))
        .route(
            "/deploy2/report_push_completed",
            post(deploy_config2::report_push_completed_handler),
        )
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|_: Infallible| async {
                    StatusCode::INTERNAL_SERVER_ERROR
                }))
                .layer(RequestDecompressionLayer::new())
                .layer(DefaultBodyLimit::max(*MAX_PUSH_BYTES)),
        )
        .route("/get_config", post(get_config))
        .route("/get_config_hashes", post(get_config_hashes))
        .route("/schema_state/{schema_id}", get(schema_state))
        .route("/stream_udf_execution", get(stream_udf_execution))
        .route("/stream_function_logs", get(stream_function_logs))
        .merge(import_routes())
        .layer(cli_cors());

    let snapshot_export_routes = Router::new()
        .route("/request/zip", post(request_zip_export))
        .route("/zip/{id}", get(get_zip_export))
        .route("/set_expiration/{snapshot_id}", post(set_export_expiration))
        .route("/cancel/{snapshot_id}", post(cancel_export));

    let (platform_routes, platform_openapi) =
        OpenApiRouter::with_openapi(PlatformApiDoc::openapi())
            .merge(platform_router())
            .split_for_parts();
    let platform_openapi_spec = platform_openapi.to_pretty_json().unwrap();
    let platform_routes = Router::new().merge(platform_routes).route(
        "/openapi.json",
        axum::routing::get(move || async { platform_openapi_spec }),
    );

    let api_routes = Router::new()
        .merge(cli_routes)
        .merge(dashboard_routes)
        .merge(streaming_export_routes())
        .nest(
            "/actions",
            action_callback_routes().layer(axum::middleware::map_request_with_state(
                st.clone(),
                add_extension::<LocalAppState, _>,
            )),
        )
        .nest("/export", snapshot_export_routes)
        .nest("/logs", log_sink_routes())
        .nest("/streaming_import", streaming_import_routes())
        .nest("/v1", platform_routes);

    // Endpoints migrated to use the RouterState trait instead of application.
    let (public_routes, public_openapi) = OpenApiRouter::with_openapi(PublicApiDoc::openapi())
        .merge(public_api_router())
        .split_for_parts();
    let public_openapi_spec = public_openapi.to_pretty_json().unwrap();

    let migrated_api_routes = Router::new()
        .merge(browser_routes)
        .merge(public_routes)
        .route("/sync", get(sync))
        .route(
            "/public_openapi.json",
            axum::routing::get({
                let spec = public_openapi_spec.clone();
                move || async move { spec }
            }),
        )
        .nest("/storage", storage_api_routes());
    let migrated = Router::new()
        .nest("/api", migrated_api_routes)
        .layer(cors())
        // Order matters. Layers only apply to routes above them.
        // Notably, any layers added here won't apply to common routes
        // added inside `serve_http`
        .nest("/http/", http_action_routes())
        .with_state(RouterState {
            api: Arc::new(st.application.clone()),
            runtime: st.application.runtime(),
        });

    let version = SERVER_VERSION_STR.to_string();

    Router::new()
        .nest("/api", api_routes)
        .merge(health_check_routes(version))
        .layer(cors())
        .with_state(st)
        .merge(migrated)
}

pub fn public_api_routes<S>() -> Router<S>
where
    RouterState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    let (routes, _openapi_spec) = OpenApiRouter::with_openapi(PlatformApiDoc::openapi())
        .merge(public_api_router())
        .split_for_parts();
    routes.route("/sync", get(sync))
}

pub fn storage_api_routes() -> Router<RouterState> {
    Router::new()
        .route("/upload", post(storage_upload))
        .route("/{storage_id}", get(storage_get))
}

// IMPORTANT NOTE: Those routes are proxied by Usher. Any changes to the router,
// such as adding or removing a route, or changing limits, also need to be
// applied to `crates_private/usher/src/proxy.rs`.
pub fn action_callback_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Send + Sync + Clone + 'static,
{
    Router::new()
        .route("/query", post(internal_query_post))
        .route("/mutation", post(internal_mutation_post))
        .route("/action", post(internal_action_post))
        .route("/schedule_job", post(schedule_job))
        .route("/vector_search", post(vector_search))
        .route("/cancel_job", post(cancel_developer_job))
        .route("/create_function_handle", post(create_function_handle))
        // file storage endpoints
        .route("/storage_generate_upload_url", post(storage_generate_upload_url))
        .route("/storage_get_url", post(storage_get_url))
        .route("/storage_get_metadata", post(storage_get_metadata))
        .route("/storage_delete", post(storage_delete))
        // All routes above this line get the increased limit
        .layer(DefaultBodyLimit::max(*MAX_BACKEND_RPC_REQUEST_SIZE))
        .layer(axum::middleware::from_fn(action_callbacks_middleware))
}

pub fn import_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/import", post(import))
        .route("/import/start_upload", post(import_start_upload))
        .route("/import/upload_part", post(import_upload_part))
        .route("/import/finish_upload", post(import_finish_upload))
        .route("/perform_import", post(perform_import))
        .route("/cancel_import", post(cancel_import))
}

pub fn http_action_routes() -> Router<RouterState> {
    Router::new()
        .route("/{*rest}", http_action_handler())
        .route("/", http_action_handler())
        .layer(DefaultBodyLimit::max(HTTP_ACTION_BODY_LIMIT))
}

pub fn app_metrics_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/stream_udf_execution", get(stream_udf_execution))
        .route("/stream_function_logs", get(stream_function_logs))
        .route("/udf_rate", get(udf_rate))
        .route("/failure_percentage_top_k", get(failure_percentage_top_k))
        .route(
            "/cache_hit_percentage_top_k",
            get(cache_hit_percentage_top_k),
        )
        .route("/cache_hit_percentage", get(cache_hit_percentage))
        .route("/table_rate", get(table_rate))
        .route("/latency_percentiles", get(latency_percentiles))
        .route("/scheduled_job_lag", get(scheduled_job_lag))
}

// Routes with the same handlers for the local backend + closed source backend
pub fn common_dashboard_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    let (dashboard_routes_from_openapi, _dashboard_openapi_spec) =
        OpenApiRouter::with_openapi(DashboardApiDoc::openapi())
            .merge(common_dashboard_api_router())
            .split_for_parts();
    Router::new()
        .merge(dashboard_routes_from_openapi)
        // Metrics routes
        .nest("/app_metrics", app_metrics_routes())
}

pub fn health_check_routes<S>(version: String) -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/instance_name",
            get(|State(st): State<LocalAppState>| async move { st.instance_name.clone() }),
        )
        .route("/instance_version", get(|| async move { version }))
        .route(
            "/",
            get(|| async { "This Convex deployment is running. See https://docs.convex.dev/." }),
        )
        .route(
            "/echo",
            post(|body: axum::body::Body| async move { body })
        // Limit requests to 128MiB to help mitigate DDoS attacks.
        .layer(DefaultBodyLimit::max(*MAX_ECHO_BYTES)),
        )
        .layer(cors())
}

// IMPORTANT NOTE: Those routes are proxied by Usher. Any changes to the router,
// such as adding or removing a route, or changing limits, also need to be
// applied to `crates_private/usher/src/proxy.rs`.
pub fn streaming_import_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/import_airbyte_records",
            post(import_airbyte_records).layer(DefaultBodyLimit::max(
                *AIRBYTE_STREAMING_IMPORT_REQUEST_SIZE_LIMIT,
            )),
        )
        .route(
            "/apply_fivetran_operations",
            post(apply_fivetran_operations),
        )
        .route("/get_schema", get(get_schema))
        .route("/replace_tables", post(replace_tables))
        .route("/clear_tables", put(clear_tables))
        .route("/fivetran_truncate_table", post(fivetran_truncate_table))
        .route("/fivetran_create_table", post(fivetran_create_table))
        .route("/add_primary_key_indexes", put(add_primary_key_indexes))
        .route("/primary_key_indexes_ready", get(primary_key_indexes_ready))
}

// IMPORTANT NOTE: Those routes are proxied by Usher. Any changes to the router,
// such as adding or removing a route, or changing limits, also need to be
// applied to `crates_private/usher/src/proxy.rs`.
pub fn streaming_export_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/document_deltas", get(document_deltas_get))
        .route("/document_deltas", post(document_deltas_post))
        .route("/list_snapshot", get(list_snapshot_get))
        .route("/list_snapshot", post(list_snapshot_post))
        .route("/json_schemas", get(json_schemas))
        .route(
            "/test_streaming_export_connection",
            get(test_streaming_export_connection),
        )
        .route("/get_tables_and_columns", get(get_tables_and_columns))
        .route("/get_table_column_names", get(get_table_column_names))
}

// IMPORTANT NOTE: Those routes are proxied by Usher. Any changes to the router,
// such as adding or removing a route, or changing limits, also need to be
// applied to `crates_private/usher/src/proxy.rs`.
pub fn log_sink_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/datadog_sink", post(add_datadog_sink))
        .route("/webhook_sink", post(add_webhook_sink))
        .route("/axiom_sink", post(add_axiom_sink))
        .route("/sentry_sink", post(add_sentry_sink))
        .route("/delete_sink", delete(delete_log_sink))
}

pub fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::OPTIONS,
            Method::PATCH,
            Method::DELETE,
            Method::PUT,
        ])
        .allow_origin(AllowOrigin::mirror_request())
        .max_age(Duration::from_secs(86400))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Context;
    use axum::body::Body;
    use axum_extra::headers::authorization::Credentials;
    use http::Request;
    use runtime::prod::ProdRuntime;

    use crate::test_helpers::setup_backend_for_test;

    const DASHBOARD_SPEC_FILE: &str =
        "../../npm-packages/dashboard/dashboard-deployment-openapi.json";
    const PUBLIC_SPEC_FILE: &str =
        "../../npm-packages/@convex-dev/platform/public-deployment-openapi.json";
    const PLATFORM_SPEC_FILE: &str =
        "../../npm-packages/@convex-dev/platform/deployment-openapi.json";

    #[convex_macro::prod_rt_test]
    async fn test_api_specs_match(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        let dashboard_req = Request::builder()
            .uri("/api/dashboard_openapi.json")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Host", "localhost")
            .body(Body::empty())?;

        let public_req = Request::builder()
            .uri("/api/public_openapi.json")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Host", "localhost")
            .body(Body::empty())?;

        let platform_req = Request::builder()
            .uri("/api/v1/openapi.json")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Host", "localhost")
            .body(Body::empty())?;

        let actual_dashboard: serde_json::Value = backend.expect_success(dashboard_req).await?;
        let actual_public: serde_json::Value = backend.expect_success(public_req).await?;
        let actual_platform: serde_json::Value = backend.expect_success(platform_req).await?;

        let actual_dashboard = serde_json::to_string_pretty(&actual_dashboard)?;
        let actual_public = serde_json::to_string_pretty(&actual_public)?;
        let actual_platform = serde_json::to_string_pretty(&actual_platform)?;

        let expected_dashboard = fs::read_to_string(DASHBOARD_SPEC_FILE)
            .context(format!("Couldn't read {DASHBOARD_SPEC_FILE}"))?;
        let expected_public = fs::read_to_string(PUBLIC_SPEC_FILE)
            .context(format!("Couldn't read {PUBLIC_SPEC_FILE}"))?;
        let expected_platform = fs::read_to_string(PLATFORM_SPEC_FILE)
            .context(format!("Couldn't read {PLATFORM_SPEC_FILE}"))?;

        if expected_dashboard != actual_dashboard
            || expected_public != actual_public
            || expected_platform != actual_platform
        {
            fs::write(DASHBOARD_SPEC_FILE, &actual_dashboard)?;
            fs::write(PUBLIC_SPEC_FILE, &actual_public)?;
            fs::write(PLATFORM_SPEC_FILE, &actual_platform)?;
            panic!(
                "{DASHBOARD_SPEC_FILE} or {PUBLIC_SPEC_FILE} or {PLATFORM_SPEC_FILE} does not \
                 match result of http route changes. This test will automatically update \
                 dashboard-deployment-openapi.json, deployment-public-openapi.json, and \
                 deployment-openapi.json so you can run again: `cargo test -p local_backend \
                 test_api_specs_match`"
            );
        }
        Ok(())
    }
}
