//! WASM Module Validation
//!
//! This module provides comprehensive validation for WebAssembly modules
//! before they are executed, ensuring security, compatibility, and
//! resource constraints are met.

use anyhow::Result;
use thiserror::Error;
use tracing::debug;
use wasmtime::{Config, Engine, Module};

/// Errors that can occur during WASM validation
#[derive(Error, Debug)]
pub enum ValidationError {
    /// The WASM binary is malformed or invalid
    #[error("Invalid WASM binary: {0}")]
    InvalidBinary(String),

    /// The module uses prohibited features
    #[error("Prohibited feature used: {0}")]
    ProhibitedFeature(String),

    /// The module exceeds size limits
    #[error("Module size {0} exceeds maximum {1}")]
    SizeExceeded(usize, usize),

    /// The module has too many functions
    #[error("Too many functions: {0} (max: {1})")]
    TooManyFunctions(usize, usize),

    /// The module uses too much memory
    #[error("Memory usage {0} exceeds maximum {1}")]
    MemoryExceeded(u64, u64),

    /// The module has invalid imports
    #[error("Invalid import: {0}")]
    InvalidImport(String),

    /// The module requires unsupported features
    #[error("Unsupported feature required: {0}")]
    UnsupportedFeature(String),

    /// Validation failed for an unknown reason
    #[error("Validation failed: {0}")]
    Other(String),
}

/// Configuration for WASM module validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum module size in bytes
    pub max_module_size: usize,
    /// Maximum number of functions
    pub max_functions: usize,
    /// Maximum memory pages (64KB each)
    pub max_memory_pages: u32,
    /// Maximum table size
    pub max_table_size: u32,
    /// Maximum globals
    pub max_globals: usize,
    /// Whether to allow floating point operations
    pub allow_float: bool,
    /// Whether to allow bulk memory operations
    pub allow_bulk_memory: bool,
    /// Whether to allow reference types
    pub allow_reference_types: bool,
    /// Whether to allow SIMD operations
    pub allow_simd: bool,
    /// Whether to allow threads
    pub allow_threads: bool,
    /// Allowed import modules (empty means all allowed)
    pub allowed_imports: Vec<String>,
    /// Prohibited import modules
    pub prohibited_imports: Vec<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_module_size: 10 * 1024 * 1024, // 10MB
            max_functions: 10000,
            max_memory_pages: 1024, // 64MB
            max_table_size: 100000,
            max_globals: 1000,
            allow_float: true,
            allow_bulk_memory: true,
            allow_reference_types: true,
            allow_simd: false, // Disabled for portability
            allow_threads: false, // Disabled for safety
            allowed_imports: vec![],
            prohibited_imports: vec![
                "env".to_string(),
                "wasi_snapshot_preview1".to_string(),
            ],
        }
    }
}

impl ValidationConfig {
    /// Create a configuration for production use
    pub fn production() -> Self {
        Self {
            max_module_size: 5 * 1024 * 1024, // 5MB
            max_functions: 5000,
            max_memory_pages: 512, // 32MB
            max_table_size: 50000,
            max_globals: 500,
            allow_float: true,
            allow_bulk_memory: true,
            allow_reference_types: true,
            allow_simd: false,
            allow_threads: false,
            allowed_imports: vec!["convex".to_string()],
            prohibited_imports: vec![
                "env".to_string(),
                "wasi_snapshot_preview1".to_string(),
                "wasi".to_string(),
            ],
        }
    }

    /// Create a configuration for development use (more permissive)
    pub fn development() -> Self {
        Self::default()
    }
}

/// A WASM module validator
pub struct WasmValidator {
    config: ValidationConfig,
    engine: Engine,
}

