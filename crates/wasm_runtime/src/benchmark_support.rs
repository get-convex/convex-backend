use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
    sync::{
        Arc,
        Barrier,
    },
    time::{
        Duration,
        Instant,
    },
};

use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value,
};
use wasmtime::{
    Config,
    Engine,
    Instance,
    InstanceAllocationStrategy,
    InstancePre,
    Memory,
    Module,
    PoolingAllocationConfig,
    Store,
    TypedFunc,
};
use wizer::Wizer;

use crate::host::{
    new_linker,
    new_store,
    HostState,
    GUEST_BUNDLE_DIR,
};

#[derive(Clone)]
pub struct SetupTimings {
    pub bundle: Duration,
    pub guest_build: Duration,
    pub preinit: Duration,
}

#[derive(Clone)]
pub struct RequestBenchmarkResult {
    pub case: String,
    pub iterations_per_worker: usize,
    pub concurrency: usize,
    pub workers: usize,
    pub requests: usize,
    pub wall_total: Duration,
    pub instantiate_total: Duration,
    pub invoke_total: Duration,
}

pub struct FixtureArtifacts {
    pub fixture: String,
    pub output_dir: PathBuf,
    pub raw_wasm: PathBuf,
    pub preinitialized_wasm: Vec<u8>,
    pub bundle_time: Duration,
    pub guest_build_time: Duration,
    pub preinit_time: Duration,
}

const PREINITIALIZED_WASM_FILENAME: &str = "guest.preinitialized.wasm";
const SETUP_TIMINGS_FILENAME: &str = "setup-timings.json";

#[derive(Serialize, Deserialize)]
struct SetupTimingsFile {
    bundle_ms: f64,
    guest_build_ms: f64,
    preinit_ms: f64,
}

#[derive(Clone)]
pub enum BenchmarkScenario {
    Sync {
        handler: &'static str,
        args_json: &'static str,
    },
    CpuHeavy {
        work: u64,
    },
    FetchParallel {
        url: String,
        fanout: usize,
    },
    AsyncRoundTrip,
    AsyncFanout,
    SleepHost {
        ms: u64,
    },
}

impl BenchmarkScenario {
    fn handler_name(&self) -> &'static str {
        match self {
            Self::Sync { handler, .. } => handler,
            Self::CpuHeavy { .. } => "burn",
            Self::FetchParallel { .. } => "fetchInParallel",
            Self::AsyncRoundTrip => "roundTrip",
            Self::AsyncFanout => "fanout",
            Self::SleepHost { .. } => "sleepFor",
        }
    }

    fn args_json(&self) -> String {
        match self {
            Self::Sync { args_json, .. } => (*args_json).to_owned(),
            Self::CpuHeavy { work } => format!("[{work}]"),
            Self::FetchParallel { url, fanout } => {
                let urls = vec![url.clone(); *fanout];
                serde_json::to_string(&vec![urls]).expect("fetch benchmark args should serialize")
            },
            Self::AsyncRoundTrip => r#"["bench", {"name":"Jamie","count":1}]"#.to_owned(),
            Self::AsyncFanout => r#"[["x","y","z"]]"#.to_owned(),
            Self::SleepHost { ms } => format!("[{ms}]"),
        }
    }

    fn prepare(&self, harness: &mut GuestHarness) {
        if matches!(self, Self::AsyncFanout) {
            harness.seed_record("x", json!({"value": "x"}));
            harness.seed_record("y", json!({"value": "y"}));
            harness.seed_record("z", json!({"value": "z"}));
        }
    }
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `wasm-runtime-fixtures` package in `npm-packages`, whose fixtures the
/// build script bundles.
pub fn fixtures_package_dir() -> PathBuf {
    PathBuf::from(env!("WASM_RUNTIME_FIXTURES_DIR"))
}

