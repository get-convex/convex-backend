#![feature(try_blocks)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(coroutines)]
#![feature(round_char_boundary)]
#![feature(duration_constructors)]
#![feature(duration_constructors_lite)]
#![feature(assert_matches)]

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    ops::Bound,
    sync::Arc,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};

use ::exports::interface::ExportProvider;
use ::log_streaming::{
    LogManager,
    LogManagerClient,
};
use airbyte_import::{
    AirbyteRecord,
    PrimaryKey,
    ValidatedAirbyteStream,
};
use anyhow::Context;
use authentication::{
    application_auth::ApplicationAuth,
    validate_id_token,
    AuthIdToken,
};
use aws_s3::storage::S3Storage;
use bytes::Bytes;
use chrono::{
    DateTime,
    Utc,
};
use common::{
    auth::{
        AuthConfig,
        AuthInfo,
    },
    bootstrap_model::{
        components::handles::FunctionHandle,
        index::{
            database_index::IndexedFields,
            index_validation_error,
            IndexMetadata,
        },
        schema::{
            invalid_schema_id,
            parse_schema_id,
            SchemaState,
        },
    },
    components::{
        CanonicalizedComponentFunctionPath,
        CanonicalizedComponentModulePath,
        ComponentDefinitionPath,
        ComponentId,
        ComponentPath,
        PublicFunctionPath,
        Reference,
        Resource,
    },
    document::{
        DocumentUpdate,
        CREATION_TIME_FIELD_PATH,
    },
    errors::{
        report_error,
        JsError,
    },
    http::{
        fetch::FetchClient,
        RequestDestination,
    },
    knobs::{
        APPLICATION_MAX_CONCURRENT_UPLOADS,
        ENABLE_INDEX_BACKFILL,
        MAX_JOBS_CANCEL_BATCH,
        MAX_USER_MODULES,
    },
    log_lines::LogLines,
    log_streaming::LogSender,
    paths::FieldPath,
    persistence::Persistence,
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
    },
    query_journal::QueryJournal,
    runtime::{
        shutdown_and_join,
        JoinSet,
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    schemas::{
        DatabaseSchema,
        TableDefinition,
    },
    shutdown::ShutdownSignal,
    types::{
        env_var_limit_met,
        env_var_name_not_unique,
        AllowedVisibility,
        ConvexOrigin,
        ConvexSite,
        CursorMs,
        EnvVarName,
        EnvVarValue,
        FullyQualifiedObjectKey,
        FunctionCaller,
        IndexId,
        IndexName,
        ModuleEnvironment,
        NodeDependency,
        ObjectKey,
        RepeatableTimestamp,
        TableName,
        Timestamp,
        UdfIdentifier,
        UdfType,
        ENV_VAR_LIMIT,
    },
    RequestId,
};
use convex_fivetran_destination::{
    api_types::{
        BatchWriteRow,
        DeleteType,
    },
    constants::FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
};
use cron_jobs::CronJobExecutor;
use database::{
    unauthorized_error,
    BootstrapComponentsModel,
    Database,
    FastForwardIndexWorker,
    IndexModel,
    IndexWorker,
    OccRetryStats,
    ResolvedQuery,
    SchemaModel,
    SearchIndexWorkers,
    Snapshot,
    Subscription,
    TableModel,
    Token,
    Transaction,
    UserFacingModel,
    WriteSource,
};
use either::Either;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use fastrace::{
    collector::SpanContext,
    func_path,
    future::FutureExt,
    Span,
};
use file_storage::{
    FileRangeStream,
    FileStorage,
    FileStream,
};
use function_log::{
    FunctionExecution,
    FunctionExecutionPart,
};
use function_runner::FunctionRunner;
use futures::stream::BoxStream;
use headers::{
    ContentLength,
    ContentType,
};
use http_client::{
    cached_http_client_for,
    ClientPurpose,
};
use isolate::helpers::source_map_from_slice;
use keybroker::{
    Identity,
    KeyBroker,
};
use log_streaming::add_local_log_sink_on_startup;
use maplit::{
    btreemap,
    btreeset,
};
use model::{
    airbyte_import::{
        AirbyteImportModel,
        AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR,
    },
    auth::AuthInfoModel,
    backend_info::BackendInfoModel,
    backend_state::BackendStateModel,
    canonical_urls::{
        types::CanonicalUrl,
        CanonicalUrlsModel,
    },
    components::{
        config::ComponentConfigModel,
        handles::FunctionHandlesModel,
        types::ProjectConfig,
        ComponentsModel,
    },
    config::{
        module_loader::ModuleLoader,
        types::{
            ConfigFile,
            ConfigMetadata,
            ModuleConfig,
            AUTH_CONFIG_FILE_NAME,
        },
        ConfigModel,
    },
    database_globals::{
        types::StorageTagInitializer,
        DatabaseGlobalsModel,
    },
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    environment_variables::{
        types::EnvironmentVariable,
        EnvironmentVariablesModel,
    },
    exports::{
        types::{
            Export,
            ExportFormat,
            ExportRequestor,
        },
        ExportsModel,
    },
    external_packages::{
        types::{
            ExternalDepsPackage,
            ExternalDepsPackageId,
        },
        ExternalPackagesModel,
    },
    file_storage::{
        types::FileStorageEntry,
        FileStorageId,
    },
    fivetran_import::FivetranImportModel,
    migrations::MigrationWorker,
    modules::{
        module_versions::{
            AnalyzedModule,
            Visibility,
        },
        ModuleModel,
    },
    scheduled_jobs::SchedulerModel,
    session_requests::types::SessionRequestIdentifier,
    snapshot_imports::types::{
        ImportFormat,
        ImportMode,
        ImportRequestor,
    },
    source_packages::{
        types::{
            NodeVersion,
            PackageSize,
            SourcePackage,
        },
        upload_download::upload_package,
        SourcePackageModel,
    },
    udf_config::{
        types::UdfConfig,
        UdfConfigModel,
    },
};
use node_executor::Actions;
use parking_lot::Mutex;
use rand::Rng;
use scheduled_jobs::ScheduledJobRunner;
use schema_worker::SchemaWorker;
use search::{
    query::RevisionWithKeys,
    searcher::{
        Searcher,
        SegmentTermMetadataFetcher,
    },
};
use semver::Version;
use serde_json::Value as JsonValue;
use short_future::ShortBoxFuture;
use snapshot_import::{
    clear_tables,
    start_stored_import,
};
use storage::{
    BufferedUpload,
    ClientDrivenUploadPartToken,
    ClientDrivenUploadToken,
    LocalDirStorage,
    Storage,
    StorageExt,
    StorageGetStream,
    StorageUseCase,
    Upload,
};
use sync_types::{
    AuthenticationToken,
    CanonicalizedModulePath,
    CanonicalizedUdfPath,
    FunctionName,
    ModulePath,
    SerializedQueryJournal,
};
use system_table_cleanup::SystemTableCleanupWorker;
use table_summary_worker::{
    TableSummaryClient,
    TableSummaryWorker,
};
use tokio::sync::{
    oneshot,
    Semaphore,
};
use udf::{
    environment::{
        system_env_var_overrides,
        CONVEX_ORIGIN,
        CONVEX_SITE,
    },
    helpers::parse_udf_args,
    HttpActionRequest,
    HttpActionResponseStreamer,
    HttpActionResult,
};
use udf_metrics::{
    MetricsWindow,
    Percentile,
    Timeseries,
};
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
    UsageCounter,
};
use value::{
    id_v6::DeveloperDocumentId,
    sha256::Sha256Digest,
    JsonPackedValue,
    Namespace,
    ResolvedDocumentId,
    TableNamespace,
};
use vector::{
    PublicVectorSearchQueryResult,
    VectorSearch,
};

use crate::{
    application_function_runner::ApplicationFunctionRunner,
    exports::worker::ExportWorker,
    function_log::{
        FunctionExecutionLog,
        TableRate,
        UdfMetricSummary,
        UdfRate,
    },
    log_visibility::LogVisibility,
    module_cache::ModuleCache,
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
    snapshot_import::SnapshotImportWorker,
};

pub mod airbyte_import;
pub mod api;
pub mod application_function_runner;
mod cache;
pub mod cron_jobs;
pub mod deploy_config;
mod exports;
pub mod function_log;
mod log_streaming;
pub mod log_visibility;
mod metrics;
mod module_cache;
pub mod redaction;
pub mod scheduled_jobs;
mod schema_worker;
pub mod snapshot_import;
mod streaming_export;
mod system_table_cleanup;
mod table_summary_worker;
pub mod valid_identifier;

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;
#[cfg(test)]
mod tests;

pub use crate::cache::QueryCache;
use crate::metrics::{
    log_external_deps_package,
    log_source_package_size_bytes_total,
};

pub struct ConfigMetadataAndSchema {
    pub config_metadata: ConfigMetadata,
    pub schema: Option<DatabaseSchema>,
}

#[derive(Clone)]
pub struct ApplyConfigArgs {
    pub auth_module: Option<ModuleConfig>,
    pub config_file: ConfigFile,
    pub schema_id: Option<String>,
    pub modules: Vec<ModuleConfig>,
    pub udf_config: UdfConfig,
    pub source_package: SourcePackage,
    pub analyze_results: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
}

#[derive(Debug)]
pub struct QueryReturn {
    pub result: Result<JsonPackedValue, JsError>,
    pub log_lines: LogLines,
    pub token: Token,
    pub journal: QueryJournal,
}

#[derive(Debug)]
pub struct RedactedQueryReturn {
    pub result: Result<JsonPackedValue, RedactedJsError>,
    pub log_lines: RedactedLogLines,
    pub token: Token,
    pub journal: SerializedQueryJournal,
}

#[derive(Debug)]
pub struct MutationReturn {
    pub value: JsonPackedValue,
    pub log_lines: LogLines,
    pub ts: Timestamp,
}

#[derive(Debug)]
pub struct RedactedMutationReturn {
    pub value: JsonPackedValue,
    pub log_lines: RedactedLogLines,
    pub ts: Timestamp,
}

#[derive(thiserror::Error, Debug)]
#[error("Mutation failed: {error}")]
pub struct MutationError {
    pub error: JsError,
    pub log_lines: LogLines,
}

#[derive(thiserror::Error, Debug)]
#[error("Mutation failed: {error}")]
pub struct RedactedMutationError {
    pub error: RedactedJsError,
    pub log_lines: RedactedLogLines,
}

#[derive(Debug)]
pub struct ActionReturn {
    pub value: JsonPackedValue,
    pub log_lines: LogLines,
}

#[derive(Debug)]
pub struct RedactedActionReturn {
    pub value: JsonPackedValue,
    pub log_lines: RedactedLogLines,
}

#[derive(thiserror::Error, Debug)]
#[error("Action failed: {error}")]
pub struct ActionError {
    pub error: JsError,
    pub log_lines: LogLines,
}

#[derive(thiserror::Error, Debug)]
#[error("Action failed: {error}")]
pub struct RedactedActionError {
    pub error: RedactedJsError,
    pub log_lines: RedactedLogLines,
}

#[derive(Debug)]
pub struct FunctionReturn {
    pub value: JsonPackedValue,
    pub log_lines: RedactedLogLines,
}

#[derive(thiserror::Error, Debug)]
#[error("Function failed: {error}")]
pub struct FunctionError {
    pub error: RedactedJsError,
    pub log_lines: RedactedLogLines,
}

// Ordered so that all unsets come before sets
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum EnvVarChange {
    Unset(EnvVarName),
    Set(EnvironmentVariable),
}

#[derive(Clone)]
pub struct ApplicationStorage {
    pub files_storage: Arc<dyn Storage>,
    pub modules_storage: Arc<dyn Storage>,
    search_storage: Arc<dyn Storage>,
    pub exports_storage: Arc<dyn Storage>,
    snapshot_imports_storage: Arc<dyn Storage>,
}