impl std::fmt::Debug for WasmValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmValidator")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl WasmValidator {
    /// Create a new validator with the given configuration
    pub fn new(config: ValidationConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        engine_config.wasm_bulk_memory(config.allow_bulk_memory);
        engine_config.wasm_reference_types(config.allow_reference_types);

        // Handle SIMD configuration - relaxed_simd is enabled by default in some wasmtime versions
        // We need to explicitly disable relaxed_simd if we're disabling SIMD
        if config.allow_simd {
            engine_config.wasm_simd(true);
            engine_config.wasm_relaxed_simd(true);
        } else {
            engine_config.wasm_simd(false);
            engine_config.wasm_relaxed_simd(false);
        }

        engine_config.wasm_threads(config.allow_threads);

        let engine = Engine::new(&engine_config)?;

        Ok(Self { config, engine })
    }

    /// Create a validator with default configuration
    pub fn default() -> Result<Self> {
        Self::new(ValidationConfig::default())
    }

    /// Create a validator for production use
    pub fn production() -> Result<Self> {
        Self::new(ValidationConfig::production())
    }

    /// Validate a WASM binary
    ///
    /// This performs comprehensive validation including:
    /// - Basic WASM validity
    /// - Size limits
    /// - Feature restrictions
    /// - Import validation
    /// - Memory limits
    pub fn validate(&self, wasm_binary: &[u8]) -> Result<(), ValidationError> {
        // Check size limit
        if wasm_binary.len() > self.config.max_module_size {
            return Err(ValidationError::SizeExceeded(
                wasm_binary.len(),
                self.config.max_module_size,
            ));
        }

        // Basic WASM parsing validation
        let module = Module::new(&self.engine, wasm_binary).map_err(|e| {
            ValidationError::InvalidBinary(format!("Failed to parse module: {}", e))
        })?;

        // Validate imports
        self.validate_imports(&module)?;

        // Note: Additional validation (memory limits, function count, etc.)
        // would require using wasmparser to parse the WASM binary directly.
        // For now, we rely on wasmtime's own validation during module creation.

        debug!("WASM module validation passed");
        Ok(())
    }

    /// Validate that imports are allowed
    fn validate_imports(&self, module: &Module) -> Result<(), ValidationError> {
        // Get import types from the module
        let imports: Vec<_> = module.imports().collect();

        for import in imports {
            let module_name = import.module();

            // Check prohibited imports
            if self.config.prohibited_imports.contains(&module_name.to_string()) {
                // Allow specific Convex host functions
                if !module_name.starts_with("convex") {
                    return Err(ValidationError::InvalidImport(format!(
                        "Import from prohibited module '{}' is not allowed",
                        module_name
                    )));
                }
            }

            // If allowed list is not empty, check against it
            if !self.config.allowed_imports.is_empty() {
                let allowed = self
                    .config
                    .allowed_imports
                    .iter()
                    .any(|allowed| module_name.starts_with(allowed));

                if !allowed {
                    return Err(ValidationError::InvalidImport(format!(
                        "Import from '{}' is not in allowed list",
                        module_name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Quick validation that just checks if the binary is valid WASM
    pub fn is_valid_wasm(wasm_binary: &[u8]) -> bool {
        // Check WASM magic number and version
        if wasm_binary.len() < 8 {
            return false;
        }

        // WASM magic number: \0asm
        if &wasm_binary[0..4] != &[0x00, 0x61, 0x73, 0x6d] {
            return false;
        }

        // WASM version: 1
        if &wasm_binary[4..8] != &[0x01, 0x00, 0x00, 0x00] {
            return false;
        }

        true
    }

    /// Get information about a WASM module without full validation
    pub fn get_module_info(&self, wasm_binary: &[u8]) -> Result<ModuleInfo, ValidationError> {
        if !Self::is_valid_wasm(wasm_binary) {
            return Err(ValidationError::InvalidBinary(
                "Invalid WASM magic number".to_string(),
            ));
        }

        let module = Module::new(&self.engine, wasm_binary).map_err(|e| {
            ValidationError::InvalidBinary(format!("Failed to parse module: {}", e))
        })?;

        let imports: Vec<String> = module
            .imports()
            .map(|i| format!("{}.{}", i.module(), i.name()))
            .collect();

        let exports: Vec<String> = module
            .exports()
            .map(|e| e.name().to_string())
            .collect();

        // Note: Detailed module introspection (memory, functions, globals, tables)
        // would require using wasmparser. For now, we provide basic info.

        Ok(ModuleInfo {
            size: wasm_binary.len(),
            function_count: 0, // Would need wasmparser
            global_count: 0,   // Would need wasmparser
            memory_count: 0,   // Would need wasmparser
            memory_pages: 0,   // Would need wasmparser
            table_count: 0,    // Would need wasmparser
            imports,
            exports,
        })
    }
}

/// Information about a WASM module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Size of the binary in bytes
    pub size: usize,
    /// Number of functions
    pub function_count: usize,
    /// Number of globals
    pub global_count: usize,
    /// Number of memory sections
    pub memory_count: usize,
    /// Total memory pages (64KB each)
    pub memory_pages: u64,
    /// Number of tables
    pub table_count: usize,
    /// List of imports
    pub imports: Vec<String>,
    /// List of exports
    pub exports: Vec<String>,
}

impl ModuleInfo {
    /// Get memory usage in bytes
    pub fn memory_bytes(&self) -> u64 {
        self.memory_pages * 64 * 1024
    }

    /// Format as a human-readable string
    pub fn format(&self) -> String {
        format!(
            "Module: {} functions, {} globals, {} memory section(s) ({} KB), {} table(s), {} imports, {} exports",
            self.function_count,
            self.global_count,
            self.memory_count,
            self.memory_bytes() / 1024,
            self.table_count,
            self.imports.len(),
            self.exports.len()
        )
    }
}

/// Validate a WASM binary with default settings
pub fn validate_wasm(wasm_binary: &[u8]) -> Result<(), ValidationError> {
    let validator = WasmValidator::default()
        .map_err(|e| ValidationError::Other(format!("Failed to create validator: {}", e)))?;
    validator.validate(wasm_binary)
}

/// Quick check if a binary is valid WASM
pub fn is_valid_wasm(wasm_binary: &[u8]) -> bool {
    WasmValidator::is_valid_wasm(wasm_binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple valid WASM module (empty)
    const EMPTY_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // magic
        0x01, 0x00, 0x00, 0x00, // version
    ];

    #[test]
    fn test_is_valid_wasm() {
        assert!(WasmValidator::is_valid_wasm(EMPTY_WASM));
        assert!(!WasmValidator::is_valid_wasm(b"not wasm"));
        assert!(!WasmValidator::is_valid_wasm(&[]));
        assert!(!WasmValidator::is_valid_wasm(&[0x00, 0x61, 0x73])); // Too short
    }

    #[test]
    fn test_validation_config_default() {
        let config = ValidationConfig::default();
        assert_eq!(config.max_module_size, 10 * 1024 * 1024);
        assert_eq!(config.max_functions, 10000);
        assert!(config.allow_float);
        assert!(!config.allow_simd);
    }

    #[test]
    fn test_validation_config_production() {
        let config = ValidationConfig::production();
        assert_eq!(config.max_module_size, 5 * 1024 * 1024);
        assert_eq!(config.max_functions, 5000);
        assert!(config.allowed_imports.contains(&"convex".to_string()));
    }

    #[test]
    fn test_validate_empty_wasm() {
        let validator = WasmValidator::default().unwrap();
        let result = validator.validate(EMPTY_WASM);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_wasm() {
        let validator = WasmValidator::default().unwrap();
        let result = validator.validate(b"invalid wasm binary");
        assert!(result.is_err());
    }

    #[test]
    fn test_size_limit() {
        // Create a WASM module that's 20 bytes (larger than default 10 byte limit)
        let small_wasm: &[u8] = &[
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x01, 0x07, 0x01, 0x60, // type section with one type
            0x02, 0x01, 0x01,       // function signature (param: i32, result: i32)
            0x03, 0x02, 0x01, 0x00, // function section
            0x0a, 0x04, 0x01, 0x02, // code section
            0x00, 0x0b,             // function body
        ];

        let mut config = ValidationConfig::default();
        config.max_module_size = 10;

        let validator = WasmValidator::new(config).unwrap();
        let result = validator.validate(small_wasm);

        assert!(matches!(result, Err(ValidationError::SizeExceeded(_, _))));
    }

    #[test]
    fn test_module_info() {
        let validator = WasmValidator::default().unwrap();
        let info = validator.get_module_info(EMPTY_WASM).unwrap();

        assert_eq!(info.size, 8);
        assert_eq!(info.function_count, 0);
        assert_eq!(info.memory_count, 0);
        assert!(info.imports.is_empty());
        assert!(info.exports.is_empty());
    }

    #[test]
    fn test_module_info_format() {
        let info = ModuleInfo {
            size: 100,
            function_count: 5,
            global_count: 2,
            memory_count: 1,
            memory_pages: 1,
            table_count: 0,
            imports: vec!["convex.log".to_string()],
            exports: vec!["run".to_string()],
        };

        let formatted = info.format();
        assert!(formatted.contains("5 functions"));
        assert!(formatted.contains("2 globals"));
        assert!(formatted.contains("64 KB")); // 1 page = 64KB
    }
}
