use std::{
    cmp::Ordering,
    collections::BTreeMap,
    sync::Arc,
    time::Duration,
};

use anyhow::Context as AnyhowContext;
use common::{
    errors::JsError,
    execution_context::ExecutionContext,
    log_lines::{
        LogLevel,
        LogLine,
        LogLines,
    },
    query::Query,
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
    types::{
        PersistenceVersion,
        UdfType,
    },
    version::Version,
};
use database::{
    query::TableFilter,
    DeveloperQuery,
    Transaction,
};
use errors::ErrorMetadata;
use futures::{
    channel::{
        mpsc,
        oneshot,
    },
    FutureExt,
    StreamExt,
};
use keybroker::KeyBroker;
use model::{
    environment_variables::{
        EnvironmentVariablesModel,
        PreloadedEnvironmentVariables,
    },
    file_storage::{
        types::FileStorageEntry,
        FileStorageId,
    },
    udf_config::UdfConfigModel,
};
use parking_lot::Mutex;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use sync_types::UdfPath;
use tokio::sync::Semaphore;
use value::{
    ConvexArray,
    TableIdAndTableNumber,
    TableMapping,
    TableName,
    TableNumber,
    VirtualTableMapping,
};

use super::{
    client::{
        AsyncSyscallCompletion,
        EvaluateReady,
        EvaluateResult,
        IsolateThreadClient,
        IsolateThreadRequest,
        QueryId,
    },
    context::Context,
    environment::{
        Environment,
        EnvironmentOutcome,
    },
    session::Session,
    thread::Thread,
};
use crate::{
    client::initialize_v8,
    environment::{
        helpers::{
            module_loader::{
                module_specifier_from_path,
                path_from_module_specifier,
            },
            MAX_LOG_LINES,
        },
        udf::{
            async_syscall::{
                AsyncSyscallBatch,
                AsyncSyscallProvider,
                DatabaseSyscallsV1,
                ManagedQuery,
            },
            syscall::{
                syscall_impl,
                SyscallProvider,
            },
        },
    },
    JsonPackedValue,
    ModuleLoader,
    ModuleNotFoundError,
    SyscallTrace,
    UdfOutcome,
    ValidatedUdfPathAndArgs,
};

fn handle_request(
    session: &mut Session,
    context: &mut Context,
    request: IsolateThreadRequest,
) -> anyhow::Result<()> {
    match request {
        IsolateThreadRequest::RegisterModule {
            name,
            source,
            source_map,
            response,
        } => {
            let result = context.enter(session, |mut ctx| {
                ctx.register_module(&name, &source, source_map)
            });
            response
                .send(result)
                .map_err(|_| anyhow::anyhow!("Canceled"))?;
        },
        IsolateThreadRequest::EvaluateModule { name, response } => {
            let result = context.enter(session, |mut ctx| {
                ctx.evaluate_module(&name)?;
                anyhow::Ok(())
            });
            response
                .send(result)
                .map_err(|_| anyhow::anyhow!("Canceled"))?;
        },
        IsolateThreadRequest::StartFunction {
            udf_type,
            udf_path,
            arguments,
            response,
        } => {
            let r = context.start_function(session, udf_type, udf_path, arguments);
            response.send(r).map_err(|_| anyhow::anyhow!("Canceled"))?;
        },
        IsolateThreadRequest::PollFunction {
            function_id,
            completions,
            response,
        } => {
            let r = context.poll_function(session, function_id, completions);
            response.send(r).map_err(|_| anyhow::anyhow!("Canceled"))?;
        },
    }
    Ok(())
}

async fn v8_thread(
    mut receiver: mpsc::Receiver<IsolateThreadRequest>,
    environment: Box<dyn Environment>,
) -> anyhow::Result<()> {
    let mut thread = Thread::new();
    let mut session = Session::new(&mut thread);
    let mut context = Context::new(&mut session, environment)?;

    while let Some(request) = receiver.next().await {
        handle_request(&mut session, &mut context, request)?;
    }

    drop(context);
    drop(session);
    drop(thread);

    Ok(())
}

#[derive(Debug, Copy, Clone)]
pub struct SeedData {
    pub rng_seed: [u8; 32],
    pub unix_timestamp: UnixTimestamp,
}

