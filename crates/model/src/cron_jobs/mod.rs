use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use anyhow::Context;
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
    defaults::system_index,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use futures_async_stream::try_stream;
use sync_types::CanonicalizedModulePath;
use types::CronJobMetadata;
use value::{
    heap_size::WithHeapSize,
    ConvexValue,
    DeveloperDocumentId,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    config::types::CronDiff,
    cron_jobs::{
        next_ts::compute_next_ts,
        types::{
            CronIdentifier,
            CronJob,
            CronJobLog,
            CronJobLogLines,
            CronJobState,
            CronJobStatus,
            CronNextRun,
            CronSpec,
        },
    },
    modules::module_versions::AnalyzedModule,
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

pub static DEPRECATED_CRON_JOBS_INDEX_BY_NEXT_TS: LazyLock<IndexName> =
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
                name: DEPRECATED_CRON_JOBS_INDEX_BY_NEXT_TS.clone(),
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
        ParseDocument::<CronJobMetadata>::parse(document).map(|_| ())
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

const MAX_LOGS_PER_CRON: usize = 5;

pub struct CronModel<'a, RT: Runtime> {
    pub tx: &'a mut Transaction<RT>,
    pub component: ComponentId,
}

impl<'a, RT: Runtime> CronModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, component: ComponentId) -> Self {
        Self { tx, component }
    }

    #[fastrace::trace]
    pub async fn apply(
        &mut self,
        analyze_results: &BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    ) -> anyhow::Result<CronDiff> {
        let crons_js = "crons.js".parse()?;
        let new_crons: WithHeapSize<BTreeMap<CronIdentifier, CronSpec>> =
            if let Some(module) = analyze_results.get(&crons_js) {
                module.cron_specs.clone().unwrap_or_default()
            } else {
                WithHeapSize::default()
            };

        let old_crons = self.list().await?;
        let mut added_crons: Vec<&CronIdentifier> = vec![];
        let mut updated_crons: Vec<&CronIdentifier> = vec![];
        let mut deleted_crons: Vec<&CronIdentifier> = vec![];
        for (name, cron_spec) in &new_crons {
            match old_crons.get(&name.clone()) {
                Some(cron_job) => {
                    if cron_job.cron_spec != cron_spec.clone() {
                        self.update(cron_job.clone(), cron_spec.clone()).await?;
                        updated_crons.push(name);
                    }
                },
                None => {
                    self.create(name.clone(), cron_spec.clone()).await?;
                    added_crons.push(name);
                },
            }
        }
        for (name, cron_job) in &old_crons {
            match new_crons.get(&name.clone()) {
                Some(_) => {},
                None => {
                    self.delete(cron_job.clone()).await?;
                    deleted_crons.push(name);
                },
            }
        }
        tracing::info!(
            "Crons Added: {added_crons:?}, Updated: {updated_crons:?}, Deleted: {deleted_crons:?}"
        );
        let cron_diff = CronDiff::new(added_crons, updated_crons, deleted_crons);
        Ok(cron_diff)
    }

    pub async fn create(
        &mut self,
        name: CronIdentifier,
        cron_spec: CronSpec,
    ) -> anyhow::Result<()> {
        let now = self.runtime().generate_timestamp()?;
        let next_ts = compute_next_ts(&cron_spec, None, now)?;
        let cron = CronJobMetadata {
            name,
            cron_spec,
            state: Some(CronJobState::Pending),
            prev_ts: None,
            next_ts: Some(next_ts),
        };

        let cron_job_id = SystemMetadataModel::new(self.tx, self.component.into())
            .insert(&CRON_JOBS_TABLE, cron.try_into()?)
            .await?
            .developer_id;

        let next_run = CronNextRun {
            cron_job_id,
            state: CronJobState::Pending,
            prev_ts: None,
            next_ts,
        };

        SystemMetadataModel::new(self.tx, self.component.into())
            .insert(&CRON_NEXT_RUN_TABLE, next_run.try_into()?)
            .await?;

        Ok(())
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
        query_stream
            .expect_at_most_one(self.tx)
            .await?
            .map(|v| v.parse())
            .transpose()
    }

    pub async fn update(
        &mut self,
        mut cron_job: CronJob,
        new_cron_spec: CronSpec,
    ) -> anyhow::Result<()> {
        if new_cron_spec.cron_schedule != cron_job.cron_spec.cron_schedule {
            let now = self.runtime().generate_timestamp()?;
            cron_job.next_ts = compute_next_ts(&new_cron_spec, cron_job.prev_ts, now)?;
        }
        cron_job.cron_spec = new_cron_spec;
        self.update_job_state(cron_job).await?;
        Ok(())
    }

    pub async fn delete(&mut self, cron_job: CronJob) -> anyhow::Result<()> {
        SystemMetadataModel::new(self.tx, self.component.into())
            .delete(cron_job.id)
            .await?;
        let next_run = self
            .next_run(cron_job.id.developer_id)
            .await?
            .context("No next run found")?;
        SystemMetadataModel::new(self.tx, self.component.into())
            .delete(next_run.id())
            .await?;
        self.apply_job_log_retention(cron_job.name.clone(), 0)
            .await?;
        Ok(())
    }

    pub async fn update_job_state(&mut self, job: CronJob) -> anyhow::Result<()> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(self.component.into())
            .tablet_matches_name(job.id.tablet_id, &CRON_JOBS_TABLE));
        let cron_job = CronJobMetadata {
            name: job.name,
            cron_spec: job.cron_spec,
            state: Some(job.state),
            prev_ts: job.prev_ts,
            next_ts: Some(job.next_ts),
        };
        SystemMetadataModel::new(self.tx, self.component.into())
            .replace(job.id, cron_job.try_into()?)
            .await?;

        let next_run = CronNextRun {
            cron_job_id: job.id.developer_id,
            state: job.state,
            prev_ts: job.prev_ts,
            next_ts: job.next_ts,
        };
        let existing_next_run = self
            .next_run(job.id.developer_id)
            .await?
            .context("No next run found")?;
        SystemMetadataModel::new(self.tx, self.component.into())
            .replace(existing_next_run.id(), next_run.try_into()?)
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
        SystemMetadataModel::new(self.tx, self.component.into())
            .insert_metadata(&CRON_JOB_LOGS_TABLE, cron_job_log.try_into()?)
            .await?;
        self.apply_job_log_retention(job.name.clone(), MAX_LOGS_PER_CRON)
            .await?;
        Ok(())
    }

    pub async fn get(&mut self, id: ResolvedDocumentId) -> anyhow::Result<Option<CronJob>> {
        let Some(job) = self.tx.get(id).await? else {
            return Ok(None);
        };
        let cron: ParsedDocument<CronJobMetadata> = job.parse()?;
        let next_run = self
            .next_run(id.developer_id)
            .await?
            .context("No next run found")?
            .into_value();
        Ok(Some(CronJob::new(cron, self.component, next_run)))
    }

    pub async fn list(&mut self) -> anyhow::Result<BTreeMap<CronIdentifier, CronJob>> {
        let cron_query = Query::full_table_scan(CRON_JOBS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, self.component.into(), cron_query)?;
        let mut cron_jobs = BTreeMap::new();
        while let Some(job) = query_stream.next(self.tx, None).await? {
            let cron: ParsedDocument<CronJobMetadata> = job.parse()?;
            let next_run = self
                .next_run(cron.id().developer_id)
                .await?
                .context("No next run found")?
                .into_value();
            cron_jobs.insert(
                cron.name.clone(),
                CronJob::new(cron, self.component, next_run),
            );
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
        let mut query_stream = ResolvedQuery::new(self.tx, self.component.into(), index_query)?;
        let mut num_logs = 0;
        let mut to_delete = Vec::new();
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            num_logs += 1;
            if num_logs > limit {
                to_delete.push(doc.id());
            }
        }
        for doc_id in to_delete.into_iter() {
            SystemMetadataModel::new(self.tx, self.component.into())
                .delete(doc_id)
                .await?;
        }
        Ok(())
    }
}

