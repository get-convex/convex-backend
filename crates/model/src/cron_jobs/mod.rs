use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use common::{
    document::{
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
    defaults::system_index,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    ConvexValue,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    cron_jobs::{
        next_ts::compute_next_ts,
        types::{
            CronIdentifier,
            CronJob,
            CronJobLog,
            CronJobLogLines,
            CronJobState,
            CronJobStatus,
            CronSpec,
        },
    },
    SystemIndex,
    SystemTable,
};

pub mod next_ts;
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
        ParsedDocument::<CronJob>::try_from(document).map(|_| ())
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
        ParsedDocument::<CronJobLog>::try_from(document).map(|_| ())
    }
}

const MAX_LOGS_PER_CRON: usize = 5;

pub struct CronModel<'a, RT: Runtime> {
    pub tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> CronModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn create(
        &mut self,
        name: CronIdentifier,
        cron_spec: CronSpec,
    ) -> anyhow::Result<()> {
        let now = self.runtime().generate_timestamp()?;
        let cron = CronJob {
            name,
            next_ts: compute_next_ts(&cron_spec, None, now)?,
            cron_spec,
            state: CronJobState::Pending,
            prev_ts: None,
        };
        SystemMetadataModel::new(self.tx)
            .insert(&CRON_JOBS_TABLE, cron.try_into()?)
            .await?;
        Ok(())
    }

    pub async fn update(
        &mut self,
        cron_job: ParsedDocument<CronJob>,
        new_cron_spec: CronSpec,
    ) -> anyhow::Result<()> {
        let (job_id, mut cron_job) = cron_job.into_id_and_value();
        if new_cron_spec.cron_schedule != cron_job.cron_spec.cron_schedule {
            let now = self.runtime().generate_timestamp()?;
            cron_job.next_ts = compute_next_ts(&new_cron_spec, cron_job.prev_ts, now)?;
        }
        cron_job.cron_spec = new_cron_spec;
        self.update_job_state(job_id, cron_job).await?;
        Ok(())
    }

    pub async fn delete(&mut self, cron_job: ParsedDocument<CronJob>) -> anyhow::Result<()> {
        SystemMetadataModel::new(self.tx)
            .delete(cron_job.clone().id())
            .await?;
        self.apply_job_log_retention(cron_job.name.clone(), 0)
            .await?;
        Ok(())
    }

    pub async fn update_job_state(
        &mut self,
        id: ResolvedDocumentId,
        job: CronJob,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .number_matches_name(id.table().table_number, &CRON_JOBS_TABLE));
        SystemMetadataModel::new(self.tx)
            .replace(id, job.try_into()?)
            .await?;
        Ok(())
    }

    pub async fn insert_cron_job_log(
        &mut self,
        job: &CronJob,
        status: CronJobStatus,
        log_lines: CronJobLogLines,
        execution_time: f64,
    ) -> anyhow::Result<()> {
        let cron_job_log = CronJobLog {
            name: job.name.clone(),
            ts: job.next_ts,
            udf_path: job.cron_spec.udf_path.clone(),
            udf_args: job.cron_spec.udf_args.clone(),
            status,
            log_lines,
            execution_time,
        };
        SystemMetadataModel::new(self.tx)
            .insert_metadata(&CRON_JOB_LOGS_TABLE, cron_job_log.try_into()?)
            .await?;
        self.apply_job_log_retention(job.name.clone(), MAX_LOGS_PER_CRON)
            .await?;
        Ok(())
    }

    pub async fn list(
        &mut self,
    ) -> anyhow::Result<BTreeMap<CronIdentifier, ParsedDocument<CronJob>>> {
        let cron_query = Query::full_table_scan(CRON_JOBS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, cron_query)?;
        let mut cron_jobs = BTreeMap::new();
        while let Some(job) = query_stream.next(self.tx, None).await? {
            let cron: ParsedDocument<CronJob> = job.try_into()?;
            cron_jobs.insert(cron.name.clone(), cron);
        }
        Ok(cron_jobs)
    }

    fn runtime(&self) -> &RT {
        self.tx.runtime()
    }

    // Keep up to `limit` of the newest logs per cron
    async fn apply_job_log_retention(
        &mut self,
        name: CronIdentifier,
        limit: usize,
    ) -> anyhow::Result<()> {
        let index_query = Query::index_range(IndexRange {
            index_name: CRON_JOB_LOGS_INDEX_BY_NAME_TS.clone(),
            range: vec![IndexRangeExpression::Eq(
                CRON_JOB_LOGS_NAME_FIELD.clone(),
                ConvexValue::try_from(name.to_string())?.into(),
            )],
            order: Order::Desc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, index_query)?;
        let mut num_logs = 0;
        let mut to_delete = Vec::new();
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            num_logs += 1;
            if num_logs > limit {
                to_delete.push(doc.id());
            }
        }
        for doc_id in to_delete.into_iter() {
            SystemMetadataModel::new(self.tx).delete(doc_id).await?;
        }
        Ok(())
    }
}