struct UdfEnvironment<RT: Runtime> {
    rt: RT,
    is_system: bool,

    log_lines: LogLines,

    import_time_seed: SeedData,
    execution_time_seed: SeedData,

    phase: UdfPhase,

    shared: UdfShared<RT>,

    #[allow(unused)]
    env_vars: PreloadedEnvironmentVariables,
}

#[derive(Debug)]
enum UdfPhase {
    Importing {
        rng: ChaCha12Rng,
    },
    Executing {
        rng: ChaCha12Rng,
        observed_time: bool,
        observed_rng: bool,
    },
    Finalized,
}

impl<RT: Runtime> UdfEnvironment<RT> {
    pub fn new(
        rt: RT,
        is_system: bool,
        import_time_seed: SeedData,
        execution_time_seed: SeedData,
        shared: UdfShared<RT>,
        env_vars: PreloadedEnvironmentVariables,
    ) -> Self {
        let rng = ChaCha12Rng::from_seed(import_time_seed.rng_seed);
        Self {
            rt,
            is_system,
            log_lines: vec![].into(),

            import_time_seed,
            execution_time_seed,

            phase: UdfPhase::Importing { rng },

            shared,
            env_vars,
        }
    }

    fn check_executing(&self) -> anyhow::Result<()> {
        let UdfPhase::Executing { .. } = self.phase else {
            // TODO: Is this right? Should we just be using JsError?
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDbDuringImport",
                "Can't use database at import time",
            ))
        };
        Ok(())
    }
}

impl<RT: Runtime> SyscallProvider<RT> for UdfEnvironment<RT> {
    fn table_filter(&self) -> TableFilter {
        if self.is_system {
            TableFilter::IncludePrivateSystemTables
        } else {
            TableFilter::ExcludePrivateSystemTables
        }
    }

    fn lookup_table(&mut self, name: &TableName) -> anyhow::Result<Option<TableIdAndTableNumber>> {
        self.check_executing()?;
        self.shared.lookup_table(name)
    }

    fn lookup_virtual_table(&mut self, name: &TableName) -> anyhow::Result<Option<TableNumber>> {
        self.check_executing()?;
        self.shared.lookup_virtual_table(name)
    }

    fn start_query(&mut self, query: Query, version: Option<Version>) -> anyhow::Result<QueryId> {
        self.check_executing()?;
        let query_id = self.shared.start_query(query, version);
        Ok(query_id)
    }

    fn cleanup_query(&mut self, query_id: u32) -> bool {
        self.shared.cleanup_query(query_id)
    }
}

impl<RT: Runtime> Environment for UdfEnvironment<RT> {
    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue> {
        syscall_impl(self, name, args)
    }

    fn trace(
        &mut self,
        level: common::log_lines::LogLevel,
        messages: Vec<String>,
    ) -> anyhow::Result<()> {
        // - 1 to reserve for the [ERROR] log line
        match self.log_lines.len().cmp(&(&MAX_LOG_LINES - 1)) {
            Ordering::Less => self.log_lines.push(LogLine::new_developer_log_line(
                level,
                messages,
                // Note: accessing the current time here is still deterministic since
                // we don't externalize the time to the function.
                self.rt.unix_timestamp(),
            )),
            Ordering::Equal => {
                // Add a message about omitting log lines once
                self.log_lines.push(LogLine::new_developer_log_line(
                    LogLevel::Error,
                    vec![format!(
                        "Log overflow (maximum {MAX_LOG_LINES}). Remaining log lines omitted."
                    )],
                    // Note: accessing the current time here is still deterministic since
                    // we don't externalize the time to the function.
                    self.rt.unix_timestamp(),
                ))
            },
            Ordering::Greater => (),
        };
        Ok(())
    }

