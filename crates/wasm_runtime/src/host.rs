//! The `convex_host` ABI.
//!
//! The guest sees four generic imports rather than one import per operation:
//!
//! ```text
//! syscall(name, args, out) -> i64
//! start_async_syscall(name, args) -> i32
//! completed_ops(out) -> i64
//! take_op_result(op_id, out) -> i64
//! ```
//!
//! Everything else is dispatched by name against [`HostState::syscall`] and
//! [`HostState::start_async_syscall`], mirroring `IsolateEnvironment::syscall`
//! in the convex isolate crate. Adding a syscall is a host-side table entry
//! plus a JS shim; it never requires rebuilding the guest ABI.
//!
//! Arguments are always a JSON array. Results are returned by the host
//! allocating a buffer in guest memory through the guest's own `alloc` export,
//! copying the payload in, and returning a packed `(ptr, len)` — the same shape
//! `invoke` uses in the other direction, and what the component model's
//! canonical ABI does via `cabi_realloc`. Ownership of the buffer transfers to
//! the guest, which frees it.
//!
//! Sizing is exact and every call is a single round trip. A negative return is
//! one of the `SYSCALL_*` codes; a non-negative one packs `len` in the high 32
//! bits and `ptr` in the low 32.

use std::{
    collections::BTreeMap,
    path::Path,
    time::{
        Duration,
        Instant,
    },
};

use crossbeam_channel::{
    unbounded,
    Receiver,
    Sender,
};
use serde_json::{
    json,
    Value,
};
use wasmtime::{
    Caller,
    Engine,
    Linker,
    Memory,
    Store,
};
use wasmtime_wasi::{
    p2::WasiCtxBuilder,
    preview1,
    preview1::WasiP1Ctx,
    DirPerms,
    FilePerms,
};

use crate::fetch::{
    parse_fetch_request,
    spawn_fetch,
    FetchCompletion,
    FetchRequest,
};

pub const FIXED_NOW_MS: i64 = 1_700_000_000_000;

/// No handler is registered under that syscall name.
pub const SYSCALL_UNKNOWN: i64 = -2;
/// The handler exists but rejected its arguments.
pub const SYSCALL_INVALID_ARGS: i64 = -3;
/// The result is larger than [`MAX_HOST_RESULT_BYTES`].
pub const SYSCALL_RESULT_TOO_LARGE: i64 = -4;
/// The guest's `alloc` could not provide a buffer for the result.
pub const SYSCALL_ALLOC_FAILED: i64 = -5;

/// The largest result the host will hand back. Bounds a single guest
/// allocation, and keeps `len` inside the 31 bits [`pack_result`] has for it,
/// so a packed result can never be mistaken for an error code. Convex documents
/// cap out around 1 MB; the headroom is for fetch bodies, the one network-sized
/// input here.
pub const MAX_HOST_RESULT_BYTES: usize = 16 * 1024 * 1024;

/// A result buffer the guest now owns: `len` in the high 32 bits, `ptr` in the
/// low 32. Mirrors the guest's own `pack_result` for `invoke`.
fn pack_result(ptr: i32, len: usize) -> i64 {
    ((len as u64) << 32 | (ptr as u32 as u64)) as i64
}

#[derive(Debug)]
pub enum SyscallError {
    Unknown,
    InvalidArgs,
}

impl SyscallError {
    fn code(&self) -> i64 {
        match self {
            Self::Unknown => SYSCALL_UNKNOWN,
            Self::InvalidArgs => SYSCALL_INVALID_ARGS,
        }
    }
}

/// Where a guest that has not been preinitialized looks for its bundle; the
/// guest side of this is `BUNDLE_PATH` in `guest_js/src/lib.rs`.
pub const GUEST_BUNDLE_DIR: &str = "/bundle";

/// The store data behind the `convex_host` imports. Implementing this is what
/// makes a host usable by [`new_linker`], so the fixture host here and the UDF
/// host in [`crate::udf`] share one wire format and one set of guest exports.
///
/// Sync syscalls take and return raw JSON text: the fixture host parses it into
/// an argument array, while the UDF host forwards it to the isolate crate's
/// syscall implementations untouched.
pub trait HostAbi: Send + 'static {
    fn wasi(&mut self) -> &mut WasiP1Ctx;

    fn syscall(&mut self, name: &str, args_json: &str) -> Result<String, SyscallError>;

    fn start_async_syscall(&mut self, name: &str, args_json: &str) -> Result<i32, SyscallError>;

    /// Ops that have settled since the guest last asked.
    fn completed_ops(&mut self) -> Vec<i32>;

    fn take_op_result(&mut self, op_id: i32) -> String;
}

