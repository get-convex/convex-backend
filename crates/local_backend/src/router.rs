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
        get,
        post,
    },
    Router,
};
use common::{
    http::{
        cli_cors,
        CONVEX_CLIENT_HEADER,
    },
    knobs::{
        MAX_BACKEND_PUBLIC_API_REQUEST_SIZE,
        MAX_BACKEND_RPC_REQUEST_SIZE,
        MAX_PUSH_BYTES,
    },
};
use http::{
    header::{
        AUTHORIZATION,
        CONTENT_TYPE,
    },
    request,
    HeaderValue,
    Method,
    StatusCode,
};
use isolate::HTTP_ACTION_BODY_LIMIT;
use metrics::SERVER_VERSION_STR;
use tower::ServiceBuilder;
use tower_http::{
    cors::{
        AllowOrigin,
        CorsLayer,
    },
    decompression::RequestDecompressionLayer,
};

use crate::{
    app_metrics::{
        cache_hit_percentage,
        latency_percentiles,
        table_rate,
        udf_rate,
    },
    dashboard::{
        delete_component,
        delete_tables,
        get_indexes,
        get_source_code,
        run_test_function,
        shapes2,
    },
    deploy_config::{
        get_config,
        get_config_hashes,
        push_config,
    },
    deploy_config2,
    environment_variables::update_environment_variables,
    http_actions::http_action_handler,
    logs::{
        stream_function_logs,
        stream_udf_execution,
    },
    node_action_callbacks::{
        action_callbacks_middleware,
        cancel_developer_job,
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
    public_api::{
        public_action_post,
        public_function_post,
        public_function_post_with_path,
        public_get_query_ts,
        public_mutation_post,
        public_query_at_ts_post,
        public_query_batch_post,
        public_query_get,
        public_query_post,
    },
    scheduling::{
        cancel_all_jobs,
        cancel_job,
    },
    schema::{
        prepare_schema,
        schema_state,
    },
    snapshot_export::{
        get_zip_export,
        request_zip_export,
    },
    snapshot_import::{
        cancel_import,
        import,
        import_finish_upload,
        import_start_upload,
        import_upload_part,
        perform_import,
        prepare_import,
    },
    storage::{
        storage_get,
        storage_upload,
    },
    subs::{
        sync,
        sync_client_version_url,
    },
    LocalAppState,
    RouterState,
};

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
        .route("/:client_version/sync", get(sync_client_version_url));

    let dashboard_routes = common_dashboard_routes()
        // Scheduled jobs routes
        .route("/cancel_all_jobs", post(cancel_all_jobs))
        .route("/cancel_job", post(cancel_job))
        // Environment variable routes
        .route("/update_environment_variables", post(update_environment_variables))
        // Administrative routes for the dashboard
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
        .route("/schema_state/:schema_id", get(schema_state))
        .route("/stream_udf_execution", get(stream_udf_execution))
        .route("/stream_function_logs", get(stream_function_logs))
        .merge(import_routes())
        .layer(cli_cors());

    let snapshot_export_routes = Router::new()
        .route("/request/zip", post(request_zip_export))
        .route("/zip/:snapshot_ts", get(get_zip_export));

    let api_routes = Router::new()
        .merge(cli_routes)
        .merge(dashboard_routes)
        .nest(
            "/actions",
            action_callback_routes().layer(axum::middleware::map_request_with_state(
                st.clone(),
                add_extension::<LocalAppState, _>,
            )),
        )
        .nest("/export", snapshot_export_routes);

    // Endpoints migrated to use the RouterState trait instead of application.
    let migrated_api_routes = Router::new()
        .merge(browser_routes)
        .merge(public_api_routes())
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
            runtime: st.application.runtime().clone(),
        });

    let instance_name = st.instance_name.clone();
    let version = SERVER_VERSION_STR.to_string();

    Router::new()
        .nest("/api", api_routes)
        // /instance_name is used by the CLI and dashboard to check connectivity!
        .route("/instance_name", get(|| async move { instance_name }))
        .route("/instance_version", get(|| async move { version }))
        .layer(cors())
        .with_state(st)
        .merge(migrated)
}

pub fn public_api_routes() -> Router<RouterState> {
    Router::new()
        .route("/sync", get(sync))
        .route("/query", get(public_query_get))
        .route("/query", post(public_query_post))
        .route("/query_at_ts", post(public_query_at_ts_post))
        .route("/query_ts", post(public_get_query_ts))
        .route("/query_batch", post(public_query_batch_post))
        .route("/mutation", post(public_mutation_post))
        .route("/action", post(public_action_post))
        .route("/function", post(public_function_post))
        .route("/run/*rest", post(public_function_post_with_path))
        .layer(DefaultBodyLimit::max(*MAX_BACKEND_PUBLIC_API_REQUEST_SIZE))
}

pub fn storage_api_routes() -> Router<RouterState> {
    Router::new()
        .route("/upload", post(storage_upload))
        .route("/:storage_id", get(storage_get))
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
        .route("/prepare_import", post(prepare_import))
        .route("/perform_import", post(perform_import))
        .route("/cancel_import", post(cancel_import))
}

pub fn http_action_routes() -> Router<RouterState> {
    Router::new()
        .route("/*rest", http_action_handler())
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
        .route("/cache_hit_percentage", get(cache_hit_percentage))
        .route("/table_rate", get(table_rate))
        .route("/latency_percentiles", get(latency_percentiles))
}

// Routes with the same handlers for the local backend + closed source backend
pub fn common_dashboard_routes<S>() -> Router<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/shapes2", get(shapes2))
        .route("/get_indexes", get(get_indexes))
        .route("/delete_tables", post(delete_tables))
        .route("/delete_component", post(delete_component))
        .route("/get_source_code", get(get_source_code))
        // Metrics routes
        .nest("/app_metrics", app_metrics_routes())
}

pub fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_headers(vec![CONTENT_TYPE, "sentry-trace".parse().unwrap(), "baggage".parse().unwrap(), CONVEX_CLIENT_HEADER, AUTHORIZATION])
        .allow_credentials(true)
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::OPTIONS,
            Method::PATCH,
            Method::DELETE,
            Method::PUT,
        ])
        // Don't use tower_http::cors::any(), it causes the server to respond with
        // Access-Control-Allow-Origin: *. Browsers restrict sending credentials to other domains
        // that reply to a CORS with allow-origin *.
        //
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS/Errors/CORSNotSupportingCredentials
        //
        // Instead respond with Access-Control-Allow-Origin set to the submitted Origin header.
        //
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Origin#directives
        .allow_origin(
            AllowOrigin::predicate(|_origin: &HeaderValue, _request_head: &request::Parts| {
                true
            }),
        )
        .max_age(Duration::from_secs(86400))
}