pub struct Application<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    runner: Arc<ApplicationFunctionRunner<RT>>,
    function_log: FunctionExecutionLog<RT>,
    file_storage: FileStorage<RT>,
    application_storage: ApplicationStorage,
    usage_tracking: UsageCounter,
    key_broker: KeyBroker,
    instance_name: String,
    scheduled_job_runner: ScheduledJobRunner,
    cron_job_executor: Arc<Mutex<Box<dyn SpawnHandle>>>,
    index_worker: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
    fast_forward_worker: Arc<Mutex<Box<dyn SpawnHandle>>>,
    search_worker: Arc<Mutex<SearchIndexWorkers>>,
    search_and_vector_bootstrap_worker: Arc<Mutex<Box<dyn SpawnHandle>>>,
    table_summary_worker: TableSummaryClient,
    schema_worker: Arc<Mutex<Box<dyn SpawnHandle>>>,
    snapshot_import_worker: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
    export_worker: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
    system_table_cleanup_worker: Arc<Mutex<Box<dyn SpawnHandle>>>,
    migration_worker: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
    log_visibility: Arc<dyn LogVisibility<RT>>,
    module_cache: ModuleCache<RT>,
    system_env_var_names: HashSet<EnvVarName>,
    app_auth: Arc<ApplicationAuth>,
    log_manager_client: LogManagerClient,
}

impl<RT: Runtime> Clone for Application<RT> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            database: self.database.clone(),
            runner: self.runner.clone(),
            function_log: self.function_log.clone(),
            file_storage: self.file_storage.clone(),
            application_storage: self.application_storage.clone(),
            usage_tracking: self.usage_tracking.clone(),
            key_broker: self.key_broker.clone(),
            instance_name: self.instance_name.clone(),
            scheduled_job_runner: self.scheduled_job_runner.clone(),
            cron_job_executor: self.cron_job_executor.clone(),
            index_worker: self.index_worker.clone(),
            fast_forward_worker: self.fast_forward_worker.clone(),
            search_worker: self.search_worker.clone(),
            search_and_vector_bootstrap_worker: self.search_and_vector_bootstrap_worker.clone(),
            table_summary_worker: self.table_summary_worker.clone(),
            schema_worker: self.schema_worker.clone(),
            snapshot_import_worker: self.snapshot_import_worker.clone(),
            export_worker: self.export_worker.clone(),
            system_table_cleanup_worker: self.system_table_cleanup_worker.clone(),
            migration_worker: self.migration_worker.clone(),
            log_visibility: self.log_visibility.clone(),
            module_cache: self.module_cache.clone(),
            system_env_var_names: self.system_env_var_names.clone(),
            app_auth: self.app_auth.clone(),
            log_manager_client: self.log_manager_client.clone(),
        }
    }
}

/// Create storage based on the storage type configuration
pub async fn create_storage<RT: Runtime>(
    runtime: RT,
    storage_type: &model::database_globals::types::StorageType,
    use_case: StorageUseCase,
) -> anyhow::Result<Arc<dyn Storage>> {
    Ok(match storage_type {
        model::database_globals::types::StorageType::S3 { s3_prefix } => {
            Arc::new(S3Storage::for_use_case(use_case, s3_prefix.clone(), runtime).await?)
        },
        model::database_globals::types::StorageType::Local { dir } => {
            let storage = LocalDirStorage::for_use_case(runtime, dir, use_case)?;
            tracing::info!("{use_case} storage path: {:?}", storage.path());
            Arc::new(storage)
        },
    })
}

impl<RT: Runtime> Application<RT> {
    pub async fn initialize_storage(
        runtime: RT,
        database: &Database<RT>,
        storage_tag_initializer: StorageTagInitializer,
        instance_name: String,
    ) -> anyhow::Result<ApplicationStorage> {
        let storage_type = {
            let mut tx = database.begin_system().await?;
            let storage_type = DatabaseGlobalsModel::new(&mut tx)
                .initialize_storage_tag(storage_tag_initializer, instance_name)
                .await?;
            database
                .commit_with_write_source(tx, "init_storage")
                .await?;
            storage_type
        };

        let files_storage =
            create_storage(runtime.clone(), &storage_type, StorageUseCase::Files).await?;
        let modules_storage =
            create_storage(runtime.clone(), &storage_type, StorageUseCase::Modules).await?;
        let search_storage = create_storage(
            runtime.clone(),
            &storage_type,
            StorageUseCase::SearchIndexes,
        )
        .await?;
        let exports_storage =
            create_storage(runtime.clone(), &storage_type, StorageUseCase::Exports).await?;
        let snapshot_imports_storage = create_storage(
            runtime.clone(),
            &storage_type,
            StorageUseCase::SnapshotImports,
        )
        .await?;

        // Search storage needs to be set for Database to be fully initialized
        database.set_search_storage(search_storage.clone());
        tracing::info!("{:?} storage is configured.", storage_type);

        Ok(ApplicationStorage {
            files_storage,
            modules_storage,
            search_storage,
            exports_storage,
            snapshot_imports_storage,
        })
    }

    pub async fn new(
        runtime: RT,
        database: Database<RT>,
        file_storage: FileStorage<RT>,
        application_storage: ApplicationStorage,
        usage_tracking: UsageCounter,
        key_broker: KeyBroker,
        instance_name: String,
        function_runner: Arc<dyn FunctionRunner<RT>>,
        convex_origin: ConvexOrigin,
        convex_site: ConvexSite,
        searcher: Arc<dyn Searcher>,
        segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
        persistence: Arc<dyn Persistence>,
        node_actions: Actions<RT>,
        log_visibility: Arc<dyn LogVisibility<RT>>,
        app_auth: Arc<ApplicationAuth>,
        cache: QueryCache,
        fetch_client: Arc<dyn FetchClient>,
        local_log_sink: Option<String>,
        lease_lost_shutdown: ShutdownSignal,
        export_provider: Arc<dyn ExportProvider<RT>>,
    ) -> anyhow::Result<Self> {
        let module_cache =
            ModuleCache::new(runtime.clone(), application_storage.modules_storage.clone()).await;
        let module_loader = Arc::new(module_cache.clone());

        let default_system_env_vars = btreemap! {
            CONVEX_ORIGIN.clone() => convex_origin.parse()?,
            CONVEX_SITE.clone() => convex_site.parse()?
        };

        let mut index_worker = Arc::new(Mutex::new(None));
        if *ENABLE_INDEX_BACKFILL {
            let index_worker_fut = IndexWorker::new(
                runtime.clone(),
                persistence.clone(),
                database.retention_validator(),
                database.clone(),
            );
            index_worker = Arc::new(Mutex::new(Some(
                runtime.spawn("index_worker", index_worker_fut),
            )));
        };
        let fast_forward_worker =
            FastForwardIndexWorker::create_and_start(runtime.clone(), database.clone());
        let fast_forward_worker = Arc::new(Mutex::new(
            runtime.spawn("fast_forward_worker", fast_forward_worker),
        ));
        let search_worker = SearchIndexWorkers::create_and_start(
            runtime.clone(),
            database.clone(),
            persistence.reader(),
            application_storage.search_storage.clone(),
            searcher,
            segment_term_metadata_fetcher,
        );
        let search_worker = Arc::new(Mutex::new(search_worker));
        let search_and_vector_bootstrap_worker =
            Arc::new(Mutex::new(database.start_search_and_vector_bootstrap()));
        let table_summary_worker = TableSummaryWorker::start(
            runtime.clone(),
            database.clone(),
            persistence.clone(),
            lease_lost_shutdown,
        );
        let schema_worker = Arc::new(Mutex::new(runtime.spawn(
            "schema_worker",
            SchemaWorker::start(runtime.clone(), database.clone()),
        )));

        let system_table_cleanup_worker = SystemTableCleanupWorker::new(
            runtime.clone(),
            database.clone(),
            application_storage.exports_storage.clone(),
        );
        let system_table_cleanup_worker = Arc::new(Mutex::new(
            runtime.spawn("system_table_cleanup_worker", system_table_cleanup_worker),
        ));

        // If local_log_sink is passed in, this is a local instance, so we enable log
        // streaming by default. Otherwise, it's hard to grant the
        // entitlement in testing and in load generator. If not local, we
        // read the entitlement from the database.
        let mut tx = database.begin(Identity::system()).await?;
        let log_streaming_allowed = if let Some(path) = local_log_sink {
            add_local_log_sink_on_startup(database.clone(), path).await?;
            true
        } else {
            let mut bi = BackendInfoModel::new(&mut tx);
            bi.is_log_streaming_allowed().await?
        };

        let log_manager_client = LogManager::start(
            runtime.clone(),
            database.clone(),
            fetch_client.clone(),
            instance_name.clone(),
            log_streaming_allowed,
        )
        .await;

        let function_log = FunctionExecutionLog::new(
            runtime.clone(),
            database.usage_counter(),
            Arc::new(log_manager_client.clone()),
        );
        let runner = Arc::new(ApplicationFunctionRunner::new(
            runtime.clone(),
            database.clone(),
            key_broker.clone(),
            function_runner.clone(),
            node_actions,
            file_storage.transactional_file_storage.clone(),
            application_storage.modules_storage.clone(),
            module_loader,
            function_log.clone(),
            default_system_env_vars.clone(),
            cache,
        ));
        function_runner.set_action_callbacks(runner.clone());

        let scheduled_job_runner = ScheduledJobRunner::start(
            runtime.clone(),
            instance_name.clone(),
            database.clone(),
            runner.clone(),
            function_log.clone(),
        );

        let cron_job_executor_fut = CronJobExecutor::run(
            runtime.clone(),
            instance_name.clone(),
            database.clone(),
            runner.clone(),
            function_log.clone(),
        );
        let cron_job_executor = Arc::new(Mutex::new(
            runtime.spawn("cron_job_executor", cron_job_executor_fut),
        ));

        let export_worker = ExportWorker::new(
            runtime.clone(),
            database.clone(),
            application_storage.exports_storage.clone(),
            application_storage.files_storage.clone(),
            export_provider,
            database.usage_counter(),
            instance_name.clone(),
        );
        let export_worker = Arc::new(Mutex::new(Some(
            runtime.spawn("export_worker", export_worker),
        )));

        let snapshot_import_worker = SnapshotImportWorker::start(
            runtime.clone(),
            database.clone(),
            application_storage.snapshot_imports_storage.clone(),
            file_storage.clone(),
            database.usage_counter(),
        );
        let snapshot_import_worker = Arc::new(Mutex::new(Some(
            runtime.spawn("snapshot_import_worker", snapshot_import_worker),
        )));

        let migration_worker = MigrationWorker::new(
            runtime.clone(),
            persistence.clone(),
            database.clone(),
            application_storage.modules_storage.clone(),
        );
        let migration_worker = Arc::new(Mutex::new(Some(
            runtime.spawn("migration_worker", migration_worker.go()),
        )));

        Ok(Self {
            runtime,
            database,
            runner,
            function_log,
            file_storage,
            application_storage,
            usage_tracking,
            key_broker,
            scheduled_job_runner,
            cron_job_executor,
            instance_name,
            index_worker,
            fast_forward_worker,
            search_worker,
            search_and_vector_bootstrap_worker,
            table_summary_worker,
            schema_worker,
            export_worker,
            snapshot_import_worker,
            system_table_cleanup_worker,
            migration_worker,
            log_visibility,
            module_cache,
            system_env_var_names: default_system_env_vars.into_keys().collect(),
            app_auth,
            log_manager_client,
        })
    }

    pub fn runtime(&self) -> RT {
        self.runtime.clone()
    }

    pub fn modules_storage(&self) -> &Arc<dyn Storage> {
        &self.application_storage.modules_storage
    }

    pub fn modules_cache(&self) -> &ModuleCache<RT> {
        &self.module_cache
    }

    pub fn key_broker(&self) -> &KeyBroker {
        &self.key_broker
    }

    pub fn runner(&self) -> Arc<ApplicationFunctionRunner<RT>> {
        self.runner.clone()
    }

    pub fn function_log(&self) -> FunctionExecutionLog<RT> {
        self.function_log.clone()
    }

    pub fn log_manager_client(&self) -> &LogManagerClient {
        &self.log_manager_client
    }

    pub fn now_ts_for_reads(&self) -> RepeatableTimestamp {
        self.database.now_ts_for_reads()
    }

    pub fn instance_name(&self) -> String {
        self.instance_name.clone()
    }

