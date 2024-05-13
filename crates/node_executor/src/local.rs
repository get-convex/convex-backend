use std::{
    fs,
    path::PathBuf,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use common::log_lines::LogLine;
use futures::{
    channel::mpsc,
    select_biased,
    FutureExt,
    StreamExt,
};
use isolate::bundled_js::node_executor_file;
use serde_json::Value as JsonValue;
use tempfile::TempDir;
use tokio::process::Command as TokioCommand;
use tokio_process_stream::{
    Item,
    ProcessLineStream,
};

use crate::executor::{
    parse_streamed_response,
    ExecutorRequest,
    InvokeResponse,
    NodeExecutor,
    ResponsePart,
    EXECUTE_TIMEOUT_RESPONSE_JSON,
};

/// Always use node version specified in .nvmrc for lambda execution, even if
/// we're using older version for CLI.
const NODE_VERSION: &str = include_str!("../../../.nvmrc");

pub struct LocalNodeExecutor {
    _source_dir: TempDir,
    source_path: PathBuf,
    node_path: String,
    node_process_timeout: Duration,
}

impl LocalNodeExecutor {
    pub fn new(node_process_timeout: Duration) -> anyhow::Result<Self> {
        // Write the source of local.cjs to a temp file.
        let source_dir = TempDir::new()?;
        let (source, source_map) =
            node_executor_file("local.cjs").expect("local.cjs not generated!");
        let source_map = source_map.context("Missing local.cjs.map")?;
        let source_path = source_dir.path().join("local.cjs");
        let source_map_path = source_dir.path().join("local.cjs.map");
        fs::write(&source_path, source.as_bytes())?;
        fs::write(source_map_path, source_map.as_bytes())?;
        tracing::info!(
            "Using local node executor. Source: {}",
            source_path.to_str().expect("Path is not UTF-8 string?"),
        );
        let node_version = NODE_VERSION.trim();

        // Look for node16 in a few places. CI nvm installer uses `mynvm`
        let mut node_path = "node".to_string();
        for nvm_dir in [".nvm", "mynvm"] {
            let possible_path = home::home_dir()
                .unwrap()
                .join(nvm_dir)
                .join(format!("versions/node/v{node_version}/bin/node"));
            if possible_path.exists() {
                node_path = possible_path.to_string_lossy().to_string();
            }
        }

        Ok(Self {
            _source_dir: source_dir,
            source_path,
            node_path,
            node_process_timeout,
        })
    }

    async fn check_version(&self) -> anyhow::Result<()> {
        let cmd = TokioCommand::new(&self.node_path)
            .arg("--version")
            .output()
            .await?;
        let version = String::from_utf8_lossy(&cmd.stdout);
        anyhow::ensure!(
            version.starts_with("v18."),
            format!(
                "Wrong node version {} installed at {}",
                version, &self.node_path
            )
        );
        Ok(())
    }
}

#[async_trait]
impl NodeExecutor for LocalNodeExecutor {
    fn enable(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn invoke(
        &self,
        request: ExecutorRequest,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
    ) -> anyhow::Result<InvokeResponse> {
        let request = JsonValue::try_from(request)?;
        self.check_version().await?;
        let request = serde_json::to_string(&request)?;
        tracing::info!(
            "{} {} --request='{}'",
            &self.node_path,
            self.source_path.to_str().expect("Must be utf-8"),
            &request,
        );
        let mut _cmd = TokioCommand::new(&self.node_path);
        let cmd = _cmd
            .arg(&self.source_path)
            .arg("--request")
            .arg(request)
            .kill_on_drop(true);
        let mut result_values = vec![];

        let mut procstream = ProcessLineStream::try_from(cmd)?.fuse();

        let response = loop {
            select_biased! {
                item = procstream.select_next_some() => {
                    match item {
                        Item::Stdout(line) => {
                            let parts = parse_streamed_response(&line)?;
                            for part in parts {
                                match part {
                                    ResponsePart::LogLine(log_line) => {
                                        log_line_sender.unbounded_send(log_line)?;
                                    },
                                    ResponsePart::Result(result) => result_values.push(result)
                                }
                            }
                        },
                        Item::Done(status) => {
                            anyhow::ensure!(status?.success(), "Local process did not exit successfully");
                            anyhow::ensure!(result_values.len() <= 1, "Received more than one result from lambda response");
                            let value = result_values.pop().ok_or_else(|| anyhow::anyhow!("Received no result from lambda response"))?;
                            break value;
                        }
                        Item::Stderr(_) => ()
                    }
                },
                _ = tokio::time::sleep(self.node_process_timeout).fuse() => {
                    break EXECUTE_TIMEOUT_RESPONSE_JSON.clone();
                },
            }
        };
        Ok(InvokeResponse {
            response,
            // constant is good enough for measuring local executor
            memory_used_in_mb: 512,
            aws_request_id: None,
        })
    }

    fn shutdown(&self) {}
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{
            Arc,
            LazyLock,
        },
        time::Duration,
    };

    use cmd_util::env::config_test;
    use common::{
        assert_obj,
        components::{
            CanonicalizedComponentFunctionPath,
            CanonicalizedComponentModulePath,
            ComponentId,
        },
        execution_context::ExecutionContext,
        log_lines::{
            run_function_and_collect_log_lines,
            LogLines,
        },
        types::{
            ModuleEnvironment,
            UdfType,
        },
        value::ConvexValue,
        version::Version,
    };
    use errors::ErrorMetadataAnyhowExt;
    use futures::{
        channel::mpsc,
        FutureExt,
    };
    use isolate::{
        test_helpers::TEST_SOURCE,
        ValidatedPathAndArgs,
    };
    use keybroker::{
        testing::TestUserIdentity,
        UserIdentity,
    };
    use maplit::btreemap;
    use minitrace::collector::SpanContext;
    use model::{
        config::types::ModuleConfig,
        modules::{
            args_validator::ArgsValidator,
            module_versions::{
                AnalyzedFunction,
                AnalyzedSourcePosition,
                SourceMap,
                Visibility,
            },
        },
    };
    use runtime::prod::ProdRuntime;
    use storage::{
        LocalDirStorage,
        Storage,
    };
    use sync_types::ModulePath;
    use value::{
        array,
        id_v6::DeveloperDocumentId,
        ConvexArray,
        ConvexObject,
    };

    use super::LocalNodeExecutor;
    use crate::{
        executor::{
            NodeActionOutcome,
            Package,
        },
        source_package::upload_package,
        Actions,
        AnalyzeRequest,
        ExecuteRequest,
        SourcePackage,
    };

    const TEST_BACKEND_ADDRESS: &str = "http://127.0.0.1:8080";
    // Use lower timeouts for tests.
    const TEST_USER_TIMEOUT: Duration = Duration::from_secs(2);
    const TEST_NODE_PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

    static VERSION: LazyLock<Option<Version>> =
        LazyLock::new(|| Some("0.18.0".parse().expect("Failed to parse version")));

    async fn upload_modules(
        storage: Arc<dyn Storage>,
        modules: Vec<ModuleConfig>,
    ) -> anyhow::Result<SourcePackage> {
        config_test();
        let package = modules
            .iter()
            .map(|m| (m.path.clone().canonicalize(), m))
            .collect();
        let (key, sha256, _) = upload_package(package, storage.clone(), None).await?;
        let uri = storage
            .signed_url(key.clone(), Duration::from_secs(10))
            .await?;
        Ok(SourcePackage {
            bundled_source: Package { uri, key, sha256 },
            external_deps: None,
        })
    }

    fn create_args(args_object: ConvexObject) -> anyhow::Result<ConvexArray> {
        array![ConvexValue::Object(args_object)]
    }

    fn execute_request(
        path_and_args: ValidatedPathAndArgs,
        source_package: SourcePackage,
    ) -> ExecuteRequest {
        ExecuteRequest {
            path_and_args,
            source_package,
            source_package_id: DeveloperDocumentId::min().into(),
            user_identity: None,
            auth_header: None,
            environment_variables: btreemap! {},
            callback_token: "".to_owned(),
            context: ExecutionContext::new_for_test(),
            encoded_parent_trace: SpanContext::current_local_parent()
                .map(|ctx| ctx.encode_w3c_traceparent()),
        }
    }

    async fn execute(
        actions: &Actions,
        execute_request: ExecuteRequest,
        source_maps: &BTreeMap<CanonicalizedComponentModulePath, SourceMap>,
    ) -> anyhow::Result<(NodeActionOutcome, LogLines)> {
        let (log_line_sender, log_line_receiver) = mpsc::unbounded();
        let execute_future = Box::pin(
            actions
                .execute(execute_request, source_maps, log_line_sender)
                .fuse(),
        );
        let (result, log_lines) =
            run_function_and_collect_log_lines(execute_future, log_line_receiver, |_| {}).await;
        match result {
            Ok(outcome) => Ok((outcome, log_lines)),
            Err(e) => Err(e),
        }
    }

    #[convex_macro::prod_rt_test]
    async fn test_success(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;

        let source_maps = BTreeMap::new();

        let numbers: ConvexArray = array![1f64.into(), 7f64.into()]?;
        let args = create_args(assert_obj!("numbers" => ConvexValue::Array(numbers)))?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:addNumbers".parse()?,
            },
            args,
            VERSION.clone(),
        );
        let (response, _log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;

        assert_eq!(response.result?, ConvexValue::from(8.));

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_log_lines(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:logHelloWorldAndReturn7".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;

        assert_eq!(response.result?, ConvexValue::from(7.));
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string())
                .collect::<Vec<_>>(),
            vec!["[INFO] 'Hello'".to_owned(), "[ERROR] 'World!'".to_owned()]
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_auth_syscall(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let identity = UserIdentity::test();

        let source_maps = BTreeMap::new();
        let (response, _log_lines) = execute(
            &actions,
            ExecuteRequest {
                path_and_args: ValidatedPathAndArgs::new_for_tests(
                    CanonicalizedComponentFunctionPath {
                        component: ComponentId::Root,
                        udf_path: "node_actions.js:getUserIdentity".parse()?,
                    },
                    array![],
                    VERSION.clone(),
                ),
                source_package,
                source_package_id: DeveloperDocumentId::min().into(),
                user_identity: Some(identity.attributes.clone()),
                auth_header: None,
                environment_variables: btreemap! {},
                callback_token: "".to_owned(),
                context: ExecutionContext::new_for_test(),
                encoded_parent_trace: None,
            },
            &source_maps,
        )
        .await?;

        assert_eq!(
            serde_json::Value::from(response.result?),
            serde_json::Value::try_from(identity.attributes)?,
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_query_syscall(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            "http://localhost:8719".into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();

        let args = create_args(assert_obj!(
            "name" =>  "getCounter.js"
        ))?;
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    CanonicalizedComponentFunctionPath {
                        component: ComponentId::Root,
                        udf_path: "node_actions.js:runQuery".parse()?,
                    },
                    args,
                    VERSION.clone(),
                ),
                source_package,
            ),
            &source_maps,
        )
        .await?;

        // This won't work since the backend is not running but we can check
        // that we are calling the right url
        let err = response.result.unwrap_err();
        assert!(&err.message[..].contains("fetch failed"));

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_schedule_syscall(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            "http://localhost:8719".into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let args = create_args(assert_obj!(
            "name" =>  "getCounter.js"
        ))?;
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    CanonicalizedComponentFunctionPath {
                        component: ComponentId::Root,
                        udf_path: "node_actions.js:scheduleJob".parse()?,
                    },
                    args,
                    VERSION.clone(),
                ),
                source_package,
            ),
            &source_maps,
        )
        .await?;

        // This won't work since the backend is not running but we can check
        // that we are calling the right url
        let err = response.result.unwrap_err();
        assert!(&err.message[..].contains("fetch failed"));

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_error(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = TEST_SOURCE
            .clone()
            .into_iter()
            .map(|m| {
                (
                    CanonicalizedComponentModulePath {
                        component: ComponentId::Root,
                        module_path: m.path.canonicalize(),
                    },
                    m.source_map.expect("Missing source map"),
                )
            })
            .collect();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:logAndThrowError".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;

        let error = response.result.unwrap_err();
        assert_eq!(&error.message, "Uncaught Error: Oh, no!");
        let frames = &error.frames.as_ref().unwrap().0;
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].file_name.as_ref().map(|s| &s[..]),
            Some("../convex/node_actions.ts")
        );
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string())
                .collect::<Vec<_>>(),
            vec!["[LOG] 'About to do something...'".to_owned()]
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_forgot_await(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = TEST_SOURCE
            .clone()
            .into_iter()
            .map(|m| {
                (
                    CanonicalizedComponentModulePath {
                        component: ComponentId::Root,
                        module_path: m.path.canonicalize(),
                    },
                    m.source_map.expect("Missing source map"),
                )
            })
            .collect();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:forgotAwait".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;

        assert_eq!(response.result?, ConvexValue::Null);
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string())
                .collect::<Vec<_>>(),
            vec![
                "[WARN] 'You have an outstanding mutation call. Operations should \
                be awaited or they might not run. Not awaiting promises might result \
                in unexpected failures. See https://docs.convex.dev/functions/actions#dangling-promises \
                for more information.'"
                    .to_owned()
            ]
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_missing_export(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:hello".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, _log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;

        let error = response.result.unwrap_err();
        assert_eq!(
            &error.message[..],
            "Couldn't find action `hello` in `node_actions.js`"
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_environment_variables(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let mut environment_variables = BTreeMap::new();
        environment_variables.insert("TEST_NAME".parse()?, "TEST_VALUE".parse()?);
        let (response, _log_lines) = execute(
            &actions,
            ExecuteRequest {
                path_and_args: ValidatedPathAndArgs::new_for_tests(
                    CanonicalizedComponentFunctionPath {
                        component: ComponentId::Root,
                        udf_path: "node_actions.js:getTestEnvVar".parse()?,
                    },
                    array![],
                    VERSION.clone(),
                ),
                source_package,
                source_package_id: DeveloperDocumentId::min().into(),
                user_identity: None,
                auth_header: None,
                environment_variables,
                callback_token: "".to_owned(),
                context: ExecutionContext::new_for_test(),
                encoded_parent_trace: None,
            },
            &source_maps,
        )
        .await?;
        assert_eq!(response.result?, ConvexValue::try_from("TEST_VALUE")?);

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_user_timeout(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:sleepAnHour".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;
        // This should be hitting the user timeout in executor.ts, not the Node
        // process timeout.
        assert_eq!(
            &response.result.unwrap_err().message[..],
            "Action `sleepAnHour` execution timed out (maximum duration 2s)"
        );
        // Make sure we have log lines
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string())
                .collect::<Vec<_>>(),
            vec!["[LOG] 'I am very sleepy. I am going to take a nap.'".to_owned()]
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_partial_escape_sequence_result(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:partialEscapeSequence".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let err = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await
        .unwrap_err();
        assert_eq!(err.short_msg(), "FunctionReturnInvalidJson");
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_process_timeout(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:workHardForAnHour".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;
        // Since this is a busy loop, we should be hitting the process timeout.
        assert_eq!(
            &response.result.unwrap_err().message[..],
            "Function execution unexpectedly timed out. Check your function for infinite loops or \
             other long-running operations."
        );
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string())
                .collect::<Vec<_>>(),
            vec!["[LOG] 'I am going to work really hard for 1 hour'".to_owned()]
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_deadlock(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            CanonicalizedComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "node_actions.js:deadlock".parse()?,
            },
            array![],
            VERSION.clone(),
        );
        let (response, _log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            &source_maps,
        )
        .await?;
        assert_eq!(
            &response.result.unwrap_err().message[..],
            "Action `deadlock` execution timed out (maximum duration 2s)"
        );

        Ok(())
    }

    // We analyze static source since maintaining the test is hard if source
    // passes via esbuild
    const MODULE_ANALYZE: &str = r#"
async function invokeAction(func, requestId, argsStr) {
  throw new Error("unimplemented");
}
var actionGeneric = func => {
  const q = func;
  if (q.isRegistered) {
    throw new Error("Function registered twice " + func);
  }
  q.isRegistered = true;
  q.isAction = true;
  q.isPublic = true;
  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);
  return q;
};
var internalActionGeneric = func => {
    const q = func;
    if (q.isRegistered) {
      throw new Error("Function registered twice " + func);
    }
    q.isRegistered = true;
    q.isAction = true;
    q.isInternal = true;
    q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);
    return q;
  };