/// Bundle a fixture rather than reading what the build script already bundled:
/// the benchmark reports bundling as part of the per-deploy setup cost, so it
/// has to do the work itself to time it.
pub fn bundle_fixture(fixture: &str, out_dir: &Path) -> Result<Duration, String> {
    let start = Instant::now();
    let status = Command::new("node")
        .arg("scripts/bundle-fixtures.mjs")
        .arg("--out-dir")
        .arg(out_dir)
        .arg(fixture)
        .current_dir(fixtures_package_dir())
        .status()
        .map_err(|error| format!("failed to execute bundle script: {error}"))?;

    if !status.success() {
        return Err(format!("bundle script failed for {fixture}"));
    }

    Ok(start.elapsed())
}

pub fn guest_wasm_path() -> PathBuf {
    workspace_root()
        .join("target")
        .join("bench")
        .join("guest_js")
        .join("wasm32-wasip1")
        .join("release")
        .join("guest_js.wasm")
}

/// Build the guest, which carries no fixture code: every fixture in a run
/// shares one build, so only the first one pays for it.
pub fn build_guest() -> Result<(PathBuf, Duration), String> {
    let target_dir = workspace_root()
        .join("target")
        .join("bench")
        .join("guest_js");
    let start = Instant::now();
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg("guest_js/Cargo.toml")
        .arg("--target")
        .arg("wasm32-wasip1")
        .arg("--target-dir")
        .arg(&target_dir)
        .current_dir(workspace_root())
        .status()
        .map_err(|error| format!("failed to compile guest_js: {error}"))?;

    if !status.success() {
        return Err("guest_js build failed".to_owned());
    }

    Ok((guest_wasm_path(), start.elapsed()))
}

/// Preinitialize the guest against the bundle in `bundle_dir`, which it reads
/// through the mapped directory while `wizer_initialize` runs. This is the
/// per-app step: the guest binary above is shared, the snapshot is not.
pub fn preinitialize_guest(
    wasm_path: &Path,
    bundle_dir: &Path,
) -> Result<(Vec<u8>, Duration), String> {
    let input_wasm = fs::read(wasm_path)
        .map_err(|error| format!("failed to read {}: {error}", wasm_path.display()))?;
    let start = Instant::now();
    let output = Wizer::new()
        .allow_wasi(true)
        .map_err(|error| format!("failed to enable WASI for Wizer: {error}"))?
        .map_dir(GUEST_BUNDLE_DIR, bundle_dir)
        .init_func("wizer_initialize")
        .run(&input_wasm)
        .map_err(|error| format!("Wizer failed: {error}"))?;
    Ok((output, start.elapsed()))
}

pub fn prepare_fixture(fixture: &str) -> Result<FixtureArtifacts, String> {
    let output_dir = artifact_output_dir(fixture);
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;

    let bundle_time = bundle_fixture(fixture, &output_dir)?;
    let (raw_wasm, guest_build_time) = build_guest()?;
    let (preinitialized_wasm, preinit_time) = preinitialize_guest(&raw_wasm, &output_dir)?;

    let preinitialized_path = output_dir.join(PREINITIALIZED_WASM_FILENAME);
    fs::write(&preinitialized_path, &preinitialized_wasm)
        .map_err(|error| format!("failed to write {}: {error}", preinitialized_path.display()))?;

    let setup_timings_path = output_dir.join(SETUP_TIMINGS_FILENAME);
    let setup_timings = SetupTimingsFile {
        bundle_ms: bundle_time.as_secs_f64() * 1_000.0,
        guest_build_ms: guest_build_time.as_secs_f64() * 1_000.0,
        preinit_ms: preinit_time.as_secs_f64() * 1_000.0,
    };
    fs::write(
        &setup_timings_path,
        serde_json::to_vec_pretty(&setup_timings)
            .map_err(|error| format!("failed to serialize setup timings: {error}"))?,
    )
    .map_err(|error| format!("failed to write {}: {error}", setup_timings_path.display()))?;

    Ok(FixtureArtifacts {
        fixture: fixture.to_owned(),
        output_dir,
        raw_wasm,
        preinitialized_wasm,
        bundle_time,
        guest_build_time,
        preinit_time,
    })
}

