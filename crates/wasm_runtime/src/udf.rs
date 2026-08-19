//! Running a Convex query or mutation inside the guest.
//!
//! This mirrors the loop `DatabaseUdfEnvironment::run_inner` runs against V8,
//! with the wasm guest standing in for the isolate:
//!
//! 1. call the function, which returns a promise
//! 2. drain the microtask queue as far as it will go
//! 3. if the promise has not settled, the guest must be blocked on syscalls, so
//!    run a batch of them and hand the results back
//! 4. repeat until the promise settles
//!
//! The syscalls themselves are not implemented here. Everything Convex-specific
//! arrives through [`ConvexSyscallHost`], which the isolate crate implements
//! over its existing syscall code, so this crate stays free of any dependency
//! on the database.

use std::{
    collections::{
        BTreeMap,
        HashMap,
        VecDeque,
    },
    sync::OnceLock,
};

use anyhow::Context as _;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::json;
use sha2::{
    Digest,
    Sha256,
};
use wasmtime::{
    Engine,
    Instance,
    Memory,
    Module,
    Store,
    TypedFunc,
};
use wasmtime_wasi::{
    p2::WasiCtxBuilder,
    preview1::WasiP1Ctx,
};

use crate::host::{
    new_linker,
    HostAbi,
    SyscallError,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UdfTypeTag {
    Query,
    Mutation,
}

impl UdfTypeTag {
    fn as_str(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Mutation => "mutation",
        }
    }
}

/// An async syscall the guest has parked a promise against. `args` is the JSON
/// text the JS layer passed to `Convex.asyncSyscall`, forwarded verbatim.
#[derive(Clone, Debug)]
pub struct PendingSyscall {
    pub op_id: i32,
    pub name: String,
    pub args: String,
}

/// The result of one syscall: either JSON text to resolve with, or a
/// user-facing message to reject with. System errors are not represented here —
/// they come back as the outer `anyhow::Error` and abort the whole run, which
/// is the same split `resolve_promise` makes on the V8 side.
pub type SyscallOutcome = Result<String, String>;

#[allow(async_fn_in_trait)]
pub trait ConvexSyscallHost: Send + 'static {
    /// A sync syscall. Returns immediately; the guest is blocked on it.
    fn syscall(&mut self, name: &str, args_json: &str) -> anyhow::Result<SyscallOutcome>;

    /// Run some prefix of `pending` and return an outcome for each op run. The
    /// implementation chooses how many to take, which is where the isolate's
    /// batching rules live; returning fewer than all of them just means the
    /// rest are offered again on the next turn.
    async fn async_syscalls(
        &mut self,
        pending: &[PendingSyscall],
    ) -> anyhow::Result<Vec<(i32, SyscallOutcome)>>;

    fn trace(&mut self, level: &str, messages: Vec<String>) -> anyhow::Result<()>;
}

/// Why an invocation did not produce a value. These mirror the checks
/// `run_inner` makes before and after calling into JS; the caller turns them
/// into user-visible errors because only it knows the function's path.
#[derive(Debug)]
pub enum UdfError {
    FunctionNotFound {
        function_name: String,
    },
    /// The export exists but its `isQuery`/`isMutation` marker does not match
    /// the type it is being run as.
    FunctionType {
        is_query: bool,
        is_mutation: bool,
    },
    /// The function threw. `message` is already user-facing.
    Handler {
        message: String,
        stack: Option<String>,
    },
}

pub struct UdfHostState<H> {
    wasi: WasiP1Ctx,
    host: H,
    next_op_id: i32,
    pending: VecDeque<PendingSyscall>,
    settled: BTreeMap<i32, SyscallOutcome>,
    /// Set when a syscall fails for a reason that is not the developer's fault.
    /// JS keeps running until the next host call, so like the V8 path's
    /// termination flag this is checked by the driver rather than trusted to
    /// stop execution on its own.
    system_error: Option<anyhow::Error>,
}

impl<H: ConvexSyscallHost> UdfHostState<H> {
    fn new(host: H) -> Self {
        Self {
            wasi: WasiCtxBuilder::new().build_p1(),
            host,
            next_op_id: 1,
            pending: VecDeque::new(),
            settled: BTreeMap::new(),
            system_error: None,
        }
    }

    fn take_system_error(&mut self) -> Option<anyhow::Error> {
        self.system_error.take()
    }

