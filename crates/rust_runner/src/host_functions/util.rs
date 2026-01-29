//! Utility host functions for WASM guest
//!
//! These functions provide logging, random number generation, and time access.
//!
//! # Security Notes
//!
//! - **Logging**: Rate-limited to prevent log spam (MAX_LOG_LINES per execution)
//! - **Random**: Deterministic (seeded) for queries/mutations, system random for actions
//! - **Time**: Virtual time for queries/mutations, system time for actions

use common::runtime::Runtime;
use common::types::UdfType;
use serde::{Deserialize, Serialize};
use wasmtime::{Caller, Func, FuncType, Store, Val, ValType};

use crate::host_functions::{write_json_response, HostContext};

/// Maximum number of log lines per function execution
const MAX_LOG_LINES: usize = 1000;

/// Maximum log message length (10 KB)
const MAX_LOG_MESSAGE_LENGTH: usize = 10 * 1024;

/// Log levels matching JavaScript console
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum LogLevel {
    Debug = 0,
    Log = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl LogLevel {
    fn from_i32(level: i32) -> Option<Self> {
        match level {
            0 => Some(LogLevel::Debug),
            1 => Some(LogLevel::Log),
            2 => Some(LogLevel::Info),
            3 => Some(LogLevel::Warn),
            4 => Some(LogLevel::Error),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Log => "LOG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Random bytes request
#[derive(Debug, Serialize)]
struct RandomBytesResult {
    success: bool,
    data: Option<Vec<u8>>,
    error: Option<String>,
}

/// Time result
#[derive(Debug, Serialize)]
struct TimeResult {
    success: bool,
    timestamp_ms: i64,
    error: Option<String>,
}

/// Create all utility host functions
pub fn create_util_functions<RT: Runtime>(
    store: &mut Store<HostContext<RT>>,
) -> Vec<(String, Func)> {
    vec![
        ("__convex_log".to_string(), create_log_function(store)),
        (
            "__convex_random_bytes".to_string(),
            create_random_bytes_function(store),
        ),
        ("__convex_now_ms".to_string(), create_now_ms_function(store)),
        (
            "__convex_get_user_identity".to_string(),
            create_get_user_identity_function(store),
        ),
    ]
}

/// Create the log host function
///
/// Parameters:
/// - i32: log level (0=debug, 1=log, 2=info, 3=warn, 4=error)
/// - i32: message pointer
/// - i32: message length
///
/// Returns: nothing (void)
fn create_log_function<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32, ValType::I32], // level, msg_ptr, msg_len
        vec![],                                         // no return
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], _results: &mut [Val]| {
            let level_i32 = params[0].i32().unwrap_or(1);
            let msg_ptr = params[1].i32().unwrap_or(0);
            let msg_len = params[2].i32().unwrap_or(0);

            // Check rate limit
            if !caller.data_mut().check_log_rate_limit() {
                // Log rate limit exceeded - return error indicator
                eprintln!("[WARN] Log rate limit exceeded (max {} lines per execution). Subsequent log messages dropped.", MAX_LOG_LINES);
                return Ok(());
            }

            // Parse log level
            let level = LogLevel::from_i32(level_i32).unwrap_or(LogLevel::Log);

            // Read message from WASM memory
            let message = match read_memory_string(&mut caller, msg_ptr, msg_len) {
                Ok(s) => {
                    // Truncate if too long
                    if s.len() > MAX_LOG_MESSAGE_LENGTH {
                        eprintln!("[WARN] Log message truncated (exceeded {} bytes)", MAX_LOG_MESSAGE_LENGTH);
                        format!("{}... (truncated)", &s[..MAX_LOG_MESSAGE_LENGTH])
                    } else {
                        s
                    }
                },
                Err(e) => {
                    eprintln!("[ERROR] Failed to read log message from WASM memory: {}", e);
                    return Ok(());
                }
            };

            // Output to stdout/stderr based on level
            match level {
                LogLevel::Debug | LogLevel::Log | LogLevel::Info => {
                    println!("[{}] {}", level.as_str(), message);
                },
                LogLevel::Warn | LogLevel::Error => {
                    eprintln!("[{}] {}", level.as_str(), message);
                },
            }

            Ok(())
        },
    )
}

