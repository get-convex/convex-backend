#![feature(try_blocks)]
#![feature(lazy_cell)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(coroutines)]

use std::{
    collections::{
        BTreeMap,
        HashSet,
    },
    ops::Bound,
    sync::{
        Arc,
        LazyLock,
    },
    time::SystemTime,
};

use anyhow::Context;
use authentication::{
    validate_id_token,
    Auth0IdToken,
};
use bytes::Bytes;
use common::{
    auth::AuthInfo,
    bootstrap_model::{
        index::{
            database_index::IndexedFields,
            index_validation_error,
            IndexMetadata,
        },
        schema::{
            invalid_schema_id,
            parse_schema_id,
        },
    },
    document::{
        DocumentUpdate,
        ParsedDocument,
        CREATION_TIME_FIELD_PATH,
    },
    errors::{
        report_error,
        JsError,
    },
    http::fetch::FetchClient,
    knobs::{
        MAX_JOBS_CANCEL_BATCH,
        SNAPSHOT_LIST_LIMIT,
    },
    log_streaming::LogSender,
    paths::FieldPath,
    pause::PauseClient,
    persistence::Persistence,
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    schemas::DatabaseSchema,
    types::{
        env_var_limit_met,
        env_var_name_not_unique,
        AllowedVisibility,
        ConvexOrigin,
        ConvexSite,
        CursorMs,
        EnvVarName,
        FunctionCaller,
        IndexId,
        IndexName,
        ModuleEnvironment,
        NodeDependency,
        ObjectKey,
        PersistenceVersion,
        RepeatableTimestamp,
        TableName,
        Timestamp,
        UdfIdentifier,
        UdfType,
        ENV_VAR_LIMIT,
    },
    RequestId,
};
use cron_jobs::CronJobExecutor;
use database::{
    unauthorized_error,
    Database,
    DocumentDeltas,
    FastForwardIndexWorker,
    IndexModel,
    IndexWorker,
    OccRetryStats,
    SearchIndexWorker,
    ShortBoxFuture,
    Snapshot,
    SnapshotPage,
    Subscription,
    SystemMetadataModel,
    TableModel,
    Token,
    Transaction,
    WriteSource,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
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
use futures::{
    channel::oneshot,
    stream::BoxStream,
    Stream,
};
use headers::{
    ContentLength,
    ContentType,
};
use http_client::cached_http_client;
use isolate::{
    parse_udf_args,
    AuthConfig,
    HttpActionRequest,
    HttpActionResponse,
    UdfOutcome,
    CONVEX_ORIGIN,
    CONVEX_SITE,
};
use keybroker::{
    Identity,
    InstanceSecret,
    KeyBroker,
};
use maplit::btreemap;
use model::{
    auth::AuthInfoModel,
    config::{
        types::{
            ConfigFile,
            ConfigMetadata,
            ModuleConfig,
            AUTH_CONFIG_FILE_NAME,
        },
        ConfigModel,
    },
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    environment_variables::{
        types::EnvironmentVariable,
        EnvironmentVariablesModel,
        ENVIRONMENT_VARIABLES_TABLE,
    },
    exports::{
        types::{
            Export,
            ExportFormat,
            ExportObjectKeys,
        },
        EXPORTS_TABLE,
    },
    external_packages::{
        types::{
            ExternalDepsPackage,
            ExternalDepsPackageId,
        },
        ExternalPackagesModel,
    },
    file_storage::FileStorageId,
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
    },
    source_packages::types::SourcePackage,
    udf_config::{
        types::UdfConfig,
        UdfConfigModel,
    },
};
use node_executor::{
    source_package::upload_package,
    Actions,
};
use parking_lot::Mutex;
use rand::Rng;
use scheduled_jobs::ScheduledJobRunner;
use schema_worker::SchemaWorker;
use search::{
    query::RevisionWithKeys,
    searcher::Searcher,
};
use semver::Version;
use serde_json::Value as JsonValue;
use snapshot_import::{
    clear_tables,
    store_uploaded_import,
};
use storage::{
    BufferedUpload,
    ClientDrivenUploadPartToken,
    ClientDrivenUploadToken,
    Storage,
    StorageExt,
    StorageGetStream,
    Upload,
};
use sync_types::{
    AuthenticationToken,
    CanonicalizedModulePath,
    CanonicalizedUdfPath,
    UdfPath,
};
use table_summary_worker::{
    TableSummaryClient,
    TableSummaryWorker,
};
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
    UsageCounter,
};
use value::{
    id_v6::DocumentIdV6,
    sha256::Sha256Digest,
    ConvexValue,
    Namespace,
    ResolvedDocumentId,
};
use vector::{
    PublicVectorSearchQueryResult,
    VectorSearch,
};

use crate::{
    application_function_runner::ApplicationFunctionRunner,
    export_worker::ExportWorker,
    function_log::{
        FunctionExecutionLog,
        MetricsWindow,
        Percentile,
        TableRate,
        Timeseries,
        UdfMetricSummary,
        UdfRate,
    },
    log_visibility::LogVisibility,
    module_cache::{
        ModuleCache,
        ModuleCacheWorker,
    },
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
    snapshot_import::SnapshotImportWorker,
};

pub mod application_function_runner;
mod cache;
pub mod cron_jobs;
mod export_worker;
pub mod function_log;
pub mod log_visibility;
mod metrics;
mod module_cache;
pub mod redaction;
pub mod scheduled_jobs;
mod schema_worker;
pub mod snapshot_import;
mod table_summary_worker;
pub mod valid_identifier;

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;
#[cfg(test)]
mod tests;

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
    pub source_package: Option<SourcePackage>,
    pub analyze_results: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
}

#[derive(Debug)]
pub struct QueryReturn {
    pub result: Result<ConvexValue, RedactedJsError>,
    pub log_lines: RedactedLogLines,
    pub token: Token,
    pub ts: Timestamp,
    pub journal: QueryJournal,
}