    fn trace_system(
        &mut self,
        level: common::log_lines::LogLevel,
        messages: Vec<String>,
        system_log_metadata: common::log_lines::SystemLogMetadata,
    ) -> anyhow::Result<()> {
        self.log_lines.push(LogLine::new_system_log_line(
            level,
            messages,
            // Note: accessing the current time here is still deterministic since
            // we don't externalize the time to the function.
            self.rt.unix_timestamp(),
            system_log_metadata,
        ));
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut rand_chacha::ChaCha12Rng> {
        match self.phase {
            UdfPhase::Importing { ref mut rng } => Ok(rng),
            UdfPhase::Executing {
                ref mut rng,
                ref mut observed_rng,
                ..
            } => {
                *observed_rng = true;
                Ok(rng)
            },
            UdfPhase::Finalized => anyhow::bail!("RNG not available in finalized phase"),
        }
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        let result = match self.phase {
            UdfPhase::Importing { .. } => self.import_time_seed.unix_timestamp,
            UdfPhase::Executing {
                ref mut observed_time,
                ..
            } => {
                *observed_time = true;
                self.execution_time_seed.unix_timestamp
            },
            UdfPhase::Finalized => anyhow::bail!("Time not available in finalized phase"),
        };
        Ok(result)
    }

    fn get_environment_variable(
        &mut self,
        _name: common::types::EnvVarName,
    ) -> anyhow::Result<Option<common::types::EnvVarValue>> {
        todo!()
    }

    fn start_execution(&mut self) -> anyhow::Result<()> {
        let UdfPhase::Importing { .. } = self.phase else {
            anyhow::bail!("Phase was already {:?}", self.phase)
        };
        self.phase = UdfPhase::Executing {
            rng: ChaCha12Rng::from_seed(self.execution_time_seed.rng_seed),
            observed_time: false,
            observed_rng: false,
        };
        Ok(())
    }

    fn finish_execution(&mut self) -> anyhow::Result<EnvironmentOutcome> {
        let UdfPhase::Executing {
            observed_time,
            observed_rng,
            ..
        } = self.phase
        else {
            anyhow::bail!("Phase was not executing")
        };
        self.phase = UdfPhase::Finalized;
        Ok(EnvironmentOutcome {
            observed_rng,
            observed_time,
        })
    }
}

async fn run_request<RT: Runtime>(
    rt: RT,
    tx: &mut Transaction<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    execution_time_seed: SeedData,
    client: &mut IsolateThreadClient<RT>,
    udf_type: UdfType,
    path_and_args: ValidatedUdfPathAndArgs,
    shared: UdfShared<RT>,
) -> anyhow::Result<UdfOutcome> {
    let (udf_path, arguments, udf_server_version) = path_and_args.consume();

    // Phase 1: Load and register all source needed, and evaluate the UDF's module.
    let r: anyhow::Result<_> = try {
        let mut stack = vec![udf_path.module().clone()];

        while let Some(module_path) = stack.pop() {
            let module_specifier = module_specifier_from_path(&module_path)?;
            let Some(module_metadata) = module_loader.get_module(tx, module_path.clone()).await?
            else {
                let err = ModuleNotFoundError::new(module_path.as_str());
                Err(JsError::from_message(format!("{err}")))?
            };
            let requests = client
                .register_module(
                    module_specifier,
                    module_metadata.source.clone(),
                    module_metadata.source_map.clone(),
                )
                .await?;
            for requested_module_specifier in requests {
                let module_path = path_from_module_specifier(&requested_module_specifier)?;
                stack.push(module_path);
            }
        }

        let udf_module_specifier = module_specifier_from_path(udf_path.module())?;
        client.evaluate_module(udf_module_specifier.clone()).await?;
        anyhow::Ok(())
    };
    if let Err(e) = r {
        let js_error = e.downcast::<JsError>()?;
        let outcome = UdfOutcome {
            udf_path,
            arguments,
            identity: tx.inert_identity(),
            rng_seed: execution_time_seed.rng_seed,
            observed_rng: false,
            unix_timestamp: execution_time_seed.unix_timestamp,
            observed_time: false,
            log_lines: vec![].into(),
            journal: QueryJournal::new(),
            result: Err(js_error),
            syscall_trace: SyscallTrace::new(),
            udf_server_version,
        };
        return Ok(outcome);
    }

    // Phase 2: Start the UDF, execute its async syscalls, and poll until
    // completion.
    let mut provider = Isolate2SyscallProvider::new(
        tx,
        rt,
        execution_time_seed.unix_timestamp,
        QueryJournal::new(),
        QueryJournal::new(),
        udf_path.is_system(),
        shared,
    );
    let r: anyhow::Result<_> = try {
        // Update our shared state with the updated table mappings before reentering
        // user code.
        provider.shared.update_table_mappings(provider.tx);
        let (function_id, mut result) = client
            .start_function(udf_type, udf_path.clone(), arguments.clone())
            .await?;
        loop {
            let pending = match result {
                EvaluateResult::Ready(r) => break r,
                EvaluateResult::Pending(p) => p,
            };
            let mut completions = vec![];

            let mut syscall_batch = None;
            let mut batch_promise_ids = vec![];

            for async_syscall in pending.async_syscalls {
                let promise_id = async_syscall.promise_id;
                match syscall_batch {
                    None => {
                        syscall_batch = Some(AsyncSyscallBatch::new(
                            async_syscall.name,
                            async_syscall.args,
                        ));
                        assert!(batch_promise_ids.is_empty());
                        batch_promise_ids.push(promise_id);
                    },
                    Some(ref mut batch)
                        if batch.can_push(&async_syscall.name, &async_syscall.args) =>
                    {
                        batch.push(async_syscall.name, async_syscall.args)?;
                        batch_promise_ids.push(promise_id);
                    },
                    Some(batch) => {
                        let results =
                            DatabaseSyscallsV1::run_async_syscall_batch(&mut provider, batch)
                                .await?;
                        assert_eq!(results.len(), batch_promise_ids.len());

                        for (promise_id, result) in batch_promise_ids.drain(..).zip(results) {
                            completions.push(AsyncSyscallCompletion { promise_id, result });
                        }

                        syscall_batch = None;
                    },
                }
            }
            if let Some(batch) = syscall_batch {
                let results =
                    DatabaseSyscallsV1::run_async_syscall_batch(&mut provider, batch).await?;
                assert_eq!(results.len(), batch_promise_ids.len());

                for (promise_id, result) in batch_promise_ids.into_iter().zip(results) {
                    completions.push(AsyncSyscallCompletion { promise_id, result });
                }
            }

            provider.shared.update_table_mappings(provider.tx);
            result = client.poll_function(function_id, completions).await?;
        }
    };

    let (result, outcome) = match r {
        Ok(EvaluateReady { result, outcome }) => (Ok(JsonPackedValue::pack(result)), outcome),
        Err(e) => {
            let js_error = e.downcast::<JsError>()?;
            // TODO: Ask the V8 thread for its outcome.
            let outcome = EnvironmentOutcome {
                observed_rng: false,
                observed_time: false,
            };
            (Err(js_error), outcome)
        },
    };
    let outcome = UdfOutcome {
        udf_path,
        arguments,
        identity: provider.tx.inert_identity(),
        rng_seed: execution_time_seed.rng_seed,
        observed_rng: outcome.observed_rng,
        unix_timestamp: execution_time_seed.unix_timestamp,
        observed_time: outcome.observed_time,
        log_lines: vec![].into(),
        journal: provider.next_journal,
        result,
        syscall_trace: provider.syscall_trace,
        udf_server_version,
    };
    Ok(outcome)
}

struct UdfShared<RT: Runtime> {
    inner: Arc<Mutex<UdfSharedInner<RT>>>,
}

impl<RT: Runtime> Clone for UdfShared<RT> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<RT: Runtime> UdfShared<RT> {
    pub fn new(table_mapping: TableMapping, virtual_table_mapping: VirtualTableMapping) -> Self {
        Self {
            inner: Arc::new(Mutex::new(UdfSharedInner {
                next_query_id: 0,
                queries: BTreeMap::new(),
                table_mapping,
                virtual_table_mapping,
            })),
        }
    }

