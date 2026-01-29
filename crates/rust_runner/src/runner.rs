//! Main runner for Rust functions
//!
//! This module provides secure execution of WASM functions with:
//! - Execution timeouts
//! - Memory limits
//! - CPU fuel metering
//! - Deterministic random/time for queries/mutations
//! - Database access via DatabaseClient

use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Context,
    Result,
};
use common::{
    runtime::Runtime,
    types::UdfType,
};
use tokio::time::timeout;
use wasmtime::{
    Instance,
    Linker,
    Module,
    Store,
};

use crate::{
    determinism::DeterminismContext,
    host_functions::HostContext,
    limits::{
        ExecutionLimits,
        ResourceLimiter,
    },
    module::RustModule,
    source_maps::{
        MappedError,
        SourceMap,
        SourceMapManager,
    },
    wasi::create_secure_wasi_context,
    DatabaseClient,
};

/// Store state that holds the host context, resource limiter, and determinism
/// context
pub struct StoreState<RT: Runtime> {
    host_ctx: HostContext<RT>,
    limiter: ResourceLimiter,
    determinism: DeterminismContext,
}

impl<RT: Runtime> StoreState<RT> {
    /// Create a new store state
    pub fn new(
        host_ctx: HostContext<RT>,
        limiter: ResourceLimiter,
        determinism: DeterminismContext,
    ) -> Self {
        Self {
            host_ctx,
            limiter,
            determinism,
        }
    }

    /// Get a reference to the host context
    pub fn host_ctx(&self) -> &HostContext<RT> {
        &self.host_ctx
    }

    /// Get a mutable reference to the host context
    pub fn host_ctx_mut(&mut self) -> &mut HostContext<RT> {
        &mut self.host_ctx
    }

    /// Get a reference to the resource limiter
    pub fn limiter(&self) -> &ResourceLimiter {
        &self.limiter
    }

    /// Get a mutable reference to the resource limiter
    pub fn limiter_mut(&mut self) -> &mut ResourceLimiter {
        &mut self.limiter
    }
}

impl<RT: Runtime> std::ops::Deref for StoreState<RT> {
    type Target = HostContext<RT>;

    fn deref(&self) -> &Self::Target {
        &self.host_ctx
    }
}

impl<RT: Runtime> std::ops::DerefMut for StoreState<RT> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.host_ctx
    }
}

/// Runner for Rust/WASM functions
pub struct RustFunctionRunner<RT: Runtime> {
    engine: Arc<wasmtime::Engine>,
    runtime: RT,
    module_cache: std::sync::Mutex<std::collections::HashMap<String, Module>>,
    source_map_manager: SourceMapManager,
}

