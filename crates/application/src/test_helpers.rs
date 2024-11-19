use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::Path,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use authentication::{
    access_token_auth::NullAccessTokenAuth,
    application_auth::ApplicationAuth,
};
use cmd_util::env::config_test;
use common::{
    bootstrap_model::index::database_index::IndexedFields,
    components::ComponentId,
    db_schema,
    http::fetch::StaticFetchClient,
    knobs::ACTION_USER_TIMEOUT,
    log_streaming::NoopLogSender,
    pause::{
        PauseClient,
        PauseController,
    },
    persistence::Persistence,
    runtime::Runtime,
    testing::TestPersistence,
    types::{
        ConvexOrigin,
        FullyQualifiedObjectKey,
    },
};
use database::{
    Database,
    SchemaModel,
    ShutdownSignal,
    Transaction,
};
use events::usage::{
    NoOpUsageEventLogger,
    UsageEventLogger,
};
use file_storage::{
    FileStorage,
    TransactionalFileStorage,
};
use function_runner::server::{
    InProcessFunctionRunner,
    InstanceStorage,
};
use isolate::{
    bundled_js::OUT_DIR,
    test_helpers::{
        TEST_SOURCE,
        TEST_SOURCE_ISOLATE_ONLY,
    },
};
use keybroker::{
    Identity,
    KeyBroker,
    DEV_INSTANCE_NAME,
    DEV_SECRET,
};
use model::{
    config::{
        types::ConfigMetadata,
        ConfigModel,
    },
    cron_jobs::types::CronJob,
    exports::{
        types::{
            Export,
            ExportFormat,
            ExportRequestor,
        },
        ExportsModel,
    },
    initialize_application_system_tables,
    scheduled_jobs::types::ScheduledJob,
    udf_config::types::UdfConfig,
    virtual_system_mapping,
};
use node_executor::{
    local::LocalNodeExecutor,
    Actions,
};
use storage::{
    LocalDirStorage,
    Storage,
    StorageUseCase,
};
use value::{
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    cron_jobs::CronJobExecutor,
    deploy_config::{
        SchemaStatus,
        StartPushRequest,
    },
    log_visibility::AllowLogging,
    scheduled_jobs::{
        ScheduledJobExecutor,
        SCHEDULED_JOB_COMMITTING,
        SCHEDULED_JOB_EXECUTED,
    },
    Application,
};

pub static OBJECTS_TABLE: LazyLock<TableName> = LazyLock::new(|| "objects".parse().unwrap());
pub static OBJECTS_TABLE_COMPONENT: ComponentId = ComponentId::test_user();

#[derive(Default)]
pub struct ApplicationFixtureArgs {
    pub tp: Option<TestPersistence>,
    pub snapshot_import_pause_client: Option<PauseClient>,
    pub scheduled_jobs_pause_client: PauseClient,
    pub event_logger: Option<Arc<dyn UsageEventLogger>>,
}

impl ApplicationFixtureArgs {
    pub fn with_scheduled_jobs_pause_client() -> (Self, PauseController) {
        let (pause_controller, pause_client) = PauseController::new(vec![SCHEDULED_JOB_EXECUTED]);
        let args = ApplicationFixtureArgs {
            scheduled_jobs_pause_client: pause_client,
            ..Default::default()
        };
        (args, pause_controller)
    }

    pub fn with_scheduled_jobs_fault_client() -> (Self, PauseController) {
        let (pause_controller, pause_client) =
            PauseController::new(vec![SCHEDULED_JOB_COMMITTING, SCHEDULED_JOB_EXECUTED]);
        let args = ApplicationFixtureArgs {
            scheduled_jobs_pause_client: pause_client,
            ..Default::default()
        };
        (args, pause_controller)
    }

    pub fn with_event_logger(event_logger: Arc<dyn UsageEventLogger>) -> Self {
        Self {
            event_logger: Some(event_logger),
            ..Default::default()
        }
    }
}

