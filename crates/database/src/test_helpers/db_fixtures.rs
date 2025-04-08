use std::sync::Arc;

use common::{
    persistence::Persistence,
    runtime::Runtime,
    shutdown::ShutdownSignal,
    testing::TestPersistence,
    virtual_system_mapping::VirtualSystemMapping,
};
use events::testing::TestUsageEventLogger;
use search::{
    searcher::{
        InProcessSearcher,
        SearcherStub,
    },
    Searcher,
};
use storage::{
    LocalDirStorage,
    Storage,
};

use crate::{
    text_index_worker::BuildTextIndexArgs,
    Database,
    Transaction,
};

pub struct DbFixtures<RT: Runtime> {
    pub tp: Arc<dyn Persistence>,
    pub db: Database<RT>,
    pub searcher: Arc<dyn Searcher>,
    pub search_storage: Arc<dyn Storage>,
    pub build_index_args: BuildTextIndexArgs,
    pub test_usage_logger: TestUsageEventLogger,
}

#[derive(Clone)]
pub struct DbFixturesArgs {
    pub tp: Option<Arc<dyn Persistence>>,
    pub searcher: Option<Arc<dyn Searcher>>,
    pub search_storage: Option<Arc<dyn Storage>>,
    pub virtual_system_mapping: VirtualSystemMapping,
    pub bootstrap_search_and_vector_indexes: bool,
    pub bootstrap_table_summaries: bool,
}

impl Default for DbFixturesArgs {
    fn default() -> Self {
        Self {
            tp: None,
            searcher: None,
            search_storage: None,
            virtual_system_mapping: Default::default(),
            bootstrap_search_and_vector_indexes: true,
            bootstrap_table_summaries: true,
        }
    }
}

impl<RT: Runtime> DbFixtures<RT> {
    pub async fn new(rt: &RT) -> anyhow::Result<Self> {
        Self::new_with_args(rt, DbFixturesArgs::default()).await
    }

    pub async fn new_with_args(
        rt: &RT,
        DbFixturesArgs {
            tp,
            searcher,
            search_storage,
            virtual_system_mapping,
            bootstrap_search_and_vector_indexes,
            bootstrap_table_summaries,
        }: DbFixturesArgs,
    ) -> anyhow::Result<Self> {
        let tp = tp.unwrap_or_else(|| Arc::new(TestPersistence::new()));
        let searcher = searcher.unwrap_or_else(|| Arc::new(SearcherStub {}));
        let search_storage = match search_storage {
            Some(ss) => ss,
            None => Arc::new(LocalDirStorage::new(rt.clone())?),
        };
        let test_usage_logger = TestUsageEventLogger::new();
        let db = Database::load(
            tp.clone(),
            rt.clone(),
            searcher.clone(),
            ShutdownSignal::panic(),
            virtual_system_mapping,
            Arc::new(test_usage_logger.clone()),
        )
        .await?;
        db.set_search_storage(search_storage.clone());
        if bootstrap_search_and_vector_indexes {
            let handle = db.start_search_and_vector_bootstrap();
            handle.join().await?;
        }
        let build_index_args = BuildTextIndexArgs {
            search_storage: search_storage.clone(),
            segment_term_metadata_fetcher: Arc::new(InProcessSearcher::new(rt.clone()).await?),
        };
        if bootstrap_table_summaries {
            db.finish_table_summary_bootstrap().await?;
        }
        Ok(Self {
            tp,
            db,
            searcher,
            search_storage,
            build_index_args,
            test_usage_logger,
        })
    }
}

pub async fn new_test_database<RT: Runtime>(rt: RT) -> Database<RT> {
    DbFixtures::new(&rt).await.unwrap().db
}

pub async fn new_tx<RT: Runtime>(rt: RT) -> anyhow::Result<Transaction<RT>> {
    new_test_database(rt).await.begin_system().await
}
