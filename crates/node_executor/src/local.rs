use std::{
    fs,
    path::PathBuf,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use common::log_lines::LogLine;
use errors::ErrorMetadata;
use futures::{
    select_biased,
    FutureExt,
    StreamExt,
};
use isolate::bundled_js::node_executor_file;
use serde_json::Value as JsonValue;
use tempfile::TempDir;
use tokio::{
    process::Command as TokioCommand,
    sync::mpsc,
};
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

        // Look for node in a few places.
        let possible_path = home::home_dir()
            .unwrap()
            .join(".nvm")
            .join(format!("versions/node/v{node_version}/bin/node"));
        let node_path = if possible_path.exists() {
            possible_path.to_string_lossy().to_string()
        } else {
            "node".to_string()
        };

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

        if !version.starts_with("v18.") {
            anyhow::bail!(ErrorMetadata::bad_request(
                "DeploymentNotConfiguredForNodeActions",
                "Deployment is not configured to deploy \"use node\" actions. \
                 Node.js v18 is not installed. \
                 Install Node.js 18 with nvm (https://github.com/nvm-sh/nvm) \
                 to deploy Node.js actions."
            ))
        }
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
        let mut err_lines = vec![];

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
                                        log_line_sender.send(log_line)?;
                                    },
                                    ResponsePart::Result(result) => result_values.push(result)
                                }
                            }
                        },
                        Item::Done(status) => {
                            if !status?.success() {
                                for line in err_lines {
                                    tracing::error!("{line}");
                                }
                                anyhow::bail!("Local process did not exit successfully");
                            }
                            anyhow::ensure!(result_values.len() <= 1, "Received more than one result from lambda response");
                            let value = result_values.pop().ok_or_else(|| anyhow::anyhow!("Received no result from lambda response"))?;
                            break value;
                        }
                        Item::Stderr(line) => err_lines.push(line),
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
        future::Future,
        sync::{
            Arc,
            LazyLock,
        },
        time::Duration,
    };

    use cmd_util::env::config_test;
    use common::{
        assert_obj,
        execution_context::ExecutionContext,
        fastrace_helpers::EncodedSpan,
        log_lines::{
            run_function_and_collect_log_lines,
            LogLines,
        },
        runtime::Runtime,
        types::{
            ModuleEnvironment,
            UdfType,
        },
        value::ConvexValue,
        version::Version,
    };
    use errors::ErrorMetadataAnyhowExt;
    use futures::FutureExt;
    use isolate::test_helpers::TEST_SOURCE;
    use keybroker::{
        testing::TestUserIdentity,
        UserIdentity,
    };
    use maplit::btreemap;
    use model::{
        config::types::ModuleConfig,
        modules::{
            function_validators::{
                ArgsValidator,
                ReturnsValidator,
            },
            module_versions::{
                AnalyzedFunction,
                AnalyzedSourcePosition,
                SourceMap,
                Visibility,
            },
        },
        source_packages::upload_download::upload_package,
    };
    use runtime::prod::ProdRuntime;
    use serde_json::json;
    use storage::{
        LocalDirStorage,
        Storage,
    };
    use sync_types::{
        CanonicalizedModulePath,
        ModulePath,
    };
    use tokio::sync::mpsc;
    use udf::validation::ValidatedPathAndArgs;
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
            source_package_id: DeveloperDocumentId::MIN.into(),
            user_identity: None,
            auth_header: None,
            environment_variables: btreemap! {},
            callback_token: "".to_owned(),
            context: ExecutionContext::new_for_test(),
            encoded_parent_trace: EncodedSpan::from_parent().0,
        }
    }

    #[rustfmt::skip]
    async fn execute<RT: Runtime>(
        actions: &Actions<RT>,
        execute_request: ExecuteRequest,
        source_maps_callback: impl Future<Output = anyhow::Result<
            BTreeMap<CanonicalizedModulePath, SourceMap>>>
            + Send,
    ) -> anyhow::Result<(NodeActionOutcome, LogLines)> {
        let (log_line_sender, log_line_receiver) = mpsc::unbounded_channel();
        let execute_future = Box::pin(
            actions
                .execute(execute_request, log_line_sender, source_maps_callback)
                .fuse(),
        );
        let (result, log_lines) =
            run_function_and_collect_log_lines(execute_future, log_line_receiver, |_| {}).await;
        match result {
            Ok(outcome) => Ok((outcome, log_lines)),
            Err(e) => Err(e),
        }
    }

    fn create_actions<RT: Runtime>(rt: RT) -> Actions<RT> {
        Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT).unwrap()),
            TEST_BACKEND_ADDRESS.into(),
            TEST_USER_TIMEOUT,
            rt,
        )
    }

    async fn empty_source_maps_callback(
    ) -> anyhow::Result<BTreeMap<CanonicalizedModulePath, SourceMap>> {
        Ok(BTreeMap::new())
    }

    #[convex_macro::prod_rt_test]
    async fn test_success(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;

        let numbers: ConvexArray = array![1f64.into(), 7f64.into()]?;
        let args = create_args(assert_obj!("numbers" => ConvexValue::Array(numbers)))?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:addNumbers".parse()?,
            args,
            VERSION.clone(),
        );
        let (response, _log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
        )
        .await?;

        assert_eq!(response.result?, ConvexValue::from(8.));

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_log_lines(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:logHelloWorldAndReturn7".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
        )
        .await?;

        assert_eq!(response.result?, ConvexValue::from(7.));
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
                .collect::<Vec<_>>(),
            vec!["[INFO] 'Hello'".to_owned(), "[ERROR] 'World!'".to_owned()]
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_auth_syscall(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let identity = UserIdentity::test();

        let (response, _log_lines) = execute(
            &actions,
            ExecuteRequest {
                path_and_args: ValidatedPathAndArgs::new_for_tests(
                    "node_actions.js:getUserIdentity".parse()?,
                    array![],
                    VERSION.clone(),
                ),
                source_package,
                source_package_id: DeveloperDocumentId::MIN.into(),
                user_identity: Some(identity.attributes.clone()),
                auth_header: None,
                environment_variables: btreemap! {},
                callback_token: "".to_owned(),
                context: ExecutionContext::new_for_test(),
                encoded_parent_trace: None,
            },
            empty_source_maps_callback(),
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            "http://localhost:8719".into(),
            TEST_USER_TIMEOUT,
            rt,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;

        let args = create_args(assert_obj!(
            "name" =>  "getCounter.js"
        ))?;
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    "node_actions.js:runQuery".parse()?,
                    args,
                    VERSION.clone(),
                ),
                source_package,
            ),
            empty_source_maps_callback(),
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = Actions::new(
            Arc::new(LocalNodeExecutor::new(TEST_NODE_PROCESS_TIMEOUT)?),
            "http://localhost:8719".into(),
            TEST_USER_TIMEOUT,
            rt,
        );
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let args = create_args(assert_obj!(
            "name" =>  "getCounter.js"
        ))?;
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    "node_actions.js:scheduleJob".parse()?,
                    args,
                    VERSION.clone(),
                ),
                source_package,
            ),
            empty_source_maps_callback(),
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = TEST_SOURCE
            .clone()
            .into_iter()
            .map(|m| {
                (
                    m.path.canonicalize(),
                    m.source_map.expect("Missing source map"),
                )
            })
            .collect();
        let source_maps_callback = async { Ok(source_maps) };
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:logAndThrowError".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            source_maps_callback,
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
                .map(|l| l.to_pretty_string_test_only())
                .collect::<Vec<_>>(),
            vec!["[LOG] 'About to do something...'".to_owned()]
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_forgot_await(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let source_maps = TEST_SOURCE
            .clone()
            .into_iter()
            .map(|m| {
                (
                    m.path.canonicalize(),
                    m.source_map.expect("Missing source map"),
                )
            })
            .collect();
        let source_maps_callback = async { Ok(source_maps) };
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:forgotAwait".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            source_maps_callback,
        )
        .await?;

        assert_eq!(response.result?, ConvexValue::Null);
        assert_eq!(
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string_test_only())
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:hello".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, _log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let mut environment_variables = BTreeMap::new();
        environment_variables.insert("TEST_NAME".parse()?, "TEST_VALUE".parse()?);
        let (response, _log_lines) = execute(
            &actions,
            ExecuteRequest {
                path_and_args: ValidatedPathAndArgs::new_for_tests(
                    "node_actions.js:getTestEnvVar".parse()?,
                    array![],
                    VERSION.clone(),
                ),
                source_package,
                source_package_id: DeveloperDocumentId::MIN.into(),
                user_identity: None,
                auth_header: None,
                environment_variables,
                callback_token: "".to_owned(),
                context: ExecutionContext::new_for_test(),
                encoded_parent_trace: None,
            },
            empty_source_maps_callback(),
        )
        .await?;
        assert_eq!(response.result?, ConvexValue::try_from("TEST_VALUE")?);

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_user_timeout(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:sleepAnHour".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
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
                .map(|l| l.to_pretty_string_test_only())
                .collect::<Vec<_>>(),
            vec!["[LOG] 'I am very sleepy. I am going to take a nap.'".to_owned()]
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_partial_escape_sequence_result(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:partialEscapeSequence".parse()?,
            array![],
            VERSION.clone(),
        );
        let err = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
        )
        .await
        .unwrap_err();
        assert_eq!(err.short_msg(), "FunctionReturnInvalidJson");
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_process_timeout(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:workHardForAnHour".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
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
                .map(|l| l.to_pretty_string_test_only())
                .collect::<Vec<_>>(),
            vec!["[LOG] 'I am going to work really hard for 1 hour'".to_owned()]
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_deadlock(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;
        let path_and_args = ValidatedPathAndArgs::new_for_tests(
            "node_actions.js:deadlock".parse()?,
            array![],
            VERSION.clone(),
        );
        let (response, _log_lines) = execute(
            &actions,
            execute_request(path_and_args, source_package),
            empty_source_maps_callback(),
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
var actionGeneric = (func) => {
  const q = func;
  if (q.isRegistered) {
    throw new Error("Function registered twice " + func);
  }
  q.isRegistered = true;
  q.isAction = true;
  q.isPublic = true;
  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);
  q.exportArgs = () => `{ "type": "any" }`;
  q.exportReturns = () => `null`;
  return q;
};
var internalActionGeneric = (func) => {
  const q = func;
  if (q.isRegistered) {
    throw new Error("Function registered twice " + func);
  }
  q.isRegistered = true;
  q.isAction = true;
  q.isInternal = true;
  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);
  q.exportArgs = () => `{ "type": "any" }`;
  q.exportReturns = () => `null`;
  return q;
};
var actionWithStringArgNamedAAndStringReturnValue = (func) => {
  const q = actionGeneric(func);
  q.exportArgs = () => `{"type": "object", "value": {"a": {"fieldType": {"type": "string"}, "optional": false}}}`;
  q.exportReturns = () => `{"type": "string"}`;
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
var argsAndReturns = actionWithStringArgNamedAAndStringReturnValue(async ({}, { a }) => {
  console.log("analyze me pls");
  return a;
});
export {
  argsAndReturns,
  hello,
  internalHello
};
"#;

    // Generated via `npx esbuild static_node_source.js --bundle --format=esm
    // --target=esnext --sourcemap=linked --outfile=out.js`
    const SOURCE_MAP: &str = r#"
{
  "version": 3,
  "sources": ["static_node_source.js"],
  "sourcesContent": ["async function invokeAction(func, requestId, argsStr) {\n  throw new Error(\"unimplemented\");\n}\nvar actionGeneric = func => {\n  const q = func;\n  if (q.isRegistered) {\n    throw new Error(\"Function registered twice \" + func);\n  }\n  q.isRegistered = true;\n  q.isAction = true;\n  q.isPublic = true;\n  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n  q.exportArgs =  () => `{ \"type\": \"any\" }`\n  q.exportReturns =  () => `null`\n  return q;\n};\nvar internalActionGeneric = func => {\n  const q = func;\n  if (q.isRegistered) {\n    throw new Error(\"Function registered twice \" + func);\n  }\n  q.isRegistered = true;\n  q.isAction = true;\n  q.isInternal = true;\n  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n  q.exportArgs =  () => `{ \"type\": \"any\" }`\n  q.exportReturns =  () => `null`\n  return q;\n};\nvar actionWithStringArgNamedAAndStringReturnValue = func => {\n  const q = actionGeneric(func);\n  q.exportArgs = () => `{\"type\": \"object\", \"value\": {\"a\": {\"fieldType\": {\"type\": \"string\"}, \"optional\": false}}}`\n  q.exportReturns = () => `{\"type\": \"string\"}`\n  return q;\n}\nvar action = actionGeneric;\nvar internalAction = internalActionGeneric;\nvar hello = action(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nvar internalHello = internalAction(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nconst argsAndReturns = actionWithStringArgNamedAAndStringReturnValue(async ({}, {a}) => {\n  console.log(\"analyze me pls\");\n  return a;\n});\nexport { argsAndReturns, hello, internalHello };\n"],
  "mappings": ";AAAA,eAAe,aAAa,MAAM,WAAW,SAAS;AACpD,QAAM,IAAI,MAAM,eAAe;AACjC;AACA,IAAI,gBAAgB,UAAQ;AAC1B,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,WAAW;AACb,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,IAAE,aAAc,MAAM;AACtB,IAAE,gBAAiB,MAAM;AACzB,SAAO;AACT;AACA,IAAI,wBAAwB,UAAQ;AAClC,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,aAAa;AACf,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,IAAE,aAAc,MAAM;AACtB,IAAE,gBAAiB,MAAM;AACzB,SAAO;AACT;AACA,IAAI,gDAAgD,UAAQ;AAC1D,QAAM,IAAI,cAAc,IAAI;AAC5B,IAAE,aAAa,MAAM;AACrB,IAAE,gBAAgB,MAAM;AACxB,SAAO;AACT;AACA,IAAI,SAAS;AACb,IAAI,iBAAiB;AACrB,IAAI,QAAQ,OAAO,OAAO,CAAC,MAAM;AAC/B,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;AACD,IAAI,gBAAgB,eAAe,OAAO,CAAC,MAAM;AAC/C,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;AACD,IAAM,iBAAiB,8CAA8C,OAAO,CAAC,GAAG,EAAC,EAAC,MAAM;AACtF,UAAQ,IAAI,gBAAgB;AAC5B,SAAO;AACT,CAAC;",
  "names": []
}
"#;

    #[convex_macro::prod_rt_test]
    async fn test_analyze(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
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
        let path: CanonicalizedModulePath = "static_node_source.js".parse()?;
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
                AnalyzedFunction::new(
                    "hello".parse()?,
                    Some(AnalyzedSourcePosition {
                        path: "static_node_source.js".parse()?,
                        start_lineno: 38,
                        start_col: modules[&path].functions[0].pos.as_ref().unwrap().start_col,
                    }),
                    UdfType::Action,
                    Some(Visibility::Public),
                    ArgsValidator::Unvalidated,
                    ReturnsValidator::Unvalidated,
                )?,
                AnalyzedFunction::new(
                    "internalHello".parse()?,
                    Some(AnalyzedSourcePosition {
                        path: "static_node_source.js".parse()?,
                        start_lineno: 41,
                        start_col: modules[&path].functions[1].pos.as_ref().unwrap().start_col,
                    }),
                    UdfType::Action,
                    Some(Visibility::Internal),
                    ArgsValidator::Unvalidated,
                    ReturnsValidator::Unvalidated,
                )?,
                AnalyzedFunction::new(
                    "argsAndReturns".parse()?,
                    Some(AnalyzedSourcePosition {
                        path: "static_node_source.js".parse()?,
                        start_lineno: 44,
                        start_col: modules[&path].functions[2].pos.as_ref().unwrap().start_col,
                    }),
                    UdfType::Action,
                    Some(Visibility::Public),
                    ArgsValidator::try_from(json!({"type": "object", "value": {"a": {"fieldType": {"type": "string"}, "optional": false}}})).unwrap(),
                    ReturnsValidator::try_from(json!({"type": "string"})).unwrap()
                )?,
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
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
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let actions = create_actions(rt);
        let source_package = upload_modules(storage.clone(), TEST_SOURCE.clone()).await?;

        // First, try to execute an action with a syscall that fails. In this case,
        // we'll call into a query where the backend isn't actually running.
        let args = create_args(assert_obj!("name" =>  "getCounter.js"))?;
        let (response, _log_lines) = execute(
            &actions,
            execute_request(
                ValidatedPathAndArgs::new_for_tests(
                    "node_actions.js:runQuery".parse()?,
                    args,
                    VERSION.clone(),
                ),
                source_package.clone(),
            ),
            empty_source_maps_callback(),
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
                    "node_actions.js:getUserIdentity".parse()?,
                    array![],
                    VERSION.clone(),
                ),
                source_package,
            ),
            empty_source_maps_callback(),
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