#[async_trait]
pub trait ApplicationTestExt<RT: Runtime> {
    async fn new_for_tests(rt: &RT) -> anyhow::Result<Application<RT>>;
    async fn new_for_tests_with_args(
        rt: &RT,
        args: ApplicationFixtureArgs,
    ) -> anyhow::Result<Application<RT>>;
    async fn test_one_off_scheduled_job_executor_run(
        &self,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<()>;
    /// Load the modules from npm-packages/udf-tests
    async fn load_udf_tests_modules(&self) -> anyhow::Result<()>;
    async fn load_udf_tests_modules_with_node(&self) -> anyhow::Result<()>;
    /// Load the modules form npm-packages/component-tests
    async fn load_component_tests_modules(&self, layout: &str) -> anyhow::Result<()>;
    async fn test_one_off_cron_job_executor_run(
        &self,
        job: CronJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<()>;
    fn validate_user_defined_index_fields(
        &self,
        fields: IndexedFields,
    ) -> anyhow::Result<IndexedFields>;
    fn database(&self) -> &Database<RT>;
    fn snapshot_imports_storage(&self) -> Arc<dyn Storage>;
    fn exports_storage(&self) -> Arc<dyn Storage>;
    async fn export_and_wait(&self) -> anyhow::Result<FullyQualifiedObjectKey>;
}

#[async_trait]
impl<RT: Runtime> ApplicationTestExt<RT> for Application<RT> {
    async fn new_for_tests(rt: &RT) -> anyhow::Result<Application<RT>> {
        Self::new_for_tests_with_args(rt, Default::default()).await
    }

    async fn new_for_tests_with_args(
        rt: &RT,
        args: ApplicationFixtureArgs,
    ) -> anyhow::Result<Application<RT>> {
        config_test();
        let kb = KeyBroker::dev();
        let storage_dir = tempfile::TempDir::new()?;
        let convex_origin: ConvexOrigin = "http://127.0.0.1:8000".into();
        let convex_site = "http://127.0.0.1:8001".into();
        let searcher = Arc::new(search::searcher::SearcherStub {});
        let segment_term_metadata_fetcher = Arc::new(search::searcher::SearcherStub {});
        let persistence = args.tp.unwrap_or_else(TestPersistence::new);
        let snapshot_import_pause_client = args.snapshot_import_pause_client.unwrap_or_default();
        let database = Database::load(
            Arc::new(persistence.clone()),
            rt.clone(),
            searcher.clone(),
            ShutdownSignal::panic(),
            virtual_system_mapping(),
            args.event_logger.unwrap_or(Arc::new(NoOpUsageEventLogger)),
        )
        .await?;
        initialize_application_system_tables(&database).await?;
        let files_storage = Arc::new(LocalDirStorage::for_use_case(
            rt.clone(),
            &storage_dir.path().to_string_lossy(),
            StorageUseCase::Files,
        )?);
        let modules_storage = Arc::new(LocalDirStorage::for_use_case(
            rt.clone(),
            &storage_dir.path().to_string_lossy(),
            StorageUseCase::Modules,
        )?);
        let search_storage = Arc::new(LocalDirStorage::for_use_case(
            rt.clone(),
            &storage_dir.path().to_string_lossy(),
            StorageUseCase::SearchIndexes,
        )?);
        let exports_storage = Arc::new(LocalDirStorage::for_use_case(
            rt.clone(),
            &storage_dir.path().to_string_lossy(),
            StorageUseCase::Exports,
        )?);
        let snapshot_imports_storage = Arc::new(LocalDirStorage::for_use_case(
            rt.clone(),
            &storage_dir.path().to_string_lossy(),
            StorageUseCase::SnapshotImports,
        )?);

        let fetch_client = Arc::new(StaticFetchClient::new());
        let function_runner = Arc::new(
            InProcessFunctionRunner::new(
                DEV_INSTANCE_NAME.into(),
                DEV_SECRET.try_into()?,
                convex_origin.clone(),
                rt.clone(),
                persistence.reader(),
                InstanceStorage {
                    files_storage: files_storage.clone(),
                    modules_storage: modules_storage.clone(),
                },
                database.clone(),
                fetch_client,
            )
            .await?,
        );

        let file_storage = FileStorage {
            transactional_file_storage: TransactionalFileStorage::new(
                rt.clone(),
                files_storage.clone(),
                convex_origin.clone(),
            ),
            database: database.clone(),
        };

        let node_process_timeout = *ACTION_USER_TIMEOUT + Duration::from_secs(5);
        let node_executor = Arc::new(LocalNodeExecutor::new(node_process_timeout)?);
        let actions = Actions::new(node_executor, convex_origin.clone(), *ACTION_USER_TIMEOUT);

        let application = Application::new(
            rt.clone(),
            database.clone(),
            file_storage.clone(),
            files_storage.clone(),
            modules_storage.clone(),
            search_storage.clone(),
            exports_storage.clone(),
            snapshot_imports_storage.clone(),
            database.usage_counter(),
            kb.clone(),
            DEV_INSTANCE_NAME.into(),
            function_runner,
            convex_origin,
            convex_site,
            searcher,
            segment_term_metadata_fetcher,
            Arc::new(persistence.clone()),
            actions,
            Arc::new(NoopLogSender),
            Arc::new(AllowLogging),
            snapshot_import_pause_client,
            args.scheduled_jobs_pause_client,
            Arc::new(ApplicationAuth::new(
                kb.clone(),
                Arc::new(NullAccessTokenAuth),
            )),
        )
        .await?;

        Ok(application)
    }

    fn snapshot_imports_storage(&self) -> Arc<dyn Storage> {
        self.snapshot_imports_storage.clone()
    }

    fn exports_storage(&self) -> Arc<dyn Storage> {
        self.exports_storage.clone()
    }

    async fn test_one_off_scheduled_job_executor_run(
        &self,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        let test_executor = ScheduledJobExecutor::new(
            self.runtime.clone(),
            DEV_INSTANCE_NAME.into(),
            self.database.clone(),
            self.runner.clone(),
            self.function_log.clone(),
        );
        test_executor.execute_job(job, job_id).await;
        Ok(())
    }

    async fn test_one_off_cron_job_executor_run(
        &self,
        job: CronJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        let test_executor = CronJobExecutor::new(
            self.runtime.clone(),
            DEV_INSTANCE_NAME.into(),
            self.database.clone(),
            self.runner.clone(),
            self.function_log.clone(),
        );
        test_executor.execute_job(job, job_id).await;
        Ok(())
    }

    async fn load_udf_tests_modules(&self) -> anyhow::Result<()> {
        self.load_udf_tests_modules_inner(false).await
    }

    async fn load_udf_tests_modules_with_node(&self) -> anyhow::Result<()> {
        self.load_udf_tests_modules_inner(true).await
    }

    async fn load_component_tests_modules(&self, layout: &str) -> anyhow::Result<()> {
        let request = Self::load_start_push_request(Path::new(layout))?;
        let dry_run = request.dry_run;
        let config = request.into_project_config()?;
        let start_push = self.start_push(&config, dry_run).await?;
        loop {
            let schema_status = self
                .wait_for_schema(
                    Identity::system(),
                    start_push.schema_change.clone(),
                    Duration::from_secs(10),
                )
                .await?;
            match schema_status {
                SchemaStatus::InProgress { .. } => continue,
                SchemaStatus::Complete => break,
                _ => anyhow::bail!("Unexpected schema status: {schema_status:?}"),
            }
        }
        self.finish_push(Identity::system(), start_push).await?;
        Ok(())
    }

    fn validate_user_defined_index_fields(
        &self,
        fields: IndexedFields,
    ) -> anyhow::Result<IndexedFields> {
        self._validate_user_defined_index_fields(fields)
    }

    fn database(&self) -> &Database<RT> {
        &self.database
    }

    async fn export_and_wait(&self) -> anyhow::Result<FullyQualifiedObjectKey> {
        let export_id = self
            .request_export(
                Identity::system(),
                ExportFormat::Zip {
                    include_storage: true,
                },
                ComponentId::Root,
                ExportRequestor::CloudBackup,
                None,
            )
            .await?;
        let export_object_key = loop {
            let mut tx = self.begin(Identity::system()).await?;
            let export_doc = ExportsModel::new(&mut tx)
                .get(export_id)
                .await?
                .context("Missing?")?
                .into_value();
            let Export::Completed { zip_object_key, .. } = export_doc else {
                continue;
            };
            break zip_object_key;
        };
        Ok(self.cloud_export_key(export_object_key))
    }
}

impl<RT: Runtime> Application<RT> {
    fn load_start_push_request(layout_path: &Path) -> anyhow::Result<StartPushRequest> {
        let path = Path::new(OUT_DIR)
            .join(layout_path)
            .join("start_push_request.json");
        let mut file = File::open(path)?;
        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        let output: StartPushRequest = serde_json::from_slice(&contents)?;
        Ok(output)
    }

    async fn load_udf_tests_modules_inner(&self, include_node: bool) -> anyhow::Result<()> {
        let test_source = if include_node {
            TEST_SOURCE.clone()
        } else {
            TEST_SOURCE_ISOLATE_ONLY.clone()
        };
        let mut tx = self.begin(Identity::system()).await?;
        let udf_config = UdfConfig::new_for_test(&self.runtime(), "1000.0.0".parse()?);
        // TODO(rakeeb): add external packages to udf test modules
        let source_package = self.upload_package(&test_source, None).await?;
        let analyze_results = self
            .analyze(
                udf_config.clone(),
                test_source.clone(),
                source_package.clone(),
                BTreeMap::new(),
            )
            .await??;
        let schema_id = insert_validated_schema(&mut tx).await?;

        ConfigModel::new(&mut tx, ComponentId::test_user())
            .apply(
                ConfigMetadata::new(),
                test_source.clone(),
                udf_config,
                Some(source_package),
                analyze_results,
                Some(schema_id),
            )
            .await?;
        self.commit_test(tx).await?;
        Ok(())
    }
}

// The contents of the schema are irrelevant for the modules, but we need one to
// be present for the rest of apply_config.
async fn insert_validated_schema<RT: Runtime>(
    tx: &mut Transaction<RT>,
) -> anyhow::Result<ResolvedDocumentId> {
    let schema = db_schema!();
    let mut model = SchemaModel::new(tx, TableNamespace::test_user());
    let (schema_id, _) = model.submit_pending(schema).await?;
    model.mark_validated(schema_id).await?;
    Ok(schema_id)
}