pub fn load_prepared_fixture(fixture: &str) -> Result<FixtureArtifacts, String> {
    let output_dir = artifact_output_dir(fixture);
    let raw_wasm = guest_wasm_path();
    let preinitialized_path = output_dir.join(PREINITIALIZED_WASM_FILENAME);
    let setup_timings_path = output_dir.join(SETUP_TIMINGS_FILENAME);

    let preinitialized_wasm = fs::read(&preinitialized_path)
        .map_err(|error| format!("failed to read {}: {error}", preinitialized_path.display()))?;
    let setup_timings: SetupTimingsFile =
        serde_json::from_slice(&fs::read(&setup_timings_path).map_err(|error| {
            format!("failed to read {}: {error}", setup_timings_path.display())
        })?)
        .map_err(|error| format!("failed to parse {}: {error}", setup_timings_path.display()))?;

    Ok(FixtureArtifacts {
        fixture: fixture.to_owned(),
        output_dir,
        raw_wasm,
        preinitialized_wasm,
        bundle_time: Duration::from_secs_f64(setup_timings.bundle_ms / 1_000.0),
        guest_build_time: Duration::from_secs_f64(setup_timings.guest_build_ms / 1_000.0),
        preinit_time: Duration::from_secs_f64(setup_timings.preinit_ms / 1_000.0),
    })
}

pub fn artifact_output_dir(fixture: &str) -> PathBuf {
    workspace_root()
        .join("target")
        .join("bench-artifacts")
        .join(fixture)
}

pub fn setup_timings(fixture: &FixtureArtifacts) -> SetupTimings {
    SetupTimings {
        bundle: fixture.bundle_time,
        guest_build: fixture.guest_build_time,
        preinit: fixture.preinit_time,
    }
}

pub struct GuestHarness {
    store: Store<HostState>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    dealloc: TypedFunc<(i32, i32), ()>,
    init: Option<TypedFunc<(), ()>>,
    invoke: TypedFunc<(i32, i32, i32, i32), i64>,
    start_invoke: TypedFunc<(i32, i32, i32, i32), i64>,
    poll_invoke: TypedFunc<(), i64>,
}

impl GuestHarness {
    pub fn new_from_path(wasm_path: &Path) -> Result<Self, String> {
        let bytes = fs::read(wasm_path)
            .map_err(|error| format!("failed to read {}: {error}", wasm_path.display()))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let engine = Engine::default();
        let module =
            Module::new(&engine, bytes).map_err(|error| format!("module load failed: {error}"))?;
        let linker = new_linker(&engine);
        let mut store = new_store(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|error| format!("instance creation failed: {error}"))?;
        Self::from_instance(store, instance)
    }