/// Create the random_bytes host function
///
/// Fills a buffer with random bytes. Uses deterministic random for queries/mutations
/// and cryptographically secure random for actions.
///
/// Parameters:
/// - i32: buffer pointer
/// - i32: buffer length
///
/// Returns:
/// - i32: result pointer (JSON with success flag)
fn create_random_bytes_function<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // buf_ptr, buf_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let buf_ptr = params[0].i32().unwrap_or(0);
            let buf_len = params[1].i32().unwrap_or(0);

            if buf_ptr < 0 || buf_len < 0 || buf_len > 65536 {
                // Limit to 64KB per call
                let error_result = RandomBytesResult {
                    success: false,
                    data: None,
                    error: Some("Invalid buffer size (max 64KB)".to_string()),
                };
                let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                results[0] = Val::I32(ptr);
                return Ok(());
            }

            let buf_len = buf_len as usize;
            let mut buffer = vec![0u8; buf_len];

            // Fill with random bytes based on execution context
            let udf_type = caller.data().udf_type();
            if matches!(udf_type, UdfType::Query | UdfType::Mutation) {
                // Deterministic random for queries/mutations
                if let Err(e) = caller.data_mut().fill_random_bytes_deterministic(&mut buffer) {
                    let error_result = RandomBytesResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to generate deterministic random bytes: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                }
            } else {
                // Secure random for actions
                caller.data_mut().fill_random_bytes_secure(&mut buffer);
            }

            // Write random bytes to WASM memory
            let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => {
                    let error_result = RandomBytesResult {
                        success: false,
                        data: None,
                        error: Some("Memory not found".to_string()),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            if let Err(e) = memory.write(&mut caller, buf_ptr as usize, &buffer) {
                let error_result = RandomBytesResult {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to write to memory: {}", e)),
                };
                let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                results[0] = Val::I32(ptr);
                return Ok(());
            }

            // Return success
            let success_result = RandomBytesResult {
                success: true,
                data: None,
                error: None,
            };
            let ptr = write_json_response(&mut caller, &success_result).unwrap_or(-1);
            results[0] = Val::I32(ptr);

            Ok(())
        },
    )
}

/// Create the now_ms host function
///
/// Returns the current timestamp in milliseconds.
/// Uses virtual time for queries/mutations and system time for actions.
///
/// Parameters: none
///
/// Returns:
/// - i32: result pointer (JSON with timestamp_ms)
fn create_now_ms_function<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![],           // no parameters
        vec![ValType::I32], // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, _params: &[Val], results: &mut [Val]| {
            let udf_type = caller.data().udf_type();

            let timestamp_ms = if matches!(udf_type, UdfType::Query | UdfType::Mutation) {
                // Virtual time for queries/mutations
                caller.data().deterministic_timestamp_ms()
            } else {
                // System time for actions
                match std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64) {
                    Ok(ts) => ts,
                    Err(e) => {
                        eprintln!("[ERROR] System time is before Unix epoch: {}", e);
                        0
                    }
                }
            };

            let time_result = TimeResult {
                success: true,
                timestamp_ms,
                error: None,
            };

            let ptr = write_json_response(&mut caller, &time_result).unwrap_or(-1);
            results[0] = Val::I32(ptr);

            Ok(())
        },
    )
}

/// Read data from WASM memory
fn read_memory<RT: Runtime>(
    caller: &mut Caller<'_, HostContext<RT>>,
    ptr: i32,
    len: i32,
) -> anyhow::Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        anyhow::bail!("Invalid memory pointer or length");
    }

    let ptr = ptr as usize;
    let len = len as usize;

    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("Memory export not found"))?;

    let mut data = vec![0u8; len];
    memory.read(caller, ptr, &mut data)?;

    Ok(data)
}

/// Read a string from WASM memory
fn read_memory_string<RT: Runtime>(
    caller: &mut Caller<'_, HostContext<RT>>,
    ptr: i32,
    len: i32,
) -> anyhow::Result<String> {
    let bytes = read_memory(caller, ptr, len)?;
    String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
}

/// Result from the get_user_identity host function
#[derive(Debug, Serialize, Deserialize)]
struct UserIdentityResult {
    success: bool,
    identity: Option<serde_json::Value>,
    error: Option<String>,
}

/// Create the __convex_get_user_identity host function
///
/// Returns the current user's identity (or null if not authenticated).
pub fn create_get_user_identity_function<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![],           // No parameters
        vec![ValType::I32], // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, _params: &[Val], results: &mut [Val]| {
            let result = match caller.data().identity() {
                Some(identity) => UserIdentityResult {
                    success: true,
                    identity: Some(identity.clone()),
                    error: None,
                },
                None => UserIdentityResult {
                    success: true,
                    identity: None,
                    error: None,
                },
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);

            Ok(())
        },
    )
}
