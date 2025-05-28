use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use common::{
    components::ComponentId,
    document::{
        ParseDocument,
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::IndexName,
};
use database::{
    defaults::{
        system_index,
        SystemIndex,
        SystemTable,
    },
    ResolvedQuery,
    Transaction,
};
use value::{
    ConvexValue,
    DeveloperDocumentId,
    FieldPath,
    TableName,
};

use crate::migr_119::cron_jobs::types::{
    CronIdentifier,
    CronJob,
    CronJobLog,
    CronNextRun,
};

pub mod types;

pub static CRON_JOBS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_cron_jobs"
        .parse()
        .expect("_cron_jobs is not a valid system table name")
});

pub static CRON_JOBS_INDEX_BY_NEXT_TS: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&CRON_JOBS_TABLE, "by_next_ts"));
pub static CRON_JOBS_INDEX_BY_NAME: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&CRON_JOBS_TABLE, "by_name"));
static CRON_JOBS_NAME_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "name".parse().expect("invalid name field"));
static CRON_JOBS_NEXT_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "nextTs".parse().expect("invalid nextTs field"));

pub static CRON_JOB_LOGS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_cron_job_logs"
        .parse()
        .expect("_cron_job_logs is not a valid system table name")
});

pub static CRON_JOB_LOGS_INDEX_BY_NAME_TS: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&CRON_JOB_LOGS_TABLE, "by_name_and_ts"));
pub static CRON_JOB_LOGS_NAME_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "name".parse().expect("invalid name field"));
static CRON_JOB_LOGS_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "ts".parse().expect("invalid ts field"));

pub static CRON_NEXT_RUN_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_cron_next_run"
        .parse()
        .expect("_cron_next_run is not a valid system table name")
});

pub static CRON_NEXT_RUN_INDEX_BY_NEXT_TS: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&CRON_NEXT_RUN_TABLE, "by_next_ts"));
pub static CRON_NEXT_RUN_INDEX_BY_CRON_JOB_ID: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&CRON_NEXT_RUN_TABLE, "by_cron_job_id"));
static CRON_NEXT_RUN_NEXT_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "nextTs".parse().expect("invalid nextTs field"));
static CRON_NEXT_RUN_CRON_JOB_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "cronJobId".parse().expect("invalid cronJobId field"));

pub struct CronJobsTable;
impl SystemTable for CronJobsTable {
    fn table_name(&self) -> &'static TableName {
        &CRON_JOBS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![
            // Used to find next jobs to execute for crons.
            SystemIndex {
                name: CRON_JOBS_INDEX_BY_NEXT_TS.clone(),
                fields: vec![CRON_JOBS_NEXT_TS_FIELD.clone()].try_into().unwrap(),
            },
            // Used to find cron job by name
            SystemIndex {
                name: CRON_JOBS_INDEX_BY_NAME.clone(),
                fields: vec![CRON_JOBS_NAME_FIELD.clone()].try_into().unwrap(),
            },
        ]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParseDocument::<CronJob>::parse(document).map(|_| ())
    }
}

pub struct CronJobLogsTable;
impl SystemTable for CronJobLogsTable {
    fn table_name(&self) -> &'static TableName {
        &CRON_JOB_LOGS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: CRON_JOB_LOGS_INDEX_BY_NAME_TS.clone(),
            fields: vec![
                CRON_JOB_LOGS_NAME_FIELD.clone(),
                CRON_JOB_LOGS_TS_FIELD.clone(),
            ]
            .try_into()
            .unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParseDocument::<CronJobLog>::parse(document).map(|_| ())
    }
}

pub struct CronNextRunTable;
impl SystemTable for CronNextRunTable {
    fn table_name(&self) -> &'static TableName {
        &CRON_NEXT_RUN_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![
            SystemIndex {
                name: CRON_NEXT_RUN_INDEX_BY_NEXT_TS.clone(),
                fields: vec![CRON_NEXT_RUN_NEXT_TS_FIELD.clone()]
                    .try_into()
                    .unwrap(),
            },
            SystemIndex {
                name: CRON_NEXT_RUN_INDEX_BY_CRON_JOB_ID.clone(),
                fields: vec![CRON_NEXT_RUN_CRON_JOB_ID_FIELD.clone()]
                    .try_into()
                    .unwrap(),
            },
        ]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParseDocument::<CronNextRun>::parse(document).map(|_| ())
    }
}

pub struct CronModel<'a, RT: Runtime> {
    pub tx: &'a mut Transaction<RT>,
    pub component: ComponentId,
}

impl<'a, RT: Runtime> CronModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, component: ComponentId) -> Self {
        Self { tx, component }
    }

    pub async fn next_run(
        &mut self,
        cron_job_id: DeveloperDocumentId,
    ) -> anyhow::Result<Option<ParsedDocument<CronNextRun>>> {
        let query = Query::index_range(IndexRange {
            index_name: CRON_NEXT_RUN_INDEX_BY_CRON_JOB_ID.clone(),
            range: vec![IndexRangeExpression::Eq(
                CRON_NEXT_RUN_CRON_JOB_ID_FIELD.clone(),
                ConvexValue::from(cron_job_id).into(),
            )],
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, self.component.into(), query)?;
        let next_run = query_stream.expect_at_most_one(self.tx).await?;
        next_run.map(|v| v.parse()).transpose()
    }

    pub async fn list(
        &mut self,
    ) -> anyhow::Result<BTreeMap<CronIdentifier, ParsedDocument<CronJob>>> {
        let cron_query = Query::full_table_scan(CRON_JOBS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, self.component.into(), cron_query)?;
        let mut cron_jobs = BTreeMap::new();
        while let Some(job) = query_stream.next(self.tx, None).await? {
            let cron: ParsedDocument<CronJob> = job.parse()?;
            cron_jobs.insert(cron.name.clone(), cron);
        }
        Ok(cron_jobs)
    }
}