impl<RT: Runtime> Clone for RustFunctionRunner<RT> {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            runtime: self.runtime.clone(),
            module_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            source_map_manager: self.source_map_manager.clone(),
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
            source_map_manager: SourceMapManager::new(),
        })
    }

    /// Load a source map for a module
    pub fn load_source_map(
        &mut self,
        module_id: impl Into<String>,
        source_map_json: &str,
    ) -> Result<Arc<SourceMap>> {
        self.source_map_manager
            .load_from_json(module_id, source_map_json)
    }

    /// Get a cached source map
    pub fn get_source_map(&self, module_id: &str) -> Option<Arc<SourceMap>> {
        self.source_map_manager.get(module_id)
    }

    /// Map an error using source maps
    fn map_error(
        &self,
        module_id: &str,
        error: anyhow::Error,
        wasm_offset: Option<u32>,
    ) -> anyhow::Error {
        if let Some(offset) = wasm_offset {
            if let Some(source_map) = self.source_map_manager.get(module_id) {
                if let Some(location) = source_map.lookup(offset) {
                    let mapped = MappedError::new(error.to_string())
                        .with_wasm_offset(offset)
                        .with_source_location(location.clone());
                    return anyhow::anyhow!(mapped.format());
                }
            }
        }
        error
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
        self.run_function_with_db(
            udf_type,
            module,
            function_name,
            args,
            seed,
            timestamp_ms,
            None,
        )
        .await
    }

    /// Run a Rust function with database access
    ///
    /// # Arguments
    /// * `udf_type` - Type of UDF (Query, Mutation, Action, HttpAction)
    /// * `module` - The compiled Rust module
    /// * `function_name` - Name of the function to call
    /// * `args` - Arguments to pass to the function
    /// * `seed` - Random seed for deterministic execution (queries/mutations)
    /// * `timestamp_ms` - Virtual timestamp for deterministic execution
    /// * `database_client` - Optional database client for database operations
    pub async fn run_function_with_db(
        &self,
        udf_type: UdfType,
        module: &RustModule,
        function_name: &str,
        args: Vec<serde_json::Value>,
        seed: u64,
        timestamp_ms: i64,
        database_client: Option<Arc<dyn DatabaseClient>>,
    ) -> Result<serde_json::Value> {
        // Get execution limits based on UDF type
        let limits = Self::get_limits_for_udf_type(udf_type);

        // Get or compile the module
        let wasm_module = self.get_or_compile_module(module).await?;

        // Set up secure WASI context with minimal capabilities
        let wasi_ctx = create_secure_wasi_context();

        // Create host context with WASI and runtime
        let mut host_ctx = HostContext::new(wasi_ctx, udf_type, self.runtime.clone());

        // Add database client if provided
        if let Some(db_client) = database_client {
            host_ctx = host_ctx.with_database_client(db_client);
        }

        // Create determinism context based on UDF type
        let determinism = match udf_type {
            UdfType::Query | UdfType::Mutation => {
                DeterminismContext::deterministic(seed, timestamp_ms)
            },
            UdfType::Action | UdfType::HttpAction => DeterminismContext::non_deterministic(),
        };

        // Create resource limiter
        let limiter = ResourceLimiter::new(limits.max_memory_bytes, limits.max_table_size);

        // Create store state combining host context, limiter, and determinism
        let state = StoreState::new(host_ctx, limiter, determinism);

        // Create store with context and limits
        let mut store = Store::new(&self.engine, state);

        // Set up resource limiter
        store.limiter(|state| state.limiter_mut());

        // Set fuel for CPU limiting
        store
            .set_fuel(limits.max_fuel)
            .context("Failed to set fuel for store")?;

        // Create a linker and add WASI and host functions
        let mut linker: Linker<StoreState<RT>> = Linker::new(&self.engine);

        // Add WASI functions to the linker
        // Access the wasi field through host_ctx (which is pub(crate))
        wasmtime_wasi::add_to_linker(&mut linker, |state: &mut StoreState<RT>| {
            &mut state.host_ctx.wasi
        })?;

        // Add Convex host functions to the linker
        // The host functions need to work with StoreState, not HostContext directly
        // We create them with a wrapper that accesses HostContext through Deref
        add_convex_host_functions(&mut linker)?;

        // Instantiate the module with the linker (includes host functions)
        let instance = linker.instantiate(&mut store, &wasm_module)?;

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

    /// Run a Rust function with simplified API (uses default seed and current
    /// time)
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
    async fn call_with_timeout<'a, Params: wasmtime::WasmParams, Results: wasmtime::WasmResults>(
        &self,
        store: &'a mut Store<StoreState<RT>>,
        func: &'a wasmtime::TypedFunc<Params, Results>,
        params: Params,
        duration: Duration,
    ) -> Result<Results> {
        // Use tokio's timeout for async execution
        let result = timeout(duration, func.call_async(store, params)).await;

        match result {
            Ok(Ok(results)) => Ok(results),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => anyhow::bail!("Function execution timed out after {:?}", duration),
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
        store: &mut Store<StoreState<RT>>,
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
            .or_else(|| {
                instance
                    .get_typed_func::<i32, i32>(&mut *store, "alloc")
                    .ok()
            })
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
        store: &mut Store<StoreState<RT>>,
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

/// Add Convex host functions to the linker
///
/// This function creates and links all Convex-specific host functions
/// (database, storage, http, logging, etc.) to the WASM linker.
fn add_convex_host_functions<RT: Runtime>(
    linker: &mut Linker<StoreState<RT>>,
) -> Result<()> {
    // Database query function
    linker.func_wrap(
        "env",
        "__convex_db_query",
        move |mut caller: wasmtime::Caller<'_, StoreState<RT>>, table_ptr: i32, table_len: i32| -> i32 {
            // Get the database client first, then drop the borrow
            let db_client = caller.data().database_client();

            // Read table name from memory
            let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return -1,
            };

            let mut table_bytes = vec![0u8; table_len as usize];
            if mem.read(&caller, table_ptr as usize, &mut table_bytes).is_err() {
                return -1;
            }

            let table_name = match String::from_utf8(table_bytes) {
                Ok(s) => s,
                Err(_) => return -1,
            };

            // Query the database
            let result = if let Some(db_client) = db_client {
                match db_client.query(table_name) {
                    Ok(docs) => {
                        let docs_json: Vec<serde_json::Value> = docs
                            .into_iter()
                            .map(|(id, value)| {
                                serde_json::json!({
                                    "id": id,
                                    "value": value
                                })
                            })
                            .collect();
                        serde_json::json!({
                            "success": true,
                            "data": docs_json,
                            "error": None::<String>
                        })
                    },
                    Err(e) => serde_json::json!({
                        "success": false,
                        "data": None::<Vec<serde_json::Value>>,
                        "error": format!("Database query failed: {}", e)
                    }),
                }
            } else {
                serde_json::json!({
                    "success": true,
                    "data": Vec::<serde_json::Value>::new(),
                    "error": None::<String>
                })
            };

            // Write result to memory
            let result_bytes = serde_json::to_vec(&result).unwrap_or_default();
            let result_len = result_bytes.len() as i32;

            // Allocate memory for result
            let alloc_func = match caller.get_export("__convex_alloc").and_then(|e| e.into_func()) {
                Some(f) => f,
                None => return -1,
            };

            let mut result_ptr = [wasmtime::Val::I32(0)];
            if alloc_func.call(&mut caller, &[wasmtime::Val::I32(result_len + 4)], &mut result_ptr).is_err() {
                return -1;
            }

            let ptr = result_ptr[0].i32().unwrap_or(0);
            if ptr == 0 {
                return -1;
            }

            // Write length prefix
            let len_bytes = result_len.to_le_bytes();
            let _ = mem.write(&mut caller, ptr as usize, &len_bytes);
            // Write data
            let _ = mem.write(&mut caller, ptr as usize + 4, &result_bytes);

            ptr
        },
    )?;

    // Note: Additional host functions would be added here following the same pattern
    // For now, we implement the core database functions. The full implementation
    // would include: db_get, db_insert, db_patch, db_delete, db_count,
    // storage_store, storage_get, http_fetch, log, random, etc.

    Ok(())
}

#[cfg(test)]
mod tests {
    use common::runtime::testing::TestRuntime;

    use super::*;

    #[test]
    fn test_limits_for_udf_types() {
        let query_limits =
            RustFunctionRunner::<TestRuntime>::get_limits_for_udf_type(UdfType::Query);
        assert_eq!(query_limits.max_duration, Duration::from_secs(30));

        let action_limits =
            RustFunctionRunner::<TestRuntime>::get_limits_for_udf_type(UdfType::Action);
        assert_eq!(action_limits.max_duration, Duration::from_secs(300));
    }
}
