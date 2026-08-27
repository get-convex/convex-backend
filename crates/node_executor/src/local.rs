use std::{
    fs,
    path::PathBuf,
    process::{
        Child,
        Command,
    },
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use common::log_lines::LogLine;
use errors::ErrorMetadata;
use futures::{
    select_biased,
    FutureExt,
};
use futures_async_stream::try_stream;
use isolate::bundled_js::node_executor_file;
use rand::Rng;
use reqwest::Client;
use serde_json::Value as JsonValue;
use tempfile::TempDir;
use tokio::{
    process::Command as TokioCommand,
    sync::{
        mpsc,
        Mutex,
    },
};

#[cfg(windows)]
use crate::windows_job::KillOnCloseJob;
use crate::{
    executor::{
        ExecutorRequest,
        InvokeResponse,
        NodeExecutor,
        ARGS_TOO_LARGE_RESPONSE_MESSAGE,
        EXECUTE_TIMEOUT_RESPONSE_JSON,
    },
    handle_node_executor_stream,
    NodeExecutorStreamPart,
};

const NVMRC_VERSION: &str = include_str!("../../../.nvmrc");
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(100);
const MAX_HEALTH_CHECK_ATTEMPTS: u32 = 50;
const TEMP_DIR_CLEANUP_INTERVAL: Duration = Duration::from_millis(50);
const TEMP_DIR_CLEANUP_MAX_ATTEMPTS: u32 = 40;

pub struct LocalNodeExecutor {
    inner: Arc<Mutex<Option<InnerLocalNodeExecutor>>>,
    config: LocalNodeExecutorConfig,
}

struct LocalNodeExecutorConfig {
    node_process_timeout: Duration,
    /// Overrides the initial callback retry backoff in the spawned node
    /// process (read by syscalls.ts at module load). Tests zero this so
    /// callbacks retrying against an unreachable backend settle within test
    /// timeouts.
    callback_initial_backoff: Option<Duration>,
}

struct InnerLocalNodeExecutor {
    // Stop and reap Node before removing the directory it uses for source and
    // runtime files. This order matters on Windows, where open files prevent
    // recursive directory removal.
    _server_handle: NodeExecutorProcess,
    _source_dir: TempDir,
    client: reqwest::Client,
}

struct NodeExecutorProcess {
    child: Child,
    #[cfg(windows)]
    _job: KillOnCloseJob,
}

impl Drop for NodeExecutorProcess {
    fn drop(&mut self) {
        // std::process::Child does not kill or reap on drop. The local executor
        // is owned by this backend, so stop it before TempDir cleanup runs.
        let _ = self.shutdown();
    }
}

impl NodeExecutorProcess {
    fn shutdown(&mut self) -> anyhow::Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill().context("kill local Node executor")?;
        }
        self.child.wait().context("reap local Node executor")?;
        Ok(())
    }
}

impl InnerLocalNodeExecutor {
    async fn new(config: &LocalNodeExecutorConfig) -> anyhow::Result<Self> {
        tracing::info!("Initializing inner local node executor");
        // Create a single temp directory for both source files and Node.js temp files
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

        let socket_path = if cfg!(unix) {
            source_dir.path().join(".executor.sock")
        } else if cfg!(windows) {
            PathBuf::from(format!(
                r"\\.\pipe\cvx-node-executor-{:016x}",
                rand::rng().random::<u64>()
            ))
        } else {
            panic!("not supported");
        };
        let server_handle =
            Self::start_node_with_listener(config, &source_path, &source_dir, &socket_path).await?;
        // Don't keep idle connections in the pool. The Node HTTP server closes
        // idle keep-alive connections after its (default 5s) `keepAliveTimeout`,
        // but hyper's pool would hold one much longer and reuse it right as the
        // server closes it, surfacing as a spurious "connection reset by peer".
        // Opening a fresh connection per request is cheap over a local socket.
        let mut client_builder = Client::builder().pool_max_idle_per_host(0);
        #[cfg(unix)]
        {
            client_builder = client_builder.unix_socket(socket_path);
        }
        #[cfg(windows)]
        {
            client_builder = client_builder.windows_named_pipe(socket_path);
        }
        let client = client_builder.build()?;

        // Wait for the Node process to be ready to handle HTTP requests.
        for _ in 0..MAX_HEALTH_CHECK_ATTEMPTS {
            if Self::check_server_health(&client).await? {
                return Ok(Self {
                    _server_handle: server_handle,
                    _source_dir: source_dir,
                    client,
                });
            }
            tokio::time::sleep(HEALTH_CHECK_INTERVAL).await;
        }
        anyhow::bail!("Node executor server failed to start and become healthy")
    }

