use std::{
    cell::RefCell,
    slice,
    str,
};

use rquickjs::{
    context::EvalOptions,
    Context,
    Ctx,
    Function,
    Object,
    Runtime,
};
use serde_json::json;

/// Mirrors `MAX_HOST_RESULT_BYTES` in `src/host.rs`. The host refuses results
/// past this rather than allocating them here, which bounds any single buffer
/// the guest is handed.
const MAX_HOST_RESULT_BYTES: usize = 16 * 1024 * 1024;

// The whole host ABI: name-dispatched sync and async syscalls plus op-result
// retrieval. Adding a host operation is a host table entry and a JS shim, not
// a guest rebuild.
#[link(wasm_import_module = "convex_host")]
unsafe extern "C" {
    #[link_name = "syscall"]
    fn host_syscall(
        name_ptr: *const u8,
        name_len: usize,
        args_ptr: *const u8,
        args_len: usize,
    ) -> i64;
    #[link_name = "start_async_syscall"]
    fn host_start_async_syscall(
        name_ptr: *const u8,
        name_len: usize,
        args_ptr: *const u8,
        args_len: usize,
    ) -> i32;
    #[link_name = "completed_ops"]
    fn host_completed_ops() -> i64;
    #[link_name = "take_op_result"]
    fn host_take_op_result(op_id: i32) -> i64;
}

/// The bundle the host preopens for preinitialization, under the guest
/// directory `GUEST_BUNDLE_DIR` in `src/host.rs`. Reading it at init rather
/// than compiling it in is what lets one guest module serve every bundle: the
/// per-app artifact is the Wizer snapshot, not a fresh guest build.
const BUNDLE_PATH: &str = "/bundle/guest-bundle.js";

/// The name the bundle's stack frames carry, which the host's sourcemap
/// remapping keys off.
const BUNDLE_FILENAME: &str = "guest-bundle.js";

/// The manifest the bundler writes beside the bundle. Its `kind` names which
/// global surface the bundle expects, which is what lets one guest binary serve
/// both fixtures and UDFs; a bundle whose manifest does not name one is a
/// fixture bundle.
const MANIFEST_PATH: &str = "/bundle/manifest.json";

#[derive(Clone, Copy, PartialEq, Eq)]
enum BundleKind {
    Fixture,
    Udf,
}

fn bundle_kind() -> BundleKind {
    let Ok(text) = std::fs::read_to_string(MANIFEST_PATH) else {
        return BundleKind::Fixture;
    };
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(manifest) if manifest["kind"] == "udf" => BundleKind::Udf,
        _ => BundleKind::Fixture,
    }
}

fn load_bundle() -> Result<String, GuestErrorDetail> {
    std::fs::read_to_string(BUNDLE_PATH).map_err(|error| GuestErrorDetail {
        message: format!(
            "failed to read {BUNDLE_PATH}: {error}. A guest that was not preinitialized can only \
             read its bundle from a host that preopens one."
        ),
        stack: None,
    })
}

enum InitState {
    Uninitialized,
    Ready,
    Failed(GuestErrorDetail),
}

#[derive(Clone)]
struct GuestErrorDetail {
    message: String,
    stack: Option<String>,
}

struct GuestRuntime {
    runtime: Runtime,
    context: Context,
    init_state: InitState,
}

impl GuestRuntime {
    fn new() -> Self {
        let runtime = Runtime::new().expect("QuickJS runtime should initialize");
        let context = Context::full(&runtime).expect("QuickJS context should initialize");

        Self {
            runtime,
            context,
            init_state: InitState::Uninitialized,
        }
    }
}

thread_local! {
    static GUEST_RUNTIME: RefCell<GuestRuntime> = RefCell::new(GuestRuntime::new());
}

