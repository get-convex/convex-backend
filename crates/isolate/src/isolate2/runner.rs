use std::{
    cmp::Ordering,
    sync::Arc,
    time::Duration,
};

use common::{
    execution_context::ExecutionContext,
    log_lines::{
        LogLevel,
        LogLine,
        LogLines,
    },
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
};
use database::{
    query::TableFilter,
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
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use sync_types::{
    CanonicalizedUdfPath,
    UdfPath,
};
use tokio::sync::Semaphore;
use value::{
    ConvexObject,
    ConvexValue,
};

use super::{
    client::{
        AsyncSyscallCompletion,
        EvaluateResult,
        IsolateThreadClient,
        IsolateThreadRequest,
    },
    context::Context,
    environment::Environment,
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
                QueryManager,
            },
            syscall::{
                syscall_impl,
                SyscallProvider,
            },
        },
    },
    ModuleLoader,
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
            response,
        } => {
            let imports = context.enter(session, |mut ctx| ctx.register_module(&name, &source))?;
            let _ = response.send(imports);
        },
        IsolateThreadRequest::EvaluateModule { name, response } => {
            context.enter(session, |mut ctx| {
                ctx.evaluate_module(&name)?;
                anyhow::Ok(())
            })?;
            let _ = response.send(());
        },
        IsolateThreadRequest::StartFunction {
            udf_type,
            module,
            name,
            args,
            response,
        } => {
            let r = context.start_function(session, udf_type, &module, &name, args)?;
            let _ = response.send(r);
        },
        IsolateThreadRequest::PollFunction {
            function_id,
            completions,
            response,
        } => {
            let r = context.poll_function(session, function_id, completions)?;
            let _ = response.send(r);
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

struct UdfEnvironment<RT: Runtime> {
    rt: RT,
    log_lines: LogLines,

    // TODO:
    // Initialize this with the seed and rng from the database during import time.
    // Flip it with begin_execution.
    rng: ChaCha12Rng,
    unix_timestamp: UnixTimestamp,
}

impl<RT: Runtime> UdfEnvironment<RT> {
    pub fn new(rt: RT, rng_seed: [u8; 32], unix_timestamp: UnixTimestamp) -> Self {
        Self {
            rt,
            log_lines: vec![].into(),
            rng: ChaCha12Rng::from_seed(rng_seed),
            unix_timestamp,
        }
    }
}

impl<RT: Runtime> SyscallProvider<RT> for UdfEnvironment<RT> {
    fn table_filter(&self) -> TableFilter {
        todo!();
    }

    fn query_manager(&mut self) -> &mut QueryManager<RT> {
        todo!();
    }

    fn tx(&mut self) -> Result<&mut Transaction<RT>, ErrorMetadata> {
        todo!();
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
        Ok(&mut self.rng)
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        Ok(self.unix_timestamp)
    }

    fn get_environment_variable(
        &mut self,
        _name: common::types::EnvVarName,
    ) -> anyhow::Result<Option<common::types::EnvVarValue>> {
        // TODO!
        Ok(None)
    }
}

async fn run_request<RT: Runtime>(
    rt: RT,
    tx: &mut Transaction<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    unix_timestamp: UnixTimestamp,
    client: &mut IsolateThreadClient<RT>,
    udf_type: UdfType,
    udf_path: CanonicalizedUdfPath,
    args: ConvexObject,
) -> anyhow::Result<ConvexValue> {
    let mut stack = vec![udf_path.module().clone()];

    while let Some(module_path) = stack.pop() {
        let module_specifier = module_specifier_from_path(&module_path)?;
        let Some(module_metadata) = module_loader.get_module(tx, module_path.clone()).await? else {
            anyhow::bail!("Module not found: {module_path:?}")
        };
        let requests = client
            .register_module(module_specifier, module_metadata.source.clone())
            .await?;
        for requested_module_specifier in requests {
            let module_path = path_from_module_specifier(&requested_module_specifier)?;
            stack.push(module_path);
        }
    }

    let udf_module_specifier = module_specifier_from_path(udf_path.module())?;

    client.evaluate_module(udf_module_specifier.clone()).await?;

    let mut provider = Isolate2SyscallProvider {
        tx,
        rt,
        query_manager: QueryManager::new(),
        unix_timestamp,
        prev_journal: QueryJournal::new(),
        next_journal: QueryJournal::new(),
        is_system: udf_path.is_system(),
    };

    let (function_id, mut result) = client
        .start_function(
            udf_type,
            udf_module_specifier.clone(),
            udf_path.function_name().to_string(),
            args,
        )
        .await?;

    loop {
        let async_syscalls = match result {
            EvaluateResult::Ready(r) => return Ok(r),
            EvaluateResult::Pending { async_syscalls } => async_syscalls,
        };

        let mut completions = vec![];

        let mut syscall_batch = None;
        let mut batch_promise_ids = vec![];

        for async_syscall in async_syscalls {
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
                Some(ref mut batch) if batch.can_push(&async_syscall.name, &async_syscall.args) => {
                    batch.push(async_syscall.name, async_syscall.args)?;
                    batch_promise_ids.push(promise_id);
                },
                Some(batch) => {
                    let results =
                        DatabaseSyscallsV1::run_async_syscall_batch(&mut provider, batch).await?;
                    assert_eq!(results.len(), batch_promise_ids.len());

                    for (promise_id, result) in batch_promise_ids.drain(..).zip(results) {
                        // TODO: Avoid reparsing the result here.
                        let result: JsonValue = serde_json::from_str(&(result?))?;
                        completions.push(AsyncSyscallCompletion {
                            promise_id,
                            result: Ok(result),
                        });
                    }

                    syscall_batch = None;
                },
            }
        }
        if let Some(batch) = syscall_batch {
            let results = DatabaseSyscallsV1::run_async_syscall_batch(&mut provider, batch).await?;
            assert_eq!(results.len(), batch_promise_ids.len());

            for (promise_id, result) in batch_promise_ids.into_iter().zip(results) {
                // TODO: Avoid reparsing the result here.
                let result: JsonValue = serde_json::from_str(&(result?))?;
                completions.push(AsyncSyscallCompletion {
                    promise_id,
                    result: Ok(result),
                });
            }
        }

        result = client.poll_function(function_id, completions).await?;
    }
}