    async fn check_node_version(node_path: &str) -> anyhow::Result<()> {
        let cmd = TokioCommand::new(node_path)
            .arg("--version")
            .output()
            .await?;
        let version = String::from_utf8_lossy(&cmd.stdout);

        if !version.starts_with("v20.")
            && !version.starts_with("v22.")
            && !version.starts_with("v24.")
        {
            anyhow::bail!(ErrorMetadata::bad_request(
                "DeploymentNotConfiguredForNodeActions",
                "Deployment is not configured to deploy \"use node\" actions. \
                 Node.js v20, 22, or 24 is not installed. \
                 Install a supported Node.js version with nvm (https://github.com/nvm-sh/nvm) \
                 to deploy Node.js actions."
            ))
        }
        Ok(())
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        let Self {
            mut _server_handle,
            _source_dir,
            client,
        } = self;
        let source_dir_path = _source_dir.path().to_owned();

        // Release the named-pipe client before stopping Node, then wait for the
        // process handle to close before removing the files it was using.
        drop(client);
        _server_handle.shutdown()?;
        drop(_server_handle);

        if _source_dir.close().is_ok() {
            return Ok(());
        }

        // Windows may report a transient sharing violation just after the
        // process exits. TempDir's Drop suppresses that error, which made old
        // local-dev sessions silently leak. Retry within a fixed bound and
        // make persistent cleanup failure part of backend shutdown.
        for _ in 0..TEMP_DIR_CLEANUP_MAX_ATTEMPTS {
            match fs::remove_dir_all(&source_dir_path) {
                Ok(()) => return Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(_) => tokio::time::sleep(TEMP_DIR_CLEANUP_INTERVAL).await,
            }
        }

        fs::remove_dir_all(&source_dir_path).with_context(|| {
            format!(
                "remove local Node executor temporary directory {}",
                source_dir_path.display()
            )
        })
    }

    async fn check_server_health(client: &Client) -> anyhow::Result<bool> {
        match client
            .get("http://localhost/health".to_string())
            .timeout(Duration::from_secs(1))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => Ok(true),
            _ => Ok(false),
        }
    }

    async fn start_node_with_listener(
        config: &LocalNodeExecutorConfig,
        source_path: &PathBuf,
        temp_dir: &TempDir,
        socket_path: &PathBuf,
    ) -> anyhow::Result<NodeExecutorProcess> {
        let preferred_node_version = NVMRC_VERSION.trim();

        // Look for node in a few places.
        let possible_path = home::home_dir()
            .unwrap()
            .join(".nvm")
            .join(format!("versions/node/v{preferred_node_version}/bin/node"));
        let node_path = if possible_path.exists() {
            possible_path.to_string_lossy().to_string()
        } else {
            "node".to_string()
        };
        Self::check_node_version(&node_path).await?;

        let mut cmd = Command::new(node_path);
        cmd.arg(source_path)
            .arg("--ipc-path")
            .arg(socket_path)
            .arg("--tempdir")
            .arg(temp_dir.path());
        if let Some(backoff) = config.callback_initial_backoff {
            cmd.env(
                "CALLBACK_INITIAL_BACKOFF_MS",
                backoff.as_millis().to_string(),
            );
        }

        #[cfg(windows)]
        let job = KillOnCloseJob::new().context("Failed to create Node executor job object")?;

        let mut child = cmd.spawn()?;

        #[cfg(windows)]
        if let Err(error) = job.assign(&child) {
            // Assignment is part of spawning a managed executor. Do not leave
            // an unmanaged process running if Windows rejects the job.
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("Failed to assign Node executor to job object");
        }

        Ok(NodeExecutorProcess {
            child,
            #[cfg(windows)]
            _job: job,
        })
    }
}