fn initialize_runtime(state: &mut GuestRuntime) -> Result<(), GuestErrorDetail> {
    match &state.init_state {
        InitState::Ready => return Ok(()),
        InitState::Failed(error) => return Err(error.clone()),
        InitState::Uninitialized => {},
    }

    let result: Result<(), GuestErrorDetail> = state.context.with(|ctx| {
        install_host_functions(ctx.clone()).map_err(|error| GuestErrorDetail {
            message: format!("host shim bootstrap failed: {error}"),
            stack: None,
        })?;

        ctx.eval::<(), _>(SHARED_PRELUDE)
            .map_err(|error| describe_exception(&ctx, "host shim bootstrap", error))?;

        let kind = bundle_kind();
        let globals_source = match kind {
            BundleKind::Udf => UDF_GLOBALS,
            BundleKind::Fixture => FIXTURE_GLOBALS,
        };
        ctx.eval::<(), _>(globals_source)
            .map_err(|error| describe_exception(&ctx, "global installation", error))?;

        let mut bundle_options = EvalOptions::default();
        bundle_options.filename = Some(BUNDLE_FILENAME.to_owned());
        ctx.eval_with_options::<(), _>(load_bundle()?, bundle_options)
            .map_err(|error| describe_exception(&ctx, "bundle evaluation", error))?;

        ctx.eval::<(), _>(INVOKE_RUNTIME)
            .map_err(|error| describe_exception(&ctx, "runtime bootstrap", error))?;

        if kind == BundleKind::Udf {
            ctx.eval::<(), _>(UDF_INVOKE_RUNTIME)
                .map_err(|error| describe_exception(&ctx, "udf runtime bootstrap", error))?;
        }

        let globals = ctx.globals();
        let _: Object<'_> = globals
            .get("__convex_exports")
            .map_err(|error| describe_exception(&ctx, "exports lookup", error))?;
        let _: Function<'_> = globals
            .get("__convex_start_invoke")
            .map_err(|error| describe_exception(&ctx, "invoke start lookup", error))?;
        let _: Function<'_> = globals
            .get("__convex_take_invoke_result")
            .map_err(|error| describe_exception(&ctx, "invoke result lookup", error))?;
        let _: Function<'_> = globals
            .get("__convex_poll_pending_ops")
            .map_err(|error| describe_exception(&ctx, "pending op poll lookup", error))?;

        Ok(())
    });

    match result {
        Ok(()) => {
            state.init_state = InitState::Ready;
            Ok(())
        },
        Err(error) => {
            state.init_state = InitState::Failed(error.clone());
            Err(error)
        },
    }
}

/// State both bundle kinds need: the op table the host resolves against, and
/// the flag that keeps module-initialization code (which Wizer runs at build
/// time, with no host attached) from reaching the host.
const SHARED_PRELUDE: &str = include_str!("js/shared_prelude.js");

/// The Convex syscall surface, as `npm-packages/udf-syscall-ffi` expects to
/// find it. Sync and async syscalls both carry `{ok, value}` envelopes where
/// `value` is already-serialized JSON text, because that is what
/// `performSyscall`/`performAsyncSyscall` hand back to their callers.
///
/// `op` and `jsSyscall` are the V8 ops layer, which has no wasm implementation
/// yet; they throw rather than silently returning something wrong.
const UDF_GLOBALS: &str = include_str!("js/udf_globals.js");

const FIXTURE_GLOBALS: &str = include_str!("js/fixture_globals.js");

/// The invocation machinery both kinds share: op-result delivery, the pending
/// op count the host polls, and the fixture-style `handler(...args)` entry.
const INVOKE_RUNTIME: &str = include_str!("js/invoke_runtime.js");

/// UDF dispatch, mirroring `run_inner` in the isolate crate: find the export,
/// check its `isQuery`/`isMutation` marker against the requested type, and call
/// `invokeQuery`/`invokeMutation` with the serialized args. Those return a JSON
/// string, so the settled `value` is text the host hands straight back.
const UDF_INVOKE_RUNTIME: &str = include_str!("js/udf_invoke_runtime.js");

/// Pulls the message and stack off the exception QuickJS just threw, which
/// is only reachable from inside the context.
const DESCRIBE_EXCEPTION: &str = include_str!("js/describe_exception.js");

fn throw_host_error(ctx: &Ctx<'_>, message: String) -> rquickjs::Error {
    let error = ctx
        .eval::<rquickjs::Value<'_>, _>(format!(
            "new Error({})",
            serde_json::to_string(&message).expect("message should serialize")
        ))
        .expect("constructing host error should succeed");
    ctx.throw(error)
}