struct Isolate2SyscallProvider<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    rt: RT,

    query_manager: QueryManager<RT>,

    unix_timestamp: UnixTimestamp,

    prev_journal: QueryJournal,
    next_journal: QueryJournal,

    is_system: bool,
}

impl<'a, RT: Runtime> AsyncSyscallProvider<RT> for Isolate2SyscallProvider<'a, RT> {
    fn rt(&self) -> &RT {
        &self.rt
    }

    fn tx(&mut self) -> Result<&mut Transaction<RT>, ErrorMetadata> {
        // TODO: phases.
        Ok(self.tx)
    }

    fn key_broker(&self) -> &KeyBroker {
        todo!()
    }

    fn context(&self) -> &ExecutionContext {
        todo!()
    }

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        // TODO: phases.
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

    fn log_async_syscall(&mut self, _name: String, _duration: Duration, _is_success: bool) {}

    fn query_manager(&mut self) -> &mut QueryManager<RT> {
        &mut self.query_manager
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
    ) -> anyhow::Result<(UdfPath, value::ConvexArray)> {
        todo!()
    }

    fn file_storage_generate_upload_url(&self) -> anyhow::Result<String> {
        todo!()
    }

    async fn file_storage_get_url(
        &mut self,
        _storage_id: model::file_storage::FileStorageId,
    ) -> anyhow::Result<Option<String>> {
        todo!()
    }

    async fn file_storage_delete(
        &mut self,
        _storage_id: model::file_storage::FileStorageId,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn file_storage_get_entry(
        &mut self,
        _storage_id: model::file_storage::FileStorageId,
    ) -> anyhow::Result<Option<model::file_storage::types::FileStorageEntry>> {
        todo!()
    }
}

async fn tokio_thread<RT: Runtime>(
    rt: RT,
    mut tx: Transaction<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    unix_timestamp: UnixTimestamp,
    mut client: IsolateThreadClient<RT>,
    total_timeout: Duration,
    mut sender: oneshot::Sender<anyhow::Result<ConvexValue>>,
    udf_type: UdfType,
    udf_path: CanonicalizedUdfPath,
    args: ConvexObject,
) {
    let request = run_request(
        rt.clone(),
        &mut tx,
        module_loader,
        unix_timestamp,
        &mut client,
        udf_type,
        udf_path,
        args,
    );

    let r = futures::select_biased! {
        r = request.fuse() => r,

        // Eventually we'll attempt to cleanup the isolate thread in these conditions.
        _ = rt.wait(total_timeout) => Err(anyhow::anyhow!("Total timeout exceeded")),
        _ = sender.cancellation().fuse() => Err(anyhow::anyhow!("Cancelled")),
    };
    let _ = sender.send(r);
    drop(client);
}

pub async fn run_isolate_v2_udf<RT: Runtime>(
    rt: RT,
    tx: Transaction<RT>,
    module_loader: Arc<dyn ModuleLoader<RT>>,
    seed: [u8; 32],
    unix_timestamp: UnixTimestamp,
    udf_type: UdfType,
    udf_path: &str,
    args: ConvexObject,
) -> anyhow::Result<ConvexValue> {
    let udf_path: UdfPath = udf_path.parse()?;
    let udf_path = udf_path.canonicalize();

    initialize_v8();

    let semaphore = Arc::new(Semaphore::new(8));
    let user_timeout = Duration::from_secs(5);

    // We actually don't really care about "system timeout" but rather "total
    // timeout", both for how long we're tying up a request thread + serving
    // based on a tx timestamp that may be out of retention.
    let total_timeout = Duration::from_secs(10);

    let environment = UdfEnvironment::new(rt.clone(), seed, unix_timestamp);

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
            unix_timestamp,
            client,
            total_timeout,
            sender,
            udf_type,
            udf_path,
            args,
        ),
    );

    let r = receiver.await??;

    tokio_handle.into_join_future().await?;
    v8_handle.into_join_future().await?;

    Ok(r)
}