var action = actionGeneric;
var internalAction = internalActionGeneric;
var hello = action(async ({}) => {
  console.log("analyze me pls");
});
var internalHello = internalAction(async ({}) => {
  console.log("analyze me pls");
});
export { hello, internalHello };
"#;

    // Generated via `npx esbuild static_node_source.js --bundle --format=esm
    // --target=esnext --sourcemap=linked --outfile=out.js`
    const SOURCE_MAP: &str = r#"
{
  "version": 3,
  "sources": ["static_node_source.js"],
  "sourcesContent": ["async function invokeAction(func, requestId, argsStr) {\n  throw new Error(\"unimplemented\");\n}\nvar actionGeneric = func => {\n  const q = func;\n  if (q.isRegistered) {\n    throw new Error(\"Function registered twice \" + func);\n  }\n  q.isRegistered = true;\n  q.isAction = true;\n  q.isPublic = true;\n  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n  return q;\n};\nvar internalActionGeneric = func => {\n    const q = func;\n    if (q.isRegistered) {\n      throw new Error(\"Function registered twice \" + func);\n    }\n    q.isRegistered = true;\n    q.isAction = true;\n    q.isInternal = true;\n    q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n    return q;\n  };\nvar action = actionGeneric;\nvar internalAction = internalActionGeneric;\nvar hello = action(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nvar internalHello = internalAction(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nexport { hello, internalHello };\n"],
  "mappings": ";AAAA,eAAe,aAAa,MAAM,WAAW,SAAS;AACpD,QAAM,IAAI,MAAM,eAAe;AACjC;AACA,IAAI,gBAAgB,UAAQ;AAC1B,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,WAAW;AACb,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,SAAO;AACT;AACA,IAAI,wBAAwB,UAAQ;AAChC,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,aAAa;AACf,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,SAAO;AACT;AACF,IAAI,SAAS;AACb,IAAI,iBAAiB;AACrB,IAAI,QAAQ,OAAO,OAAO,CAAC,MAAM;AAC/B,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;AACD,IAAI,gBAAgB,eAAe,OAAO,CAAC,MAAM;AAC/C,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;",
  "names": []
}
"#;

    #[convex_macro::prod_rt_test]
    async fn test_analyze(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let path: ModulePath = "static_node_source.js".parse()?;
        let source_package = upload_modules(
            storage.clone(),
            vec![ModuleConfig {
                path: path.clone(),
                source: MODULE_ANALYZE.to_owned(),
                source_map: Some(SOURCE_MAP.to_string()),
                environment: ModuleEnvironment::Node,
            }],
        )
        .await?;
        let mut source_maps = BTreeMap::new();
        let path = CanonicalizedComponentModulePath {
            component: ComponentId::Root,
            module_path: "static_node_source.js".parse()?,
        };
        source_maps.insert(path.clone(), SOURCE_MAP.to_string());
        let modules = actions
            .analyze(
                AnalyzeRequest {
                    source_package,
                    environment_variables: BTreeMap::new(),
                },
                &source_maps,
            )
            .await??;

        assert_eq!(
            Vec::from(modules[&path].functions.clone()),
            &[
                AnalyzedFunction {
                    name: "hello".parse()?,
                    pos: Some(AnalyzedSourcePosition {
                        path: "static_node_source.js".parse()?,
                        start_lineno: 28,
                        start_col: modules[&path].functions[0].pos.as_ref().unwrap().start_col,
                    }),
                    udf_type: UdfType::Action,
                    visibility: Some(Visibility::Public),
                    args: ArgsValidator::Unvalidated
                },
                AnalyzedFunction {
                    name: "internalHello".parse()?,
                    pos: Some(AnalyzedSourcePosition {
                        path: "static_node_source.js".parse()?,
                        start_lineno: 31,
                        start_col: modules[&path].functions[1].pos.as_ref().unwrap().start_col,
                    }),
                    udf_type: UdfType::Action,
                    visibility: Some(Visibility::Internal),
                    args: ArgsValidator::Unvalidated
                },
            ]
        );
        Ok(())
    }

    const MODULE_ANALYZE_QUERY: &str = r#"