fn install_host_functions(ctx: Ctx<'_>) -> Result<(), String> {
    let globals = ctx.globals();

    let ctx_for_syscall = ctx.clone();
    globals
        .set(
            "__convex_syscall",
            Function::new(
                ctx.clone(),
                move |name: String, args_json: String| -> rquickjs::Result<String> {
                    syscall(&name, &args_json)
                        .map_err(|message| throw_host_error(&ctx_for_syscall, message))
                },
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

    let ctx_for_async = ctx.clone();
    globals
        .set(
            "__convex_start_async_syscall",
            Function::new(
                ctx.clone(),
                move |name: String, args_json: String| -> rquickjs::Result<i32> {
                    start_async_syscall(&name, &args_json)
                        .map_err(|message| throw_host_error(&ctx_for_async, message))
                },
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

    let ctx_for_completed = ctx.clone();
    globals
        .set(
            "__convex_completed_ops",
            Function::new(ctx.clone(), move || -> rquickjs::Result<String> {
                completed_ops().map_err(|message| throw_host_error(&ctx_for_completed, message))
            })
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

    let ctx_for_take = ctx.clone();
    globals
        .set(
            "__convex_take_op_result",
            Function::new(ctx, move |op_id: i32| -> rquickjs::Result<String> {
                take_op_result(op_id).map_err(|message| throw_host_error(&ctx_for_take, message))
            })
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn syscall(name: &str, args_json: &str) -> Result<String, String> {
    let packed = unsafe {
        host_syscall(
            name.as_ptr(),
            name.len(),
            args_json.as_ptr(),
            args_json.len(),
        )
    };

    take_host_result(name, packed)
}

fn start_async_syscall(name: &str, args_json: &str) -> Result<i32, String> {
    let op_id = unsafe {
        host_start_async_syscall(
            name.as_ptr(),
            name.len(),
            args_json.as_ptr(),
            args_json.len(),
        )
    };

    if op_id < 0 {
        Err(format!("{name} failed with status {op_id}"))
    } else {
        Ok(op_id)
    }
}

fn completed_ops() -> Result<String, String> {
    take_host_result("completed_ops", unsafe { host_completed_ops() })
}

fn take_op_result(op_id: i32) -> Result<String, String> {
    take_host_result("take_op_result", unsafe { host_take_op_result(op_id) })
}

/// Take ownership of a result the host allocated in our heap. The host calls
/// `alloc`, fills the buffer, and returns it packed as `len` in the high 32
/// bits and `ptr` in the low 32; reconstructing the `Vec` here is what frees
/// it.
fn take_host_result(name: &str, packed: i64) -> Result<String, String> {
    if packed < 0 {
        return Err(format!("{name} failed with status {packed}"));
    }

    let ptr = (packed as u64 & 0xffff_ffff) as *mut u8;
    let len = (packed as u64 >> 32) as usize;
    if len == 0 {
        return Ok(String::new());
    }

    // The host refuses to allocate past the ceiling, so exceeding it here means
    // the buffer did not come from a host that shares this ABI.
    if len > MAX_HOST_RESULT_BYTES || ptr.is_null() {
        return Err(format!("{name} returned an unusable buffer of {len} bytes"));
    }

    // `alloc` hands out `Vec::with_capacity(len)`, so capacity == len.
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
    String::from_utf8(bytes).map_err(|error| format!("{name} returned invalid UTF-8: {error}"))
}

fn describe_exception(ctx: &Ctx<'_>, stage: &str, error: rquickjs::Error) -> GuestErrorDetail {
    if matches!(error, rquickjs::Error::Exception) {
        let globals = ctx.globals();
        if globals.set("__convex_last_error", ctx.catch()).is_ok()
            && let Ok(serialized) = ctx.eval::<String, _>(DESCRIBE_EXCEPTION)
        {
            if let Ok(detail) = serde_json::from_str::<serde_json::Value>(&serialized) {
                return GuestErrorDetail {
                    message: format!(
                        "{stage} failed: {}",
                        detail["message"].as_str().unwrap_or("unknown error")
                    ),
                    stack: detail["stack"].as_str().map(str::to_owned),
                };
            }

            return GuestErrorDetail {
                message: format!("{stage} failed: {serialized}"),
                stack: None,
            };
        }
    }

    GuestErrorDetail {
        message: format!("{stage} failed: {error}"),
        stack: None,
    }
}

fn start_invoke_internal(
    state: &mut GuestRuntime,
    handler_name: &str,
    args_json: &str,
) -> Result<(), GuestErrorDetail> {
    initialize_runtime(state)?;

    state.context.with(|ctx| {
        let globals = ctx.globals();
        let start_invoke: Function<'_> = globals
            .get("__convex_start_invoke")
            .map_err(|error| describe_exception(&ctx, "invoke start lookup", error))?;

        start_invoke
            .call::<_, ()>((handler_name.to_owned(), args_json.to_owned()))
            .map_err(|error| describe_exception(&ctx, "handler invocation", error))
    })
}

fn start_udf_invoke_internal(
    state: &mut GuestRuntime,
    function_name: &str,
    udf_type: &str,
    args_json: &str,
) -> Result<(), GuestErrorDetail> {
    initialize_runtime(state)?;

    state.context.with(|ctx| {
        let globals = ctx.globals();
        let start_invoke: Function<'_> = globals
            .get("__convex_start_udf_invoke")
            .map_err(|error| describe_exception(&ctx, "udf invoke start lookup", error))?;

        start_invoke
            .call::<_, ()>((
                function_name.to_owned(),
                udf_type.to_owned(),
                args_json.to_owned(),
            ))
            .map_err(|error| describe_exception(&ctx, "udf invocation", error))
    })
}

fn poll_invoke_internal(state: &mut GuestRuntime) -> Result<Option<String>, GuestErrorDetail> {
    while state.runtime.is_job_pending() {
        state
            .runtime
            .execute_pending_job()
            .map_err(|error| GuestErrorDetail {
                message: format!("pending job execution failed: {error}"),
                stack: None,
            })?;
    }

    let pending_after_poll = state.context.with(|ctx| {
        let globals = ctx.globals();
        let poll_pending: Function<'_> = globals
            .get("__convex_poll_pending_ops")
            .map_err(|error| describe_exception(&ctx, "pending op poll lookup", error))?;

        poll_pending
            .call::<_, i32>(())
            .map_err(|error| describe_exception(&ctx, "pending op poll", error))
    })?;

    while state.runtime.is_job_pending() {
        state
            .runtime
            .execute_pending_job()
            .map_err(|error| GuestErrorDetail {
                message: format!("pending job execution failed: {error}"),
                stack: None,
            })?;
    }

    let result = state.context.with(|ctx| {
        let globals = ctx.globals();
        let take_result: Function<'_> = globals
            .get("__convex_take_invoke_result")
            .map_err(|error| describe_exception(&ctx, "invoke result lookup", error))?;

        take_result
            .call::<_, Option<String>>(())
            .map_err(|error| describe_exception(&ctx, "invoke result retrieval", error))
    })?;

    if result.is_none() && pending_after_poll == 0 && !state.runtime.is_job_pending() {
        let pending_ops = state.context.with(|ctx| {
            let globals = ctx.globals();
            let pending_count: Function<'_> = globals
                .get("__convex_pending_op_count")
                .map_err(|error| describe_exception(&ctx, "pending op count lookup", error))?;

            pending_count
                .call::<_, i32>(())
                .map_err(|error| describe_exception(&ctx, "pending op count", error))
        })?;

        if pending_ops == 0 {
            return Err(GuestErrorDetail {
                message: "invoke stalled without a settled result or pending ops".to_owned(),
                stack: None,
            });
        }
    }

    Ok(result)
}

fn error_payload(kind: &str, message: &str) -> String {
    error_payload_with_stack(kind, message, None)
}

fn error_payload_with_stack(kind: &str, message: &str, stack: Option<&str>) -> String {
    json!({
        "ok": false,
        "error": {
            "kind": kind,
            "message": message,
            "stack": stack,
        }
    })
    .to_string()
}

fn pack_result(ptr: *mut u8, len: usize) -> u64 {
    let ptr_bits = ptr as usize as u64;
    let len_bits = len as u64;
    (len_bits << 32) | (ptr_bits & 0xffff_ffff)
}

fn unpack_input(ptr: *const u8, len: usize) -> Result<String, String> {
    if len == 0 {
        return Ok(String::new());
    }

    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    str::from_utf8(bytes)
        .map_err(|error| format!("input was not valid UTF-8: {error}"))
        .map(|value| value.to_owned())
}

fn write_output(output: String) -> u64 {
    let bytes = output.into_bytes();
    let len = bytes.len();
    if len == 0 {
        return pack_result(std::ptr::null_mut(), 0);
    }

    let ptr = alloc(len);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
    }
    pack_result(ptr, len)
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }

    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// # Safety
///
/// `ptr` must be a buffer of `len` bytes that came from [`alloc`] and has not
/// been freed, either by this function or by the guest taking ownership of it
/// through `take_host_result`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    unsafe {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Panics rather than recording the failure: a guest that cannot evaluate its
/// bundle would otherwise be snapshotted in a state that fails every later
/// request, so preinitialization has to fail where the bundle is still in hand.
#[unsafe(no_mangle)]
pub extern "C" fn wizer_initialize() {
    GUEST_RUNTIME.with(|runtime| {
        let mut state = runtime.borrow_mut();
        if let Err(error) = initialize_runtime(&mut state) {
            panic!("guest initialization failed: {}", error.message);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn start_invoke(
    handler_ptr: *const u8,
    handler_len: usize,
    args_ptr: *const u8,
    args_len: usize,
) -> u64 {
    let handler_name = match unpack_input(handler_ptr, handler_len) {
        Ok(value) => value,
        Err(error) => return write_output(error_payload("InvalidInput", &error)),
    };
    let args_json = match unpack_input(args_ptr, args_len) {
        Ok(value) => value,
        Err(error) => return write_output(error_payload("InvalidInput", &error)),
    };

    GUEST_RUNTIME.with(|runtime| {
        let mut state = runtime.borrow_mut();
        match start_invoke_internal(&mut state, &handler_name, &args_json) {
            Ok(()) => 0,
            Err(error) => write_output(error_payload_with_stack(
                "InitError",
                &error.message,
                error.stack.as_deref(),
            )),
        }
    })
}

/// Start a query or mutation. Unlike `start_invoke`, the export is looked up as
/// a Convex function rather than a bare handler, so it also carries the UDF
/// type the host expects it to be registered as.
#[unsafe(no_mangle)]
pub extern "C" fn udf_start_invoke(
    function_ptr: *const u8,
    function_len: usize,
    udf_type_ptr: *const u8,
    udf_type_len: usize,
    args_ptr: *const u8,
    args_len: usize,
) -> u64 {
    let inputs: Result<(String, String, String), String> = (|| {
        Ok((
            unpack_input(function_ptr, function_len)?,
            unpack_input(udf_type_ptr, udf_type_len)?,
            unpack_input(args_ptr, args_len)?,
        ))
    })();
    let (function_name, udf_type, args_json) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => return write_output(error_payload("InvalidInput", &error)),
    };

    GUEST_RUNTIME.with(|runtime| {
        let mut state = runtime.borrow_mut();
        match start_udf_invoke_internal(&mut state, &function_name, &udf_type, &args_json) {
            Ok(()) => 0,
            Err(error) => write_output(error_payload_with_stack(
                "InitError",
                &error.message,
                error.stack.as_deref(),
            )),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn poll_invoke() -> u64 {
    GUEST_RUNTIME.with(|runtime| {
        let mut state = runtime.borrow_mut();
        match poll_invoke_internal(&mut state) {
            Ok(Some(result)) => write_output(result),
            Ok(None) => 0,
            Err(error) => write_output(error_payload_with_stack(
                "RuntimeError",
                &error.message,
                error.stack.as_deref(),
            )),
        }
    })
}