    #[fastrace::trace]
    pub async fn begin(&self, identity: Identity) -> anyhow::Result<Transaction<RT>> {
        self.database.begin(identity).await
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn commit_test(&self, transaction: Transaction<RT>) -> anyhow::Result<Timestamp> {
        self.commit(transaction, "test").await
    }

    #[fastrace::trace]
    pub async fn commit(
        &self,
        transaction: Transaction<RT>,
        write_source: impl Into<WriteSource>,
    ) -> anyhow::Result<Timestamp> {
        self.database
            .commit_with_write_source(transaction, write_source)
            .await
    }

    #[fastrace::trace]
    pub async fn subscribe(&self, token: Token) -> anyhow::Result<Subscription> {
        self.database.subscribe(token).await
    }

    pub fn usage_counter(&self) -> UsageCounter {
        self.database.usage_counter()
    }

    pub fn snapshot(&self, ts: RepeatableTimestamp) -> anyhow::Result<Snapshot> {
        self.database.snapshot(ts)
    }

    pub fn latest_snapshot(&self) -> anyhow::Result<Snapshot> {
        self.database.latest_snapshot()
    }

    pub fn app_auth(&self) -> &Arc<ApplicationAuth> {
        &self.app_auth
    }

    pub async fn search_with_compiled_query(
        &self,
        index_id: IndexId,
        printable_index_name: IndexName,
        query: pb::searchlight::TextQuery,
        pending_updates: Vec<DocumentUpdate>,
        ts: RepeatableTimestamp,
    ) -> anyhow::Result<RevisionWithKeys> {
        self.database
            .search_with_compiled_query(index_id, printable_index_name, query, pending_updates, ts)
            .await
    }

    pub async fn vector_search(
        &self,
        identity: Identity,
        query: VectorSearch,
    ) -> anyhow::Result<(Vec<PublicVectorSearchQueryResult>, FunctionUsageStats)> {
        self.database.vector_search(identity, query).await
    }

    pub async fn get_source_code(
        &self,
        identity: Identity,
        path: ModulePath,
        component: ComponentId,
    ) -> anyhow::Result<Option<String>> {
        let mut tx = self.begin(identity).await?;
        let path = CanonicalizedComponentModulePath {
            component,
            module_path: path.canonicalize(),
        };
        let Some(metadata) = ModuleModel::new(&mut tx).get_metadata(path.clone()).await? else {
            return Ok(None);
        };
        let Some(analyze_result) = &metadata.analyze_result else {
            return Ok(None);
        };
        let Some(source_index) = analyze_result.source_index else {
            return Ok(None);
        };
        let Some(full_source) = self.module_cache.get_module(&mut tx, path).await? else {
            return Ok(None);
        };
        let Some(source_map_str) = &full_source.source_map else {
            return Ok(None);
        };
        let Some(source_map) = source_map_from_slice(source_map_str.as_bytes()) else {
            return Ok(None);
        };
        let Some(source_map_content) = source_map.get_source_contents(source_index) else {
            return Ok(None);
        };
        Ok(Some(source_map_content.to_owned()))
    }

    pub async fn storage_generate_upload_url(
        &self,
        identity: Identity,
        component: ComponentId,
    ) -> anyhow::Result<String> {
        let issued_ts = self.runtime().unix_timestamp();
        let mut tx = self.begin(identity).await?;
        let url = self
            .file_storage
            .transactional_file_storage
            .generate_upload_url(&mut tx, self.key_broker(), issued_ts, component)
            .await?;

        Ok(url)
    }

    pub async fn read_only_udf(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        caller: FunctionCaller,
    ) -> anyhow::Result<RedactedQueryReturn> {
        let ts = *self.now_ts_for_reads();
        self.read_only_udf_at_ts(request_id, path, args, identity, ts, None, caller)
            .await
    }

    #[fastrace::trace]
    pub async fn read_only_udf_at_ts(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        ts: Timestamp,
        journal: Option<Option<String>>,
        caller: FunctionCaller,
    ) -> anyhow::Result<RedactedQueryReturn> {
        let persistence_version = self.database.persistence_version();
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                caller.allowed_visibility(),
            )
            .await?;

        let query_return: anyhow::Result<_> = try {
            let journal = journal
                .map(|serialized_journal| {
                    self.key_broker
                        .decrypt_query_journal(serialized_journal, persistence_version)
                })
                .transpose()?;
            self.runner
                .run_query_at_ts(
                    request_id.clone(),
                    path,
                    args,
                    identity,
                    ts,
                    journal,
                    caller,
                )
                .await?
        };

        let redacted_query_return = match query_return {
            Ok(query_return) => RedactedQueryReturn {
                result: match query_return.result {
                    Ok(r) => Ok(r),
                    Err(e) => Err(RedactedJsError::from_js_error(e, block_logging, request_id)),
                },
                log_lines: RedactedLogLines::from_log_lines(query_return.log_lines, block_logging),
                token: query_return.token,
                journal: self
                    .key_broker
                    .encrypt_query_journal(&query_return.journal, persistence_version),
            },
            Err(e) if e.is_deterministic_user_error() => RedactedQueryReturn {
                result: Err(RedactedJsError::from_js_error(
                    JsError::from_error(e),
                    block_logging,
                    request_id,
                )),
                log_lines: RedactedLogLines::empty(),
                // Create a token for an empty read set because we haven't
                // done any reads yet.
                token: Token::empty(ts),
                journal: self
                    .key_broker
                    .encrypt_query_journal(&QueryJournal::new(), persistence_version),
            },
            Err(e) => anyhow::bail!(e),
        };
        Ok(redacted_query_return)
    }

    #[fastrace::trace]
    pub async fn mutation_udf(
        &self,
        request_id: RequestId,
        path: PublicFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
        caller: FunctionCaller,
        mutation_queue_length: Option<usize>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>> {
        identity.ensure_can_run_function(UdfType::Mutation)?;
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                caller.allowed_visibility(),
            )
            .await?;
        let result = match self
            .runner
            .retry_mutation(
                request_id.clone(),
                path,
                args,
                identity,
                mutation_identifier,
                caller,
                mutation_queue_length,
            )
            .await
        {
            Ok(Ok(mutation_return)) => Ok(RedactedMutationReturn {
                value: mutation_return.value,
                log_lines: RedactedLogLines::from_log_lines(
                    mutation_return.log_lines,
                    block_logging,
                ),
                ts: mutation_return.ts,
            }),
            Ok(Err(mutation_error)) => Err(RedactedMutationError {
                error: RedactedJsError::from_js_error(
                    mutation_error.error,
                    block_logging,
                    request_id,
                ),
                log_lines: RedactedLogLines::from_log_lines(
                    mutation_error.log_lines,
                    block_logging,
                ),
            }),
            Err(e) if e.is_deterministic_user_error() => Err(RedactedMutationError {
                error: RedactedJsError::from_js_error(
                    JsError::from_error(e),
                    block_logging,
                    request_id,
                ),
                log_lines: RedactedLogLines::empty(),
            }),
            Err(e) => anyhow::bail!(e),
        };
        Ok(result)
    }

    #[fastrace::trace]
    pub async fn action_udf(
        &self,
        request_id: RequestId,
        name: PublicFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
        identity.ensure_can_run_function(UdfType::Action)?;

        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                caller.allowed_visibility(),
            )
            .await?;