    fn update_table_mappings(&self, tx: &mut Transaction<RT>) {
        let mut inner = self.inner.lock();
        // TODO: Avoid cloning here if the table mapping hasn't changed.
        inner.table_mapping = tx.table_mapping().clone();
        inner.virtual_table_mapping = tx.virtual_table_mapping().clone();
    }

    fn lookup_table(&self, name: &TableName) -> anyhow::Result<Option<TableIdAndTableNumber>> {
        let inner = self.inner.lock();
        Ok(inner.table_mapping.id_and_number_if_exists(name))
    }

    fn lookup_virtual_table(&self, name: &TableName) -> anyhow::Result<Option<TableNumber>> {
        let inner = self.inner.lock();
        Ok(inner.virtual_table_mapping.number_if_exists(name))
    }

    fn start_query(&self, query: Query, version: Option<Version>) -> QueryId {
        let mut inner = self.inner.lock();
        let query_id = inner.next_query_id;
        inner.next_query_id += 1;
        inner
            .queries
            .insert(query_id, ManagedQuery::Pending { query, version });
        query_id
    }

    fn take_query(&self, query_id: QueryId) -> Option<ManagedQuery<RT>> {
        let mut inner = self.inner.lock();
        inner.queries.remove(&query_id)
    }

    fn insert_query(&self, query_id: QueryId, query: DeveloperQuery<RT>) {
        let mut inner = self.inner.lock();
        inner.queries.insert(query_id, ManagedQuery::Active(query));
    }