    /// Record a system error and give the guest something to reject with. The
    /// message is deliberately vague: the real error goes to the driver.
    fn poison(&mut self, error: anyhow::Error) -> String {
        let message = "A system error occurred during execution".to_owned();
        self.system_error.get_or_insert(error);
        message
    }

    fn console_message(&mut self, args_json: &str) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct Message {
            level: String,
            messages: Vec<String>,
        }
        let message: Message =
            serde_json::from_str(args_json).context("console message should deserialize")?;
        self.host.trace(&message.level, message.messages)
    }
}

fn envelope(outcome: SyscallOutcome) -> String {
    match outcome {
        Ok(value) => json!({ "ok": true, "value": value }).to_string(),
        Err(message) => json!({ "ok": false, "message": message }).to_string(),
    }
}

impl<H: ConvexSyscallHost> HostAbi for UdfHostState<H> {
    fn wasi(&mut self) -> &mut WasiP1Ctx {
        &mut self.wasi
    }

    fn syscall(&mut self, name: &str, args_json: &str) -> Result<String, SyscallError> {
        if name == "console/message" {
            let outcome = match self.console_message(args_json) {
                Ok(()) => Ok("null".to_owned()),
                Err(error) => Err(self.poison(error)),
            };
            return Ok(envelope(outcome));
        }

        let outcome = match self.host.syscall(name, args_json) {
            Ok(outcome) => outcome,
            Err(error) => Err(self.poison(error)),
        };
        Ok(envelope(outcome))
    }

    fn start_async_syscall(&mut self, name: &str, args_json: &str) -> Result<i32, SyscallError> {
        let op_id = self.next_op_id;
        self.next_op_id += 1;
        self.pending.push_back(PendingSyscall {
            op_id,
            name: name.to_owned(),
            args: args_json.to_owned(),
        });
        Ok(op_id)
    }

    fn completed_ops(&mut self) -> Vec<i32> {
        self.settled.keys().copied().collect()
    }

    fn take_op_result(&mut self, op_id: i32) -> String {
        match self.settled.remove(&op_id) {
            Some(outcome) => envelope(outcome),
            None => envelope(Err(format!("op {op_id} has not run yet"))),
        }
    }
}

/// Compiling a module is the expensive part of starting an instance and depends
/// only on the artifact bytes, so it is shared process-wide.
fn module_for(artifact: &[u8]) -> anyhow::Result<(Engine, Module)> {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    static MODULES: OnceLock<Mutex<HashMap<[u8; 32], Module>>> = OnceLock::new();

    let engine = ENGINE.get_or_init(Engine::default).clone();
    let key: [u8; 32] = Sha256::digest(artifact).into();

    let modules = MODULES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(module) = modules.lock().get(&key) {
        return Ok((engine, module.clone()));
    }

    let module = Module::new(&engine, artifact).context("failed to load guest module")?;
    modules.lock().insert(key, module.clone());
    Ok((engine, module))
}

struct GuestInstance<H: ConvexSyscallHost> {
    store: Store<UdfHostState<H>>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    dealloc: TypedFunc<(i32, i32), ()>,
    start_invoke: TypedFunc<(i32, i32, i32, i32, i32, i32), i64>,
    poll_invoke: TypedFunc<(), i64>,
}

impl<H: ConvexSyscallHost> GuestInstance<H> {
    fn new(artifact: &[u8], host: H) -> anyhow::Result<Self> {
        let (engine, module) = module_for(artifact)?;
        let linker = new_linker::<UdfHostState<H>>(&engine);
        let mut store = Store::new(&engine, UdfHostState::new(host));
        let instance: Instance = linker
            .instantiate(&mut store, &module)
            .context("failed to instantiate guest")?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .context("guest is missing its memory export")?;
        let alloc = instance.get_typed_func::<i32, i32>(&mut store, "alloc")?;
        let dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")?;
        let start_invoke = instance.get_typed_func::<(i32, i32, i32, i32, i32, i32), i64>(
            &mut store,
            "udf_start_invoke",
        )?;
        let poll_invoke = instance.get_typed_func::<(), i64>(&mut store, "poll_invoke")?;

        Ok(Self {
            store,
            memory,
            alloc,
            dealloc,
            start_invoke,
            poll_invoke,
        })
    }