#[derive(Debug)]
pub struct MutationReturn {
    pub value: ConvexValue,
    pub log_lines: RedactedLogLines,
    pub ts: Timestamp,
}

#[derive(thiserror::Error, Debug)]
#[error("Mutation failed: {error}")]
pub struct MutationError {
    pub error: RedactedJsError,
    pub log_lines: RedactedLogLines,
}

#[derive(Debug)]
pub struct ActionReturn {
    pub value: ConvexValue,
    pub log_lines: RedactedLogLines,
}

#[derive(thiserror::Error, Debug)]
#[error("Action failed: {error}")]
pub struct ActionError {
    pub error: RedactedJsError,
    pub log_lines: RedactedLogLines,
}

#[derive(Debug)]
pub struct FunctionReturn {
    pub value: ConvexValue,
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

pub struct Application<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    runner: Arc<ApplicationFunctionRunner<RT>>,
    function_log: FunctionExecutionLog<RT>,
    file_storage: FileStorage<RT>,
    files_storage: Arc<dyn Storage>,
    modules_storage: Arc<dyn Storage>,
    search_storage: Arc<dyn Storage>,
    exports_storage: Arc<dyn Storage>,
    snapshot_imports_storage: Arc<dyn Storage>,
    usage_tracking: UsageCounter,
    key_broker: KeyBroker,
    instance_name: String,
    scheduled_job_runner: ScheduledJobRunner<RT>,
    cron_job_executor: Arc<Mutex<RT::Handle>>,
    index_worker: Arc<Mutex<RT::Handle>>,
    fast_forward_worker: Arc<Mutex<RT::Handle>>,
    search_worker: Arc<Mutex<RT::Handle>>,
    search_and_vector_bootstrap_worker: Arc<Mutex<RT::Handle>>,
    table_summary_worker: TableSummaryClient<RT>,
    schema_worker: Arc<Mutex<RT::Handle>>,
    snapshot_import_worker: Arc<Mutex<RT::Handle>>,
    export_worker: Arc<Mutex<RT::Handle>>,
    log_sender: Arc<dyn LogSender>,
    log_visibility: Arc<dyn LogVisibility<RT>>,
    module_cache: ModuleCache<RT>,
    system_env_var_names: HashSet<EnvVarName>,
}

impl<RT: Runtime> Clone for Application<RT> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            database: self.database.clone(),
            runner: self.runner.clone(),
            function_log: self.function_log.clone(),
            file_storage: self.file_storage.clone(),
            files_storage: self.files_storage.clone(),
            modules_storage: self.modules_storage.clone(),
            search_storage: self.search_storage.clone(),
            exports_storage: self.exports_storage.clone(),
            snapshot_imports_storage: self.snapshot_imports_storage.clone(),
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
            log_sender: self.log_sender.clone(),
            log_visibility: self.log_visibility.clone(),
            module_cache: self.module_cache.clone(),
            system_env_var_names: self.system_env_var_names.clone(),
        }
    }
}

impl<RT: Runtime> Application<RT> {
    pub async fn new(
        runtime: RT,
        database: Database<RT>,
        file_storage: FileStorage<RT>,
        files_storage: Arc<dyn Storage>,
        modules_storage: Arc<dyn Storage>,
        search_storage: Arc<dyn Storage>,
        exports_storage: Arc<dyn Storage>,
        snapshot_imports_storage: Arc<dyn Storage>,
        usage_tracking: UsageCounter,
        key_broker: KeyBroker,
        instance_name: String,
        instance_secret: InstanceSecret,
        function_runner: Arc<dyn FunctionRunner<RT>>,
        convex_origin: ConvexOrigin,
        convex_site: ConvexSite,
        searcher: Arc<dyn Searcher>,
        persistence: Arc<dyn Persistence>,
        node_actions: Actions,
        fetch_client: Arc<dyn FetchClient>,
        log_sender: Arc<dyn LogSender>,
        log_visibility: Arc<dyn LogVisibility<RT>>,
        snapshot_import_pause_client: PauseClient,
        scheduled_jobs_pause_client: PauseClient,
    ) -> anyhow::Result<Self> {
        let module_cache = ModuleCacheWorker::start(runtime.clone(), database.clone()).await;
        let module_loader = Arc::new(module_cache.clone());

        let system_env_vars = btreemap! {
            CONVEX_ORIGIN.clone() => convex_origin.parse()?,
            CONVEX_SITE.clone() => convex_site.parse()?
        };

        let index_worker = IndexWorker::new(
            runtime.clone(),
            persistence.clone(),
            database.retention_validator(),
            database.clone(),
        );
        let index_worker = Arc::new(Mutex::new(runtime.spawn("index_worker", index_worker)));
        let fast_forward_worker =
            FastForwardIndexWorker::create_and_start(runtime.clone(), database.clone());
        let fast_forward_worker = Arc::new(Mutex::new(
            runtime.spawn("fast_forward_worker", fast_forward_worker),
        ));
        let search_worker = SearchIndexWorker::create_and_start(
            runtime.clone(),
            database.clone(),
            search_storage.clone(),
            searcher,
        );
        let search_worker = Arc::new(Mutex::new(runtime.spawn("search_worker", search_worker)));
        let search_and_vector_bootstrap_worker = Arc::new(Mutex::new(
            database.start_search_and_vector_bootstrap(PauseClient::new()),
        ));
        let table_summary_worker =
            TableSummaryWorker::start(runtime.clone(), database.clone(), persistence.clone());
        let schema_worker = Arc::new(Mutex::new(runtime.spawn(
            "schema_worker",
            SchemaWorker::start(runtime.clone(), database.clone()),
        )));

        let function_log = FunctionExecutionLog::new(
            runtime.clone(),
            database.usage_counter(),
            log_sender.clone(),
        );
        let runner = Arc::new(ApplicationFunctionRunner::new(
            instance_name.clone(),
            instance_secret,
            runtime.clone(),
            database.clone(),
            key_broker.clone(),
            function_runner.clone(),
            node_actions,
            file_storage.transactional_file_storage.clone(),
            modules_storage.clone(),
            module_loader,
            function_log.clone(),
            system_env_vars.clone(),
            fetch_client,
        ));
        function_runner.set_action_callbacks(runner.clone());

        let scheduled_job_runner = ScheduledJobRunner::start(
            runtime.clone(),
            database.clone(),
            runner.clone(),
            function_log.clone(),
            scheduled_jobs_pause_client,
        );

        let cron_job_executor_fut = CronJobExecutor::start(
            runtime.clone(),
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
            exports_storage.clone(),
            files_storage.clone(),
            database.usage_counter().clone(),
        );
        let export_worker = Arc::new(Mutex::new(runtime.spawn("export_worker", export_worker)));

        let snapshot_import_worker = SnapshotImportWorker::new(
            runtime.clone(),
            database.clone(),
            snapshot_imports_storage.clone(),
            file_storage.clone(),
            database.usage_counter().clone(),
            snapshot_import_pause_client,
        );
        let snapshot_import_worker = Arc::new(Mutex::new(
            runtime.spawn("snapshot_import_worker", snapshot_import_worker),
        ));

        Ok(Self {
            runtime,
            database,
            runner,
            function_log,
            file_storage,
            files_storage,
            modules_storage,
            search_storage,
            exports_storage,
            snapshot_imports_storage,
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
            log_sender,
            log_visibility,
            module_cache,
            system_env_var_names: system_env_vars.into_keys().collect(),
        })
    }