type SyscallResult = Result<Value, SyscallError>;

enum PendingOp {
    Sleep { ready_at: Instant },
    FetchPending,
    Ready(String),
    Error(String),
}

pub struct HostState {
    wasi: WasiP1Ctx,
    records: BTreeMap<String, Value>,
    logs: Vec<String>,
    next_uuid: u64,
    next_op_id: i32,
    pending_ops: BTreeMap<i32, PendingOp>,
    fetch_completion_tx: Sender<FetchCompletion>,
    fetch_completion_rx: Receiver<FetchCompletion>,
    syscall_counts: BTreeMap<String, usize>,
}

impl Default for HostState {
    fn default() -> Self {
        Self::new()
    }
}

impl HostState {
    pub fn new() -> Self {
        Self::with_wasi(WasiCtxBuilder::new().build_p1())
    }

    /// A state whose guest can read its JS bundle out of `bundle_dir`, which
    /// only a guest that has not been preinitialized needs: the snapshot a
    /// preinitialized guest starts from already holds the evaluated bundle, so
    /// the request path hands the guest no filesystem at all.
    pub fn with_bundle_dir(bundle_dir: &Path) -> anyhow::Result<Self> {
        let mut builder = WasiCtxBuilder::new();
        builder.preopened_dir(
            bundle_dir,
            GUEST_BUNDLE_DIR,
            DirPerms::READ,
            FilePerms::READ,
        )?;
        Ok(Self::with_wasi(builder.build_p1()))
    }

    fn with_wasi(wasi: WasiP1Ctx) -> Self {
        let (fetch_completion_tx, fetch_completion_rx) = unbounded();
        Self {
            wasi,
            records: BTreeMap::new(),
            logs: Vec::new(),
            next_uuid: 1,
            next_op_id: 1,
            pending_ops: BTreeMap::new(),
            fetch_completion_tx,
            fetch_completion_rx,
            syscall_counts: BTreeMap::new(),
        }
    }

    pub fn seed_record(&mut self, key: &str, value: Value) {
        self.records.insert(key.to_owned(), value);
    }

