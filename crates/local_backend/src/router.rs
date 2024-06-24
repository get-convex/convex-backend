use std::{
    sync::Arc,
    time::Duration,
};

use axum::{
    error_handling::HandleErrorLayer,
    extract::DefaultBodyLimit,
    routing::{
        get,
        post,
    },
    BoxError,
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
use tower::ServiceBuilder;
use tower_http::{
    cors::{
        AllowOrigin,
        CorsLayer,
    },
    decompression::RequestDecompressionLayer,
};

use crate::{
    dashboard::{
        delete_tables,
        get_indexes,
        get_source_code,
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
    import::{
        import,
        import_finish_upload,
        import_start_upload,
        import_upload_part,
        perform_import,
        prepare_import,
    },
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
        public_mutation_post,
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
        get_export,
        get_zip_export,
        request_export,
        request_zip_export,
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

pub async fn router(st: LocalAppState) -> Router {
    let browser_routes = Router::new()
        // Called by the browser (and optionally authenticated by a cookie or `Authorization`
        // header). Passes version in the URL because websockets can't do it in header.
        .route("/:client_version/sync", get(sync_client_version_url));

    let dashboard_routes = Router::new()
        // Scheduled jobs routes
        .route("/cancel_all_jobs", post(cancel_all_jobs))
        .route("/cancel_job", post(cancel_job))
        // Environment variable routes
        .route("/update_environment_variables", post(update_environment_variables))
        // Administrative routes for the dashboard
        .route("/shapes2", get(shapes2))
        .route("/get_indexes", get(get_indexes))
        .route("/delete_tables", post(delete_tables))
        .route("/get_source_code", get(get_source_code))
        // Metrics routes
        .route("/app_metrics/stream_udf_execution", get(stream_udf_execution))
        .route("/app_metrics/stream_function_logs", get(stream_function_logs))
        .layer(ServiceBuilder::new());

    let cli_routes = Router::new()
        .route("/push_config", post(push_config))
        .route("/prepare_schema", post(prepare_schema))
        .route("/deploy2/start_push", post(deploy_config2::start_push))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::INTERNAL_SERVER_ERROR
                }))
                .layer(RequestDecompressionLayer::new())
                .layer(DefaultBodyLimit::max(*MAX_PUSH_BYTES)),
        )
        .route(
            "/deploy2/wait_for_schema",
            post(deploy_config2::wait_for_schema),
        )
        .route("/deploy2/finish_push", post(deploy_config2::finish_push))
        .route("/get_config", post(get_config))
        .route("/get_config_hashes", post(get_config_hashes))
        .route("/schema_state/:schema_id", get(schema_state))
        .route("/stream_udf_execution", get(stream_udf_execution))
        .route("/stream_function_logs", get(stream_function_logs))
        .merge(import_routes())
        .layer(cli_cors().await);

    let snapshot_export_routes = Router::new()
        .route("/request", post(request_export))
        .route("/:snapshot_ts/:table_name", get(get_export))
        .route("/request/zip", post(request_zip_export))
        .route("/zip/:snapshot_ts", get(get_zip_export));

    let api_routes = Router::new()
        .merge(cli_routes)
        .merge(dashboard_routes)
        .nest("/actions", action_callback_routes(st.clone()))
        .nest("/export", snapshot_export_routes)
        .nest("/storage", storage_api_routes());

    // Endpoints migrated to use the RouterState trait instead of application.
    let migrated_api_routes = Router::new()
        .merge(browser_routes)
        .merge(public_api_routes());
    let migrated = Router::new()
        .nest("/api", migrated_api_routes)
        .layer(cors().await)
        // Order matters. Layers only apply to routes above them.
        // Notably, any layers added here won't apply to common routes
        // added inside `serve_http`
        .nest("/http/", http_action_routes())
        .with_state(RouterState {
            api: Arc::new(st.application.clone()),
            runtime: st.application.runtime().clone(),
            live_ws_count: st.live_ws_count.clone(),
        });

    Router::new()
        .nest("/api", api_routes)
        .layer(cors().await)
        .with_state(st)
        .merge(migrated)
}

pub fn public_api_routes() -> Router<RouterState> {
    Router::new()
        .route("/sync", get(sync))
        .route("/query", get(public_query_get))
        .route("/query", post(public_query_post))
        .route("/query_batch", post(public_query_batch_post))
        .route("/mutation", post(public_mutation_post))
        .route("/action", post(public_action_post))
        .route("/function", post(public_function_post))
        .layer(DefaultBodyLimit::max(*MAX_BACKEND_PUBLIC_API_REQUEST_SIZE))
}

pub fn storage_api_routes() -> Router<LocalAppState> {
    Router::new()
        .route("/upload", post(storage_upload))
        .route("/:storage_id", get(storage_get))
}

pub fn action_callback_routes(st: LocalAppState) -> Router<LocalAppState> {
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
        .layer(axum::middleware::from_fn_with_state(st.clone(), action_callbacks_middleware))
}

pub fn import_routes() -> Router<LocalAppState> {
    Router::new()
        .route("/import", post(import))
        .route("/import/start_upload", post(import_start_upload))
        .route("/import/upload_part", post(import_upload_part))
        .route("/import/finish_upload", post(import_finish_upload))
        .route("/prepare_import", post(prepare_import))
        .route("/perform_import", post(perform_import))
}

pub fn http_action_routes() -> Router<RouterState> {
    Router::new()
        .route("/*rest", http_action_handler())
        .route("/", http_action_handler())
        .layer(DefaultBodyLimit::max(HTTP_ACTION_BODY_LIMIT))
}

pub async fn cors() -> CorsLayer {
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
