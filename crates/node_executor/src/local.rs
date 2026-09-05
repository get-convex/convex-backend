use std::{
    fs,
    path::PathBuf,
    process::{
        Child,
        Command,
    },
    sync::{
        Arc,
        Mutex as StdMutex,
    },
    thread::JoinHandle,
    time::{
        Duration,
        Instant,
    },
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
    InvokeCompletion,
    NodeExecutorStreamPart,
};

const NVMRC_VERSION: &str = include_str!("../../../.nvmrc");
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(100);
const MAX_HEALTH_CHECK_ATTEMPTS: u32 = 50;
const TEMP_DIR_CLEANUP_INTERVAL: Duration = Duration::from_millis(50);
const TEMP_DIR_CLEANUP_MAX_ATTEMPTS: u32 = 40;
const PROCESS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const PROCESS_SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub struct LocalNodeExecutor {
    inner: Arc<Mutex<Option<InnerLocalNodeExecutor>>>,
    config: LocalNodeExecutorConfig,
}

struct LocalNodeExecutorConfig {
    node_process_timeout: Duration,
    cleanup_jobs: Arc<CleanupJobs>,
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
    _server_handle: Option<NodeExecutorProcess>,
    _source_dir: Option<TempDir>,
    client: Option<reqwest::Client>,
    cleanup_jobs: Arc<CleanupJobs>,
}

struct NodeExecutorProcess {
    child: Option<Child>,
    #[cfg(windows)]
    _job: Option<KillOnCloseJob>,
}

#[derive(Default)]
struct CleanupJobs {
    jobs: StdMutex<Vec<JoinHandle<anyhow::Result<()>>>>,
}

impl CleanupJobs {
    fn spawn(&self, process: Option<NodeExecutorProcess>, source_dir: Option<TempDir>) {
        let job = std::thread::Builder::new()
            .name("local-node-executor-cleanup".to_owned())
            .spawn(move || cleanup_owned_resources(process, source_dir))
            .expect("spawn local Node executor cleanup thread");
        self.jobs.lock().expect("cleanup jobs lock").push(job);
    }

    fn wait(&self) -> anyhow::Result<()> {
        let jobs = std::mem::take(&mut *self.jobs.lock().expect("cleanup jobs lock"));
        let mut result = Ok(());
        for job in jobs {
            let job_result = job
                .join()
                .map_err(|_| anyhow::anyhow!("local Node executor cleanup thread panicked"))
                .and_then(|result| result);
            if result.is_ok() && job_result.is_err() {
                result = job_result;
            }
        }
        result
    }
}

impl Drop for CleanupJobs {
    fn drop(&mut self) {
        if let Err(error) = self.wait() {
            tracing::error!("Local Node executor cleanup failed during drop: {error:#}");
        }
    }
}

impl Drop for NodeExecutorProcess {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        #[cfg(windows)]
        let job = self._job.take();
        // Drop can run on a Tokio worker after an invoke error. Keep blocking
        // process operations off that worker; the job remains owned by the
        // cleanup thread until the child has stopped.
        let _ = std::thread::Builder::new()
            .name("local-node-executor-drop".to_owned())
            .spawn(move || {
                let _ = shutdown_child(&mut child);
                #[cfg(windows)]
                drop(job);
            });
    }
}

impl NodeExecutorProcess {
    fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            shutdown_child(&mut child)?;
        }
        #[cfg(windows)]
        drop(self._job.take());
        Ok(())
    }
}

fn shutdown_child(child: &mut Child) -> anyhow::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    child.kill().context("kill local Node executor")?;
    let deadline = Instant::now() + PROCESS_SHUTDOWN_TIMEOUT;
    loop {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("timed out reaping local Node executor after termination");
        }
        std::thread::sleep(PROCESS_SHUTDOWN_POLL_INTERVAL);
    }
}

impl Drop for InnerLocalNodeExecutor {
    fn drop(&mut self) {
        drop(self.client.take());
        let process = self._server_handle.take();
        let source_dir = self._source_dir.take();
        if process.is_none() && source_dir.is_none() {
            return;
        }
        self.cleanup_jobs.spawn(process, source_dir);
    }
}

fn cleanup_owned_resources(
    mut process: Option<NodeExecutorProcess>,
    source_dir: Option<TempDir>,
) -> anyhow::Result<()> {
    let process_result = match process.as_mut() {
        Some(process) => process.shutdown(),
        None => Ok(()),
    };
    // Release the Windows Job Object before retrying directory removal if
    // process shutdown failed. Closing the job still terminates its process
    // tree, and cleanup must not retain that ownership handle while it waits.
    drop(process);
    let source_dir_result = match source_dir {
        Some(source_dir) => cleanup_temp_dir_blocking(source_dir),
        None => Ok(()),
    };
    match (process_result, source_dir_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(process_error), Err(source_dir_error)) => Err(process_error.context(format!(
            "local Node executor directory cleanup also failed: {source_dir_error:#}"
        ))),
    }
}