async function invokeQuery(func, requestId, argsStr) {
    throw new Error("unimplemented");
}

var queryGeneric = func => {
    const q = func;
    if (q.isRegistered) {
      throw new Error("Function registered twice " + func);
    }
    q.isRegistered = true;
    q.isQuery = true;
    q.isPublic = true;
    q.invokeQuery = (requestId, argsStr) => invokeQuery(func, requestId, argsStr);
    return q;
  };

var query = queryGeneric;

var hello = query(async ({}) => {
  console.log("analyze me pls");
});

export { hello };
"#;

    #[convex_macro::prod_rt_test]
    async fn test_analyze_query(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(
            storage.clone(),
            vec![ModuleConfig {
                path: "actions/test.js".parse()?,
                source: MODULE_ANALYZE_QUERY.to_owned(),
                source_map: None,
                environment: ModuleEnvironment::Node,
            }],
        )
        .await?;
        let source_maps = BTreeMap::new();
        let err = actions
            .analyze(
                AnalyzeRequest {
                    source_package,
                    environment_variables: BTreeMap::new(),
                },
                &source_maps,
            )
            .await?
            .unwrap_err();
        assert_eq!(
            &err.message[..],
            "hello defined in actions/test.js is a Query function. Only actions can be defined in Node.js. See https://docs.convex.dev/functions/actions for more details."
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_syscall_trace(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = BTreeMap::new();

        // First, try to execute an action with a syscall that fails. In this case,
        // we'll call into a query where the backend isn't actually running.
        let args = create_args(assert_obj!("name" =>  "getCounter.js"))?;
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    CanonicalizedComponentFunctionPath {
                        component: ComponentId::Root,
                        udf_path: "node_actions.js:runQuery".parse()?,
                    },
                    args,
                    VERSION.clone(),
                ),
                source_package.clone(),
            ),
            &source_maps,
        )
        .await?;
        let syscall_trace = response.syscall_trace;

        assert_eq!(syscall_trace.async_syscalls.len(), 1);
        assert_eq!(
            syscall_trace.async_syscalls["1.0/actions/query"].invocations,
            1
        );
        assert_eq!(syscall_trace.async_syscalls["1.0/actions/query"].errors, 1);

        // Second, execute an action with a successful syscall.
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    CanonicalizedComponentFunctionPath {
                        component: ComponentId::Root,
                        udf_path: "node_actions.js:getUserIdentity".parse()?,
                    },
                    array![],
                    VERSION.clone(),
                ),
                source_package,
            ),
            &source_maps,
        )
        .await?;
        let syscall_trace = response.syscall_trace;

        assert_eq!(syscall_trace.async_syscalls.len(), 1);
        assert_eq!(
            syscall_trace.async_syscalls["1.0/getUserIdentity"].invocations,
            1
        );
        assert_eq!(
            syscall_trace.async_syscalls["1.0/getUserIdentity"].errors,
            0
        );

        Ok(())
    }
}