    fn from_instance(mut store: Store<HostState>, instance: Instance) -> Result<Self, String> {
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "memory export should exist".to_owned())?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|error| format!("alloc export missing: {error}"))?;
        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .map_err(|error| format!("dealloc export missing: {error}"))?;
        let init = instance
            .get_typed_func::<(), ()>(&mut store, "wizer_initialize")
            .ok();
        let invoke = instance
            .get_typed_func::<(i32, i32, i32, i32), i64>(&mut store, "invoke")
            .map_err(|error| format!("invoke export missing: {error}"))?;
        let start_invoke = instance
            .get_typed_func::<(i32, i32, i32, i32), i64>(&mut store, "start_invoke")
            .map_err(|error| format!("start_invoke export missing: {error}"))?;
        let poll_invoke = instance
            .get_typed_func::<(), i64>(&mut store, "poll_invoke")
            .map_err(|error| format!("poll_invoke export missing: {error}"))?;

        Ok(Self {
            store,
            memory,
            alloc,
            dealloc,
            init,
            invoke,
            start_invoke,
            poll_invoke,
        })
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        self.init
            .as_ref()
            .ok_or_else(|| "wizer_initialize export should exist".to_owned())?
            .call(&mut self.store, ())
            .map_err(|error| format!("wizer_initialize failed: {error}"))
    }

    pub fn seed_record(&mut self, key: &str, value: Value) {
        self.store.data_mut().seed_record(key, value);
    }

    pub fn invoke_json(&mut self, handler: &str, args_json: &str) -> Result<Value, String> {
        let handler_ptr = self.write_input(handler)?;
        let args_ptr = self.write_input(args_json)?;
        let packed = self
            .invoke
            .call(
                &mut self.store,
                (
                    handler_ptr,
                    handler.len() as i32,
                    args_ptr,
                    args_json.len() as i32,
                ),
            )
            .map_err(|error| format!("invoke failed: {error}"))?;

        let response = self.read_packed_string(packed)?;

        self.dealloc
            .call(&mut self.store, (handler_ptr, handler.len() as i32))
            .map_err(|error| format!("handler buffer dealloc failed: {error}"))?;
        self.dealloc
            .call(&mut self.store, (args_ptr, args_json.len() as i32))
            .map_err(|error| format!("args buffer dealloc failed: {error}"))?;

        serde_json::from_str(&response)
            .map_err(|error| format!("guest response was not JSON: {error}"))
    }

    pub fn start_invoke_json(&mut self, handler: &str, args_json: &str) -> Result<(), String> {
        let handler_ptr = self.write_input(handler)?;
        let args_ptr = self.write_input(args_json)?;
        let packed = self
            .start_invoke
            .call(
                &mut self.store,
                (
                    handler_ptr,
                    handler.len() as i32,
                    args_ptr,
                    args_json.len() as i32,
                ),
            )
            .map_err(|error| format!("start_invoke failed: {error}"))?;

        self.dealloc
            .call(&mut self.store, (handler_ptr, handler.len() as i32))
            .map_err(|error| format!("handler buffer dealloc failed: {error}"))?;
        self.dealloc
            .call(&mut self.store, (args_ptr, args_json.len() as i32))
            .map_err(|error| format!("args buffer dealloc failed: {error}"))?;

        if packed == 0 {
            return Ok(());
        }

        let response = self.read_packed_string(packed)?;
        Err(format!("start_invoke returned error payload: {response}"))
    }

    pub fn poll_invoke_json(&mut self) -> Result<Option<Value>, String> {
        let packed = self
            .poll_invoke
            .call(&mut self.store, ())
            .map_err(|error| format!("poll_invoke failed: {error}"))?;

        if packed == 0 {
            return Ok(None);
        }

        let response = self.read_packed_string(packed)?;
        let parsed = serde_json::from_str(&response)
            .map_err(|error| format!("guest response was not JSON: {error}"))?;
        Ok(Some(parsed))
    }

    fn write_input(&mut self, value: &str) -> Result<i32, String> {
        if value.is_empty() {
            return Ok(0);
        }

        let ptr = self
            .alloc
            .call(&mut self.store, value.len() as i32)
            .map_err(|error| format!("alloc failed: {error}"))?;
        self.memory
            .write(&mut self.store, ptr as usize, value.as_bytes())
            .map_err(|error| format!("memory write failed: {error}"))?;
        Ok(ptr)
    }

    fn read_packed_string(&mut self, packed: i64) -> Result<String, String> {
        let ptr = (packed as u64 & 0xffff_ffff) as usize;
        let len = ((packed as u64 >> 32) & 0xffff_ffff) as usize;
        if len == 0 {
            return Ok(String::new());
        }

        let mut bytes = vec![0_u8; len];
        self.memory
            .read(&mut self.store, ptr, &mut bytes)
            .map_err(|error| format!("memory read failed: {error}"))?;
        self.dealloc
            .call(&mut self.store, (ptr as i32, len as i32))
            .map_err(|error| format!("response buffer dealloc failed: {error}"))?;
        String::from_utf8(bytes).map_err(|error| format!("response utf8 decode failed: {error}"))
    }
}

pub struct FastInstantiationHarness {
    engine: Engine,
    instance_pre: InstancePre<HostState>,
}

