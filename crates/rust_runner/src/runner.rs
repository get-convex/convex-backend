//! Main runner for Rust functions
//!
//! This module provides secure execution of WASM functions with:
//! - Execution timeouts
//! - Memory limits
//! - CPU fuel metering
//! - Deterministic random/time for queries/mutations

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;
use wasmtime::{Instance, Module, Store};
use wasi_common::WasiCtx;

use common::runtime::Runtime;
use common::types::UdfType;

use crate::determinism::DeterminismContext;
use crate::limits::{ExecutionLimits, ResourceLimiter};
use crate::module::RustModule;
use crate::wasi::create_secure_wasi_context;

/// Store state that holds both WASI context and resource limiter
pub struct StoreState {
    wasi: WasiCtx,
    limiter: ResourceLimiter,
    determinism: DeterminismContext,
}

impl StoreState {
    /// Create a new store state with the given limits and determinism context
    pub fn new(wasi: WasiCtx, limiter: ResourceLimiter, determinism: DeterminismContext) -> Self {
        Self {
            wasi,
            limiter,
            determinism,
        }
    }

    /// Get a reference to the WASI context
    pub fn wasi(&self) -> &WasiCtx {
        &self.wasi
    }

    /// Get a mutable reference to the WASI context
    pub fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    /// Get a reference to the determinism context
    pub fn determinism(&self) -> &DeterminismContext {
        &self.determinism
    }
}

/// Runner for Rust/WASM functions
pub struct RustFunctionRunner<RT: Runtime> {
    engine: Arc<wasmtime::Engine>,
    runtime: RT,
    module_cache: std::sync::Mutex<std::collections::HashMap<String, Module>>,
}

