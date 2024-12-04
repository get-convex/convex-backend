use std::{
    collections::{
        BTreeMap,
        HashSet,
    },
    sync::Arc,
    time::Duration,
};

use common::{
    backoff::Backoff,
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
        PublicFunctionPath,
    },
    document::ParsedDocument,
    errors::{
        report_error,
        JsError,
    },
    execution_context::ExecutionContext,
    identity::InertIdentity,
    knobs::{
        SCHEDULED_JOB_EXECUTION_PARALLELISM,
        UDF_EXECUTOR_OCC_MAX_RETRIES,
    },
    log_lines::LogLines,
    minitrace_helpers::get_sampled_span,
    query::{
        IndexRange,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        FunctionCaller,
        UdfType,
    },
    RequestId,
};
use database::{
    BootstrapComponentsModel,
    Database,
    ResolvedQuery,
    Transaction,
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    future::Either,
    select_biased,
    Future,
    FutureExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use isolate::JsonPackedValue;
use keybroker::Identity;
use minitrace::future::FutureExt as _;
use model::{
    backend_state::BackendStateModel,
    cron_jobs::{
        next_ts::compute_next_ts,
        types::{
            CronJob,
            CronJobLogLines,
            CronJobResult,
            CronJobState,
            CronJobStatus,
        },
        CronModel,
        CRON_JOBS_INDEX_BY_NEXT_TS,
        CRON_JOBS_TABLE,
    },
    modules::ModuleModel,
};
use sync_types::Timestamp;
use tokio::sync::mpsc;
use usage_tracking::FunctionUsageTracker;
use value::{
    ResolvedDocumentId,
    TableNamespace,
};

use crate::{
    application_function_runner::ApplicationFunctionRunner,
    function_log::FunctionExecutionLog,
};

mod metrics;

const INITIAL_BACKOFF: Duration = Duration::from_millis(10);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

// Truncate result and log lines for cron job logs since they are only
// used for the dashboard
const CRON_LOG_MAX_RESULT_LENGTH: usize = 1000;
const CRON_LOG_MAX_LOG_LINE_LENGTH: usize = 1000;

// This code is very similar to ScheduledJobExecutor and could potentially be
// refactored later.
#[derive(Clone)]
pub struct CronJobExecutor<RT: Runtime> {
    rt: RT,
    instance_name: String,
    database: Database<RT>,
    runner: Arc<ApplicationFunctionRunner<RT>>,
    function_log: FunctionExecutionLog<RT>,
}

impl<RT: Runtime> CronJobExecutor<RT> {
    pub fn start(
        rt: RT,
        instance_name: String,
        database: Database<RT>,
        runner: Arc<ApplicationFunctionRunner<RT>>,
        function_log: FunctionExecutionLog<RT>,
    ) -> impl Future<Output = ()> + Send {
        let executor = Self {
            rt,
            instance_name,
            database,
            runner,
            function_log,
        };
        async move {
            let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
            while let Err(mut e) = executor.run(&mut backoff).await {
                // Only report OCCs that happen repeatedly
                if !e.is_occ() || (backoff.failures() as usize) > *UDF_EXECUTOR_OCC_MAX_RETRIES {
                    report_error(&mut e).await;
                }
                let delay = backoff.fail(&mut executor.rt.rng());
                tracing::error!("Cron job executor failed, sleeping {delay:?}");
                executor.rt.wait(delay).await;
            }
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        rt: RT,
        instance_name: String,
        database: Database<RT>,
        runner: Arc<ApplicationFunctionRunner<RT>>,
        function_log: FunctionExecutionLog<RT>,
    ) -> Self {
        Self {
            rt,
            instance_name,
            database,
            runner,
            function_log,
        }
    }

    async fn run(&self, backoff: &mut Backoff) -> anyhow::Result<()> {
        tracing::info!("Starting cron job executor");
        let (job_finished_tx, mut job_finished_rx) =
            mpsc::channel(*SCHEDULED_JOB_EXECUTION_PARALLELISM);
        let mut running_job_ids = HashSet::new();
        let mut next_job_ready_time = None;
        loop {
            let mut tx = self.database.begin(Identity::Unknown).await?;
            let backend_state = BackendStateModel::new(&mut tx).get_backend_state().await?;
            let is_backend_stopped = backend_state.is_stopped();

            next_job_ready_time = if is_backend_stopped {
                None
            } else if running_job_ids.len() == *SCHEDULED_JOB_EXECUTION_PARALLELISM {
                next_job_ready_time
            } else {
                self.query_and_start_jobs(&mut tx, &mut running_job_ids, &job_finished_tx)
                    .await?
            };

            let next_job_future = if let Some(next_job_ts) = next_job_ready_time {
                let now = self.rt.generate_timestamp()?;
                Either::Left(if next_job_ts < now {
                    metrics::log_cron_job_execution_lag(now - next_job_ts);
                    // If we're behind, re-run this loop every 5 seconds to log the gauge above and
                    // track how far we're behind in our metrics.
                    self.rt.wait(Duration::from_secs(5))
                } else {
                    metrics::log_cron_job_execution_lag(Duration::from_secs(0));
                    self.rt.wait(next_job_ts - now)
                })
            } else {
                metrics::log_cron_job_execution_lag(Duration::from_secs(0));
                Either::Right(std::future::pending())
            };

            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            select_biased! {
                job_id = job_finished_rx.recv().fuse() => {
                    if let Some(job_id) = job_id {
                        running_job_ids.remove(&job_id);
                    } else {
                        anyhow::bail!("Job results channel closed, this is unexpected!");
                    }
                },
                _ = next_job_future.fuse() => {
                }
                _ = subscription.wait_for_invalidation().fuse() => {
                },
            };
            backoff.reset();
        }
    }

    async fn query_and_start_jobs(
        &self,
        tx: &mut Transaction<RT>,
        running_job_ids: &mut HashSet<ResolvedDocumentId>,
        job_finished_tx: &mpsc::Sender<ResolvedDocumentId>,
    ) -> anyhow::Result<Option<Timestamp>> {
        let now = self.rt.generate_timestamp()?;
        let mut job_stream = self.stream_jobs_to_run(tx);
        while let Some(job) = job_stream.try_next().await? {
            let (job_id, job) = job.clone().into_id_and_value();
            if running_job_ids.contains(&job_id) {
                continue;
            }
            let next_ts = job.next_ts;
            // If we can't execute the job return the job's target timestamp. If we're
            // caught up, we can sleep until the timestamp. If we're behind and
            // at our concurrency limit, we can use the timestamp to log how far
            // behind we get.
            if next_ts > now || running_job_ids.len() == *SCHEDULED_JOB_EXECUTION_PARALLELISM {
                return Ok(Some(next_ts));
            }
            let root = get_sampled_span(
                &self.instance_name,
                "crons/execute_job",
                &mut self.rt.rng(),
                BTreeMap::new(),
            );
            let context = self.clone();
            let tx = job_finished_tx.clone();
            self.rt.spawn(
                "spawn_cron_job",
                async move {
                    let result = context.execute_job(job, job_id).await;
                    let _ = tx.send(result).await;
                }
                .in_span(root),
            );
            running_job_ids.insert(job_id);
        }
        Ok(None)
    }

    #[try_stream(boxed, ok = ParsedDocument<CronJob>, error = anyhow::Error)]
    async fn stream_jobs_to_run<'a>(&'a self, tx: &'a mut Transaction<RT>) {
        let namespaces: Vec<_> = tx
            .table_mapping()
            .iter()
            .filter(|(_, _, _, name)| **name == *CRON_JOBS_TABLE)
            .map(|(_, namespace, ..)| namespace)
            .collect();
        let index_query = Query::index_range(IndexRange {
            index_name: CRON_JOBS_INDEX_BY_NEXT_TS.clone(),
            range: vec![],
            order: Order::Asc,
        });
        // Key is (next_ts, namespace), where next_ts is for sorting and namespace
        // is for deduping.
        // Value is (job, query) where job is the job to run and query will get
        // the next job to run in that namespace.
        let mut queries = BTreeMap::new();
        for namespace in namespaces {
            let mut query = ResolvedQuery::new(tx, namespace, index_query.clone())?;
            if let Some(doc) = query.next(tx, None).await? {
                let job: ParsedDocument<CronJob> = doc.try_into()?;
                let next_ts = job.next_ts;
                queries.insert((next_ts, namespace), (job, query));
            }
        }
        while let Some(((_min_next_ts, namespace), (min_job, mut query))) = queries.pop_first() {
            yield min_job;
            if let Some(doc) = query.next(tx, None).await? {
                let job: ParsedDocument<CronJob> = doc.try_into()?;
                let next_ts = job.next_ts;
                queries.insert((next_ts, namespace), (job, query));
            }
        }
    }

    // This handles re-running the cron job on transient errors. It
    // guarantees that the job was successfully run or the job state changed.
    pub async fn execute_job(
        &self,
        job: CronJob,
        job_id: ResolvedDocumentId,
    ) -> ResolvedDocumentId {
        let mut function_backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            // Use a new request_id for every cron job execution attempt.
            let request_id = RequestId::new();
            let result = self.run_function(request_id, job.clone(), job_id).await;
            match result {
                Ok(result) => {
                    metrics::log_cron_job_success(function_backoff.failures());
                    return result;
                },
                Err(mut e) => {
                    let delay = function_backoff.fail(&mut self.rt.rng());
                    tracing::error!("System error executing job:, sleeping {delay:?}");
                    report_error(&mut e).await;
                    metrics::log_cron_job_failure(&e);
                    self.rt.wait(delay).await;
                },
            }
        }
    }

    async fn run_function(
        &self,
        request_id: RequestId,
        job: CronJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let usage_tracker = FunctionUsageTracker::new();
        let Some(mut tx) = self
            .new_transaction_for_job_state(job_id, &job, usage_tracker.clone())
            .await?
        else {
            // Continue without running function since the job state has changed
            return Ok(job_id);
        };
        let (_, component_path) = self.get_job_component(&mut tx, job_id).await?;
        tracing::info!("Executing {:?}!", job.cron_spec.udf_path);

        // Since we don't specify the function type in the cron, we have to use
        // the analyzed result.
        let path = CanonicalizedComponentFunctionPath {
            component: component_path,
            udf_path: job.cron_spec.udf_path.clone(),
        };
        let udf_type = ModuleModel::new(&mut tx)
            .get_analyzed_function(&path)
            .await?
            .map_err(|e| {
                anyhow::anyhow!(
                    "Cron trying to execute missing function. This should have been checked \
                     during analyze. Error: {e}"
                )
            })?
            .udf_type;

        match udf_type {
            UdfType::Mutation => {
                self.handle_mutation(request_id, tx, job, job_id, usage_tracker)
                    .await?
            },
            UdfType::Action => {
                self.handle_action(request_id, tx, job, job_id, usage_tracker)
                    .await?
            },
            udf_type => {
                anyhow::bail!(
                    "Cron trying to execute {} which is a {} function. This should have been \
                     checked during analyze.",
                    job.cron_spec.udf_path,
                    udf_type
                );
            },
        };

        Ok(job_id)
    }

    fn truncate_result(&self, result: JsonPackedValue) -> CronJobResult {
        let value = result.unpack();
        let mut value_str = value.to_string();
        if value_str.len() <= CRON_LOG_MAX_RESULT_LENGTH {
            CronJobResult::Default(value)
        } else {
            value_str =
                value_str[..value_str.floor_char_boundary(CRON_LOG_MAX_RESULT_LENGTH)].to_string();
            CronJobResult::Truncated(value_str)
        }
    }

    fn truncate_log_lines(&self, log_lines: LogLines) -> CronJobLogLines {
        let mut new_log_lines = Vec::new();
        let mut is_truncated = false;
        let mut size = 0;
        for log in log_lines
            .into_iter()
            .flat_map(|log| log.to_pretty_strings())
        {
            let line_len = log.len();
            if size + line_len <= CRON_LOG_MAX_LOG_LINE_LENGTH {
                new_log_lines.push(log);
                size += line_len;
            } else {
                is_truncated = true;
                break;
            }
        }
        CronJobLogLines {
            log_lines: new_log_lines.into(),
            is_truncated,
        }
    }

    async fn get_job_component(
        &self,
        tx: &mut Transaction<RT>,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<(ComponentId, ComponentPath)> {
        let namespace = tx.table_mapping().tablet_namespace(job_id.tablet_id)?;
        let component = match namespace {
            TableNamespace::Global => ComponentId::Root,
            TableNamespace::ByComponent(id) => ComponentId::Child(id),
        };
        let component_path = BootstrapComponentsModel::new(tx).must_component_path(component)?;
        Ok((component, component_path))
    }

    async fn handle_mutation(
        &self,
        request_id: RequestId,
        mut tx: Transaction<RT>,
        job: CronJob,
        job_id: ResolvedDocumentId,
        usage_tracker: FunctionUsageTracker,
    ) -> anyhow::Result<()> {
        let start = self.rt.monotonic_now();
        let identity = tx.inert_identity();
        let caller = FunctionCaller::Cron;
        let (component, component_path) = self.get_job_component(&mut tx, job_id).await?;
        let context = ExecutionContext::new(request_id, &caller);
        let path = CanonicalizedComponentFunctionPath {
            component: component_path,
            udf_path: job.cron_spec.udf_path.clone(),
        };
        let mutation_result = self
            .runner
            .run_mutation_no_udf_log(
                tx,
                PublicFunctionPath::Component(path.clone()),
                job.cron_spec.udf_args.clone(),
                caller.allowed_visibility(),
                context.clone(),
            )
            .await;
        let (mut tx, mut outcome) = match mutation_result {
            Ok(r) => r,
            Err(e) => {
                self.function_log.log_mutation_system_error(
                    &e,
                    path,
                    job.cron_spec.udf_args.clone(),
                    identity,
                    start,
                    caller,
                    context,
                )?;
                return Err(e);
            },
        };
        let stats = tx.take_stats();
        let execution_time = start.elapsed();
        let execution_time_f64 = execution_time.as_secs_f64();
        let truncated_log_lines = self.truncate_log_lines(outcome.log_lines.clone());

        let mut model = CronModel::new(&mut tx, component);

        if let Ok(ref result) = outcome.result {
            let truncated_result = self.truncate_result(result.clone());
            let status = CronJobStatus::Success(truncated_result);
            model
                .insert_cron_job_log(
                    &job,
                    status,
                    truncated_log_lines.clone(),
                    execution_time_f64,
                )
                .await?;
            self.complete_job_run(
                identity.clone(),
                &mut tx,
                job_id,
                &job,
                UdfType::Mutation,
                context.clone(),
            )
            .await?;
            if let Err(err) = self
                .database
                .commit_with_write_source(tx, "cron_commit_mutation")
                .await
            {
                if err.is_deterministic_user_error() {
                    outcome.result = Err(JsError::from_error(err));
                } else {
                    return Err(err);
                }
            }
        }
        if let Err(ref e) = outcome.result {
            // UDF failed due to developer error. It is not safe to commit the
            // transaction it executed in. We should remove the job in a new
            // transaction.
            let Some(mut tx) = self
                .new_transaction_for_job_state(job_id, &job, usage_tracker.clone())
                .await?
            else {
                // Continue without updating since the job state has changed
                return Ok(());
            };
            let mut model = CronModel::new(&mut tx, component);
            let status = CronJobStatus::Err(e.to_string());
            model
                .insert_cron_job_log(&job, status, truncated_log_lines, execution_time_f64)
                .await?;
            self.complete_job_run(
                identity,
                &mut tx,
                job_id,
                &job,
                UdfType::Mutation,
                context.clone(),
            )
            .await?;
            // NOTE: We should not be getting developer errors here.
            self.database
                .commit_with_write_source(tx, "cron_save_mutation_error")
                .await?;
        }

        self.function_log.log_mutation(
            outcome,
            stats,
            execution_time,
            caller,
            usage_tracker,
            context,
        );

        Ok(())
    }

    async fn handle_action(
        &self,
        request_id: RequestId,
        mut tx: Transaction<RT>,
        job: CronJob,
        job_id: ResolvedDocumentId,
        usage_tracker: FunctionUsageTracker,
    ) -> anyhow::Result<()> {
        let namespace = tx.table_mapping().tablet_namespace(job_id.tablet_id)?;
        let component = match namespace {
            TableNamespace::Global => ComponentId::Root,
            TableNamespace::ByComponent(id) => ComponentId::Child(id),
        };
        let identity = tx.identity().clone();
        let (_, component_path) = self.get_job_component(&mut tx, job_id).await?;
        let caller = FunctionCaller::Cron;
        match job.state {
            CronJobState::Pending => {
                // Set state to in progress
                let mut updated_job = job.clone();
                updated_job.state = CronJobState::InProgress;
                CronModel::new(&mut tx, component)
                    .update_job_state(job_id, updated_job.clone())
                    .await?;
                self.database
                    .commit_with_write_source(tx, "cron_in_progress")
                    .await?;

                // Execute the action
                let context = ExecutionContext::new(request_id, &caller);
                let path = CanonicalizedComponentFunctionPath {
                    component: component_path,
                    udf_path: job.cron_spec.udf_path.clone(),
                };
                let completion = self
                    .runner
                    .run_action_no_udf_log(
                        PublicFunctionPath::Component(path),
                        job.cron_spec.udf_args,
                        identity.clone(),
                        caller,
                        usage_tracker.clone(),
                        context.clone(),
                    )
                    .await?;
                let execution_time_f64 = completion.execution_time.as_secs_f64();
                let truncated_log_lines = self.truncate_log_lines(completion.log_lines.clone());

                let status = match completion.outcome.result.clone() {
                    Ok(result) => {
                        let truncated_result = self.truncate_result(result);
                        CronJobStatus::Success(truncated_result)
                    },
                    Err(e) => CronJobStatus::Err(e.to_string()),
                };

                // Mark the job as completed. Keep trying until we succeed (or
                // detect the job state has changed). Don't bubble up the error
                // since otherwise we will lose the original execution logs.
                let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
                let identity: InertIdentity = identity.into();
                while let Err(mut err) = self
                    .complete_action_run(
                        identity.clone(),
                        job_id,
                        &updated_job,
                        status.clone(),
                        truncated_log_lines.clone(),
                        execution_time_f64,
                        usage_tracker.clone(),
                        context.clone(),
                    )
                    .await
                {
                    let delay = backoff.fail(&mut self.rt.rng());
                    tracing::error!("Failed to update action state, sleeping {delay:?}");
                    report_error(&mut err).await;
                    self.rt.wait(delay).await;
                }
                self.function_log.log_action(completion, usage_tracker);
            },
            CronJobState::InProgress => {
                // This case can happen if there is a system error while executing
                // the action or if backend exits after executing the action but
                // before updating the state. Since we execute actions at most once,
                // complete this job and log the error.
                let err =
                    JsError::from_message("Transient error while executing action".to_string());
                let status = CronJobStatus::Err(err.to_string());
                let log_lines = CronJobLogLines {
                    log_lines: vec![].into(),
                    is_truncated: false,
                };

                // TODO: This is wrong. We don't know the executionId the action has been
                // started with. We generate a new executionId and use it to log the failures. I
                // guess the correct behavior here is to store the executionId in the state so
                // we can log correctly here.
                let context = ExecutionContext::new(request_id, &caller);
                let mut model = CronModel::new(&mut tx, component);
                model
                    .insert_cron_job_log(&job, status, log_lines, 0.0)
                    .await?;
                let identity: InertIdentity = identity.into();
                self.complete_job_run(
                    identity.clone(),
                    &mut tx,
                    job_id,
                    &job,
                    UdfType::Action,
                    context.clone(),
                )
                .await?;
                self.database
                    .commit_with_write_source(tx, "cron_finish_action")
                    .await?;

                let path = CanonicalizedComponentFunctionPath {
                    component: component_path,
                    udf_path: job.cron_spec.udf_path,
                };
                self.function_log.log_action_system_error(
                    &err.into(),
                    path,
                    job.cron_spec.udf_args.clone(),
                    identity,
                    self.rt.monotonic_now(),
                    caller,
                    vec![].into(),
                    context,
                )?;
            },
        }
        Ok(())
    }

    // Creates a new transaction and verifies the job state matches the given one.
    async fn new_transaction_for_job_state(
        &self,
        job_id: ResolvedDocumentId,
        expected_state: &CronJob,
        usage_tracker: FunctionUsageTracker,
    ) -> anyhow::Result<Option<Transaction<RT>>> {
        let mut tx = self
            .database
            .begin_with_usage(Identity::Unknown, usage_tracker)
            .await?;
        // Verify that the cron job has not changed.
        let new_job = tx
            .get(job_id)
            .await?
            .map(ParsedDocument::<CronJob>::try_from)
            .transpose()?
            .map(|j| j.into_value());
        Ok((new_job.as_ref() == Some(expected_state)).then_some(tx))
    }

    // Completes an action in separate transaction. Returns false if the action
    // state has changed.
    async fn complete_action_run(
        &self,
        identity: InertIdentity,
        job_id: ResolvedDocumentId,
        expected_state: &CronJob,
        status: CronJobStatus,
        log_lines: CronJobLogLines,
        execution_time: f64,
        usage_tracker: FunctionUsageTracker,
        context: ExecutionContext,
    ) -> anyhow::Result<()> {
        let Some(mut tx) = self
            .new_transaction_for_job_state(job_id, expected_state, usage_tracker)
            .await?
        else {
            // Continue without updating since the job state has changed
            return Ok(());
        };
        let namespace = tx.table_mapping().tablet_namespace(job_id.tablet_id)?;
        let component = match namespace {
            TableNamespace::Global => ComponentId::Root,
            TableNamespace::ByComponent(id) => ComponentId::Child(id),
        };
        let mut model = CronModel::new(&mut tx, component);
        model
            .insert_cron_job_log(expected_state, status, log_lines, execution_time)
            .await?;
        self.complete_job_run(
            identity,
            &mut tx,
            job_id,
            expected_state,
            UdfType::Action,
            context,
        )
        .await?;
        self.database
            .commit_with_write_source(tx, "cron_complete_action")
            .await?;
        Ok(())
    }

    async fn complete_job_run(
        &self,
        identity: InertIdentity,
        tx: &mut Transaction<RT>,
        job_id: ResolvedDocumentId,
        job: &CronJob,
        udf_type: UdfType,
        context: ExecutionContext,
    ) -> anyhow::Result<()> {
        let now = self.rt.generate_timestamp()?;
        let prev_ts = job.next_ts;
        let mut next_ts = compute_next_ts(&job.cron_spec, Some(prev_ts), now)?;
        let mut num_skipped = 0;
        let first_skipped_ts = next_ts;
        let (component, component_path) = self.get_job_component(tx, job_id).await?;
        let mut model = CronModel::new(tx, component);
        while next_ts < now {
            num_skipped += 1;
            next_ts = compute_next_ts(&job.cron_spec, Some(next_ts), now)?;
        }
        if num_skipped > 0 {
            let name = &job.name;
            tracing::info!(
                "Skipping {num_skipped} run(s) of {name} because multiple scheduled runs are in \
                 the past"
            );
            match udf_type {
                // These aren't system errors in the sense that they represent an issue with Convex
                // (e.g. they can occur due to the developer pausing their deployment)
                // but they get logged similarly, since they shouldn't count towards usage and
                // should appear as errors
                UdfType::Mutation => {
                    self.function_log.log_mutation_system_error(
                        &anyhow::anyhow!(
                            "Skipping {num_skipped} run(s) of {name} because multiple scheduled \
                             runs are in the past"
                        ),
                        CanonicalizedComponentFunctionPath {
                            component: component_path,
                            udf_path: job.cron_spec.udf_path.clone(),
                        },
                        job.cron_spec.udf_args.clone(),
                        identity,
                        self.rt.monotonic_now(),
                        FunctionCaller::Cron,
                        context,
                    )?;
                },
                UdfType::Action => {
                    self.function_log.log_action_system_error(
                        &anyhow::anyhow!(
                            "Skipping {num_skipped} run(s) of {name} because multiple scheduled \
                             runs are in the past"
                        ),
                        CanonicalizedComponentFunctionPath {
                            component: component_path,
                            udf_path: job.cron_spec.udf_path.clone(),
                        },
                        job.cron_spec.udf_args.clone(),
                        identity,
                        self.rt.monotonic_now(),
                        FunctionCaller::Cron,
                        vec![].into(),
                        context,
                    )?;
                },
                UdfType::Query | UdfType::HttpAction => {
                    anyhow::bail!("Executing unexpected function type as a cron")
                },
            }

            let status = CronJobStatus::Canceled {
                num_canceled: num_skipped,
            };
            let log_lines = CronJobLogLines {
                log_lines: vec![].into(),
                is_truncated: false,
            };
            let mut canceled_job = job.clone();
            canceled_job.next_ts = first_skipped_ts;
            model
                .insert_cron_job_log(&canceled_job, status, log_lines, 0.0)
                .await?;
        }

        let mut updated_job = job.clone();
        updated_job.state = CronJobState::Pending;
        updated_job.prev_ts = Some(prev_ts);
        updated_job.next_ts = next_ts;
        model.update_job_state(job_id, updated_job.clone()).await?;
        Ok(())
    }
}