fn cleanup_temp_dir_blocking(source_dir: TempDir) -> anyhow::Result<()> {
    let source_dir_path = source_dir.path().to_owned();
    if source_dir.close().is_ok() {
        return Ok(());
    }
    for _ in 0..TEMP_DIR_CLEANUP_MAX_ATTEMPTS {
        match fs::remove_dir_all(&source_dir_path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => std::thread::sleep(TEMP_DIR_CLEANUP_INTERVAL),
        }
    }
    fs::remove_dir_all(&source_dir_path).with_context(|| {
        format!(
            "remove local Node executor temporary directory {}",
            source_dir_path.display()
        )
    })
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
        // Assemble the owned resource boundary immediately after the process
        // starts so every later startup failure follows the same cleanup path.
        let mut inner = Self {
            _server_handle: Some(server_handle),
            _source_dir: Some(source_dir),
            client: None,
            cleanup_jobs: config.cleanup_jobs.clone(),
        };
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
        inner.client = Some(client_builder.build()?);
        // Wait for the Node process to be ready to handle HTTP requests.
        for _ in 0..MAX_HEALTH_CHECK_ATTEMPTS {
            if Self::check_server_health(inner.client.as_ref().expect("client present")).await? {
                return Ok(inner);
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

    async fn shutdown(mut self) -> anyhow::Result<()> {
        drop(self.client.take());
        let process = self._server_handle.take();
        let source_dir = self._source_dir.take();
        tokio::task::spawn_blocking(move || cleanup_owned_resources(process, source_dir))
            .await
            .context("join local Node executor cleanup")?
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

        #[cfg(windows)]
        let child = job
            .spawn_assigned(&mut cmd)
            .context("Failed to spawn Node executor in job object")?;
        #[cfg(not(windows))]
        let child = cmd.spawn()?;

        Ok(NodeExecutorProcess {
            child: Some(child),
            #[cfg(windows)]
            _job: Some(job),
        })
    }
}

impl LocalNodeExecutor {
    pub async fn new(node_process_timeout: Duration) -> anyhow::Result<Self> {
        let cleanup_jobs = Arc::new(CleanupJobs::default());
        let executor = Self {
            inner: Arc::new(Mutex::new(None)),
            config: LocalNodeExecutorConfig {
                node_process_timeout,
                callback_initial_backoff: None,
                cleanup_jobs,
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
                                anyhow::Ok(NodeExecutorStreamPart::InvokeComplete(
                                    InvokeCompletion::Success,
                                ))
                            }
                        }
                    },
                    _ = timeout_future.fuse() => {
                        anyhow::Ok(NodeExecutorStreamPart::InvokeComplete(
                            InvokeCompletion::ExplicitError(InvokeResponse {
                                response: EXECUTE_TIMEOUT_RESPONSE_JSON.clone(),
                                aws_request_id: None,
                            }),
                        ))
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
            inner.client.as_ref().expect("client present").clone()
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
        if let Some(inner) = inner {
            inner.shutdown().await?;
        }
        let cleanup_jobs = self.config.cleanup_jobs.clone();
        tokio::task::spawn_blocking(move || cleanup_jobs.wait())
            .await
            .context("join local Node executor pending cleanup")??;
        Ok(())
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
            cleanup_jobs: Arc::new(CleanupJobs::default()),
        };
        let inner = InnerLocalNodeExecutor::new(&config)
            .await
            .expect("start local Node executor");
        let source_dir = inner._source_dir.as_ref().unwrap().path().to_owned();
        let client = inner.client.as_ref().unwrap().clone();

        drop(inner);

        wait_for_path_removal(&source_dir).await;

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

    #[test]
    fn pending_cleanup_is_joined_before_shutdown_returns() {
        let cleanup_jobs = CleanupJobs::default();
        let source_dir = TempDir::new().expect("create executor temporary directory");
        let source_dir_path = source_dir.path().to_owned();

        cleanup_jobs.spawn(None, Some(source_dir));
        cleanup_jobs.wait().expect("join executor cleanup");

        assert!(
            !source_dir_path.exists(),
            "executor temporary directory survived joined cleanup: {}",
            source_dir_path.display(),
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
        let source_dir = inner._source_dir.as_ref().unwrap().path().to_owned();
        let client = inner.client.as_ref().unwrap().clone();
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
            cleanup_jobs: Arc::new(CleanupJobs::default()),
        };
        let inner = InnerLocalNodeExecutor::new(&config)
            .await
            .expect("start local Node executor");
        let source_dir = inner._source_dir.as_ref().unwrap().path().to_owned();
        let executor_pid = inner
            ._server_handle
            .as_ref()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .id();
        let client = inner.client.as_ref().unwrap().clone();
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
        wait_for_path_removal(&source_dir).await;
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

    async fn wait_for_path_removal(path: &std::path::Path) {
        let deadline = Instant::now() + PROCESS_SHUTDOWN_TIMEOUT + Duration::from_secs(1);
        while path.exists() && Instant::now() < deadline {
            tokio::time::sleep(PROCESS_SHUTDOWN_POLL_INTERVAL).await;
        }
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
