# Rust Runner Security Considerations

## Overview

The `rust_runner` crate executes user-provided WebAssembly (WASM) modules compiled from Rust source code. This document outlines security measures and considerations specific to the Rust/WASM execution path.

## Threat Model

### Trusted Boundaries
- **Trusted**: Convex backend infrastructure, host functions, database
- **Untrusted**: User-compiled WASM modules, function arguments, environment variables

### Attack Vectors
1. Malicious WASM bytecode
2. Resource exhaustion (CPU, memory, stack)
3. Host function abuse
4. Information leakage through side channels
5. Non-deterministic operations breaking system invariants

## Security Measures

### 1. WASM Sandboxing

WebAssembly provides inherent sandboxing through:
- Memory isolation (linear memory model)
- Capability-based security (no implicit access to host)
- Structured control flow (no arbitrary jumps)

We enhance this with:

```rust
// wasmtime configuration
config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
config.async_support(true);  // Enables timeout/cancellation
// TODO: Add fuel metering for CPU limits
// config.consume_fuel(true);
```

### 2. Resource Limits

**Memory Limits**: Each WASM instance gets a fixed memory limit (default: 256MB to match V8 isolate)

```rust
// TODO: Configure memory limits
store.limiter(|state| &mut state.limiter);
```

**Execution Time**: Functions must complete within timeout (configurable, default: 30s for queries, 5min for actions)

**Stack Depth**: Limited by wasmtime default stack size

### 3. WASI Capability Restrictions

Current implementation uses minimal WASI:
- ✅ Stdio inheritance (for logging)
- ❌ No filesystem access
- ❌ No network access (HTTP via host functions)
- ❌ No environment variable inheritance

```rust
// Secure WASI configuration
let wasi_ctx = WasiCtxBuilder::new()
    .inherit_stdio()  // Only for structured logging
    // Explicitly NOT calling:
    // .inherit_env()     - Don't leak host env vars
    // .inherit_args()    - Don't pass host args
    // .inherit_network() - No direct network access
    .build();
```

### 4. Host Function Security

All host functions must:
1. Validate all pointer/length pairs before dereferencing
2. Check bounds against WASM memory size
3. Validate deserialized data structures
4. Return errors rather than panic
5. Not trust WASM-provided sizes without verification

Example secure pattern:
```rust
fn db_get(caller: &mut Caller<'_, Context>, ptr: i32, len: i32) -> i32 {
    // Validate bounds
    let memory = caller.get_export("memory").unwrap();
    let memory_size = memory.data_size(caller);

    if ptr < 0 || len < 0 || (ptr as usize) + (len as usize) > memory_size {
        return -1; // Error: out of bounds
    }

    // Safely read
    let data = &memory.data(caller)[ptr as usize..(ptr + len) as usize];

    // Validate UTF-8
    let id = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return -2, // Error: invalid encoding
    };

    // Validate format (e.g., document ID pattern)
    if !is_valid_document_id(id) {
        return -3; // Error: invalid format
    }

    // Proceed with operation...
}
```

### 5. Determinism Enforcement

For queries and mutations, determinism is critical for:
- Caching query results
- Transaction replay
- Consistent replication

**Enforced restrictions:**
- Random numbers: Seeded from transaction context
- Time: Uses virtual time from execution context
- No access to: hardware counters, thread IDs, process info

**Host function availability by function type:**

| Function | Query | Mutation | Action |
|----------|-------|----------|--------|
| db_query | ✅ | ✅ | ✅ |
| db_get | ✅ | ✅ | ✅ |
| db_insert | ❌ | ✅ | ✅ |
| db_patch | ❌ | ✅ | ✅ |
| db_delete | ❌ | ✅ | ✅ |
| http_fetch | ❌ | ❌ | ✅ |
| random_bytes | ✅* | ✅* | ✅ |
| now | ✅** | ✅** | ✅ |

*Seeded random for determinism
**Virtual time for determinism

### 6. Module Validation

Before execution, WASM modules should be validated:

```rust
// TODO: Add validation pass
fn validate_module(wasm_bytes: &[u8]) -> Result<(), ValidationError> {
    // Check for:
    // - Valid WASM structure
    // - No invalid opcodes
    // - Memory limits within bounds
    // - No floating point (optional, for determinism)
    // - Expected exports present
}
```

### 7. Supply Chain Security

Rust dependencies are managed through:
- `Cargo.lock` for reproducible builds
- `cargo audit` for vulnerability scanning
- Review of unsafe code usage

## Comparison to TypeScript/V8 Security

### Advantages of WASM Approach
1. **Memory safety**: No buffer overflows in WASM sandbox
2. **Capability isolation**: Explicit host function imports
3. **Deterministic execution**: Easier to control than V8
4. **No JIT spraying**: WASM is AOT compiled

### Additional Risks vs V8
1. **Newer codebase**: Less battle-tested than V8
2. **Host function bugs**: Each host function is custom code
3. **WASI escape**: Potential bugs in WASI implementation
4. **Memory sharing**: Host functions access WASM memory directly

## Security Checklist

- [ ] WASM module validation before execution
- [ ] Memory limits enforced
- [ ] Execution timeouts configured
- [ ] WASI capabilities minimized
- [ ] All host functions validate inputs
- [ ] No panics in host functions (return errors)
- [ ] Deterministic random/time for queries/mutations
- [ ] Resource usage monitoring
- [ ] Audit logging for security events
- [ ] Regular dependency audits

## Incident Response

If a security issue is discovered:
1. Isolate affected deployments
2. Review host function implementations
3. Check for any breaches of WASM sandbox
4. Audit logs for suspicious activity
5. Update security documentation