    fn write_input(&mut self, value: &str) -> anyhow::Result<(i32, i32)> {
        let len = value.len() as i32;
        if len == 0 {
            return Ok((0, 0));
        }
        let ptr = self.alloc.call(&mut self.store, len)?;
        anyhow::ensure!(ptr != 0, "guest refused a {len} byte allocation");
        self.memory
            .write(&mut self.store, ptr as usize, value.as_bytes())?;
        Ok((ptr, len))
    }

    /// Read back a packed `(ptr, len)` the guest returned, freeing the buffer.
    fn take_output(&mut self, packed: i64) -> anyhow::Result<Option<String>> {
        if packed == 0 {
            return Ok(None);
        }
        anyhow::ensure!(packed > 0, "guest returned error code {packed}");

        let ptr = (packed as u64 & 0xffff_ffff) as i32;
        let len = (packed as u64 >> 32) as usize;
        let mut bytes = vec![0_u8; len];
        self.memory.read(&self.store, ptr as usize, &mut bytes)?;
        self.dealloc.call(&mut self.store, (ptr, len as i32))?;
        Ok(Some(String::from_utf8(bytes)?))
    }
}

#[derive(Deserialize)]
struct InvokeResult {
    ok: bool,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    error: Option<InvokeError>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvokeError {
    kind: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    stack: Option<String>,
    #[serde(default)]
    function_name: Option<String>,
    #[serde(default)]
    is_query: bool,
    #[serde(default)]
    is_mutation: bool,
}

/// Run `function_name` from `artifact` and return the host along with the JSON
/// text the function resolved to.
///
/// The host comes back so the caller can keep using whatever it wrapped — on
/// the isolate side that is the transaction the syscalls just wrote to.
pub async fn run_udf<H: ConvexSyscallHost>(
    artifact: &[u8],
    udf_type: UdfTypeTag,
    function_name: &str,
    args_json: &str,
    host: H,
) -> anyhow::Result<(H, Result<String, UdfError>)> {
    let mut guest = GuestInstance::new(artifact, host)?;

    let (function_ptr, function_len) = guest.write_input(function_name)?;
    let (type_ptr, type_len) = guest.write_input(udf_type.as_str())?;
    let (args_ptr, args_len) = guest.write_input(args_json)?;

    let started = guest.start_invoke.call(
        &mut guest.store,
        (
            function_ptr,
            function_len,
            type_ptr,
            type_len,
            args_ptr,
            args_len,
        ),
    )?;
    if let Some(failure) = guest.take_output(started)? {
        anyhow::bail!("guest failed to start the invocation: {failure}");
    }

    let payload = loop {
        let polled = guest.poll_invoke.call(&mut guest.store, ())?;
        if let Some(error) = guest.store.data_mut().take_system_error() {
            return Err(error);
        }
        if let Some(payload) = guest.take_output(polled)? {
            break payload;
        }

        let pending: Vec<PendingSyscall> = guest.store.data().pending.iter().cloned().collect();
        anyhow::ensure!(
            !pending.is_empty(),
            "the function is blocked but has no pending syscalls"
        );

        let results = guest.store.data_mut().host.async_syscalls(&pending).await?;
        anyhow::ensure!(
            !results.is_empty(),
            "the host ran none of the {} pending syscalls",
            pending.len()
        );

        let state = guest.store.data_mut();
        for (op_id, outcome) in results {
            state.pending.retain(|syscall| syscall.op_id != op_id);
            state.settled.insert(op_id, outcome);
        }
    };

    let result: InvokeResult =
        serde_json::from_str(&payload).context("guest returned an unreadable result")?;
    let outcome = if result.ok {
        Ok(result
            .value
            .context("guest reported success with no value")?)
    } else {
        let error = result
            .error
            .context("guest reported failure with no error")?;
        Err(match error.kind.as_str() {
            "FunctionNotFound" => UdfError::FunctionNotFound {
                function_name: error.function_name.unwrap_or_default(),
            },
            "FunctionType" => UdfError::FunctionType {
                is_query: error.is_query,
                is_mutation: error.is_mutation,
            },
            _ => UdfError::Handler {
                message: error.message.unwrap_or_else(|| error.kind.clone()),
                stack: error.stack,
            },
        })
    };

    Ok((guest.store.into_data().host, outcome))
}