impl LocalNodeExecutor {
    pub async fn new(node_process_timeout: Duration) -> anyhow::Result<Self> {
        let executor = Self {
            inner: Arc::new(Mutex::new(None)),
            config: LocalNodeExecutorConfig {
                node_process_timeout,
                callback_initial_backoff: None,
            },
        };

        Ok(executor)
    }

    #[try_stream(ok = NodeExecutorStreamPart, error = anyhow::Error)]
    async fn response_stream(config: &LocalNodeExecutorConfig, mut response: reqwest::Response) {
        let mut timeout_future = Box::pin(tokio::time::sleep(config.node_process_timeout));
        let timeout_future = &mut timeout_future;
        loop {
            let process_chunk = async {
                select_biased! {
                    chunk = response.chunk().fuse() => {
                        let chunk = chunk?;
                        match chunk {
                            Some(chunk) => {
                                anyhow::Ok(NodeExecutorStreamPart::Chunk(chunk))
                            }
                            None => {
                                anyhow::Ok(NodeExecutorStreamPart::InvokeComplete(Ok(())))
                            }
                        }
                    },
                    _ = timeout_future.fuse() => {
                        anyhow::Ok(NodeExecutorStreamPart::InvokeComplete(Err(InvokeResponse {
                            response: EXECUTE_TIMEOUT_RESPONSE_JSON.clone(),
                            aws_request_id: None,
                        })))
                    },
                }
            };
            let part = process_chunk.await?;
            if let NodeExecutorStreamPart::InvokeComplete(_) = part {
                yield part;
                break;
            } else {
                yield part;
            }
        }
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
        let client = {
            let mut inner = self.inner.lock().await;
            if inner.is_none() {
                *inner = Some(
                    InnerLocalNodeExecutor::new(&self.config)
                        .await
                        .context("Failed to create inner local node executor")?,
                )
            }
            let inner = inner.as_ref().unwrap();
            inner.client.clone()
        };
        let request_json = JsonValue::try_from(request)?;

        let response_result = client
            .post("http://localhost/invoke".to_string())
            .json(&request_json)
            .timeout(self.config.node_process_timeout)
            .send()
            .await;
        let response = match response_result {
            Ok(response) => response,
            Err(e) => {
                if e.is_timeout() {
                    return Ok(InvokeResponse {
                        response: EXECUTE_TIMEOUT_RESPONSE_JSON.clone(),
                        aws_request_id: None,
                    });
                } else if e.is_connect() {
                    // Connection error likely means the Node server crashed (e.g., OOM).
                    // Drop the dead server so it will be restarted on next invoke.
                    tracing::warn!("Node server connection failed, dropping server: {e}");
                    self.inner.lock().await.take();
                    return Err(anyhow::anyhow!(e).context("Node server request failed"));
                } else {
                    return Err(anyhow::anyhow!(e).context("Node server request failed"));
                }
            },
        };

        if let Err(e) = response.error_for_status_ref() {
            if e.status() == Some(reqwest::StatusCode::PAYLOAD_TOO_LARGE) {
                return Err(
                    anyhow::anyhow!(e.without_url()).context(ErrorMetadata::bad_request(
                        "ArgsTooLarge",
                        ARGS_TOO_LARGE_RESPONSE_MESSAGE,
                    )),
                );
            }
            let error = response.text().await?;
            anyhow::bail!("Node executor server returned error: {}", error);
        }
        let stream = Self::response_stream(&self.config, response);
        let stream = Box::pin(stream);
        let result = handle_node_executor_stream(log_line_sender, stream).await?;
        match result {
            Ok(payload) => {
                if payload
                    .get("exitingProcess")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    // Drop the server if it claims to be exiting.
                    self.inner.lock().await.take();
                }
                Ok(InvokeResponse {
                    response: payload,
                    aws_request_id: None,
                })
            },
            Err(e) => Ok(e),
        }
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        let inner = self.inner.lock().await.take();
        match inner {
            Some(inner) => inner.shutdown().await,
            None => Ok(()),
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::{
        env,
        fs,
        io::Read,
        process::Stdio,
        thread,
        time::Instant,
    };

    use super::*;
    use crate::windows_job::{
        is_process_running,
        terminate_process,
    };

    const BACKEND_HELPER: &str = "CONVEX_TEST_LOCAL_EXECUTOR_BACKEND_HELPER";
    const LAUNCHER_HELPER: &str = "CONVEX_TEST_LOCAL_EXECUTOR_LAUNCHER_HELPER";
    const READY_FILE: &str = "CONVEX_TEST_LOCAL_EXECUTOR_READY_FILE";
    const CLEAN_FILE: &str = "CONVEX_TEST_LOCAL_EXECUTOR_CLEAN_FILE";

    #[tokio::test]
    async fn ordinary_drop_stops_executor_and_removes_temp_dir() {
        let config = LocalNodeExecutorConfig {
            node_process_timeout: Duration::from_secs(5),
            callback_initial_backoff: Some(Duration::ZERO),
        };
        let inner = InnerLocalNodeExecutor::new(&config)
            .await
            .expect("start local Node executor");
        let source_dir = inner._source_dir.path().to_owned();
        let client = inner.client.clone();

        drop(inner);

        assert!(
            !source_dir.exists(),
            "executor temporary directory survived ordinary shutdown: {}",
            source_dir.display(),
        );
        assert!(
            client
                .get("http://localhost/health")
                .timeout(Duration::from_secs(1))
                .send()
                .await
                .is_err(),
            "executor named pipe still accepted requests after shutdown",
        );
    }

    #[tokio::test]
    async fn explicit_shutdown_stops_executor_and_removes_temp_dir() {
        let executor = LocalNodeExecutor::new(Duration::from_secs(5))
            .await
            .expect("construct local Node executor");
        let inner = InnerLocalNodeExecutor::new(&executor.config)
            .await
            .expect("start local Node executor");
        let source_dir = inner._source_dir.path().to_owned();
        let client = inner.client.clone();
        *executor.inner.lock().await = Some(inner);

        executor
            .shutdown()
            .await
            .expect("shutdown local Node executor");

        assert!(
            !source_dir.exists(),
            "executor temporary directory survived explicit shutdown: {}",
            source_dir.display(),
        );
        assert!(
            client
                .get("http://localhost/health")
                .timeout(Duration::from_secs(1))
                .send()
                .await
                .is_err(),
            "executor named pipe still accepted requests after explicit shutdown",
        );
    }

    #[tokio::test]
    async fn local_executor_backend_helper() {
        if env::var_os(BACKEND_HELPER).is_none() {
            return;
        }

        let ready_file = PathBuf::from(env::var_os(READY_FILE).expect("missing ready file"));
        let clean_file = PathBuf::from(env::var_os(CLEAN_FILE).expect("missing clean file"));
        let config = LocalNodeExecutorConfig {
            node_process_timeout: Duration::from_secs(5),
            callback_initial_backoff: Some(Duration::ZERO),
        };
        let inner = InnerLocalNodeExecutor::new(&config)
            .await
            .expect("start local Node executor");
        let source_dir = inner._source_dir.path().to_owned();
        let executor_pid = inner._server_handle.child.id();
        let client = inner.client.clone();
        fs::write(
            ready_file,
            format!(
                "{}\n{}\n{}\n",
                std::process::id(),
                executor_pid,
                source_dir.display()
            ),
        )
        .expect("publish executor ownership");

        tokio::task::spawn_blocking(|| {
            let mut stdin = std::io::stdin();
            let mut buffer = [0; 256];
            while stdin.read(&mut buffer).expect("read launcher pipe") != 0 {}
        })
        .await
        .expect("join stdin reader");

        drop(inner);
        assert!(
            !source_dir.exists(),
            "executor temporary directory survived launcher death: {}",
            source_dir.display(),
        );
        assert!(
            !InnerLocalNodeExecutor::check_server_health(&client)
                .await
                .expect("check executor health"),
            "executor named pipe survived launcher death",
        );
        fs::write(
            clean_file,
            "executor_process=stopped\nnamed_pipe=closed\ntemp_dir=removed\n",
        )
        .expect("publish clean shutdown");
    }

    #[test]
    fn local_executor_launcher_helper() {
        if env::var_os(LAUNCHER_HELPER).is_none() {
            return;
        }

        let mut backend = Command::new(env::current_exe().expect("locate test executable"))
            .args([
                "--exact",
                "local::tests::local_executor_backend_helper",
                "--nocapture",
            ])
            .env(BACKEND_HELPER, "1")
            .env(
                READY_FILE,
                env::var_os(READY_FILE).expect("missing ready file"),
            )
            .env(
                CLEAN_FILE,
                env::var_os(CLEAN_FILE).expect("missing clean file"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn backend helper");

        loop {
            assert!(
                backend
                    .try_wait()
                    .expect("inspect backend helper")
                    .is_none(),
                "backend helper exited before launcher termination",
            );
            thread::sleep(Duration::from_secs(60));
        }
    }

    #[test]
    fn force_killing_launcher_triggers_clean_executor_shutdown() {
        let temp_dir = TempDir::new().expect("create test directory");
        let ready_file = temp_dir.path().join("ready.txt");
        let clean_file = temp_dir.path().join("clean.txt");
        let mut launcher = Command::new(env::current_exe().expect("locate test executable"))
            .args([
                "--exact",
                "local::tests::local_executor_launcher_helper",
                "--nocapture",
            ])
            .env(LAUNCHER_HELPER, "1")
            .env(READY_FILE, &ready_file)
            .env(CLEAN_FILE, &clean_file)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn launcher helper");

        let (backend_pid, executor_pid) = wait_for_owned_processes(&ready_file);
        assert!(
            is_process_running(backend_pid),
            "backend helper never started"
        );
        assert!(
            is_process_running(executor_pid),
            "Node executor never started"
        );

        // Child::kill uses TerminateProcess on Windows. The launcher cannot run
        // cleanup, so the backend must observe stdin EOF and own the teardown.
        launcher.kill().expect("force kill launcher helper");
        launcher.wait().expect("reap launcher helper");

        let deadline = Instant::now() + Duration::from_secs(10);
        while !clean_file.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(25));
        }
        if !clean_file.exists() {
            terminate_process(executor_pid);
            terminate_process(backend_pid);
            panic!("backend did not publish clean shutdown after launcher death");
        }

        while (is_process_running(backend_pid) || is_process_running(executor_pid))
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(25));
        }
        if is_process_running(executor_pid) || is_process_running(backend_pid) {
            terminate_process(executor_pid);
            terminate_process(backend_pid);
            panic!("backend or executor survived launcher death");
        }

        assert_eq!(
            fs::read_to_string(clean_file).expect("read shutdown evidence"),
            "executor_process=stopped\nnamed_pipe=closed\ntemp_dir=removed\n",
        );
    }

    fn wait_for_owned_processes(ready_file: &std::path::Path) -> (u32, u32) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(contents) = fs::read_to_string(ready_file) {
                let mut lines = contents.lines();
                let backend_pid = lines
                    .next()
                    .expect("backend PID")
                    .parse()
                    .expect("valid backend PID");
                let executor_pid = lines
                    .next()
                    .expect("executor PID")
                    .parse()
                    .expect("valid executor PID");
                return (backend_pid, executor_pid);
            }
            assert!(
                Instant::now() < deadline,
                "backend did not publish executor ownership",
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
}