impl<RT: Runtime> Clone for RustFunctionRunner<RT> {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            runtime: self.runtime.clone(),
            module_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl<RT: Runtime> RustFunctionRunner<RT> {
    /// Create a new Rust function runner
    pub async fn new(runtime: RT) -> Result<Self> {
        let engine = crate::init_runtime().await?;

        Ok(Self {
            engine,
            runtime,
            module_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }

    /// Run a Rust function with full security controls
    ///
    /// # Arguments
    /// * `udf_type` - Type of UDF (Query, Mutation, Action, HttpAction)
    /// * `module` - The compiled Rust module
    /// * `function_name` - Name of the function to call
    /// * `args` - Arguments to pass to the function
    /// * `seed` - Random seed for deterministic execution (queries/mutations)
    /// * `timestamp_ms` - Virtual timestamp for deterministic execution
    pub async fn run_function(
        &self,
        udf_type: UdfType,
        module: &RustModule,
        function_name: &str,
        args: Vec<serde_json::Value>,
        seed: u64,
        timestamp_ms: i64,
    ) -> Result<serde_json::Value> {
        // Get execution limits based on UDF type
        let limits = Self::get_limits_for_udf_type(udf_type);

        // Get or compile the module
        let wasm_module = self.get_or_compile_module(module).await?;

        // Set up secure WASI context with minimal capabilities
        let wasi_ctx = create_secure_wasi_context();

        // Create determinism context based on UDF type
        let determinism = match udf_type {
            UdfType::Query | UdfType::Mutation => {
                DeterminismContext::deterministic(seed, timestamp_ms)
            }
            UdfType::Action | UdfType::HttpAction => DeterminismContext::non_deterministic(),
        };

        // Create resource limiter
        let limiter = ResourceLimiter::new(limits.max_memory_bytes, limits.max_table_size);

        // Create store state
        let state = StoreState::new(wasi_ctx, limiter, determinism);

        // Create store with context and limits
        let mut store = Store::new(&self.engine, state);

        // Set up resource limiter
        store.limiter(|state| &mut state.limiter);

        // Set fuel for CPU limiting
        store.set_fuel(limits.max_fuel)
            .context("Failed to set fuel for store")?;

        // Instantiate the module
        let instance = Instance::new(&mut store, &wasm_module, &[])?;

        // Get the exported function
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, function_name)
            .with_context(|| format!("Function '{}' not found in WASM module", function_name))?;

        // Serialize arguments
        let args_json = serde_json::to_string(&args)?;
        let args_bytes = args_json.into_bytes();

        // Allocate memory in WASM and write args
        let (ptr, len) = self
            .allocate_and_write(&mut store, &instance, &args_bytes)
            .await?;

        // Call the function with timeout
        let result_ptr = self
            .call_with_timeout(&mut store, &func, (ptr, len), limits.max_duration)
            .await?;

        // Read the result from WASM memory
        let result = self.read_result(&mut store, &instance, result_ptr).await?;

        // Parse result as JSON
        let result_value: serde_json::Value = serde_json::from_slice(&result)?;

        Ok(result_value)
    }

    /// Run a Rust function with simplified API (uses default seed and current time)
    pub async fn run_function_simple(
        &self,
        udf_type: UdfType,
        module: &RustModule,
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let seed = rand::random();
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        self.run_function(udf_type, module, function_name, args, seed, timestamp_ms)
            .await
    }

    /// Get execution limits for a given UDF type
    fn get_limits_for_udf_type(udf_type: UdfType) -> ExecutionLimits {
        match udf_type {
            UdfType::Query => ExecutionLimits::query(),
            UdfType::Mutation => ExecutionLimits::mutation(),
            UdfType::Action => ExecutionLimits::action(),
            UdfType::HttpAction => ExecutionLimits::http_action(),
        }
    }

    /// Call a WASM function with a timeout
    async fn call_with_timeout<
        'a,
        Params: wasmtime::WasmParams,
        Results: wasmtime::WasmResults,
    >(
        &self,
        store: &'a mut Store<StoreState>,
        func: &'a wasmtime::TypedFunc<Params, Results>,
        params: Params,
        duration: Duration,
    ) -> Result<Results> {
        // Use tokio's timeout for async execution
        let result = timeout(duration, func.call_async(store, params)).await;

        match result {
            Ok(Ok(results)) => Ok(results),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => anyhow::bail!(
                "Function execution timed out after {:?}",
                duration
            ),
        }
    }

    async fn get_or_compile_module(&self, module: &RustModule) -> Result<Module> {
        let mut cache = self.module_cache.lock().unwrap();

        if let Some(cached) = cache.get(&module.id) {
            return Ok(cached.clone());
        }

        let wasm_bytes = module.wasm_binary();
        let compiled = Module::new(&self.engine, wasm_bytes)?;

        cache.insert(module.id.clone(), compiled.clone());
        Ok(compiled)
    }

    async fn allocate_and_write(
        &self,
        store: &mut Store<StoreState>,
        instance: &Instance,
        data: &[u8],
    ) -> Result<(i32, i32)> {
        // Get the memory export
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow::anyhow!("memory export not found"))?;

        // Get the alloc function (exported by the Rust module)
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut *store, "__convex_alloc")
            .ok()
            .or_else(|| instance.get_typed_func::<i32, i32>(&mut *store, "alloc").ok())
            .ok_or_else(|| anyhow::anyhow!("alloc function not found"))?;

        // Allocate memory
        let ptr = alloc.call_async(&mut *store, data.len() as i32).await?;

        // Write data to memory
        let mem_slice = memory.data_mut(&mut *store);
        let ptr_usize = ptr as usize;

        // Bounds check
        if ptr_usize + data.len() > mem_slice.len() {
            anyhow::bail!("Allocated memory out of bounds");
        }

        mem_slice[ptr_usize..ptr_usize + data.len()].copy_from_slice(data);

        Ok((ptr, data.len() as i32))
    }

    async fn read_result(
        &self,
        store: &mut Store<StoreState>,
        instance: &Instance,
        result_ptr: i32,
    ) -> Result<Vec<u8>> {
        // Get the memory export
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow::anyhow!("memory export not found"))?;

        // Read the length (first 4 bytes as little-endian u32)
        let mem_slice = memory.data(store);
        let ptr_usize = result_ptr as usize;

        // Bounds check for length read
        if ptr_usize + 4 > mem_slice.len() {
            anyhow::bail!("Result pointer out of bounds");
        }

        let len = u32::from_le_bytes([
            mem_slice[ptr_usize],
            mem_slice[ptr_usize + 1],
            mem_slice[ptr_usize + 2],
            mem_slice[ptr_usize + 3],
        ]) as usize;

        // Bounds check for data read
        if ptr_usize + 4 + len > mem_slice.len() {
            anyhow::bail!("Result data out of bounds");
        }

        // Read the data (after the length prefix)
        let data = mem_slice[ptr_usize + 4..ptr_usize + 4 + len].to_vec();

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limits_for_udf_types() {
        let query_limits = RustFunctionRunner::<common::testing::TestRuntime>::get_limits_for_udf_type(UdfType::Query);
        assert_eq!(query_limits.max_duration, Duration::from_secs(30));

        let action_limits = RustFunctionRunner::<common::testing::TestRuntime>::get_limits_for_udf_type(UdfType::Action);
        assert_eq!(action_limits.max_duration, Duration::from_secs(300));
    }
}