#[try_stream(boxed, ok = CronJob, error = anyhow::Error)]
pub async fn stream_cron_jobs_to_run<'a, RT: Runtime>(tx: &'a mut Transaction<RT>) {
    let namespaces: Vec<_> = tx
        .table_mapping()
        .iter()
        .filter(|(_, _, _, name)| **name == *CRON_JOBS_TABLE)
        .map(|(_, namespace, ..)| namespace)
        .collect();
    let index_query = Query::index_range(IndexRange {
        index_name: CRON_NEXT_RUN_INDEX_BY_NEXT_TS.clone(),
        range: vec![],
        order: Order::Asc,
    });
    // Key is (next_ts, namespace), where next_ts is for sorting and namespace
    // is for deduping.
    // Value is (job, query) where job is the job to run and query will get
    // the next job to run in that namespace.
    let mut queries = BTreeMap::new();
    let cron_from_doc =
        async |namespace: TableNamespace, doc: ResolvedDocument, tx: &mut Transaction<RT>| {
            let next_run: ParsedDocument<CronNextRun> = doc.parse()?;
            let cron_job_id = next_run
                .cron_job_id
                .to_resolved(tx.table_mapping().namespace(namespace).number_to_tablet())?;
            let job: ParsedDocument<CronJobMetadata> = tx
                .get(cron_job_id)
                .await?
                .context("No cron job found")?
                .parse()?;
            Ok::<_, anyhow::Error>(CronJob::new(job, namespace.into(), next_run.into_value()))
        };

    // Initialize streaming query for each namespace
    for namespace in namespaces {
        let mut query = ResolvedQuery::new(tx, namespace, index_query.clone())?;
        if let Some(doc) = query.next(tx, None).await? {
            let cron_job = cron_from_doc(namespace, doc, tx).await?;
            queries.insert((cron_job.next_ts, namespace), (cron_job, query));
        }
    }

    // Process each namespace in order of next_ts
    while let Some(((_min_next_ts, namespace), (min_job, mut query))) = queries.pop_first() {
        yield min_job;
        if let Some(doc) = query.next(tx, None).await? {
            let cron_job = cron_from_doc(namespace, doc, tx).await?;
            queries.insert((cron_job.next_ts, namespace), (cron_job, query));
        }
    }
}