        let should_spawn = caller.run_until_completion_if_cancelled();
        let runner: Arc<ApplicationFunctionRunner<RT>> = self.runner.clone();
        let request_id_ = request_id.clone();
        let span = SpanContext::current_local_parent()
            .map(|ctx| Span::root(format!("{}::actions_future", func_path!()), ctx))
            .unwrap_or(Span::noop());
        let run_action = async move {
            runner
                .run_action(request_id_, name, args, identity, caller)
                .in_span(span)
                .await
        };
        let result = if should_spawn {
            // Spawn running the action in a separate future. This way, even if we
            // get cancelled, it will continue to run to completion.
            let (tx, rx) = oneshot::channel();
            // TODO: cancel this handle with the application
            self.runtime.spawn_background("run_action", async move {
                let result = run_action.await;
                // Don't log errors if the caller has gone away.
                _ = tx.send(result);
            });
            rx.await
                .context("run_action one shot sender dropped prematurely?")?
        } else {
            // Await the action future. This means if we get cancelled the action
            // future will get dropped.
            run_action.await
        };
        let result = match result {
            Ok(Ok(action_return)) => Ok(RedactedActionReturn {
                value: action_return.value,
                log_lines: RedactedLogLines::from_log_lines(action_return.log_lines, block_logging),
            }),
            Ok(Err(action_error)) => Err(RedactedActionError {
                error: RedactedJsError::from_js_error(
                    action_error.error,
                    block_logging,
                    request_id,
                ),
                log_lines: RedactedLogLines::from_log_lines(action_error.log_lines, block_logging),
            }),
            Err(e) => anyhow::bail!(e),
        };
        Ok(result)
    }

    #[fastrace::trace]
    pub async fn http_action_udf(
        &self,
        request_id: RequestId,
        http_request: HttpActionRequest,
        identity: Identity,
        caller: FunctionCaller,
        mut response_streamer: HttpActionResponseStreamer,
    ) -> anyhow::Result<()> {
        identity.ensure_can_run_function(UdfType::HttpAction)?;
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                caller.allowed_visibility(),
            )
            .await?;
        let rt_ = self.runtime.clone();

        // Spawn running the action in a separate future. This way, even if we
        // get cancelled, it will continue to run to completion.
        let (tx, rx) = oneshot::channel();
        let runner = self.runner.clone();
        let span = SpanContext::current_local_parent()
            .map(|ctx| Span::root(format!("{}::http_actions_future", func_path!()), ctx))
            .unwrap_or(Span::noop());
        let response_streamer_ = response_streamer.clone();
        // TODO: cancel this handle with the application
        self.runtime
            .spawn_background("run_http_action", async move {
                let result = runner
                    .run_http_action(
                        request_id,
                        http_request,
                        response_streamer_,
                        identity,
                        caller,
                    )
                    .in_span(span)
                    .await;
                if let Err(Err(mut e)) = tx.send(result) {
                    // If the caller has gone away, and the result is a system error,
                    // log to sentry.
                    report_error(&mut e).await;
                }
                rt_.pause_client().wait("end_run_http_action").await;
            });
        let result = rx
            .await
            .context("run_http_action one shot sender dropped prematurely?")?;
        match result {
            Ok(HttpActionResult::Error(error)) => {
                let response_parts =
                    RedactedJsError::from_js_error(error, block_logging, RequestId::new())
                        .to_http_response_parts();
                for part in response_parts {
                    response_streamer.send_part(part)??;
                }
            },
            Ok(HttpActionResult::Streamed) => (),
            Err(e) => anyhow::bail!(e),
        };
        Ok(())
    }

    /// Run a function of an arbitrary type from its name
    pub async fn any_udf(
        &self,
        request_id: RequestId,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        identity: Identity,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                caller.allowed_visibility(),
            )
            .await?;

        // We use a separate transaction to get the type of the UDF before calling the
        // appropriate type-specific code. While this could lead to incorrect
        // "function not found" messages errors if the user changes the type of the
        // UDF between the two transactions without deleting it, this situation is
        // rare enough to disregard it.
        let mut tx_type = self.begin(identity.clone()).await?;

        let canonicalized_path = path.clone();
        let Some(analyzed_function) = ModuleModel::new(&mut tx_type)
            .get_analyzed_function(&canonicalized_path)
            .await?
            .ok()
            .filter(|af| {
                (identity.is_admin() || af.visibility == Some(Visibility::Public))
                    && af.udf_type != UdfType::HttpAction
            })
        else {
            let missing_or_internal = format!(
                "Could not find function for '{}'{}. Did you forget to run `npx convex dev` or \
                 `npx convex deploy`?",
                String::from(canonicalized_path.udf_path.strip()),
                canonicalized_path.component.in_component_str(),
            );
            return Ok(Err(FunctionError {
                error: RedactedJsError::from_js_error(
                    JsError::from_message(missing_or_internal),
                    block_logging,
                    request_id,
                ),
                log_lines: RedactedLogLines::empty(),
            }));
        };

        identity.ensure_can_run_function(analyzed_function.udf_type)?;

        match analyzed_function.udf_type {
            UdfType::Query => self
                .read_only_udf(
                    request_id,
                    PublicFunctionPath::Component(path),
                    args,
                    identity,
                    caller,
                )
                .await
                .map(
                    |RedactedQueryReturn {
                         result, log_lines, ..
                     }| {
                        match result {
                            Ok(value) => Ok(FunctionReturn { value, log_lines }),
                            Err(error) => Err(FunctionError { error, log_lines }),
                        }
                    },
                ),
            UdfType::Mutation => self
                .mutation_udf(
                    request_id,
                    PublicFunctionPath::Component(path),
                    args,
                    identity,
                    None,
                    caller,
                    None,
                )
                .await
                .map(|res| {
                    res.map(
                        |RedactedMutationReturn {
                             value, log_lines, ..
                         }| FunctionReturn { value, log_lines },
                    )
                    .map_err(
                        |RedactedMutationError {
                             error, log_lines, ..
                         }| FunctionError { error, log_lines },
                    )
                }),
            UdfType::Action => self
                .action_udf(
                    request_id,
                    PublicFunctionPath::Component(path),
                    args,
                    identity,
                    caller,
                )
                .await
                .map(|res| {
                    res.map(
                        |RedactedActionReturn {
                             value, log_lines, ..
                         }| FunctionReturn { value, log_lines },
                    )
                    .map_err(
                        |RedactedActionError {
                             error, log_lines, ..
                         }| FunctionError { error, log_lines },
                    )
                }),
            UdfType::HttpAction => {
                anyhow::bail!(
                    "HTTP actions not supported in the /functions endpoint. A \"not found\" \
                     message should be returned instead."
                )
            },
        }
    }

    pub async fn request_export(
        &self,
        identity: Identity,
        format: ExportFormat,
        component: ComponentId,
        requestor: ExportRequestor,
        expiration_ts_ns: Option<u64>,
    ) -> anyhow::Result<DeveloperDocumentId> {
        anyhow::ensure!(
            identity.is_admin() || identity.is_system(),
            unauthorized_error("request_export")
        );
        if let Some(expiration_ts_ns) = expiration_ts_ns {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("Time went backward")?;
            anyhow::ensure!(
                expiration_ts_ns >= now.as_nanos() as u64,
                ErrorMetadata::bad_request(
                    "SnapshotExpirationInPast",
                    "Snapshot expiration in past."
                )
            );
            let how_far = Duration::from_nanos(expiration_ts_ns) - now;
            anyhow::ensure!(
                how_far <= Duration::from_days(60),
                ErrorMetadata::bad_request(
                    "SnapshotExpirationTooLarge",
                    format!(
                        "Snapshot expiration is {} days in the future. Must be <= 60",
                        how_far.as_secs() / (60 * 60 * 24)
                    ),
                )
            );
        }

        let mut tx = self.begin(identity).await?;
        let mut exports_model = ExportsModel::new(&mut tx);
        let export_requested = exports_model.latest_requested().await?;
        let export_in_progress = exports_model.latest_in_progress().await?;

        let snapshot_id = match (export_requested, export_in_progress) {
            (None, None) => {
                exports_model
                    .insert_requested(format, component, requestor, expiration_ts_ns)
                    .await
            },
            _ => Err(
                anyhow::anyhow!("Can only have one export requested or in progress at once")
                    .context(ErrorMetadata::bad_request(
                        "ExportInProgress",
                        "There is already an export requested or in progress.",
                    )),
            ),
        }?;
        self.commit(tx, "request_export").await?;
        Ok(snapshot_id.into())
    }

    pub async fn get_zip_export(
        &self,
        identity: Identity,
        id: Either<DeveloperDocumentId, Timestamp>,
    ) -> anyhow::Result<(StorageGetStream, String)> {
        let (object_key, snapshot_ts) = {
            let mut tx = self.begin(identity).await?;
            let export = match id {
                Either::Left(id) => ExportsModel::new(&mut tx).get(id).await?,
                Either::Right(ts) => {
                    ExportsModel::new(&mut tx)
                        .completed_export_at_ts(ts)
                        .await?
                },
            }
            .context(ErrorMetadata::not_found(
                "ExportNotFound",
                format!("The requested export {id} was not found"),
            ))?;
            match export.into_value() {
                Export::Completed {
                    zip_object_key,
                    start_ts,
                    ..
                } => (zip_object_key, start_ts),
                Export::Failed { .. }
                | Export::Canceled { .. }
                | Export::InProgress { .. }
                | Export::Requested { .. } => {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "ExportNotComplete",
                        format!("The requested export {id} has not completed"),
                    ))
                },
            }
        };
        let storage_get_stream = self
            .application_storage
            .exports_storage
            .get(&object_key)
            .await?
            .context(ErrorMetadata::not_found(
                "ExportNotFound",
                format!("The requested export {snapshot_ts}/{object_key:?} was not found"),
            ))?;

        let filename = format!(
            // This should match the format in SnapshotExport.tsx.
            "snapshot_{}_{snapshot_ts}.zip",
            self.instance_name
        );
        Ok((storage_get_stream, filename))
    }

    /// Returns the cloud export key - fully qualified to the instance.
    pub fn cloud_export_key(&self, zip_export_key: ObjectKey) -> FullyQualifiedObjectKey {
        self.application_storage
            .exports_storage
            .fully_qualified_key(&zip_export_key)
    }

    pub async fn update_environment_variables(
        &self,
        tx: &mut Transaction<RT>,
        changes: Vec<EnvVarChange>,
    ) -> anyhow::Result<Vec<DeploymentAuditLogEvent>> {
        let mut audit_events = vec![];

        let mut model = EnvironmentVariablesModel::new(tx);
        for change in changes {
            match change {
                EnvVarChange::Set(env_var) => {
                    let name = env_var.name();
                    if let Some(_existing) = model.delete(name).await? {
                        audit_events.push(DeploymentAuditLogEvent::UpdateEnvironmentVariable {
                            name: name.clone(),
                        });
                    } else {
                        audit_events.push(DeploymentAuditLogEvent::CreateEnvironmentVariable {
                            name: name.clone(),
                        });
                    }
                    model.create(env_var, &self.system_env_var_names).await?;
                },
                EnvVarChange::Unset(name) => {
                    if let Some(_existing) = model.delete(&name).await? {
                        audit_events
                            .push(DeploymentAuditLogEvent::DeleteEnvironmentVariable { name });
                    };
                },
            }
        }

        let all_env_vars = model.get_all().await?;

        anyhow::ensure!(
            all_env_vars.len() as u64 <= (ENV_VAR_LIMIT as u64),
            env_var_limit_met(),
        );

        Self::reevaluate_existing_auth_config(self.runner().clone(), tx).await?;

        Ok(audit_events)
    }

    pub async fn create_environment_variables(
        &self,
        tx: &mut Transaction<RT>,
        environment_variables: Vec<EnvironmentVariable>,
    ) -> anyhow::Result<Vec<DeploymentAuditLogEvent>> {
        let all_env_vars = EnvironmentVariablesModel::new(tx).get_all().await?;
        anyhow::ensure!(
            environment_variables.len() as u64 + all_env_vars.len() as u64
                <= (ENV_VAR_LIMIT as u64),
            env_var_limit_met(),
        );
        for environment_variable in environment_variables.clone() {
            self.create_one_environment_variable(tx, environment_variable)
                .await?;
        }
        let audit_events = environment_variables
            .into_iter()
            .map(
                |env_variable| DeploymentAuditLogEvent::CreateEnvironmentVariable {
                    name: env_variable.name().to_owned(),
                },
            )
            .collect();
        Ok(audit_events)
    }

    async fn create_one_environment_variable(
        &self,
        tx: &mut Transaction<RT>,
        environment_variable: EnvironmentVariable,
    ) -> anyhow::Result<()> {
        let mut env_var_model = EnvironmentVariablesModel::new(tx);
        if env_var_model
            .get(environment_variable.name())
            .await?
            .is_some()
        {
            anyhow::bail!(env_var_name_not_unique(None));
        }
        env_var_model
            .create(environment_variable, &self.system_env_var_names)
            .await?;
        Ok(())
    }

    pub async fn set_initial_environment_variables(
        &self,
        environment_variables: Vec<EnvironmentVariable>,
        identity: Identity,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity).await?;

        if !EnvironmentVariablesModel::new(&mut tx)
            .get_all()
            .await?
            .is_empty()
        {
            // This deployment already has environment variables, so don't try to initialize
            // them again
            return Ok(());
        }

        match self
            .create_environment_variables(&mut tx, environment_variables)
            .await
        {
            Ok(audit_events) => {
                self.commit_with_audit_log_events(tx, audit_events, "set_initial_env_vars")
                    .await?;
                Ok(())
            },
            Err(e) => {
                if e.is_bad_request() {
                    // This should not happen and likely means we have a bug in what we allow as
                    // project default env variables. Report the error but do not fail the request.
                    report_error(&mut anyhow::anyhow!(
                        "Error setting initial environment variables: {e}"
                    ))
                    .await;
                    Ok(())
                } else {
                    Err(e)
                }
            },
        }
    }

    pub async fn delete_environment_variable(
        &self,
        tx: &mut Transaction<RT>,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<DeploymentAuditLogEvent> {
        let mut model = EnvironmentVariablesModel::new(tx);
        let Some(env_var) = model.get_by_id_legacy(id).await? else {
            anyhow::bail!(ErrorMetadata::bad_request(
                "EnvironmentVariableNotFound",
                "Environment variable not found"
            ));
        };
        let name = env_var.name().to_owned();
        model.delete(&name).await?;
        Ok(DeploymentAuditLogEvent::DeleteEnvironmentVariable { name })
    }

    pub async fn set_canonical_url(
        &self,
        tx: &mut Transaction<RT>,
        canonical_url: CanonicalUrl,
    ) -> anyhow::Result<()> {
        CanonicalUrlsModel::new(tx)
            .set_canonical_url(canonical_url.request_destination, canonical_url.url)
            .await?;
        Self::reevaluate_existing_auth_config(self.runner().clone(), tx).await
    }

    pub async fn unset_canonical_url(
        &self,
        tx: &mut Transaction<RT>,
        request_destination: RequestDestination,
    ) -> anyhow::Result<()> {
        CanonicalUrlsModel::new(tx)
            .unset_canonical_url(request_destination)
            .await?;
        Self::reevaluate_existing_auth_config(self.runner().clone(), tx).await
    }

    pub async fn analyze(
        &self,
        udf_config: UdfConfig,
        new_modules: Vec<ModuleConfig>,
        source_package: SourcePackage,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        self.runner
            .analyze(
                udf_config,
                new_modules,
                source_package,
                user_environment_variables,
                system_env_var_overrides,
            )
            .await
    }

    fn _validate_user_defined_index_fields(
        &self,
        fields: IndexedFields,
    ) -> anyhow::Result<IndexedFields> {
        // Creation time is a special case of a system field. We check that
        // first to provide a more useful error message.
        anyhow::ensure!(
            !fields.contains(&CREATION_TIME_FIELD_PATH),
            index_validation_error::fields_contain_creation_time(),
        );

        // We do not allow system fields in user defined indexes.
        anyhow::ensure!(
            fields
                .iter()
                .flat_map(|fp| fp.fields())
                .all(|f| !f.is_system()),
            index_validation_error::field_name_reserved()
        );

        // Append _creationTime to the end of each index. This is so indexes have
        // default order that is more intuitive to the user.
        let mut fields: Vec<FieldPath> = fields.into();
        fields.push(CREATION_TIME_FIELD_PATH.clone());
        fields.try_into()
    }

    pub async fn evaluate_schema(&self, schema: ModuleConfig) -> anyhow::Result<DatabaseSchema> {
        self._evaluate_schema(schema).await.map_err(|e| {
            e.wrap_error_message(|msg| format!("Hit an error while evaluating your schema:\n{msg}"))
        })
    }

    async fn _evaluate_schema(&self, schema: ModuleConfig) -> anyhow::Result<DatabaseSchema> {
        let rng_seed = self.runtime().rng().random();
        let unix_timestamp = self.runtime().unix_timestamp();
        let mut schema = self
            .runner()
            .evaluate_schema(schema.source, schema.source_map, rng_seed, unix_timestamp)
            .await?;

        for table_schema in schema.tables.values_mut() {
            for index_schema in table_schema
                .indexes
                .values_mut()
                .chain(table_schema.staged_db_indexes.values_mut())
            {
                index_schema.fields =
                    self._validate_user_defined_index_fields(index_schema.fields.clone())?;
            }
        }

        schema.check_index_references()?;

        Ok(schema)
    }

    #[fastrace::trace]
    pub async fn get_evaluated_auth_config(
        runner: Arc<ApplicationFunctionRunner<RT>>,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
        auth_config_module: Option<ModuleConfig>,
        config: &ConfigFile,
    ) -> anyhow::Result<Vec<AuthInfo>> {
        if let Some(auth_config_module) = auth_config_module {
            anyhow::ensure!(
                config.auth_info.is_none(),
                ErrorMetadata::bad_request(
                    "InvalidAuthConfig",
                    "Cannot set auth config in both auth config file and `convex.json`, remove it \
                     from `convex.json`"
                )
            );
            anyhow::ensure!(
                auth_config_module.environment != ModuleEnvironment::Node,
                "auth config file can't be analyzed in Node.js!"
            );
            let auth_config = Self::evaluate_auth_config(
                runner,
                user_environment_variables,
                system_env_var_overrides,
                auth_config_module,
                "The pushed auth config is invalid",
            )
            .await?;
            Ok(auth_config.providers)
        } else {
            config
                .auth_info
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(AuthInfo::try_from)
                .collect::<Result<Vec<_>, _>>()
        }
    }

    // This is only relevant to auth config set via `auth.config.js`.
    // Because legacy setups didn't use `auth.config.js` we do not
    // reset the auth config if `auth.config.js` is not present.
    pub async fn reevaluate_existing_auth_config(
        runner: Arc<ApplicationFunctionRunner<RT>>,
        tx: &mut Transaction<RT>,
    ) -> anyhow::Result<()> {
        let path = CanonicalizedComponentModulePath {
            component: ComponentId::Root,
            module_path: AUTH_CONFIG_FILE_NAME.parse()?,
        };
        let auth_config_metadata = ModuleModel::new(tx).get_metadata(path.clone()).await?;
        if let Some(auth_config_metadata) = auth_config_metadata {
            let environment = auth_config_metadata.environment;
            let auth_config_source = runner
                .module_cache
                .get_module(tx, path)
                .await?
                .context("Module has metadata but no source")?;
            let auth_config_module = ModuleConfig {
                path: AUTH_CONFIG_FILE_NAME.parse()?,
                source: auth_config_source.source.clone(),
                source_map: auth_config_source.source_map.clone(),
                environment,
            };
            let user_environment_variables = EnvironmentVariablesModel::new(tx).get_all().await?;
            let auth_config = Self::evaluate_auth_config(
                runner,
                user_environment_variables,
                system_env_var_overrides(tx).await?,
                auth_config_module,
                "This change would make the auth config invalid",
            )
            .await?;
            AuthInfoModel::new(tx).put(auth_config.providers).await?;
        }
        Ok(())
    }

    async fn evaluate_auth_config(
        runner: Arc<ApplicationFunctionRunner<RT>>,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
        auth_config_module: ModuleConfig,
        explanation: &str,
    ) -> anyhow::Result<AuthConfig> {
        runner
            .evaluate_auth_config(
                auth_config_module.source,
                auth_config_module.source_map,
                user_environment_variables,
                system_env_var_overrides,
                explanation,
            )
            .await
    }

    #[fastrace::trace]
    pub async fn apply_config_with_retries(
        &self,
        identity: Identity,
        apply_config_args: ApplyConfigArgs,
    ) -> anyhow::Result<(ConfigMetadataAndSchema, OccRetryStats)> {
        let runner = self.runner.clone();
        self.execute_with_audit_log_events_and_occ_retries_reporting_stats(
            identity,
            "apply_config",
            |tx| Self::_apply_config(runner.clone(), tx, apply_config_args.clone()).into(),
        )
        .await
    }

    #[fastrace::trace]
    async fn _apply_config(
        runner: Arc<ApplicationFunctionRunner<RT>>,
        tx: &mut Transaction<RT>,
        ApplyConfigArgs {
            auth_module,
            config_file,
            schema_id,
            modules,
            udf_config,
            source_package,
            analyze_results,
        }: ApplyConfigArgs,
    ) -> anyhow::Result<(ConfigMetadataAndSchema, Vec<DeploymentAuditLogEvent>)> {
        let schema_id = schema_id
            .map(|schema_id| {
                parse_schema_id(
                    &schema_id,
                    tx.table_mapping(),
                    TableNamespace::root_component(),
                )
                .context(invalid_schema_id(&schema_id))
            })
            .transpose()?;

        let user_environment_variables = EnvironmentVariablesModel::new(tx).get_all().await?;
        let system_env_var_overrides = system_env_var_overrides(tx).await?;
        let auth_providers = Self::get_evaluated_auth_config(
            runner,
            user_environment_variables,
            system_env_var_overrides,
            auth_module,
            &config_file,
        )
        .await?;

        let config_metadata = ConfigMetadata::from_file(config_file, auth_providers);

        let (config_diff, schema) = ConfigModel::new(tx, ComponentId::Root)
            .apply(
                config_metadata.clone(),
                modules,
                udf_config,
                Some(source_package),
                analyze_results,
                schema_id,
            )
            .await?;

        ComponentConfigModel::new(tx).disable_components().await?;

        Ok((
            ConfigMetadataAndSchema {
                config_metadata,
                schema,
            },
            vec![DeploymentAuditLogEvent::PushConfig { config_diff }],
        ))
    }

    #[fastrace::trace]
    pub async fn analyze_modules_with_auth_config(
        &self,
        udf_config: UdfConfig,
        modules: Vec<ModuleConfig>,
        source_package: SourcePackage,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<(
        Option<ModuleConfig>,
        BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    )> {
        // Don't analyze the auth config module
        let (auth_modules, analyzed_modules): (Vec<_>, Vec<_>) =
            modules.into_iter().partition(|module| {
                module.path.clone().canonicalize() == AUTH_CONFIG_FILE_NAME.parse().unwrap()
            });
        let auth_module = auth_modules.first();

        let mut analyze_result = self
            .analyze_modules(
                udf_config,
                analyzed_modules,
                source_package,
                user_environment_variables,
                system_env_var_overrides,
            )
            .await?;

        // Add an empty analyzed result for the auth config module
        if let Some(auth_module) = auth_module {
            analyze_result.insert(
                auth_module.path.clone().canonicalize(),
                AnalyzedModule::default(),
            );
        }
        Ok((auth_module.cloned(), analyze_result))
    }

    async fn upload_packages(
        &self,
        config: &ProjectConfig,
    ) -> anyhow::Result<(
        Option<ExternalDepsPackageId>,
        BTreeMap<ComponentDefinitionPath, SourcePackage>,
    )> {
        let upload_limit = Arc::new(Semaphore::new(*APPLICATION_MAX_CONCURRENT_UPLOADS));

        let root_future = async {
            let permit = upload_limit.acquire().await?;
            let external_deps_id_and_pkg = if !config.node_dependencies.is_empty() {
                let deps = self
                    .build_external_node_deps(config.node_dependencies.clone())
                    .await?;
                Some(deps)
            } else {
                None
            };
            let app_modules = config.app_definition.modules().cloned().collect();
            let app_pkg = self
                .upload_package(
                    &app_modules,
                    external_deps_id_and_pkg.clone(),
                    config.node_version,
                )
                .await?;
            drop(permit);
            Ok((external_deps_id_and_pkg, app_pkg))
        };

        let mut component_pkg_futures = JoinSet::new();
        for component_def in &config.component_definitions {
            let app = self.clone();
            let definition_path = component_def.definition_path.clone();
            let component_modules = component_def.modules().cloned().collect();
            let upload_limit = upload_limit.clone();
            let component_pkg_future = async move {
                let permit = upload_limit.acquire().await?;
                let component_pkg = app.upload_package(&component_modules, None, None).await?;
                drop(permit);
                anyhow::Ok((definition_path, component_pkg))
            };
            component_pkg_futures.spawn("upload_package", component_pkg_future);
        }
        // `JoinSet::join_all` was added in tokio 1.40.0.
        let component_pkg_future = async {
            let mut result = Vec::with_capacity(config.component_definitions.len());
            while let Some(component_pkg) = component_pkg_futures.join_next().await {
                result.push(component_pkg??);
            }
            anyhow::Ok(result)
        };

        let ((external_deps, app_pkg), component_pkgs) =
            tokio::try_join!(root_future, component_pkg_future)?;

        let mut total_size = PackageSize::default();
        if let Some((_, ref pkg)) = external_deps {
            total_size += pkg.package_size;
        }
        total_size += app_pkg.package_size;
        for (_, pkg) in &component_pkgs {
            total_size += pkg.package_size;
        }
        total_size.verify_size()?;

        let mut component_definition_packages = BTreeMap::new();
        component_definition_packages.insert(ComponentDefinitionPath::root(), app_pkg);
        for (definition_path, component_pkg) in component_pkgs {
            anyhow::ensure!(component_definition_packages
                .insert(definition_path, component_pkg)
                .is_none());
        }

        let external_deps_id = external_deps.map(|(id, _)| id);
        Ok((external_deps_id, component_definition_packages))
    }

    // Helper method to call analyze and throw appropriate HttpError.
    pub async fn analyze_modules(
        &self,
        udf_config: UdfConfig,
        modules: Vec<ModuleConfig>,
        source_package: SourcePackage,
        user_environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_var_overrides: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>> {
        let num_dep_modules = modules.iter().filter(|m| m.path.is_deps()).count();
        anyhow::ensure!(
            modules.len() - num_dep_modules <= *MAX_USER_MODULES,
            ErrorMetadata::bad_request(
                "InvalidModules",
                format!(
                    r#"Too many function files ({} > maximum {}) in "convex/". See our docs (https://docs.convex.dev/using/writing-convex-functions#using-libraries) for more details."#,
                    modules.len() - num_dep_modules,
                    *MAX_USER_MODULES
                ),
            )
        );
        // We exclude dependency modules from the user limit since they don't depend on
        // the developer. We don't expect dependencies to be more than the user defined
        // modules though. If we ever have crazy amount of dependency modules,
        // throw a system errors so we can debug.
        anyhow::ensure!(
            modules.len() <= 2 * *MAX_USER_MODULES,
            "Too many dependencies modules! Dependencies: {}, Total modules: {}",
            num_dep_modules,
            modules.len()
        );

        // Run analyze the modules to make sure they are valid.
        match self
            .analyze(
                udf_config,
                modules,
                source_package,
                user_environment_variables,
                system_env_var_overrides,
            )
            .await?
        {
            Ok(m) => Ok(m),
            Err(js_error) => {
                let e = ErrorMetadata::bad_request(
                    "InvalidModules",
                    format!(
                        "Loading the pushed modules encountered the following
    error:\n{js_error}"
                    ),
                );
                Err(anyhow::anyhow!(js_error).context(e))
            },
        }
    }

    pub async fn start_upload_for_snapshot_import(
        &self,
        identity: Identity,
    ) -> anyhow::Result<ClientDrivenUploadToken> {
        if !identity.is_admin() {
            anyhow::bail!(ErrorMetadata::forbidden(
                "InvalidImport",
                "Only an admin of the deployment can import"
            ));
        }
        let upload = self
            .application_storage
            .snapshot_imports_storage
            .start_client_driven_upload()
            .await?;
        Ok(upload)
    }

    pub async fn upload_part_for_snapshot_import(
        &self,
        identity: Identity,
        upload_token: ClientDrivenUploadToken,
        part_number: u16,
        part: Bytes,
    ) -> anyhow::Result<ClientDrivenUploadPartToken> {
        if !identity.is_admin() {
            anyhow::bail!(ErrorMetadata::forbidden(
                "InvalidImport",
                "Only an admin of the deployment can import"
            ));
        }
        let part_token = self
            .application_storage
            .snapshot_imports_storage
            .upload_part(upload_token, part_number, part)
            .await?;
        Ok(part_token)
    }

    pub async fn import_finish_upload(
        &self,
        identity: Identity,
        format: ImportFormat,
        mode: ImportMode,
        component_path: ComponentPath,
        upload_token: ClientDrivenUploadToken,
        part_tokens: Vec<ClientDrivenUploadPartToken>,
    ) -> anyhow::Result<DeveloperDocumentId> {
        if !identity.is_admin() {
            anyhow::bail!(ErrorMetadata::forbidden(
                "InvalidImport",
                "Only an admin of the deployment can import"
            ));
        }
        let object_key = self
            .application_storage
            .snapshot_imports_storage
            .finish_client_driven_upload(upload_token, part_tokens)
            .await?;
        let fq_key = self
            .application_storage
            .snapshot_imports_storage
            .fully_qualified_key(&object_key);
        start_stored_import(
            self,
            identity,
            format,
            mode,
            component_path,
            fq_key,
            ImportRequestor::SnapshotImport,
        )
        .await
    }

    pub async fn upload_snapshot_import(
        &self,
        body_stream: BoxStream<'_, anyhow::Result<Bytes>>,
    ) -> anyhow::Result<FullyQualifiedObjectKey> {
        let mut upload: Box<BufferedUpload> = self
            .application_storage
            .snapshot_imports_storage
            .start_upload()
            .await?;
        // unclear why this reassignment is necessary
        let mut body_stream = body_stream;
        upload.try_write_parallel(&mut body_stream).await?;
        drop(body_stream);
        let object_key = upload.complete().await?;
        Ok(self
            .application_storage
            .snapshot_imports_storage
            .fully_qualified_key(&object_key))
    }

    #[fastrace::trace]
    pub async fn upload_package(
        &self,
        modules: &Vec<ModuleConfig>,
        external_deps_id_and_pkg: Option<(ExternalDepsPackageId, ExternalDepsPackage)>,
        node_version: Option<NodeVersion>,
    ) -> anyhow::Result<SourcePackage> {
        // If there are any node actions, turn on the lambdas.
        if modules
            .iter()
            .any(|m| m.environment == ModuleEnvironment::Node)
        {
            self.runner().enable_actions()?;
        }

        tracing::info!(
            "Uploading package with {} modules to Storage",
            modules.len()
        );

        // Canonicalize the modules
        let package: BTreeMap<_, _> = modules
            .iter()
            .map(|m| (m.path.clone().canonicalize(), m))
            .collect();
        anyhow::ensure!(
            modules.len() == package.len(),
            ErrorMetadata::bad_request(
                "CanonicalizationConflict",
                "Multiple modules canonicalize to the same name.",
            )
        );

        let (external_deps_package_id, external_deps_pkg) = match external_deps_id_and_pkg {
            Some((id, pkg)) => (Some(id), Some(pkg)),
            _ => (None, None),
        };
        let (storage_key, sha256, package_size) = upload_package(
            package,
            self.application_storage.modules_storage.clone(),
            external_deps_pkg.map(|pkg| pkg.storage_key),
        )
        .await?;

        tracing::info!("Upload of {storage_key:?} successful");
        tracing::info!("Source package size: {}", package_size);
        log_source_package_size_bytes_total(package_size);

        Ok(SourcePackage {
            storage_key,
            sha256,
            external_deps_package_id,
            package_size,
            node_version,
        })
    }

    // Clear all records for specified tables concurrently, potentially taking
    // multiple transactions for each. Returns the total number of documents
    // deleted.
    pub async fn clear_tables(
        &self,
        identity: &Identity,
        table_names: Vec<(ComponentPath, TableName)>,
    ) -> anyhow::Result<u64> {
        clear_tables(self, identity, table_names).await
    }

    pub async fn execute_standalone_module(
        &self,
        request_id: RequestId,
        module: ModuleConfig,
        args: Vec<JsonValue>,
        identity: Identity,
        caller: FunctionCaller,
        component: ComponentId,
    ) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                caller.allowed_visibility(),
            )
            .await?;

        // Write (and commit) the module source to S3.
        // This will become a dangling reference since the _modules entry won't
        // be committed to the database, but we have to deal with those anyway.
        let source_package = self
            .upload_package(&vec![module.clone()], None, None)
            .await?;

        let mut tx = self.begin(identity.clone()).await?;
        let (user_environment_variables, system_env_var_overrides) = if component.is_root() {
            let user_environment_variables =
                EnvironmentVariablesModel::new(&mut tx).get_all().await?;
            (
                user_environment_variables,
                system_env_var_overrides(&mut tx).await?,
            )
        } else {
            (BTreeMap::new(), BTreeMap::new())
        };

        let mut udf_config_model = UdfConfigModel::new(&mut tx, component.into());
        let udf_config = match udf_config_model.get().await? {
            Some(udf_config) => (**udf_config).clone(),
            None => {
                // If there hasn't been a push
                // yet, act like the most recent version.
                let udf_config = UdfConfig {
                    server_version: Version::new(1000, 0, 0),
                    import_phase_rng_seed: self.runtime.rng().random(),
                    import_phase_unix_timestamp: self.runtime.unix_timestamp(),
                };
                udf_config_model.set(udf_config.clone()).await?;
                udf_config
            },
        };

        // 1. analyze the module
        // We can analyze this module by itself, without combining it with the existing
        // modules since this module should be self-contained and not import
        // from other modules.

        let analyze_results = self
            .analyze(
                udf_config.clone(),
                vec![module.clone()],
                source_package.clone(),
                user_environment_variables,
                system_env_var_overrides,
            )
            .await?
            .map_err(|js_error| {
                let metadata = ErrorMetadata::bad_request(
                    "InvalidModules",
                    format!("Could not analyze the given module:\n{js_error}"),
                );
                anyhow::anyhow!(js_error).context(metadata)
            })?;

        let module_path = module.path.clone().canonicalize();
        let analyzed_module = analyze_results
            .get(&module_path)
            .ok_or_else(|| anyhow::anyhow!("Unexpectedly missing analyze result"))?
            .clone();

        // 2. get the function type
        let mut analyzed_function = None;
        for function in &analyzed_module.functions {
            if function.name.is_default_export() {
                analyzed_function = Some(function.clone());
            } else {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidTestQuery",
                    "Only `export default` is supported."
                ));
            }
        }
        let analyzed_function = analyzed_function.context(ErrorMetadata::bad_request(
            "InvalidTestQuery",
            "Default export is not a Convex function.",
        ))?;

        let source_package_id = SourcePackageModel::new(&mut tx, component.into())
            .put(source_package)
            .await?;

        // 3. Add the module
        let path = CanonicalizedComponentModulePath {
            component,
            module_path: module_path.clone(),
        };
        let module_id = ModuleModel::new(&mut tx)
            .get_metadata(path.clone())
            .await?
            .map(|m| m.id());
        ModuleModel::new(&mut tx)
            .put(
                module_id,
                path,
                module.source,
                source_package_id,
                module.source_map,
                Some(analyzed_module),
                ModuleEnvironment::Isolate,
            )
            .await?;

        // 4. run the function within the transaction
        let function_name = FunctionName::default_export();
        let component_path =
            BootstrapComponentsModel::new(&mut tx).must_component_path(component)?;
        let path = CanonicalizedComponentFunctionPath {
            component: component_path,
            udf_path: CanonicalizedUdfPath::new(module_path, function_name),
        };
        let arguments = parse_udf_args(&path.udf_path, args)?;
        let (result, log_lines) = match analyzed_function.udf_type {
            UdfType::Query => {
                self.runner
                    .run_query_without_caching(request_id.clone(), tx, path, arguments, caller)
                    .await
            },
            UdfType::Mutation => {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "UnsupportedTestQuery",
                    "Mutations are not supported in the REPL yet."
                ))
            },
            UdfType::Action => {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "UnsupportedTestQuery",
                    "Actions are not supported in the REPL yet."
                ))
            },
            UdfType::HttpAction => {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "UnsupportedTestQuery",
                    "HTTP actions are not supported in the REPL yet."
                ))
            },
        }?;
        let log_lines = RedactedLogLines::from_log_lines(log_lines, block_logging);
        Ok(match result {
            Ok(value) => Ok(FunctionReturn { value, log_lines }),
            Err(error) => Err(FunctionError {
                error: RedactedJsError::from_js_error(error, block_logging, request_id),
                log_lines,
            }),
        })
    }

    #[fastrace::trace]
    pub async fn build_external_node_deps(
        &self,
        deps: Vec<NodeDependency>,
    ) -> anyhow::Result<(ExternalDepsPackageId, ExternalDepsPackage)> {
        // Check cache to see if we've built this package recently
        let mut tx = self.begin(Identity::system()).await?;
        let mut model = ExternalPackagesModel::new(&mut tx);
        let cached_match = model.get_cached_package_match(deps.clone()).await?;
        if let Some((cached_id, cached_pkg)) = cached_match {
            tracing::info!("Cache hit for external deps package!");
            log_external_deps_package(true);
            return Ok((cached_id, cached_pkg));
        } else {
            log_external_deps_package(false);
            tracing::info!("Cache miss for external deps package, running build_deps...");
        }

        let result = self.runner().build_deps(deps).await?;
        let pkg = match result {
            Ok(pkg) => pkg,
            Err(js_error) => {
                let e = ErrorMetadata::bad_request(
                    "InvalidExternalModules",
                    format!(
                        "Loading the pushed modules encountered the following error:\n{js_error}"
                    ),
                );
                return Err(anyhow::anyhow!(js_error).context(e));
            },
        };

        // Write package to system table
        let id = self._upload_external_deps_package(pkg.clone()).await?;
        Ok((id, pkg))
    }

    #[fastrace::trace]
    async fn _upload_external_deps_package(
        &self,
        external_deps_package: ExternalDepsPackage,
    ) -> anyhow::Result<ExternalDepsPackageId> {
        let mut tx = self.begin(Identity::system()).await?;
        let mut model = ExternalPackagesModel::new(&mut tx);
        let result = model.put(external_deps_package).await?;
        self.commit(tx, "upload_exteral_deps_package").await?;
        Ok(result)
    }

    /// Deletes the given user tables in one transaction.
    /// Returns the total number of documents in all tables deleted.
    pub async fn delete_tables(
        &self,
        identity: &Identity,
        table_names: Vec<TableName>,
        table_namespace: TableNamespace,
    ) -> anyhow::Result<u64> {
        let mut tx = self.begin(identity.clone()).await?;
        let mut count = 0;
        for table_name in table_names {
            anyhow::ensure!(
                !table_name.is_system(),
                "cannot delete system table {table_name}"
            );
            let mut table_model = TableModel::new(&mut tx);
            count += table_model.must_count(table_namespace, &table_name).await?;
            table_model
                .delete_active_table(table_namespace, table_name)
                .await?;
        }
        self.commit(tx, "delete_tables").await?;
        Ok(count)
    }

    pub async fn delete_component(
        &self,
        identity: &Identity,
        component_id: ComponentId,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity.clone()).await?;
        ComponentConfigModel::new(&mut tx)
            .delete_component(component_id)
            .await?;
        self.commit(tx, "delete_component").await?;
        Ok(())
    }

    /// Add system indexes if they do not already exist and update
    /// existing indexes if needed.
    pub async fn _add_system_indexes(
        &self,
        identity: &Identity,
        indexes: BTreeMap<IndexName, IndexedFields>,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity.clone()).await?;
        let namespace = TableNamespace::by_component_TODO();
        for (index_name, index_fields) in indexes.into_iter() {
            let index_fields = self._validate_user_defined_index_fields(index_fields)?;
            let index_metadata =
                IndexMetadata::new_backfilling(*tx.begin_timestamp(), index_name, index_fields);
            let mut model = IndexModel::new(&mut tx);
            if let Some(existing_index_metadata) = model
                .pending_index_metadata(namespace, &index_metadata.name)?
                .or(model.enabled_index_metadata(namespace, &index_metadata.name)?)
            {
                if !index_metadata
                    .config
                    .same_spec(&existing_index_metadata.config)
                {
                    IndexModel::new(&mut tx)
                        .drop_index(existing_index_metadata.id())
                        .await?;
                    IndexModel::new(&mut tx)
                        .add_system_index(namespace, index_metadata)
                        .await?;
                }
            } else {
                IndexModel::new(&mut tx)
                    .add_system_index(namespace, index_metadata)
                    .await?;
            }
        }
        self.commit(tx, "add_system_indexes").await?;
        Ok(())
    }

    async fn bail_if_not_running(&self) -> anyhow::Result<()> {
        let backend_state = BackendStateModel::new(&mut self.begin(Identity::Unknown(None)).await?)
            .get_backend_state()
            .await?;
        if backend_state.is_stopped() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "BackendIsNotRunning",
                "Cannot perform this operation when the backend is not running"
            ));
        }
        Ok(())
    }

    pub async fn store_file(
        &self,
        component: ComponentId,
        content_length: Option<ContentLength>,
        content_type: Option<ContentType>,
        expected_sha256: Option<Sha256Digest>,
        body: BoxStream<'_, anyhow::Result<Bytes>>,
    ) -> anyhow::Result<DeveloperDocumentId> {
        self.bail_if_not_running().await?;
        let storage_id = self
            .file_storage
            .store_file(
                component.into(),
                content_length,
                content_type,
                body,
                expected_sha256,
                &self.usage_tracking,
            )
            .await?;
        Ok(storage_id)
    }

    pub async fn store_file_entry(
        &self,
        component: ComponentId,
        entry: FileStorageEntry,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let storage_id = self
            .file_storage
            .store_entry(component.into(), entry, &self.usage_tracking)
            .await?;
        Ok(storage_id)
    }

    pub async fn get_file_entry(
        &self,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<FileStorageEntry> {
        let mut file_storage_tx = self.begin(Identity::system()).await?;

        let Some(file_entry) = self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_entry(&mut file_storage_tx, component.into(), storage_id.clone())
            .await?
        else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("File {storage_id} not found"),
            )
            .into());
        };
        Ok(file_entry)
    }

    pub async fn get_file(
        &self,
        component: ComponentId,
        storage_id: FileStorageId,
    ) -> anyhow::Result<FileStream> {
        self.bail_if_not_running().await?;
        let mut file_storage_tx = self.begin(Identity::system()).await?;
        let Some(file_entry) = self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_entry(&mut file_storage_tx, component.into(), storage_id.clone())
            .await?
        else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("File {storage_id} not found"),
            )
            .into());
        };
        let Some(component_path) = file_storage_tx.get_component_path(component) else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("Component {component:?} not found"),
            )
            .into());
        };
        self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_stream(component_path, file_entry, self.usage_tracking.clone())
            .await
    }

    pub async fn get_file_range(
        &self,
        component: ComponentId,
        storage_id: FileStorageId,
        bytes_range: (Bound<u64>, Bound<u64>),
    ) -> anyhow::Result<FileRangeStream> {
        self.bail_if_not_running().await?;
        let mut file_storage_tx = self.begin(Identity::system()).await?;

        let Some(file_entry) = self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_entry(&mut file_storage_tx, component.into(), storage_id.clone())
            .await?
        else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("File {storage_id} not found"),
            )
            .into());
        };
        let Some(component_path) = file_storage_tx.get_component_path(component) else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("Component {component:?} not found"),
            )
            .into());
        };

        self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_range_stream(
                component_path,
                file_entry,
                bytes_range,
                self.usage_tracking.clone(),
            )
            .await
    }

    pub async fn authenticate(
        &self,
        token: AuthenticationToken,
        system_time: SystemTime,
    ) -> anyhow::Result<Identity> {
        let identity = match token {
            AuthenticationToken::Admin(token, acting_as) => {
                let admin_identity = self
                    .app_auth()
                    .check_key(token.to_string(), self.instance_name())
                    .await?;

                match acting_as {
                    Some(acting_user) => {
                        // Act as the given user
                        let Identity::InstanceAdmin(i) = admin_identity else {
                            anyhow::bail!(
                                "Admin identity returned from check_admin_key was not an admin."
                            );
                        };
                        Identity::ActingUser(i, acting_user)
                    },
                    None => admin_identity,
                }
            },
            AuthenticationToken::User(id_token) => {
                let mut tx = self.begin(Identity::system()).await?;
                let auth_infos = AuthInfoModel::new(&mut tx).get().await?;

                let auth_info_values: Vec<_> = auth_infos
                    .into_iter()
                    .map(|auth_info| auth_info.into_value())
                    .collect();

                let should_redact_errors = self
                    .log_visibility
                    .should_redact_logs_and_error(
                        &mut tx,
                        Identity::Unknown(None),
                        AllowedVisibility::PublicOnly,
                    )
                    .await?;

                let identity_result = validate_id_token(
                    // This is any JWT.
                    AuthIdToken(id_token.clone()),
                    cached_http_client_for(ClientPurpose::ProviderMetadata),
                    auth_info_values.clone(),
                    system_time,
                    should_redact_errors,
                )
                .await;

                Identity::user(identity_result?)
            },
            AuthenticationToken::PlaintextUser(token) => {
                // For plaintext authentication, create a PlaintextUser identity
                // The server is responsible for validating the token
                Identity::PlaintextUser(token)
            },
            AuthenticationToken::None => Identity::Unknown(None),
        };
        Ok(identity)
    }

    pub async fn validate_component_id(
        &self,
        identity: Identity,
        component_id: ComponentId,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity).await?;
        anyhow::ensure!(
            tx.get_component_path(component_id).is_some(),
            "Component {component_id:?} not found"
        );
        Ok(())
    }

    pub async fn udf_rate(
        &self,
        identity: Identity,
        identifier: UdfIdentifier,
        metric: UdfRate,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("udf_rate"));
        }
        self.function_log.udf_rate(identifier, metric, window)
    }

    pub async fn failure_percentage_top_k(
        &self,
        identity: Identity,
        window: MetricsWindow,
        k: usize,
    ) -> anyhow::Result<Vec<(String, Timeseries)>> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("failure_percentage_top_k"));
        }
        self.function_log.failure_percentage_top_k(window, k)
    }

    pub async fn cache_hit_percentage_top_k(
        &self,
        identity: Identity,
        window: MetricsWindow,
        k: usize,
    ) -> anyhow::Result<Vec<(String, Timeseries)>> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("failure_percentage_top_k"));
        }
        self.function_log.cache_hit_percentage_top_k(window, k)
    }

    pub async fn cache_hit_percentage(
        &self,
        identity: Identity,
        identifier: UdfIdentifier,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("cache_hit_percentage"));
        }
        self.function_log.cache_hit_percentage(identifier, window)
    }

    pub async fn latency_percentiles(
        &self,
        identity: Identity,
        identifier: UdfIdentifier,
        percentiles: Vec<Percentile>,
        window: MetricsWindow,
    ) -> anyhow::Result<BTreeMap<Percentile, Timeseries>> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("latency_percentiles_ms"));
        }
        self.function_log
            .latency_percentiles(identifier, percentiles, window)
    }

    pub async fn udf_summary(
        &self,
        identity: Identity,
        cursor: Option<CursorMs>,
    ) -> anyhow::Result<(Option<UdfMetricSummary>, Option<CursorMs>)> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("latency_percentiles_ms"));
        }
        Ok(self.function_log.udf_summary(cursor))
    }

    pub async fn table_rate(
        &self,
        identity: Identity,
        name: TableName,
        metric: TableRate,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("table_rate"));
        }
        self.function_log.table_rate(name, metric, window)
    }

    pub async fn stream_udf_execution(
        &self,
        identity: Identity,
        cursor: CursorMs,
    ) -> anyhow::Result<(Vec<FunctionExecution>, CursorMs)> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("stream_udf_execution"));
        }
        Ok(self.function_log.stream(cursor).await)
    }

    pub async fn stream_function_logs(
        &self,
        identity: Identity,
        cursor: CursorMs,
    ) -> anyhow::Result<(Vec<FunctionExecutionPart>, CursorMs)> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("stream_function_logs"));
        }
        Ok(self.function_log.stream_parts(cursor).await)
    }

    pub async fn scheduled_job_lag(
        &self,
        identity: Identity,
        window: MetricsWindow,
    ) -> anyhow::Result<Timeseries> {
        if !(identity.is_admin() || identity.is_system()) {
            anyhow::bail!(unauthorized_error("scheduled_job_lag"));
        }
        self.function_log.scheduled_job_lag(window)
    }

    pub async fn cancel_all_jobs(
        &self,
        component_id: ComponentId,
        path: Option<CanonicalizedComponentFunctionPath>,
        identity: Identity,
        start_next_ts: Option<Timestamp>,
        end_next_ts: Option<Timestamp>,
    ) -> anyhow::Result<()> {
        loop {
            let count = self
                .execute_with_audit_log_events_and_occ_retries(
                    identity.clone(),
                    "application_cancel_all_jobs",
                    |tx| {
                        Self::_cancel_all_jobs(
                            tx,
                            component_id,
                            path.clone(),
                            *MAX_JOBS_CANCEL_BATCH,
                            start_next_ts,
                            end_next_ts,
                        )
                        .into()
                    },
                )
                .await?;
            if count < *MAX_JOBS_CANCEL_BATCH {
                break;
            }
        }
        Ok(())
    }

    async fn _cancel_all_jobs(
        tx: &mut Transaction<RT>,
        component_id: ComponentId,
        path: Option<CanonicalizedComponentFunctionPath>,
        max_jobs: usize,
        start_next_ts: Option<Timestamp>,
        end_next_ts: Option<Timestamp>,
    ) -> anyhow::Result<(usize, Vec<DeploymentAuditLogEvent>)> {
        let count = SchedulerModel::new(tx, component_id.into())
            .cancel_all(path, max_jobs, start_next_ts, end_next_ts)
            .await?;
        Ok((count, vec![]))
    }

    /// Commit a transaction and send audit log events to the log manager if the
    /// transaction commits successfully.
    pub async fn commit_with_audit_log_events(
        &self,
        mut transaction: Transaction<RT>,
        events: Vec<DeploymentAuditLogEvent>,
        write_source: impl Into<WriteSource>,
    ) -> anyhow::Result<Timestamp> {
        DeploymentAuditLogModel::new(&mut transaction)
            .insert(events.clone())
            .await?;
        let ts = self.commit(transaction, write_source).await?;
        let logs = events
            .into_iter()
            .map(|event| {
                DeploymentAuditLogEvent::to_log_event(event, UnixTimestamp::from_nanos(ts.into()))
            })
            .try_collect()?;

        self.log_manager_client.send_logs(logs);
        Ok(ts)
    }

    // TODO CX-5139 Remove this when audit logs are being processed in LogManager.
    async fn insert_deployment_audit_log_events<'b, F, T>(
        tx: &mut Transaction<RT>,
        f: F,
    ) -> anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>
    where
        T: Send,
        F: Send + Sync,
        F: for<'c> Fn(
            &'c mut Transaction<RT>,
        )
            -> ShortBoxFuture<'c, 'b, anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>>,
    {
        let (t, events) = f(tx).0.await?;
        DeploymentAuditLogModel::new(tx)
            .insert(events.clone())
            .await?;
        Ok((t, events))
    }

    pub async fn execute_with_audit_log_events_and_occ_retries<'a, F, T>(
        &self,
        identity: Identity,
        write_source: impl Into<WriteSource>,
        f: F,
    ) -> anyhow::Result<T>
    where
        F: Send + Sync,
        T: Send + 'static,
        F: for<'b> Fn(
            &'b mut Transaction<RT>,
        )
            -> ShortBoxFuture<'b, 'a, anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>>,
    {
        self.execute_with_audit_log_events_and_occ_retries_with_pause_client(
            identity,
            write_source,
            f,
        )
        .await
        .map(|(t, _)| t)
    }

    pub async fn execute_with_audit_log_events_and_occ_retries_reporting_stats<'a, F, T>(
        &self,
        identity: Identity,
        write_source: impl Into<WriteSource>,
        f: F,
    ) -> anyhow::Result<(T, OccRetryStats)>
    where
        F: Send + Sync,
        T: Send + 'static,
        F: for<'b> Fn(
            &'b mut Transaction<RT>,
        )
            -> ShortBoxFuture<'b, 'a, anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>>,
    {
        self.execute_with_audit_log_events_and_occ_retries_with_pause_client(
            identity,
            write_source,
            f,
        )
        .await
    }

    pub async fn execute_with_audit_log_events_and_occ_retries_with_pause_client<'a, F, T>(
        &self,
        identity: Identity,
        write_source: impl Into<WriteSource>,
        f: F,
    ) -> anyhow::Result<(T, OccRetryStats)>
    where
        F: Send + Sync,
        T: Send + 'static,
        F: for<'b> Fn(
            &'b mut Transaction<RT>,
        )
            -> ShortBoxFuture<'b, 'a, anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>>,
    {
        let db = self.database.clone();
        let (ts, (t, events), stats) = db
            .execute_with_occ_retries(identity, FunctionUsageTracker::new(), write_source, |tx| {
                Self::insert_deployment_audit_log_events(tx, &f).into()
            })
            .await?;
        // Send deployment audit logs
        // TODO CX-5139 Remove this when audit logs are being processed in LogManager.
        let logs = events
            .into_iter()
            .map(|event| {
                DeploymentAuditLogEvent::to_log_event(event, UnixTimestamp::from_nanos(ts.into()))
            })
            .try_collect()?;

        self.log_manager_client.send_logs(logs);
        Ok((t, stats))
    }

    pub async fn execute_with_occ_retries<'a, T, F>(
        &'a self,
        identity: Identity,
        usage: FunctionUsageTracker,
        write_source: impl Into<WriteSource>,
        f: F,
    ) -> anyhow::Result<(Timestamp, T)>
    where
        F: Send + Sync,
        T: Send + 'static,
        F: for<'b> Fn(&'b mut Transaction<RT>) -> ShortBoxFuture<'b, 'a, anyhow::Result<T>>,
    {
        self.database
            .execute_with_occ_retries(identity, usage, write_source, f)
            .await
            .map(|(ts, t, _)| (ts, t))
    }

    pub async fn lookup_function_handle(
        &self,
        identity: Identity,
        handle: FunctionHandle,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath> {
        let mut tx = self.begin(identity).await?;
        FunctionHandlesModel::new(&mut tx).lookup(handle).await
    }

    pub async fn canonicalized_function_path(
        &self,
        identity: Identity,
        component_id: ComponentId,
        path: Option<String>,
        reference: Option<String>,
        function_handle: Option<String>,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath> {
        if let Some(function_handle) = function_handle {
            let handle = function_handle.parse()?;
            return self.lookup_function_handle(identity, handle).await;
        }
        let reference = match (path, reference) {
            (None, None) => anyhow::bail!(ErrorMetadata::bad_request(
                "MissingUdfPathOrFunctionReference",
                "Missing UDF path or function reference. One must be provided."
            )),
            (Some(path), None) => Reference::Function(path.parse()?),
            (None, Some(reference)) => reference.parse()?,
            (Some(_), Some(_)) => anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidUdfPathOrFunctionReference",
                "Both UDF path and function reference provided. Only one must be provided."
            )),
        };
        // Reading from a separate transaction here is safe because the component id to
        // component path mapping is stable.
        let mut tx = self.begin(identity).await?;
        let mut components_model = ComponentsModel::new(&mut tx);
        let resource = components_model
            .resolve(component_id, None, &reference)
            .await?;
        let path = match resource {
            Resource::Function(path) => path,
            Resource::Value(_) | Resource::ResolvedSystemUdf(_) => {
                anyhow::bail!("Resource type not supported for internal query")
            },
        };
        Ok(path)
    }

    pub fn files_storage(&self) -> Arc<dyn Storage> {
        self.application_storage.files_storage.clone()
    }

    /// Add hidden primary key indexes for the given tables. Developers do not
    /// have access to these indexes.
    pub async fn add_primary_key_indexes(
        &self,
        identity: &Identity,
        indexes: BTreeMap<TableName, PrimaryKey>,
    ) -> anyhow::Result<()> {
        let indexes: BTreeMap<IndexName, IndexedFields> = indexes
            .into_iter()
            .map(|(table_name, primary_key)| {
                let index_name = IndexName::new_reserved(
                    table_name,
                    AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?;
                let index_fields = primary_key.into_indexed_fields();
                Ok((index_name, index_fields))
            })
            .collect::<anyhow::Result<_>>()?;
        self._add_system_indexes(identity, indexes).await?;
        Ok(())
    }

    pub async fn wait_for_primary_key_indexes_ready(
        &self,
        identity: Identity,
        indexes: BTreeSet<TableName>,
    ) -> anyhow::Result<()> {
        loop {
            let mut tx = self.begin(identity.clone()).await?;
            if AirbyteImportModel::new(&mut tx)
                .primary_key_indexes_ready(&indexes)
                .await?
            {
                return Ok(());
            }
            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            subscription.wait_for_invalidation().await;
        }
    }

    /// Return if the primary key indexes for the given tables have finished
    /// backfilling.
    pub async fn primary_key_indexes_ready(
        &self,
        identity: Identity,
        indexes: BTreeSet<TableName>,
    ) -> anyhow::Result<bool> {
        let mut tx = self.begin(identity).await?;
        AirbyteImportModel::new(&mut tx)
            .primary_key_indexes_ready(&indexes)
            .await
    }

    /// Inserts, replaces, or deletes the Airbyte record, depending on the sync
    /// mode.
    async fn process_record(
        &self,
        tx: &mut Transaction<RT>,
        record: AirbyteRecord,
        stream: &ValidatedAirbyteStream,
    ) -> anyhow::Result<()> {
        let table_name = record.table_name().clone();
        let namespace = TableNamespace::by_component_TODO();
        let deleted = record.deleted();
        let object = record.into_object();
        match stream {
            ValidatedAirbyteStream::Append => {
                UserFacingModel::new(tx, namespace)
                    .insert(table_name.clone(), object.clone())
                    .await?;
                Ok::<(), anyhow::Error>(())
            },
            ValidatedAirbyteStream::Dedup(primary_key) => {
                // Get the current value of the record based on the primary key
                let mut range = Vec::new();
                for field_path in primary_key.clone().into_indexed_fields().iter() {
                    let value = object
                        .get_path(field_path)
                        .context(ErrorMetadata::bad_request(
                            "MissingPrimaryKeyValue",
                            format!("Missing value for primary key field: {field_path:?}."),
                        ))?;
                    let range_expression =
                        IndexRangeExpression::Eq(field_path.clone(), value.clone().into());
                    range.push(range_expression)
                }
                let index_range = IndexRange {
                    index_name: IndexName::new_reserved(
                        table_name.clone(),
                        AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                    )?,
                    range,
                    order: Order::Asc,
                };
                let query = common::query::Query::index_range(index_range);
                let mut query_stream = ResolvedQuery::new(tx, namespace, query)?;
                // Replace or delete the record or insert if there is no existing record that
                // matches this value for the primary key.
                if let Some(doc) = query_stream.expect_at_most_one(tx).await? {
                    let doc_id = DeveloperDocumentId::from(doc.id());
                    if deleted {
                        UserFacingModel::new(tx, namespace).delete(doc_id).await?;
                    } else {
                        UserFacingModel::new(tx, namespace)
                            .replace(doc_id, object.clone())
                            .await?;
                    }
                } else {
                    UserFacingModel::new(tx, namespace)
                        .insert(table_name.clone(), object.clone())
                        .await?;
                }
                Ok(())
            },
        }
    }

    /// Insert airbyte record messages into the table for the stream. Returns
    /// the number of documents inserted.
    pub async fn import_airbyte_records(
        &self,
        identity: &Identity,
        records: Vec<AirbyteRecord>,
        tables: BTreeMap<TableName, ValidatedAirbyteStream>,
    ) -> anyhow::Result<u64> {
        let mut count = 0;
        let mut tx = self.begin(identity.clone()).await?;
        for record in records {
            let table_name = record.table_name();
            let stream = tables.get(table_name).context(ErrorMetadata::bad_request(
                "MissingStream",
                format!("Missing stream for table {table_name}"),
            ))?;
            let insert_fut = self.process_record(&mut tx, record.clone(), stream);
            match insert_fut.await {
                Ok(()) => {},
                Err(e) if e.is_pagination_limit() => {
                    self.commit(tx, "airbyte_write_page").await?;
                    tx = self.begin(identity.clone()).await?;
                    self.process_record(&mut tx, record, stream).await?;
                },
                Err(e) => anyhow::bail!(e),
            }
            count += 1;
        }
        self.commit(tx, "app_private_import_airbyte").await?;
        Ok(count)
    }

    pub async fn apply_fivetran_operations(
        &self,
        identity: &Identity,
        rows: Vec<BatchWriteRow>,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity.clone()).await?;
        let mut model = FivetranImportModel::new(&mut tx);

        for row in rows {
            match model.apply_operation(row.clone()).await {
                Ok(()) => {},
                Err(e) if e.is_pagination_limit() => {
                    self.commit(tx, "fivetran_write_page").await?;

                    tx = self.begin(identity.clone()).await?;
                    model = FivetranImportModel::new(&mut tx);

                    model.apply_operation(row).await?;
                },
                Err(e) => anyhow::bail!(e),
            }
        }

        self.commit(tx, "app_fivetran_import").await?;
        Ok(())
    }

    pub async fn fivetran_truncate(
        &self,
        identity: &Identity,
        table_name: TableName,
        delete_before: Option<DateTime<Utc>>,
        delete_type: DeleteType,
    ) -> anyhow::Result<()> {
        let mut done = false;
        while !done {
            let mut tx = self.begin(identity.clone()).await?;
            if !TableModel::new(&mut tx).table_exists(TableNamespace::Global, &table_name) {
                // Simply accept the truncate if the table exists
                return Ok(());
            }
            let mut query: ResolvedQuery<_> = FivetranImportModel::new(&mut tx)
                .synced_query(&table_name, &delete_before)
                .await?;

            loop {
                let res: anyhow::Result<()> = try {
                    match query.next(&mut tx, None).await? {
                        Some(doc) => {
                            FivetranImportModel::new(&mut tx)
                                .truncate_document(doc, delete_type)
                                .await?;
                        },
                        None => {
                            done = true;
                            break;
                        },
                    }
                };
                if let Err(e) = res {
                    if e.is_pagination_limit() {
                        // Need a new transaction: commit what we already have and continue
                        break;
                    } else {
                        anyhow::bail!(e)
                    }
                }
            }

            self.commit(tx, "app_fivetran_truncate").await?;
        }

        Ok(())
    }

    pub async fn get_schema(
        &self,
        namespace: TableNamespace,
        identity: &Identity,
    ) -> anyhow::Result<Option<Arc<DatabaseSchema>>> {
        let mut tx = self.begin(identity.clone()).await?;
        let mut model = SchemaModel::new(&mut tx, namespace);
        Ok(model
            .get_by_state(SchemaState::Active)
            .await?
            .map(|(_id, schema)| schema))
    }

    pub async fn fivetran_create_table(
        &self,
        identity: &Identity,
        table_definition: TableDefinition,
    ) -> anyhow::Result<()> {
        let table_name = table_definition.table_name;

        // Add the indexes to the table.
        let indexes: BTreeMap<IndexName, IndexedFields> = table_definition
            .indexes
            .into_iter()
            .map(|(descriptor, fields)| {
                let index_name = IndexName::new_reserved(table_name.clone(), descriptor)?;
                let index_fields = fields.fields;
                Ok((index_name, index_fields))
            })
            .collect::<anyhow::Result<_>>()?;
        self._add_system_indexes(identity, indexes).await?;

        // Wait for the indexes to be ready.
        loop {
            let mut tx = self.begin(identity.clone()).await?;
            if IndexModel::new(&mut tx)
                .indexes_ready(
                    &FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
                    &btreeset! { table_name.clone() },
                )
                .await?
            {
                return Ok(());
            }
            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            subscription.wait_for_invalidation().await;
        }
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.log_manager_client.shutdown().await?;
        self.table_summary_worker.shutdown().await?;
        self.system_table_cleanup_worker.lock().shutdown();
        self.schema_worker.lock().shutdown();
        let index_worker = self.index_worker.lock().take();
        if let Some(index_worker) = index_worker {
            shutdown_and_join(index_worker).await?;
        }
        self.search_worker.lock().shutdown();
        self.search_and_vector_bootstrap_worker.lock().shutdown();
        self.fast_forward_worker.lock().shutdown();
        let export_worker = self.export_worker.lock().take();
        if let Some(export_worker) = export_worker {
            shutdown_and_join(export_worker).await?;
        }
        let snapshot_import_worker = self.snapshot_import_worker.lock().take();
        if let Some(snapshot_import_worker) = snapshot_import_worker {
            shutdown_and_join(snapshot_import_worker).await?;
        }
        self.runner.shutdown().await?;
        self.scheduled_job_runner.shutdown();
        self.cron_job_executor.lock().shutdown();
        self.database.shutdown().await?;
        let migration_worker = self.migration_worker.lock().take();
        if let Some(migration_worker) = migration_worker {
            shutdown_and_join(migration_worker).await?;
        }
        tracing::info!("Application shut down");
        Ok(())
    }
}
