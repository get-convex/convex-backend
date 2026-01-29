# Security Gap Analysis: Rust vs TypeScript Implementation

## Current Status

This document identifies security gaps between the Rust/WASM implementation and the production TypeScript/V8 implementation.

## Critical Gaps (Must Fix Before Production)

### 1. ⚠️ Execution Timeouts

**TypeScript Implementation:**
- Queries: 30 second timeout
- Mutations: 30 second timeout
- Actions: 5 minute timeout
- Implemented via `Timeout` struct with pause/resume capability

**Rust Implementation:**
- ❌ No timeout mechanism implemented
- Functions can run indefinitely

**Fix Required:**
```rust
// Add to runner.rs
pub struct ExecutionLimits {
    pub max_execution_time: Duration,
    pub max_memory_bytes: usize,
}

// Use wasmtime's epoch_deadline or async timeout
store.set_epoch_deadline(Some(limit));
```

### 2. ⚠️ Memory Limits

**TypeScript Implementation:**
- V8 heap limit: ~256MB default
- ArrayBuffer memory limit tracked separately
- Near-heap-limit callback raises limit temporarily for cleanup

**Rust Implementation:**
- ❌ No memory limits enforced
- WASM can allocate unlimited memory

**Fix Required:**
```rust
// Add memory limiter to store
store.limiter(|state| &mut state.limiter);

// Implement ResourceLimiter
impl ResourceLimiter for MemoryLimiter {
    fn memory_growing(&mut self, current: usize, desired: usize, maximum: Option<usize>) -> bool {
        desired <= self.max_memory_bytes
    }
}
```

### 3. ⚠️ Deterministic Random/Time

**TypeScript Implementation:**
- `Math.random()` seeded from execution context for queries/mutations
- `Date.now()` uses virtual unix_timestamp from context
- `CryptoRng` only available in actions

**Rust Implementation:**
- ❌ No control over random number generation
- ❌ No control over time sources
- Guest can use `std::time::Instant::now()` directly

**Fix Required:**
Option A: Provide deterministic host functions and recommend against std usage
Option B: Use custom std that replaces random/time (complex)

```rust
// Host functions for deterministic operations
fn __convex_random_bytes_deterministic(buf: &mut [u8]); // Seeded RNG
fn __convex_now_ms() -> u64; // Virtual time from context

// Document that std::random and std::time should not be used
```

### 4. ⚠️ Fuel Metering (CPU Limits)

**TypeScript Implementation:**
- V8 execution naturally limited by event loop
- CPU profiling and throttling

**Rust Implementation:**
- ❌ No CPU usage limits
- Infinite loop can consume 100% CPU

**Fix Required:**
```rust
// Enable fuel consumption
config.consume_fuel(true);

// Set fuel limit (instructions)
store.add_fuel(10_000_000_000)?; // ~10B instructions

// Refuel periodically if needed
```

### 5. ⚠️ Host Function Input Validation

**TypeScript Implementation:**
- All ops validate arguments with serde_v8
- Bounds checking on all memory/array operations
- Type coercion with error handling

**Rust Implementation:**
- ⚠️ Partial validation in current stubs
- Need comprehensive bounds checking on all WASM memory access

**Fix Required:**
See `host_functions.rs` TODOs - need to implement all with proper validation.

## Medium Priority Gaps

### 6. Module Validation

**TypeScript Implementation:**
- JavaScript parsed by V8 (guaranteed valid)
- Source maps for debugging

**Rust Implementation:**
- ⚠️ Should validate WASM before instantiation
- Check for invalid opcodes
- Verify imports/exports match expected interface

### 7. Stack Depth Limiting

**TypeScript Implementation:**
- V8 has built-in stack depth limits
- Stack overflow handled gracefully

**Rust Implementation:**
- wasmtime has default stack limits
- Should verify they're appropriate

### 8. Error Handling

**TypeScript Implementation:**
- All errors converted to JsError with stack traces
- Source map support for debugging

**Rust Implementation:**
- ⚠️ Need proper error conversion
- Should not expose internal details to user

## Low Priority Gaps

### 9. Concurrency Limiting

**TypeScript Implementation:**
- `ConcurrencyLimiter` for isolate pool
- Per-tenant concurrency controls

**Rust Implementation:**
- Will be handled at higher level (FunctionRunner)
- Not a WASM-specific concern

### 10. Logging/Monitoring

**TypeScript Implementation:**
- Comprehensive metrics and logging
- Log line collection from user code

**Rust Implementation:**
- ⚠️ Basic stdio capture only
- Need structured logging integration

## Security Checklist for Production

- [ ] Execution timeouts (30s query/mutation, 5min action)
- [ ] Memory limits (256MB default)
- [ ] CPU fuel metering
- [ ] Deterministic random for queries/mutations
- [ ] Virtual time for queries/mutations
- [ ] Host function input validation
- [ ] WASM module validation
- [ ] Secure error messages
- [ ] Resource cleanup on panic/timeout
- [ ] Audit logging
- [ ] Integration with Convex metrics

## Design Decisions Needed

### Determinism Enforcement

**Option 1: Trust but Verify**
- Document that queries/mutations must be deterministic
- Provide host functions for deterministic random/time
- Detect non-determinism through retries (expensive)

**Option 2: Sandboxed std**
- Provide custom std library with controlled random/time
- Requires building with special target

**Option 3: Capability-based**
- Different WASM imports for queries vs actions
- Queries get deterministic imports only
- Actions get system imports

**Recommendation:** Option 3 - matches TypeScript design where different capabilities are available based on function type.

## Implementation Priority

1. **Week 1**: Timeouts, memory limits, fuel metering
2. **Week 2**: Deterministic imports for queries/mutations
3. **Week 3**: Host function validation
4. **Week 4**: Error handling, logging, metrics