    fn cleanup_query(&self, query_id: u32) -> bool {
        let mut inner = self.inner.lock();
        inner.queries.remove(&query_id).is_some()
    }
}

struct UdfSharedInner<RT: Runtime> {
    next_query_id: QueryId,
    queries: BTreeMap<QueryId, ManagedQuery<RT>>,

    table_mapping: TableMapping,
    virtual_table_mapping: VirtualTableMapping,
}

struct Isolate2SyscallProvider<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    rt: RT,

    shared: UdfShared<RT>,

    unix_timestamp: UnixTimestamp,

    prev_journal: QueryJournal,
    next_journal: QueryJournal,

    is_system: bool,

    syscall_trace: SyscallTrace,
}

impl<'a, RT: Runtime> Isolate2SyscallProvider<'a, RT> {
    fn new(
        tx: &'a mut Transaction<RT>,
        rt: RT,
        unix_timestamp: UnixTimestamp,
        prev_journal: QueryJournal,
        next_journal: QueryJournal,
        is_system: bool,
        shared: UdfShared<RT>,
    ) -> Self {
        Self {
            tx,
            rt,
            shared,
            unix_timestamp,
            prev_journal,
            next_journal,
            is_system,
            syscall_trace: SyscallTrace::new(),
        }
    }
}

impl<'a, RT: Runtime> AsyncSyscallProvider<RT> for Isolate2SyscallProvider<'a, RT> {
    fn rt(&self) -> &RT {
        &self.rt
    }

    fn tx(&mut self) -> Result<&mut Transaction<RT>, ErrorMetadata> {
        // We only process syscalls during the execution phase.
        Ok(self.tx)
    }

    fn key_broker(&self) -> &KeyBroker {
        todo!()
    }