    pub fn runtime(&self) -> RT {
        self.runtime.clone()
    }

    pub fn modules_storage(&self) -> &Arc<dyn Storage> {
        &self.modules_storage
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

    pub fn now_ts_for_reads(&self) -> RepeatableTimestamp {
        self.database.now_ts_for_reads()
    }

    pub fn instance_name(&self) -> String {
        self.instance_name.clone()
    }

    #[minitrace::trace]
    pub async fn begin(&self, identity: Identity) -> anyhow::Result<Transaction<RT>> {
        self.database.begin(identity).await
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn commit_test(&self, transaction: Transaction<RT>) -> anyhow::Result<Timestamp> {
        self.commit(transaction, "test").await
    }

    #[minitrace::trace]
    pub async fn commit(
        &self,
        transaction: Transaction<RT>,
        write_source: impl Into<WriteSource>,
    ) -> anyhow::Result<Timestamp> {
        self.database
            .commit_with_write_source(transaction, write_source)
            .await
    }

    pub async fn subscribe(&self, token: Token) -> anyhow::Result<Subscription> {
        self.database.subscribe(token).await
    }

    pub fn usage_counter(&self) -> UsageCounter {
        self.database.usage_counter().clone()
    }

    pub async fn document_deltas(
        &self,
        identity: Identity,
        cursor: Timestamp,
        table_filter: Option<TableName>,
        rows_read_limit: usize,
        rows_returned_limit: usize,
    ) -> anyhow::Result<DocumentDeltas> {
        self.database
            .document_deltas(
                identity,
                Some(cursor),
                table_filter,
                rows_read_limit,
                rows_returned_limit,
            )
            .await
    }

    pub async fn list_snapshot(
        &self,
        identity: Identity,
        snapshot: Option<Timestamp>,
        cursor: Option<DocumentIdV6>,
        table_filter: Option<TableName>,
    ) -> anyhow::Result<SnapshotPage> {
        self.database
            .list_snapshot(
                identity,
                snapshot,
                cursor,
                table_filter,
                *SNAPSHOT_LIST_LIMIT,
                *SNAPSHOT_LIST_LIMIT,
            )
            .await
    }

    pub async fn refresh_token(
        &self,
        token: Token,
        ts: Timestamp,
    ) -> anyhow::Result<Option<Token>> {
        self.database.refresh_token(token, ts).await
    }

    pub fn persistence_version(&self) -> PersistenceVersion {
        self.database.persistence_version()
    }

    pub fn snapshot(&self, ts: RepeatableTimestamp) -> anyhow::Result<Snapshot> {
        self.database.snapshot(ts)
    }

    pub fn latest_snapshot(&self) -> anyhow::Result<Snapshot> {
        self.database.latest_snapshot()
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

    pub async fn storage_generate_upload_url(&self) -> anyhow::Result<String> {
        let issued_ts = self.runtime().unix_timestamp();
        let url = self
            .file_storage
            .transactional_file_storage
            .generate_upload_url(self.key_broker(), issued_ts)?;

        Ok(url)
    }

    pub async fn read_only_udf(
        &self,
        request_id: RequestId,
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
    ) -> anyhow::Result<QueryReturn> {
        let ts = *self.now_ts_for_reads();
        self.read_only_udf_at_ts(
            request_id,
            name,
            args,
            identity,
            ts,
            None,
            allowed_visibility,
            caller,
        )
        .await
    }

    #[minitrace::trace]
    pub async fn read_only_udf_at_ts(
        &self,
        request_id: RequestId,
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        ts: Timestamp,
        journal: Option<Option<String>>,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
    ) -> anyhow::Result<QueryReturn> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                allowed_visibility.clone(),
            )
            .await?;
        let journal = match journal
            .map(|serialized_journal| {
                self.key_broker()
                    .decrypt_query_journal(serialized_journal, self.persistence_version())
            })
            .transpose()
        {
            Ok(journal) => journal,
            Err(e) if e.is_deterministic_user_error() => {
                return Ok(QueryReturn {
                    result: Err(RedactedJsError::from_js_error(
                        JsError::from_error(e),
                        block_logging,
                        request_id,
                    )),
                    log_lines: RedactedLogLines::empty(),
                    // Create a token for an empty read set because we haven't
                    // done any reads yet.
                    token: Token::empty(ts),
                    ts,
                    journal: QueryJournal::new(),
                });
            },
            Err(e) => anyhow::bail!(e),
        };

        match self
            .runner
            .run_query_at_ts(
                request_id.clone(),
                name,
                args,
                identity,
                ts,
                journal,
                allowed_visibility,
                caller,
                block_logging,
            )
            .await
        {
            Ok(result) => Ok(result),
            Err(e) if e.is_deterministic_user_error() => Ok(QueryReturn {
                result: Err(RedactedJsError::from_js_error(
                    JsError::from_error(e),
                    block_logging,
                    request_id,
                )),
                log_lines: RedactedLogLines::empty(),
                // Create a token for an empty read set because we haven't
                // done any reads yet.
                token: Token::empty(ts),
                ts,
                journal: QueryJournal::new(),
            }),
            Err(e) => anyhow::bail!(e),
        }
    }

    #[minitrace::trace]
    pub async fn mutation_udf(
        &self,
        request_id: RequestId,
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
        pause_client: PauseClient,
    ) -> anyhow::Result<Result<MutationReturn, MutationError>> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                allowed_visibility.clone(),
            )
            .await?;
        match self
            .runner
            .retry_mutation(
                request_id.clone(),
                name,
                args,
                identity,
                mutation_identifier,
                allowed_visibility,
                caller,
                pause_client,
                block_logging,
            )
            .await
        {
            Ok(result) => Ok(result),
            Err(e) if e.is_deterministic_user_error() => Ok(Err(MutationError {
                error: RedactedJsError::from_js_error(
                    JsError::from_error(e),
                    block_logging,
                    request_id,
                ),
                log_lines: RedactedLogLines::empty(),
            })),
            Err(e) => anyhow::bail!(e),
        }
    }

    #[minitrace::trace]
    pub async fn action_udf(
        &self,
        request_id: RequestId,
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<ActionReturn, ActionError>> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                allowed_visibility.clone(),
            )
            .await?;

        let should_spawn = caller.run_until_completion_if_cancelled();
        let runner: Arc<ApplicationFunctionRunner<RT>> = self.runner.clone();
        let request_id_ = request_id.clone();
        let run_action = async move {
            runner
                .run_action(
                    request_id_,
                    name,
                    args,
                    identity,
                    allowed_visibility,
                    caller,
                    block_logging,
                )
                .await
        };
        let result = if should_spawn {
            // Spawn running the action in a separate future. This way, even if we
            // get cancelled, it will continue to run to completion.
            let (tx, rx) = oneshot::channel();
            self.runtime.spawn("run_action", async move {
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
        match result {
            Ok(result) => Ok(result),
            Err(e) => anyhow::bail!(e),
        }
    }

    pub async fn http_action_udf(
        &self,
        request_id: RequestId,
        name: UdfPath,
        http_request: HttpActionRequest,
        identity: Identity,
        caller: FunctionCaller,
    ) -> anyhow::Result<HttpActionResponse> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                AllowedVisibility::PublicOnly,
            )
            .await?;

        // Spawn running the action in a separate future. This way, even if we
        // get cancelled, it will continue to run to completion.
        let (tx, rx) = oneshot::channel();
        let runner = self.runner.clone();
        self.runtime.spawn("run_http_action", async move {
            let result = runner
                .run_http_action(
                    request_id,
                    name,
                    http_request,
                    identity,
                    caller,
                    runner.clone(),
                )
                .await;
            // Don't log errors if the caller has gone away.
            _ = tx.send(result);
        });
        let result = rx
            .await
            .context("run_action one shot sender dropped prematurely?")?;
        match result {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(error)) => Ok(HttpActionResponse::from(RedactedJsError::from_js_error(
                error,
                block_logging,
                RequestId::new(),
            ))),
            Err(e) => anyhow::bail!(e),
        }
    }

    /// Run a function of an arbitrary type from its name
    pub async fn any_udf(
        &self,
        request_id: RequestId,
        name: UdfPath,
        args: Vec<JsonValue>,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                allowed_visibility.clone(),
            )
            .await?;

        // We use a separate transaction to get the type of the UDF before calling the
        // appropriate type-specific code. While this could lead to incorrect
        // “function not found” messages errors if the user changes the type of the
        // UDF between the two transactions without deleting it, this situation is
        // rare enough to disregard it.
        let mut tx_type = self.begin(identity.clone()).await?;

        let canonicalized_name: CanonicalizedUdfPath = name.clone().canonicalize();
        let Some(analyzed_function) = self
            .runner
            .module_cache
            .get_analyzed_function(&mut tx_type, &canonicalized_name)
            .await?
            .ok()
            .filter(|af| {
                (identity.is_admin() || af.visibility == Some(Visibility::Public))
                    && af.udf_type != UdfType::HttpAction
            })
        else {
            let missing_or_internal = format!(
                "Could not find function for '{}'. Did you forget to run `npx convex dev` or `npx \
                 convex deploy`?",
                String::from(canonicalized_name.strip())
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

        match analyzed_function.udf_type {
            UdfType::Query => self
                .read_only_udf(request_id, name, args, identity, allowed_visibility, caller)
                .await
                .map(
                    |QueryReturn {
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
                    name,
                    args,
                    identity,
                    None,
                    allowed_visibility,
                    caller,
                    PauseClient::new(),
                )
                .await
                .map(|res| {
                    res.map(
                        |MutationReturn {
                             value, log_lines, ..
                         }| FunctionReturn { value, log_lines },
                    )
                    .map_err(
                        |MutationError {
                             error, log_lines, ..
                         }| FunctionError { error, log_lines },
                    )
                }),
            UdfType::Action => self
                .action_udf(request_id, name, args, identity, allowed_visibility, caller)
                .await
                .map(|res| {
                    res.map(
                        |ActionReturn {
                             value, log_lines, ..
                         }| FunctionReturn { value, log_lines },
                    )
                    .map_err(
                        |ActionError {
                             error, log_lines, ..
                         }| FunctionError { error, log_lines },
                    )
                }),
            UdfType::HttpAction => {
                anyhow::bail!(
                    "HTTP actions not supported in the /functions endpoint. A “not found” message \
                     should be returned instead."
                )
            },
        }
    }

    pub async fn request_export(
        &self,
        identity: Identity,
        zip: bool,
        include_storage: bool,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(identity.is_admin(), unauthorized_error("request_export"));
        let snapshot = self.latest_snapshot()?;
        let user_table_count = snapshot.table_registry.user_table_names().count();
        if user_table_count == 0 {
            return Err(ErrorMetadata::bad_request(
                "NoTables",
                format!("There are no tables to export."),
            )
            .into());
        }
        let mut tx = self.begin(identity).await?;
        let export_requested = ExportWorker::export_in_state(&mut tx, "requested").await?;
        let export_in_progress = ExportWorker::export_in_state(&mut tx, "in_progress").await?;
        match (export_requested, export_in_progress) {
            (None, None) => {
                let format = if zip {
                    ExportFormat::Zip { include_storage }
                } else {
                    match UdfConfigModel::new(&mut tx).get().await? {
                        Some(udf_config) => {
                            // Maintain legacy internal export format for older NPM versions
                            if udf_config.server_version
                                > *MAX_UDF_SERVER_VERSION_WITHOUT_CLEAN_EXPORT
                            {
                                ExportFormat::CleanJsonl
                            } else {
                                ExportFormat::InternalJson
                            }
                        },
                        // They haven't pushed functions yet - give them clean export.
                        None => ExportFormat::CleanJsonl,
                    }
                };
                SystemMetadataModel::new(&mut tx)
                    .insert(&EXPORTS_TABLE, Export::requested(format).try_into()?)
                    .await?;
                Ok(())
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
        Ok(())
    }

    pub async fn get_zip_export(
        &self,
        identity: Identity,
        snapshot_ts: Timestamp,
    ) -> anyhow::Result<(StorageGetStream, String)> {
        let stream = self
            .get_export_inner(identity, snapshot_ts, move |keys| {
                let key = match keys {
                    ExportObjectKeys::Zip(key) => key,
                    _ => anyhow::bail!(ErrorMetadata::bad_request(
                        "NoExportForZip",
                        "Expected export with zip object key"
                    )),
                };
                Ok(key)
            })
            .await?;
        let filename = format!(
            // This should match the format in SnapshotExport.tsx.
            "snapshot_{}_{snapshot_ts}.zip",
            self.instance_name
        );
        Ok((stream, filename))
    }

    pub async fn get_export(
        &self,
        identity: Identity,
        snapshot_ts: Timestamp,
        table_name: TableName,
    ) -> anyhow::Result<StorageGetStream> {
        self.get_export_inner(identity, snapshot_ts, move |keys| {
            let key = match keys {
                ExportObjectKeys::ByTable(tables) => tables
                    .get(&table_name)
                    .context(ErrorMetadata::bad_request(
                        "NoExportForTable",
                        format!(
                            "The requested export {snapshot_ts} does not have an export for \
                             {table_name}"
                        ),
                    ))?
                    .clone(),
                _ => anyhow::bail!(ErrorMetadata::bad_request(
                    "NoExportForTable",
                    "Expected export with tables"
                )),
            };
            Ok(key)
        })
        .await
    }

    async fn get_export_inner(
        &self,
        identity: Identity,
        snapshot_ts: Timestamp,
        get_object_key: impl FnOnce(ExportObjectKeys) -> anyhow::Result<ObjectKey>,
    ) -> anyhow::Result<StorageGetStream> {
        let object_key = {
            let mut tx = self.begin(identity).await?;
            let export_doc = ExportWorker::completed_export_at_ts(&mut tx, snapshot_ts).await?;
            let export: ParsedDocument<Export> = export_doc
                .context(ErrorMetadata::not_found(
                    "ExportNotFound",
                    format!("The requested export {snapshot_ts} was not found"),
                ))?
                .try_into()?;
            match export.into_value() {
                Export::Completed { object_keys, .. } => get_object_key(object_keys)?,
                Export::Failed { .. } | Export::InProgress { .. } | Export::Requested { .. } => {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "ExportNotComplete",
                        format!("The requested export {snapshot_ts} has not completed"),
                    ))
                },
            }
        };
        let storage_get_stream =
            self.exports_storage
                .get(&object_key)
                .await?
                .context(ErrorMetadata::not_found(
                    "ExportNotFound",
                    format!("The requested export {snapshot_ts}/{object_key:?} was not found"),
                ))?;
        Ok(storage_get_stream)
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

        anyhow::ensure!(
            TableModel::new(tx)
                .count(&ENVIRONMENT_VARIABLES_TABLE.clone())
                .await?
                <= (ENV_VAR_LIMIT as u64),
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
        anyhow::ensure!(
            environment_variables.len() as u64
                + TableModel::new(tx)
                    .count(&ENVIRONMENT_VARIABLES_TABLE.clone())
                    .await?
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
                    ));
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

    pub async fn analyze(
        &self,
        udf_config: UdfConfig,
        new_modules: Vec<ModuleConfig>,
        source_package: Option<SourcePackage>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        self.runner
            .analyze(udf_config, new_modules, source_package)
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
        let rng_seed = self.runtime().with_rng(|rng| rng.gen());
        let mut schema = self
            .runner()
            .evaluate_schema(schema.source, schema.source_map, rng_seed)
            .await?;

        for table_schema in schema.tables.values_mut() {
            for index_schema in table_schema.indexes.values_mut() {
                index_schema.fields =
                    self._validate_user_defined_index_fields(index_schema.fields.clone())?;
            }
        }

        schema.check_index_references()?;

        Ok(schema)
    }

    #[minitrace::trace]
    pub async fn get_evaluated_auth_config(
        runner: Arc<ApplicationFunctionRunner<RT>>,
        tx: &mut Transaction<RT>,
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
                tx,
                auth_config_module,
                "The pushed auth config is invalid",
            )
            .await?;
            Ok(auth_config.providers)
        } else {
            Ok(config.auth_info.clone().unwrap_or_default())
        }
    }

    // This is only relevant to auth config set via `auth.config.js`.
    // Because legacy setups didn't use `auth.config.js` we do not
    // reset the auth config if `auth.config.js` is not present.
    pub async fn reevaluate_existing_auth_config(
        runner: Arc<ApplicationFunctionRunner<RT>>,
        tx: &mut Transaction<RT>,
    ) -> anyhow::Result<()> {
        let all_modules = ModuleModel::new(tx).get_application_modules().await?;
        let auth_config_module = all_modules.get(&AUTH_CONFIG_FILE_NAME.parse().unwrap());
        if let Some(auth_config_module) = auth_config_module {
            let auth_config_module = auth_config_module.clone();
            let auth_config = Self::evaluate_auth_config(
                runner,
                tx,
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
        tx: &mut Transaction<RT>,
        auth_config_module: ModuleConfig,
        explanation: &str,
    ) -> anyhow::Result<AuthConfig> {
        let auth_config = runner
            .evaluate_auth_config(
                auth_config_module.source,
                auth_config_module.source_map,
                EnvironmentVariablesModel::new(tx).get_all().await?,
            )
            .await
            .map_err(|error| {
                let error = error.to_string();
                if error.starts_with("Uncaught Error: Environment variable") {
                    // Reformatting the underlying message to be nicer
                    // here. Since we lost the underlying ErrorMetadata into the JSError,
                    // we do some string matching instead. CX-4531
                    ErrorMetadata::bad_request(
                        "AuthConfigMissingEnvironmentVariable",
                        error.trim_start_matches("Uncaught Error: ").to_string(),
                    )
                } else {
                    ErrorMetadata::bad_request(
                        "InvalidAuthConfig",
                        format!("{explanation}: {error}"),
                    )
                }
            })?;
        Ok(auth_config)
    }

    #[minitrace::trace]
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

    #[minitrace::trace]
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
                parse_schema_id(&schema_id, tx.table_mapping())
                    .context(invalid_schema_id(&schema_id))
            })
            .transpose()?;

        let auth_providers =
            Self::get_evaluated_auth_config(runner, tx, auth_module, &config_file).await?;

        let config_metadata = ConfigMetadata::from_file(config_file, auth_providers);

        let (config_diff, schema) = ConfigModel::new(tx)
            .apply(
                config_metadata.clone(),
                modules,
                udf_config,
                source_package,
                analyze_results,
                schema_id,
            )
            .await?;

        Ok((
            ConfigMetadataAndSchema {
                config_metadata,
                schema,
            },
            vec![DeploymentAuditLogEvent::PushConfig { config_diff }],
        ))
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
        upload_token: ClientDrivenUploadToken,
        part_tokens: Vec<ClientDrivenUploadPartToken>,
    ) -> anyhow::Result<DocumentIdV6> {
        if !identity.is_admin() {
            anyhow::bail!(ErrorMetadata::forbidden(
                "InvalidImport",
                "Only an admin of the deployment can import"
            ));
        }
        let object_key = self
            .snapshot_imports_storage
            .finish_client_driven_upload(upload_token, part_tokens)
            .await?;
        store_uploaded_import(self, identity, format, mode, object_key).await
    }

    pub async fn upload_snapshot_import(
        &self,
        body_stream: BoxStream<'_, anyhow::Result<Bytes>>,
    ) -> anyhow::Result<ObjectKey> {
        let mut upload: Box<BufferedUpload> = self.snapshot_imports_storage.start_upload().await?;
        // unclear why this reassignment is necessary
        let mut body_stream = body_stream;
        upload.try_write_parallel(&mut body_stream).await?;
        drop(body_stream);
        let object_key = upload.complete().await?;
        Ok(object_key)
    }

    #[minitrace::trace]
    pub async fn upload_package(
        &self,
        modules: &Vec<ModuleConfig>,
        external_deps_id_and_pkg: Option<(ExternalDepsPackageId, ExternalDepsPackage)>,
    ) -> anyhow::Result<Option<SourcePackage>> {
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
            self.modules_storage.clone(),
            external_deps_pkg.map(|pkg| pkg.storage_key),
        )
        .await?;

        tracing::info!("Upload of {storage_key:?} successful");
        tracing::info!("Source package size: {}", package_size);
        log_source_package_size_bytes_total(package_size);

        Ok(Some(SourcePackage {
            storage_key,
            sha256,
            external_deps_package_id,
            package_size,
        }))
    }

    // Clear all records for specified tables concurrently, potentially taking
    // multiple transactions for each. Returns the total number of documents
    // deleted.
    pub async fn clear_tables(
        &self,
        identity: &Identity,
        table_names: Vec<TableName>,
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
    ) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
        let block_logging = self
            .log_visibility
            .should_redact_logs_and_error(
                &mut self.begin(identity.clone()).await?,
                identity.clone(),
                AllowedVisibility::All,
            )
            .await?;

        let mut tx = self.begin(identity.clone()).await?;

        // Use the last pushed version. If there hasn't been a push
        // yet, act like the most recent version.
        let server_version = UdfConfigModel::new(&mut tx)
            .get()
            .await?
            .map(|udf_config| udf_config.server_version.clone())
            .unwrap_or_else(|| Version::parse("1000.0.0").unwrap());

        // 1. analyze the module
        // We can analyze this module by itself, without combining it with the existing
        // modules since this module should be self-contained and not import
        // from other modules.
        let udf_config = UdfConfig {
            server_version,
            import_phase_rng_seed: self.runtime.with_rng(|rng| rng.gen()),
            import_phase_unix_timestamp: self.runtime.unix_timestamp(),
        };

        let module_path = module.path.clone().canonicalize();
        let analyze_results = self
            .analyze(udf_config.clone(), vec![module.clone()], None)
            .await?
            .map_err(|js_error| {
                let metadata = ErrorMetadata::bad_request(
                    "InvalidModules",
                    format!("Could not analyze the given module:\n{js_error}"),
                );
                anyhow::anyhow!(js_error).context(metadata)
            })?;

        let analyzed_module = analyze_results
            .get(&module_path)
            .ok_or_else(|| anyhow::anyhow!("Unexpectedly missing analyze result"))?
            .clone();

        // 2. get the function type
        let mut analyzed_function = None;
        for function in &analyzed_module.functions {
            if function.name.as_ref() == "default" {
                analyzed_function = Some(function.clone());
            } else {
                anyhow::bail!("Only `export default` is supported.");
            }
        }
        let analyzed_function = analyzed_function.context("Missing default export.")?;

        // 3. Add the module
        ModuleModel::new(&mut tx)
            .put(
                module_path.clone(),
                module.source,
                None,
                module.source_map,
                Some(analyzed_module),
                ModuleEnvironment::Isolate,
            )
            .await?;

        // 4. run the function within the transaction
        let path = CanonicalizedUdfPath::new(module_path, "default".to_owned());
        let arguments = parse_udf_args(&path.clone().into(), args)?;
        let (result, log_lines) = match analyzed_function.udf_type {
            UdfType::Query => self
                .runner
                .run_query_without_caching(
                    request_id.clone(),
                    tx,
                    path,
                    arguments,
                    AllowedVisibility::All,
                    caller,
                )
                .await
                .map(
                    |UdfOutcome {
                         result, log_lines, ..
                     }| { (result, log_lines) },
                ),
            UdfType::Mutation => {
                anyhow::bail!("Mutations are not supported in the REPL yet.")
            },
            UdfType::Action => {
                anyhow::bail!("Actions are not supported in the REPL yet.")
            },
            UdfType::HttpAction => {
                anyhow::bail!(
                    "HTTP actions are not supported in the REPL. A \"not found\" message should \
                     be returned instead."
                )
            },
        }?;
        let log_lines = RedactedLogLines::from_log_lines(log_lines, block_logging);
        Ok(match result {
            Ok(value) => Ok(FunctionReturn {
                value: value.unpack(),
                log_lines,
            }),
            Err(error) => Err(FunctionError {
                error: RedactedJsError::from_js_error(error, block_logging, request_id),
                log_lines,
            }),
        })
    }

    #[minitrace::trace]
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

    #[minitrace::trace]
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
    ) -> anyhow::Result<u64> {
        let mut tx = self.begin(identity.clone()).await?;
        let mut count = 0;
        for table_name in table_names {
            anyhow::ensure!(
                !table_name.is_system(),
                "cannot delete system table {table_name}"
            );
            let mut table_model = TableModel::new(&mut tx);
            count += table_model.count(&table_name).await?;
            table_model.delete_table(table_name).await?;
        }
        self.commit(tx, "delete_tables").await?;
        Ok(count)
    }

    /// Add system indexes if they do not already exist and update
    /// existing indexes if needed.
    pub async fn _add_system_indexes(
        &self,
        identity: &Identity,
        indexes: BTreeMap<IndexName, IndexedFields>,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity.clone()).await?;
        for (index_name, index_fields) in indexes.into_iter() {
            let index_fields = self._validate_user_defined_index_fields(index_fields)?;
            let index_metadata =
                IndexMetadata::new_backfilling(*tx.begin_timestamp(), index_name, index_fields);
            let mut model = IndexModel::new(&mut tx);
            if let Some(existing_index_metadata) = model
                .pending_index_metadata(&index_metadata.name)?
                .or(model.enabled_index_metadata(&index_metadata.name)?)
            {
                if !index_metadata
                    .config
                    .same_config(&existing_index_metadata.config)
                {
                    IndexModel::new(&mut tx)
                        .drop_index(existing_index_metadata.id())
                        .await?;
                    IndexModel::new(&mut tx)
                        .add_system_index(index_metadata)
                        .await?;
                }
            } else {
                IndexModel::new(&mut tx)
                    .add_system_index(index_metadata)
                    .await?;
            }
        }
        self.commit(tx, "add_system_indexes").await?;
        Ok(())
    }

    pub async fn store_file(
        &self,
        content_length: Option<ContentLength>,
        content_type: Option<ContentType>,
        expected_sha256: Option<Sha256Digest>,
        body: impl Stream<Item = anyhow::Result<Bytes>> + Send,
    ) -> anyhow::Result<DocumentIdV6> {
        let storage_id = self
            .file_storage
            .store_file(
                content_length,
                content_type,
                body,
                expected_sha256,
                &self.usage_tracking,
            )
            .await?;
        Ok(storage_id)
    }

    pub async fn get_file(&self, storage_id: FileStorageId) -> anyhow::Result<FileStream> {
        let mut file_storage_tx = self.begin(Identity::system()).await?;

        let Some(file_entry) = self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_entry(&mut file_storage_tx, storage_id.clone())
            .await?
        else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("File {storage_id} not found"),
            )
            .into());
        };

        self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_stream(file_entry, self.usage_tracking.clone())
            .await
    }

    pub async fn get_file_range(
        &self,
        storage_id: FileStorageId,
        bytes_range: (Bound<u64>, Bound<u64>),
    ) -> anyhow::Result<FileRangeStream> {
        let mut file_storage_tx = self.begin(Identity::system()).await?;

        let Some(file_entry) = self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_entry(&mut file_storage_tx, storage_id.clone())
            .await?
        else {
            return Err(ErrorMetadata::not_found(
                "FileNotFound",
                format!("File {storage_id} not found"),
            )
            .into());
        };

        self
            .file_storage
            .transactional_file_storage
            // The transaction is not part of UDF so use the global usage counters.
            .get_file_range_stream(file_entry, bytes_range, self.usage_tracking.clone())
            .await
    }

    pub async fn authenticate(
        &self,
        token: AuthenticationToken,
        system_time: SystemTime,
    ) -> anyhow::Result<Identity> {
        let identity = match token {
            AuthenticationToken::Admin(token, acting_as) => {
                let admin_identity = self.key_broker().check_admin_key(&token).context(
                    ErrorMetadata::unauthenticated(
                        "BadAdminKey",
                        "The provided admin key was invalid for this instance",
                    ),
                )?;

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

                let identity = validate_id_token(
                    Auth0IdToken(id_token),
                    cached_http_client,
                    auth_infos
                        .into_iter()
                        .map(|auth_info| auth_info.into_value())
                        .collect(),
                    system_time,
                )
                .await?;
                Identity::user(identity)
            },
            AuthenticationToken::None => Identity::Unknown,
        };
        Ok(identity)
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

    pub async fn cancel_all_jobs(
        &self,
        udf_path: Option<String>,
        identity: Identity,
    ) -> anyhow::Result<()> {
        loop {
            let count = self
                .execute_with_audit_log_events_and_occ_retries(
                    identity.clone(),
                    "application_cancel_all_jobs",
                    |tx| {
                        Self::_cancel_all_jobs(tx, udf_path.clone(), *MAX_JOBS_CANCEL_BATCH).into()
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
        udf_path: Option<String>,
        max_jobs: usize,
    ) -> anyhow::Result<(usize, Vec<DeploymentAuditLogEvent>)> {
        let count = SchedulerModel::new(tx)
            .cancel_all(udf_path.clone(), max_jobs)
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

        self.log_sender.send_logs(logs);
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
        ) -> ShortBoxFuture<
            '_,
            'b,
            'c,
            anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>,
        >,
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
        ) -> ShortBoxFuture<
            '_,
            'a,
            'b,
            anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>,
        >,
    {
        self.execute_with_audit_log_events_and_occ_retries_with_pause_client(
            identity,
            PauseClient::new(),
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
        ) -> ShortBoxFuture<
            '_,
            'a,
            'b,
            anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>,
        >,
    {
        self.execute_with_audit_log_events_and_occ_retries_with_pause_client(
            identity,
            PauseClient::new(),
            write_source,
            f,
        )
        .await
    }

    pub async fn execute_with_audit_log_events_and_occ_retries_with_pause_client<'a, F, T>(
        &self,
        identity: Identity,
        pause_client: PauseClient,
        write_source: impl Into<WriteSource>,
        f: F,
    ) -> anyhow::Result<(T, OccRetryStats)>
    where
        F: Send + Sync,
        T: Send + 'static,
        F: for<'b> Fn(
            &'b mut Transaction<RT>,
        ) -> ShortBoxFuture<
            '_,
            'a,
            'b,
            anyhow::Result<(T, Vec<DeploymentAuditLogEvent>)>,
        >,
    {
        let db = self.database.clone();
        let (ts, (t, events), stats) = db
            .execute_with_occ_retries(
                identity,
                FunctionUsageTracker::new(),
                pause_client,
                write_source,
                |tx| Self::insert_deployment_audit_log_events(tx, &f).into(),
            )
            .await?;
        // Send deployment audit logs
        // TODO CX-5139 Remove this when audit logs are being processed in LogManager.
        let logs = events
            .into_iter()
            .map(|event| {
                DeploymentAuditLogEvent::to_log_event(event, UnixTimestamp::from_nanos(ts.into()))
            })
            .try_collect()?;

        self.log_sender.send_logs(logs);
        Ok((t, stats))
    }

    pub async fn execute_with_occ_retries<'a, T, F>(
        &'a self,
        identity: Identity,
        usage: FunctionUsageTracker,
        pause_client: PauseClient,
        write_source: impl Into<WriteSource>,
        f: F,
    ) -> anyhow::Result<(Timestamp, T)>
    where
        F: Send + Sync,
        T: Send + 'static,
        F: for<'b> Fn(&'b mut Transaction<RT>) -> ShortBoxFuture<'_, 'a, 'b, anyhow::Result<T>>,
    {
        self.database
            .execute_with_occ_retries(identity, usage, pause_client, write_source, f)
            .await
            .map(|(ts, t, _)| (ts, t))
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.log_sender.shutdown()?;
        self.table_summary_worker.shutdown().await?;
        self.schema_worker.lock().shutdown();
        self.index_worker.lock().shutdown();
        self.search_worker.lock().shutdown();
        self.search_and_vector_bootstrap_worker.lock().shutdown();
        self.export_worker.lock().shutdown();
        self.snapshot_import_worker.lock().shutdown();
        self.runner.shutdown().await?;
        self.scheduled_job_runner.shutdown();
        self.cron_job_executor.lock().shutdown();
        self.module_cache.shutdown();
        self.database.shutdown().await?;
        tracing::info!("Application shut down");
        Ok(())
    }
}

// Newer clients get a clean export in JSONL format
static MAX_UDF_SERVER_VERSION_WITHOUT_CLEAN_EXPORT: LazyLock<Version> =
    LazyLock::new(|| Version::parse("1.3.999").unwrap());