impl FastInstantiationHarness {
    pub fn new(preinitialized_wasm: &[u8], capacity: usize) -> Result<Self, String> {
        let capacity = capacity.max(8);
        let mut pool = PoolingAllocationConfig::new();
        pool.total_core_instances(capacity as u32);
        pool.total_memories(capacity as u32);
        pool.total_tables(capacity as u32);
        pool.max_memory_size(64 << 20);
        pool.table_elements(1_000);

        let mut config = Config::new();
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        config.memory_init_cow(true);

        let engine =
            Engine::new(&config).map_err(|error| format!("pooled engine init failed: {error}"))?;
        let module = Module::new(&engine, preinitialized_wasm)
            .map_err(|error| format!("module compile failed: {error}"))?;
        module
            .initialize_copy_on_write_image()
            .map_err(|error| format!("copy-on-write image init failed: {error}"))?;

        let linker = new_linker(&engine);
        let instance_pre = linker
            .instantiate_pre(&module)
            .map_err(|error| format!("instance_pre failed: {error}"))?;

        Ok(Self {
            engine,
            instance_pre,
        })
    }

    pub fn instantiate(&self) -> Result<GuestHarness, String> {
        let mut store = new_store(&self.engine);
        let instance = self
            .instance_pre
            .instantiate(&mut store)
            .map_err(|error| format!("pooled instantiation failed: {error}"))?;

        GuestHarness::from_instance(store, instance)
    }
}

pub fn measure_fast_requests(
    fixture: &FixtureArtifacts,
    case: &str,
    scenario: BenchmarkScenario,
    iterations_per_worker: usize,
    concurrency: usize,
    workers: usize,
) -> Result<RequestBenchmarkResult, String> {
    let workers = workers.max(1).min(concurrency.max(1));
    let barrier = Arc::new(Barrier::new(workers + 1));
    let mut handles = Vec::with_capacity(workers);

    let base = concurrency / workers;
    let remainder = concurrency % workers;

    for worker_index in 0..workers {
        let local_concurrency = base + usize::from(worker_index < remainder);
        let barrier = Arc::clone(&barrier);
        let wasm = fixture.preinitialized_wasm.clone();
        let scenario = scenario.clone();
        let case = case.to_owned();

        handles.push(std::thread::spawn(
            move || -> Result<(Duration, Duration), String> {
                let fast = FastInstantiationHarness::new(&wasm, local_concurrency + 4)?;
                barrier.wait();

                let mut instantiate_total = Duration::ZERO;
                let mut invoke_total = Duration::ZERO;

                for _ in 0..iterations_per_worker {
                    let mut inflight = Vec::with_capacity(local_concurrency);

                    for _ in 0..local_concurrency {
                        let instantiate_start = Instant::now();
                        let mut harness = fast.instantiate()?;
                        instantiate_total += instantiate_start.elapsed();

                        scenario.prepare(&mut harness);
                        let args_json = scenario.args_json();
                        harness.start_invoke_json(scenario.handler_name(), &args_json)?;
                        inflight.push((harness, Instant::now()));
                    }

                    while !inflight.is_empty() {
                        let mut index = 0;
                        let mut completed_any = false;

                        while index < inflight.len() {
                            let (harness, started_at) = &mut inflight[index];
                            match harness.poll_invoke_json()? {
                                Some(response) => {
                                    if response["ok"] != Value::Bool(true) {
                                        return Err(format!(
                                            "unexpected response in {case}: {response}"
                                        ));
                                    }
                                    invoke_total += started_at.elapsed();
                                    inflight.swap_remove(index);
                                    completed_any = true;
                                },
                                None => {
                                    index += 1;
                                },
                            }
                        }

                        if !completed_any {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                }

                Ok((instantiate_total, invoke_total))
            },
        ));
    }

    let wall_start = Instant::now();
    barrier.wait();

    let mut instantiate_total = Duration::ZERO;
    let mut invoke_total = Duration::ZERO;
    for handle in handles {
        let (worker_instantiate, worker_invoke) = handle
            .join()
            .map_err(|_| format!("benchmark worker panicked for {case}"))??;
        instantiate_total += worker_instantiate;
        invoke_total += worker_invoke;
    }

    let requests = iterations_per_worker * concurrency;
    Ok(RequestBenchmarkResult {
        case: format!("{}/{}", fixture.fixture, case),
        iterations_per_worker,
        concurrency,
        workers,
        requests,
        wall_total: wall_start.elapsed(),
        instantiate_total,
        invoke_total,
    })
}
