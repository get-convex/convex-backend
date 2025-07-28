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
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    components::ComponentId,
    db_schema,
    http::fetch::StaticFetchClient,
    knobs::{
        ACTION_USER_TIMEOUT,
        UDF_CACHE_MAX_SIZE,
    },
    persistence::Persistence,
    runtime::Runtime,
    shutdown::ShutdownSignal,
    testing::TestPersistence,
    types::{
        ConvexOrigin,
        FullyQualifiedObjectKey,
    },
};
use database::{
    Database,
    IndexModel,
    SchemaModel,
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
use function_runner::{
    in_process_function_runner::InProcessFunctionRunner,
    server::InstanceStorage,
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
    database_globals::types::StorageTagInitializer,
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
    noop::NoopNodeExecutor,
    Actions,
    NodeExecutor,
};
use storage::Storage;
use value::{
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    cache::QueryCache,
    cron_jobs::CronJobContext,
    deploy_config::{
        FinishPushDiff,
        SchemaStatus,
        StartPushRequest,
    },
    log_visibility::RedactLogsToClient,
    scheduled_jobs::ScheduledJobContext,
    Application,
};

pub static OBJECTS_TABLE: LazyLock<TableName> = LazyLock::new(|| "objects".parse().unwrap());
pub static OBJECTS_TABLE_COMPONENT: ComponentId = ComponentId::test_user();

#[derive(Default)]
pub struct ApplicationFixtureArgs {
    pub tp: Option<TestPersistence>,
    pub event_logger: Option<Arc<dyn UsageEventLogger>>,
    pub node_executor: Option<Arc<dyn NodeExecutor>>,
}

impl ApplicationFixtureArgs {
    pub fn with_event_logger(event_logger: Arc<dyn UsageEventLogger>) -> Self {
        Self {
            event_logger: Some(event_logger),
            ..Default::default()
        }
    }

    pub fn with_node_executor(node_executor: Arc<dyn NodeExecutor>) -> Self {
        Self {
            node_executor: Some(node_executor),
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
    async fn run_test_push(&self, request: StartPushRequest) -> anyhow::Result<FinishPushDiff>;

    async fn test_one_off_cron_job_executor_run(&self, job: CronJob) -> anyhow::Result<()>;
    fn validate_user_defined_index_fields(
        &self,
        fields: IndexedFields,
    ) -> anyhow::Result<IndexedFields>;
    fn database(&self) -> &Database<RT>;
    fn snapshot_imports_storage(&self) -> Arc<dyn Storage>;
    fn exports_storage(&self) -> Arc<dyn Storage>;
    async fn export_and_wait(&self) -> anyhow::Result<FullyQualifiedObjectKey>;

    async fn add_index(
        &self,
        index: IndexMetadata<TableName>,
    ) -> anyhow::Result<ResolvedDocumentId>;
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
        let database = Database::load(
            Arc::new(persistence.clone()),
            rt.clone(),
            searcher.clone(),
            ShutdownSignal::panic(),
            virtual_system_mapping().clone(),
            args.event_logger.unwrap_or(Arc::new(NoOpUsageEventLogger)),
        )
        .await?;
        initialize_application_system_tables(&database).await?;
        let application_storage = Application::initialize_storage(
            rt.clone(),
            &database,
            StorageTagInitializer::Local {
                dir: storage_dir.path().to_path_buf(),
            },
            DEV_INSTANCE_NAME.into(),
        )
        .await?;

        let fetch_client = Arc::new(StaticFetchClient::new());
        let function_runner = Arc::new(
            InProcessFunctionRunner::new(
                DEV_INSTANCE_NAME.into(),
                DEV_SECRET.try_into()?,
                convex_origin.clone(),
                rt.clone(),
                persistence.reader(),
                InstanceStorage {
                    files_storage: application_storage.files_storage.clone(),
                    modules_storage: application_storage.modules_storage.clone(),
                },
                database.clone(),
                fetch_client.clone(),
            )
            .await?,
        );

        let file_storage = FileStorage {
            transactional_file_storage: TransactionalFileStorage::new(
                rt.clone(),
                application_storage.files_storage.clone(),
                convex_origin.clone(),
            ),
            database: database.clone(),
        };

        let node_executor = args
            .node_executor
            .unwrap_or_else(|| Arc::new(NoopNodeExecutor::new()));
        let actions = Actions::new(
            node_executor,
            convex_origin.clone(),
            *ACTION_USER_TIMEOUT,
            rt.clone(),
        );

        let application = Application::new(
            rt.clone(),
            database.clone(),
            file_storage.clone(),
            application_storage,
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
            Arc::new(RedactLogsToClient::new(false)),
            Arc::new(ApplicationAuth::new(
                kb.clone(),
                Arc::new(NullAccessTokenAuth),
            )),
            QueryCache::new(*UDF_CACHE_MAX_SIZE),
            fetch_client,
            None, // local_log_sink
            ShutdownSignal::panic(),
        )
        .await?;

        Ok(application)
    }

    fn snapshot_imports_storage(&self) -> Arc<dyn Storage> {
        self.application_storage.snapshot_imports_storage.clone()
    }

    fn exports_storage(&self) -> Arc<dyn Storage> {
        self.application_storage.exports_storage.clone()
    }

    async fn test_one_off_scheduled_job_executor_run(
        &self,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<()> {
        let test_executor = ScheduledJobContext::new(
            self.runtime.clone(),
            self.database.clone(),
            self.runner.clone(),
            self.function_log.clone(),
        );
        test_executor.execute_job(job, job_id).await;
        Ok(())
    }

    async fn test_one_off_cron_job_executor_run(&self, job: CronJob) -> anyhow::Result<()> {
        let test_executor = CronJobContext::new(
            self.runtime.clone(),
            self.database.clone(),
            self.runner.clone(),
            self.function_log.clone(),
        );
        test_executor.execute_job(job).await;
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
        self.run_test_push(request).await?;
        Ok(())
    }

    async fn run_test_push(&self, request: StartPushRequest) -> anyhow::Result<FinishPushDiff> {
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
        let diff = self.finish_push(Identity::system(), start_push).await?;
        Ok(diff)
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

    async fn add_index(
        &self,
        index: IndexMetadata<TableName>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let mut tx = self.begin(Identity::system()).await?;
        let index_id = IndexModel::new(&mut tx)
            .add_application_index(TableNamespace::test_user(), index)
            .await?;
        self.commit_test(tx).await?;
        Ok(index_id)
    }
}

impl<RT: Runtime> Application<RT> {
    pub fn load_start_push_request(layout_path: &Path) -> anyhow::Result<StartPushRequest> {
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
