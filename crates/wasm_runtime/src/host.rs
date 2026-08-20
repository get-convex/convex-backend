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
//! Everything else is dispatched by name against [`UdfHostState`], and through
//! it against whatever [`crate::udf::ConvexSyscallHost`] the caller supplied,
//! mirroring `IsolateEnvironment::syscall` in the convex isolate crate. Adding
//! a syscall is a host-side table entry plus a JS shim; it never requires
//! rebuilding the guest ABI.
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

use std::path::Path;

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
    DirPerms,
    FilePerms,
};

use crate::udf::{
    ConvexSyscallHost,
    UdfHostState,
};

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
pub fn new_linker<H: ConvexSyscallHost>(engine: &Engine) -> Linker<UdfHostState<H>> {
    let mut linker: Linker<UdfHostState<H>> = Linker::new(engine);
    preview1::add_to_linker_sync(&mut linker, UdfHostState::wasi)
        .expect("WASI imports should link");

    linker
        .func_wrap(
            "convex_host",
            "syscall",
            |mut caller: Caller<'_, UdfHostState<H>>,
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
            |mut caller: Caller<'_, UdfHostState<H>>,
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
            |mut caller: Caller<'_, UdfHostState<H>>| -> i64 {
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
            |mut caller: Caller<'_, UdfHostState<H>>, op_id: i32| -> i64 {
                let payload = caller.data_mut().take_op_result(op_id);
                deliver(&mut caller, Ok(payload))
            },
        )
        .expect("take_op_result should link");

    linker
}

/// A store for a guest whose bundle is already baked into its snapshot, which
/// is every preinitialized artifact: the request path hands the guest no
/// filesystem at all.
pub fn new_store<H: ConvexSyscallHost>(engine: &Engine, host: H) -> Store<UdfHostState<H>> {
    Store::new(
        engine,
        UdfHostState::new(host, WasiCtxBuilder::new().build_p1()),
    )
}

/// A store for running a guest that has not been preinitialized, which has to
/// read and evaluate its bundle from `bundle_dir` before it can serve anything.
pub fn new_store_with_bundle_dir<H: ConvexSyscallHost>(
    engine: &Engine,
    host: H,
    bundle_dir: &Path,
) -> anyhow::Result<Store<UdfHostState<H>>> {
    let mut builder = WasiCtxBuilder::new();
    builder.preopened_dir(
        bundle_dir,
        GUEST_BUNDLE_DIR,
        DirPerms::READ,
        FilePerms::READ,
    )?;
    Ok(Store::new(
        engine,
        UdfHostState::new(host, builder.build_p1()),
    ))
}
