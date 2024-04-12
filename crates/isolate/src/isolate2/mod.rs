pub mod client;
pub mod context;
pub mod context_state;
pub mod entered_context;
pub mod environment;
pub mod session;
pub mod thread;

pub type PromiseId = u64;
pub type FunctionId = u64;

#[cfg(test)]
mod tests {
    use std::{
        cmp::Ordering,
        collections::BTreeMap,
        sync::Arc,
        thread,
        time::Duration,
    };

    use common::{
        log_lines::{
            LogLevel,
            LogLine,
            LogLines,
        },
        runtime::{
            Runtime,
            UnixTimestamp,
        },
        types::UdfType,
    };
    use crossbeam_channel::{
        bounded,
        Receiver,
    };
    use futures::{
        channel::oneshot,
        FutureExt,
    };
    use rand::{
        Rng,
        SeedableRng,
    };
    use rand_chacha::ChaCha12Rng;
    use runtime::prod::ProdRuntime;
    use serde_json::Value as JsonValue;
    use sync_types::{
        CanonicalizedUdfPath,
        UdfPath,
    };
    use tokio::sync::Semaphore;
    use value::{
        obj,
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
        environment::helpers::{
            module_loader::module_specifier_from_path,
            MAX_LOG_LINES,
        },
        test_helpers::TEST_SOURCE_ISOLATE_ONLY,
    };

    fn v8_thread(
        receiver: Receiver<IsolateThreadRequest>,
        environment: Box<dyn Environment>,
    ) -> anyhow::Result<()> {
        let mut thread = Thread::new();
        let mut session = Session::new(&mut thread);
        let mut context = Context::new(&mut session, environment)?;

        while let Ok(request) = receiver.recv() {
            match request {
                IsolateThreadRequest::RegisterModule {
                    name,
                    source,
                    response,
                } => {
                    let imports = context
                        .enter(&mut session, |mut ctx| ctx.register_module(&name, &source))?;
                    let _ = response.send(imports);
                },
                IsolateThreadRequest::EvaluateModule { name, response } => {
                    context.enter(&mut session, |mut ctx| {
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
                    let r = context.start_function(&mut session, udf_type, &module, &name, args)?;
                    let _ = response.send(r);
                },
                IsolateThreadRequest::PollFunction {
                    function_id,
                    completions,
                    response,
                } => {
                    let r = context.poll_function(&mut session, function_id, completions)?;
                    let _ = response.send(r);
                },
            }
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

    impl<RT: Runtime> Environment for UdfEnvironment<RT> {
        fn syscall(&mut self, op: &str, args: JsonValue) -> anyhow::Result<JsonValue> {
            if op == "echo" {
                return Ok(args);
            }
            anyhow::bail!("Syscall not implemented")
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

    async fn run_request(
        client: &mut IsolateThreadClient,
        udf_type: UdfType,
        udf_path: CanonicalizedUdfPath,
        args: ConvexObject,
    ) -> anyhow::Result<ConvexValue> {
        let mut module_source = BTreeMap::new();
        for module_config in TEST_SOURCE_ISOLATE_ONLY.iter() {
            let canonicalized = module_config.path.clone().canonicalize();
            let module_specifier = module_specifier_from_path(&canonicalized)?;
            module_source.insert(module_specifier, module_config.source.clone());
        }

        let udf_module_specifier = module_specifier_from_path(udf_path.module())?;

        let mut stack = vec![udf_module_specifier.clone()];
        while let Some(module_specifier) = stack.pop() {
            let Some(source) = module_source.get(&module_specifier) else {
                anyhow::bail!("Module not found: {module_specifier:?}")
            };
            let requests = client
                .register_module(module_specifier, source.clone())
                .await?;
            stack.extend(requests);
        }

        client.evaluate_module(udf_module_specifier.clone()).await?;

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
            for async_syscall in async_syscalls {
                let promise_id = async_syscall.promise_id;
                let result = Ok(JsonValue::from(1));
                completions.push(AsyncSyscallCompletion { promise_id, result });
            }
            result = client.poll_function(function_id, completions).await?;
        }
    }

    async fn tokio_thread(
        mut client: IsolateThreadClient,
        total_timeout: Duration,
        mut tx: oneshot::Sender<anyhow::Result<ConvexValue>>,
        udf_type: UdfType,
        udf_path: CanonicalizedUdfPath,
        args: ConvexObject,
    ) {
        let r = futures::select_biased! {
            r = run_request(&mut client, udf_type, udf_path, args).fuse() => r,

            // Eventually we'll attempt to cleanup the isolate thread in these conditions.
            _ = tokio::time::sleep(total_timeout).fuse() => Err(anyhow::anyhow!("Total timeout exceeded")),
            _ = tx.cancellation().fuse() => Err(anyhow::anyhow!("Cancelled")),
        };
        let _ = tx.send(r);
        drop(client);
    }

    async fn run_isolate_v2_test(
        rt: ProdRuntime,
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

        let seed = rt.with_rng(|rng| rng.gen());
        let unix_timestamp = rt.unix_timestamp();
        let environment = UdfEnvironment::new(rt, seed, unix_timestamp);

        // The protocol is synchronous, so there should never be more than
        // one pending request at a time.
        let (sender, receiver) = bounded(1);
        let v8_handle = thread::spawn(|| {
            if let Err(e) = v8_thread(receiver, Box::new(environment)) {
                println!("Error in isolate thread: {:?}", e);
            }
        });
        let (tx, rx) = oneshot::channel();
        let client = IsolateThreadClient::new(sender, user_timeout, semaphore);
        let tokio_handle = tokio::spawn(tokio_thread(
            client,
            total_timeout,
            tx,
            udf_type,
            udf_path,
            args,
        ));

        let r = rx.await??;

        tokio_handle.await?;
        v8_handle.join().unwrap();

        Ok(r)
    }

    #[convex_macro::prod_rt_test]
    async fn test_basic_v2(rt: ProdRuntime) -> anyhow::Result<()> {
        let result = run_isolate_v2_test(
            rt.clone(),
            UdfType::Query,
            "directory/udfs:f",
            obj!("a" => 10., "b" => 3.)?,
        )
        .await?;
        assert_eq!(result, ConvexValue::Float64(57.0));

        let result = run_isolate_v2_test(
            rt.clone(),
            UdfType::Query,
            "directory/udfs:returnsUndefined",
            obj!()?,
        )
        .await?;
        assert_eq!(result, ConvexValue::Null);

        let result = run_isolate_v2_test(
            rt.clone(),
            UdfType::Query,
            "directory/defaultTest",
            obj!("a" => 10., "b" => 3.)?,
        )
        .await?;
        assert_eq!(result, ConvexValue::Float64(110.));

        let ConvexValue::Float64(..) = run_isolate_v2_test(
            rt.clone(),
            UdfType::Query,
            "directory/udfs:pseudoRandom",
            obj!()?,
        )
        .await?
        else {
            panic!("Expected Float64");
        };

        let _ = run_isolate_v2_test(
            rt.clone(),
            UdfType::Query,
            "directory/udfs:usesDate",
            obj!()?,
        )
        .await?;

        Ok(())
    }
}