    /// How many times `name` has been dispatched. Lets a test pin down how many
    /// host round trips a handler actually costs.
    pub fn syscall_count(&self, name: &str) -> usize {
        self.syscall_counts.get(name).copied().unwrap_or(0)
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    /// The sync syscall table. Pure operations (console formatting, text
    /// encoding, structured clone) stay in the guest and never reach here.
    fn dispatch_syscall(&mut self, name: &str, args: &[Value]) -> SyscallResult {
        *self.syscall_counts.entry(name.to_owned()).or_default() += 1;

        match name {
            "db/get" => {
                let key = string_arg(args, 0)?;
                Ok(self.records.get(key).cloned().unwrap_or(Value::Null))
            },
            "db/set" => {
                let key = string_arg(args, 0)?.to_owned();
                let value = args.get(1).cloned().ok_or(SyscallError::InvalidArgs)?;
                self.records.insert(key, value);
                Ok(Value::Null)
            },
            "db/delete" => {
                let key = string_arg(args, 0)?;
                Ok(Value::Bool(self.records.remove(key).is_some()))
            },
            "console/message" => {
                let message = string_arg(args, 0)?.to_owned();
                self.logs.push(message);
                Ok(Value::Null)
            },
            "time/now" => Ok(Value::from(FIXED_NOW_MS)),
            "crypto/randomUuid" => {
                let uuid = format!("00000000-0000-4000-8000-{:012}", self.next_uuid);
                self.next_uuid += 1;
                Ok(Value::String(uuid))
            },
            _ => Err(SyscallError::Unknown),
        }
    }

    /// The async syscall table. Returns an op-id the guest parks a promise
    /// resolver against; the engine-agnostic stand-in for a
    /// `v8::Global<v8::PromiseResolver>`.
    fn dispatch_async_syscall(&mut self, name: &str, args: &[Value]) -> Result<i32, SyscallError> {
        match name {
            "sleep" => {
                let ms = args
                    .first()
                    .and_then(Value::as_i64)
                    .ok_or(SyscallError::InvalidArgs)?;
                Ok(self.start_sleep(ms.max(0)))
            },
            "fetch" => {
                let request_json = args.first().ok_or(SyscallError::InvalidArgs)?.to_string();
                let request =
                    parse_fetch_request(&request_json).map_err(|_| SyscallError::InvalidArgs)?;
                Ok(self.start_fetch(request))
            },
            _ => Err(SyscallError::Unknown),
        }
    }

    fn start_sleep(&mut self, ms: i64) -> i32 {
        let op_id = self.next_op_id;
        self.next_op_id += 1;
        self.pending_ops.insert(
            op_id,
            PendingOp::Sleep {
                ready_at: Instant::now() + Duration::from_millis(ms as u64),
            },
        );
        op_id
    }

    fn start_fetch(&mut self, request: FetchRequest) -> i32 {
        let op_id = self.next_op_id;
        self.next_op_id += 1;
        self.pending_ops.insert(op_id, PendingOp::FetchPending);
        spawn_fetch(op_id, request, self.fetch_completion_tx.clone());
        op_id
    }

    fn refresh_completed_ops(&mut self) {
        while let Ok(completion) = self.fetch_completion_rx.try_recv() {
            let op = match completion.result_json {
                Ok(payload) => PendingOp::Ready(payload),
                Err(message) => PendingOp::Error(message),
            };
            self.pending_ops.insert(completion.op_id, op);
        }
    }

    /// Every op that has settled since the guest last asked. Returning a set
    /// rather than a single id is what lets W10 batch a `Promise.all` into one
    /// host round trip.
    fn settled_ops(&mut self) -> Vec<i32> {
        self.refresh_completed_ops();

        let now = Instant::now();
        let mut completed = Vec::new();
        for (op_id, entry) in self.pending_ops.iter_mut() {
            if let PendingOp::Sleep { ready_at } = entry
                && now >= *ready_at
            {
                *entry = PendingOp::Ready("null".to_owned());
            }

            if matches!(entry, PendingOp::Ready(_) | PendingOp::Error(_)) {
                completed.push(*op_id);
            }
        }

        completed
    }

    fn take_settled_op(&mut self, op_id: i32) -> String {
        self.refresh_completed_ops();
        match self.pending_ops.remove(&op_id) {
            Some(PendingOp::Ready(payload)) => format!(r#"{{"ok":true,"value":{payload}}}"#),
            Some(PendingOp::Error(message)) => {
                json!({ "ok": false, "message": message }).to_string()
            },
            Some(entry) => {
                // Put it back; the guest asked before the op settled.
                self.pending_ops.insert(op_id, entry);
                json!({ "ok": false, "message": format!("op {op_id} is still pending") })
                    .to_string()
            },
            None => json!({ "ok": false, "message": format!("unknown op {op_id}") }).to_string(),
        }
    }
}

impl HostAbi for HostState {
    fn wasi(&mut self) -> &mut WasiP1Ctx {
        &mut self.wasi
    }

    fn syscall(&mut self, name: &str, args_json: &str) -> Result<String, SyscallError> {
        let args = parse_args(args_json)?;
        let value = self.dispatch_syscall(name, &args)?;
        Ok(serde_json::to_string(&value).expect("syscall result should serialize"))
    }

    fn start_async_syscall(&mut self, name: &str, args_json: &str) -> Result<i32, SyscallError> {
        let args = parse_args(args_json)?;
        self.dispatch_async_syscall(name, &args)
    }

    fn completed_ops(&mut self) -> Vec<i32> {
        self.settled_ops()
    }

    fn take_op_result(&mut self, op_id: i32) -> String {
        self.take_settled_op(op_id)
    }
}

fn string_arg(args: &[Value], index: usize) -> Result<&str, SyscallError> {
    args.get(index)
        .and_then(Value::as_str)
        .ok_or(SyscallError::InvalidArgs)
}

fn memory<T>(caller: &mut Caller<'_, T>) -> Memory {
    caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .expect("guest memory export should exist")
}

fn read_string<T>(caller: &mut Caller<'_, T>, ptr: i32, len: i32) -> String {
    if len == 0 {
        return String::new();
    }

    let memory = memory(caller);
    let mut bytes = vec![0_u8; len as usize];
    memory
        .read(caller, ptr as usize, &mut bytes)
        .expect("guest memory read should succeed");

    String::from_utf8(bytes).expect("guest should pass valid UTF-8")
}

/// Hand `payload` to the guest: allocate a buffer through the guest's own
/// `alloc` export, copy the bytes in, and return the packed `(ptr, len)`. The
/// guest owns the buffer from here and frees it.
///
/// Calling a guest export from inside a host call re-enters the store, which is
/// exactly what the component model's `cabi_realloc` does for returned lists.
fn deliver<T>(caller: &mut Caller<'_, T>, payload: Result<String, i64>) -> i64 {
    let payload = match payload {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    if payload.is_empty() {
        return 0;
    }

    if payload.len() > MAX_HOST_RESULT_BYTES {
        return SYSCALL_RESULT_TOO_LARGE;
    }

    let Some(ptr) = guest_alloc(caller, payload.len()) else {
        return SYSCALL_ALLOC_FAILED;
    };

    let memory = memory(caller);
    memory
        .write(caller, ptr as usize, payload.as_bytes())
        .expect("guest memory write should succeed");
    pack_result(ptr, payload.len())
}

fn guest_alloc<T>(caller: &mut Caller<'_, T>, len: usize) -> Option<i32> {
    let alloc = caller.get_export("alloc")?.into_func()?;
    let alloc = alloc.typed::<i32, i32>(&caller).ok()?;
    let ptr = alloc.call(&mut *caller, len as i32).ok()?;
    (ptr != 0).then_some(ptr)
}

fn parse_args(args_json: &str) -> Result<Vec<Value>, SyscallError> {
    match serde_json::from_str(args_json) {
        Ok(Value::Array(args)) => Ok(args),
        _ => Err(SyscallError::InvalidArgs),
    }
}

pub fn new_linker<T: HostAbi>(engine: &Engine) -> Linker<T> {
    let mut linker: Linker<T> = Linker::new(engine);
    preview1::add_to_linker_sync(&mut linker, T::wasi).expect("WASI imports should link");

    linker
        .func_wrap(
            "convex_host",
            "syscall",
            |mut caller: Caller<'_, T>,
             name_ptr: i32,
             name_len: i32,
             args_ptr: i32,
             args_len: i32|
             -> i64 {
                let name = read_string(&mut caller, name_ptr, name_len);
                let args_json = read_string(&mut caller, args_ptr, args_len);

                let result = caller
                    .data_mut()
                    .syscall(&name, &args_json)
                    .map_err(|error| error.code());

                deliver(&mut caller, result)
            },
        )
        .expect("syscall should link");

    linker
        .func_wrap(
            "convex_host",
            "start_async_syscall",
            |mut caller: Caller<'_, T>,
             name_ptr: i32,
             name_len: i32,
             args_ptr: i32,
             args_len: i32|
             -> i32 {
                let name = read_string(&mut caller, name_ptr, name_len);
                let args_json = read_string(&mut caller, args_ptr, args_len);

                match caller.data_mut().start_async_syscall(&name, &args_json) {
                    Ok(op_id) => op_id,
                    Err(error) => error.code() as i32,
                }
            },
        )
        .expect("start_async_syscall should link");

    linker
        .func_wrap(
            "convex_host",
            "completed_ops",
            |mut caller: Caller<'_, T>| -> i64 {
                let completed = caller.data_mut().completed_ops();
                let serialized =
                    serde_json::to_string(&completed).expect("op ids should serialize");
                deliver(&mut caller, Ok(serialized))
            },
        )
        .expect("completed_ops should link");

    linker
        .func_wrap(
            "convex_host",
            "take_op_result",
            |mut caller: Caller<'_, T>, op_id: i32| -> i64 {
                let payload = caller.data_mut().take_op_result(op_id);
                deliver(&mut caller, Ok(payload))
            },
        )
        .expect("take_op_result should link");

    linker
}

pub fn new_store(engine: &Engine) -> Store<HostState> {
    Store::new(engine, HostState::new())
}

/// A store for running a guest that has not been preinitialized, which has to
/// read and evaluate its bundle from `bundle_dir` before it can serve anything.
pub fn new_store_with_bundle_dir(
    engine: &Engine,
    bundle_dir: &Path,
) -> anyhow::Result<Store<HostState>> {
    Ok(Store::new(engine, HostState::with_bundle_dir(bundle_dir)?))
}