    fn context(&self) -> &ExecutionContext {
        todo!()
    }

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        Ok(self.unix_timestamp)
    }

    fn persistence_version(&self) -> PersistenceVersion {
        todo!()
    }

    fn table_filter(&self) -> TableFilter {
        if self.is_system {
            TableFilter::IncludePrivateSystemTables
        } else {
            TableFilter::ExcludePrivateSystemTables
        }
    }

    fn log_async_syscall(&mut self, name: String, duration: Duration, is_success: bool) {
        self.syscall_trace
            .log_async_syscall(name, duration, is_success);
    }

    fn prev_journal(&mut self) -> &mut QueryJournal {
        &mut self.prev_journal
    }

    fn next_journal(&mut self) -> &mut QueryJournal {
        &mut self.next_journal
    }

    async fn validate_schedule_args(
        &mut self,
        _udf_path: UdfPath,
        _args: Vec<JsonValue>,
        _scheduled_ts: UnixTimestamp,
    ) -> anyhow::Result<(UdfPath, ConvexArray)> {
        todo!()
    }

    fn file_storage_generate_upload_url(&self) -> anyhow::Result<String> {
        todo!()
    }

    async fn file_storage_get_url(
        &mut self,
        _storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>> {
        todo!()
    }

    async fn file_storage_delete(&mut self, _storage_id: FileStorageId) -> anyhow::Result<()> {
        todo!()
    }

    async fn file_storage_get_entry(
        &mut self,
        _storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>> {
        todo!()
    }

    fn insert_query(&mut self, query_id: QueryId, query: DeveloperQuery<RT>) {
        self.shared.insert_query(query_id, query)
    }

    fn take_query(&mut self, query_id: QueryId) -> Option<ManagedQuery<RT>> {
        self.shared.take_query(query_id)
    }

    fn cleanup_query(&mut self, query_id: u32) -> bool {
        self.shared.cleanup_query(query_id)
    }
}

async fn tokio_thread<RT: Runtime>(
    rt: RT,
    mut tx: Transaction<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    execution_time_seed: SeedData,
    mut client: IsolateThreadClient<RT>,
    total_timeout: Duration,
    mut sender: oneshot::Sender<anyhow::Result<(Transaction<RT>, UdfOutcome)>>,
    udf_type: UdfType,
    path_and_args: ValidatedUdfPathAndArgs,
    shared: UdfShared<RT>,
) {
    let request = run_request(
        rt.clone(),
        &mut tx,
        module_loader,
        execution_time_seed,
        &mut client,
        udf_type,
        path_and_args,
        shared,
    );

    let r = futures::select_biased! {
        r = request.fuse() => r,

        // Eventually we'll attempt to cleanup the isolate thread in these conditions.
        _ = rt.wait(total_timeout) => Err(anyhow::anyhow!("Total timeout exceeded")),
        _ = sender.cancellation().fuse() => Err(anyhow::anyhow!("Cancelled")),
    };
    let _ = sender.send(r.map(|r| (tx, r)));
    drop(client);
}

pub async fn run_isolate_v2_udf<RT: Runtime>(
    rt: RT,
    mut tx: Transaction<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    execution_time_seed: SeedData,
    udf_type: UdfType,
    path_and_args: ValidatedUdfPathAndArgs,
) -> anyhow::Result<(Transaction<RT>, UdfOutcome)> {
    initialize_v8();

    let semaphore = Arc::new(Semaphore::new(8));
    let user_timeout = Duration::from_secs(5);

    // We actually don't really care about "system timeout" but rather "total
    // timeout", both for how long we're tying up a request thread + serving
    // based on a tx timestamp that may be out of retention.
    let total_timeout = Duration::from_secs(10);

    // TODO: Move these into the timeout.
    let udf_config = UdfConfigModel::new(&mut tx).get().await?;
    let import_time_seed = SeedData {
        rng_seed: udf_config
            .as_ref()
            .map(|c| c.import_phase_rng_seed)
            .context("Missing import phase RNG seed")?,
        unix_timestamp: udf_config
            .as_ref()
            .map(|c| c.import_phase_unix_timestamp)
            .context("Missing import phase unix timestamp")?,
    };
    let env_vars = EnvironmentVariablesModel::new(&mut tx).preload().await?;

    // TODO: This unconditionally takes a table mapping dep.
    let shared = UdfShared::new(
        tx.table_mapping().clone(),
        tx.virtual_table_mapping().clone(),
    );
    let environment = UdfEnvironment::new(
        rt.clone(),
        path_and_args.udf_path.is_system(),
        import_time_seed,
        execution_time_seed,
        shared.clone(),
        env_vars,
    );

    // The protocol is synchronous, so there should never be more than
    // one pending request at a time.
    let (sender, receiver) = mpsc::channel(1);
    let v8_handle = rt.spawn_thread(|| async {
        if let Err(e) = v8_thread(receiver, Box::new(environment)).await {
            println!("Error in isolate thread: {:?}", e);
        }
    });

    let client = IsolateThreadClient::new(rt.clone(), sender, user_timeout, semaphore);
    let (sender, receiver) = oneshot::channel();
    let tokio_handle = rt.spawn(
        "tokio_thread",
        tokio_thread(
            rt.clone(),
            tx,
            module_loader,
            execution_time_seed,
            client,
            total_timeout,
            sender,
            udf_type,
            path_and_args,
            shared,
        ),
    );

    let r = receiver.await??;

    tokio_handle.into_join_future().await?;
    v8_handle.into_join_future().await?;

    Ok(r)
}
