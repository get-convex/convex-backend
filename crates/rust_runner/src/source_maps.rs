//! Source Map Support for Rust/WASM Debugging
//!
//! This module provides functionality to generate, store, and use source maps for
//! debugging Rust code compiled to WebAssembly. It enables mapping WASM instruction
//! pointers back to original Rust source locations.
//!
//! # Architecture
//!
//! Source maps are generated during compilation and stored alongside WASM binaries.
//! At runtime, when errors occur, the source map is used to translate
//! WASM instruction pointers to human-readable file/line/column information.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use syn::visit::Visit;
use tracing::{debug, trace, warn};

/// A source location in the original Rust source code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Source file path (relative to project root)
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Function name if available
    pub function: Option<String>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            function: None,
        }
    }

    /// Set the function name
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }

    /// Format as a human-readable string
    pub fn format(&self) -> String {
        match &self.function {
            Some(func) => format!("{}:{}:{} (in {})", self.file, self.line, self.column, func),
            None => format!("{}:{}:{}", self.file, self.line, self.column),
        }
    }
}

/// A mapping from WASM instruction offset to source location.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMap {
    /// Map from WASM instruction offset to source location
    mappings: HashMap<u32, SourceLocation>,
    /// Original source files content (for displaying context)
    sources: HashMap<String, String>,
    /// Version for serialization compatibility
    version: u32,
}

impl SourceMap {
    const CURRENT_VERSION: u32 = 1;

    /// Create a new empty source map
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            sources: HashMap::new(),
            version: Self::CURRENT_VERSION,
        }
    }


    /// Add a mapping from WASM offset to source location
    pub fn add_mapping(&mut self, wasm_offset: u32, location: SourceLocation) {
        trace!(
            wasm_offset,
            file = %location.file,
            line = location.line,
            "Adding source map entry"
        );
        self.mappings.insert(wasm_offset, location);
    }


    /// Add source file content
    pub fn add_source(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.sources.insert(path.into(), content.into());
    }


    /// Look up a source location by WASM instruction offset
    pub fn lookup(&self, wasm_offset: u32) -> Option<&SourceLocation> {
        self.mappings.get(&wasm_offset)
    }


    /// Look up with fallback to nearest offset
    ///
    /// If exact offset not found, returns the closest mapping before it
    pub fn lookup_nearest(&self, wasm_offset: u32) -> Option<&SourceLocation> {
        // First try exact match
        if let Some(loc) = self.mappings.get(&wasm_offset) {
            return Some(loc);
        }

        // Find nearest mapping before this offset
        self.mappings
            .iter()
            .filter(|(&off, _)| off <= wasm_offset)
            .max_by_key(|(off, _)| *off)
            .map(|(_, loc)| loc)
    }


    /// Get source content for a file
    pub fn get_source(&self, path: &str) -> Option<&str> {
        self.sources.get(path).map(|s| s.as_str())
    }


    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .context("Failed to serialize source map")
    }


    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        let map: Self = serde_json::from_str(json)
            .context("Failed to parse source map JSON")?;

        if map.version > Self::CURRENT_VERSION {
            warn!(
                version = map.version,
                current = Self::CURRENT_VERSION,
                "Source map version newer than supported"
            );
        }

        Ok(map)
    }

    /// Serialize to binary format (more compact)
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .context("Failed to serialize source map to binary")
    }

    /// Deserialize from binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .context("Failed to deserialize source map")
    }

    /// Returns true if no mappings exist
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Returns number of mappings
    pub fn len(&self) -> usize {
        self.mappings.len()
    }
}


/// DWARF-based source map extractor for WASM modules.
////// Uses the `gimli` crate to parse DWARF debug information
/// embedded in WASM binaries compiled with debug symbols.
#[cfg(feature = "dwarf")]
pub struct DwarfSourceMapExtractor;

#[cfg(feature = "dwarf")]
impl DwarfSourceMapExtractor {
    /// Extract source map from WASM binary with DWARF debug info
    pub fn extract(wasm_bytes: &[u8]) -> Result<SourceMap> {
        use gimli::{Dwarf, EndianSlice, RunTimeEndian};
        use object::{Object, ObjectSection};

        debug!("Extracting DWARF debug info from WASM module");

        // Parse WASM as object file
        let obj = object::File::parse(wasm_bytes)
            .context("Failed to parse WASM as object file")?;

        let endian = RunTimeEndian::Little;
        let mut dwarf = Dwarf::default();

        // Load DWARF sections
        for section in obj.sections() {
            if let Some(name) = section.name().ok() {
                if name.starts_with(".debug_") {
                    let data = section.data()
                        .context("Failed to read section data")?;
                    dwarf.section(
                        &gimli::SectionId::from_str(name)?,
                        EndianSlice::new(data, endian),
                    );
                }
            }
        }

        // Load supplementary sections
        dwarf.load_supplementary(&|id| {
            obj.section_by_name(id.name())
                .and_then(|s| s.data().ok())
                .map(|d| EndianSlice::new(d, endian))
        })?;

        let mut source_map = SourceMap::new();

        // Iterate through compilation units
        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = dwarf.unit(header)?;

            if let Some(program) = dwarf.line_program(&header, &unit) {
                let rows = program.rows();

                for row_result in rows {
                    let row = row_result?;

                    if let Some(file_entry) = row.file(&program) {
                        if let Some(file_path) = dwarf.attr_string(&unit, file_entry.path_name())? {
                            let location = SourceLocation::new(
                                file_path.to_string_lossy(),
                                row.line().map(|l| l.get()).unwrap_or(0) as u32,
                                row.column().map(|c| match c {
                                    gimli::ColumnType::Column(col) => col.get() as u32,
                                    gimli::ColumnType::LeftEdge => 0,
                                }).unwrap_or(0),
                            );

                            // Get function name if available
                            // Note: This is simplified - full implementation would
                            // traverse DIE tree to find containing function

                            source_map.add_mapping(row.address() as u32, location);
                        }
                    }
                }
            }
        }

        debug!(
            mappings = source_map.len(),
            "Extracted source map from DWARF"
        );

        Ok(source_map)
    }
}


/// Source map manager for caching and retrieving source maps.
#[derive(Debug, Clone, Default)]
pub struct SourceMapManager {
    /// Cache of loaded source maps by module ID
    cache: HashMap<String, Arc<SourceMap>>,
}

impl SourceMapManager {
    /// Create a new source map manager
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Load a source map from JSON string
    pub fn load_from_json(&mut self, module_id: impl Into<String>, json: &str) -> Result<Arc<SourceMap>> {
        let module_id = module_id.into();
        let source_map = SourceMap::from_json(json)?;

        debug!(%module_id, mappings = source_map.len(), "Loaded source map from JSON");

        let arc = Arc::new(source_map);
        self.cache.insert(module_id, arc.clone());
        Ok(arc)
    }

    /// Load a source map from binary bytes
    pub fn load_from_bytes(&mut self, module_id: impl Into<String>, bytes: &[u8]) -> Result<Arc<SourceMap>> {
        let module_id = module_id.into();
        let source_map = SourceMap::from_bytes(bytes)?;

        debug!(%module_id, mappings = source_map.len(), "Loaded source map from binary");

        let arc = Arc::new(source_map);
        self.cache.insert(module_id, arc.clone());
        Ok(arc)
    }


    /// Get a cached source map
    pub fn get(&self, module_id: &str) -> Option<Arc<SourceMap>> {
        self.cache.get(module_id).cloned()
    }

    /// Remove a source map from cache
    pub fn remove(&mut self, module_id: &str) -> Option<Arc<SourceMap>> {
        self.cache.remove(module_id)
    }

    /// Clear all cached source maps
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Returns number of cached source maps
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if no source maps are cached
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}


/// Enhanced error information with source mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedError {
    /// Original error message
    pub message: String,
    /// WASM instruction offset where error occurred
    pub wasm_offset: Option<u32>,
    /// Mapped source location if available
    pub source_location: Option<SourceLocation>,
    /// Stack trace with source mappings
    pub stack_trace: Vec<StackFrame>,
}

impl MappedError {
    /// Create a new mapped error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            wasm_offset: None,
            source_location: None,
            stack_trace: Vec::new(),
        }
    }

    /// Set the WASM offset
    pub fn with_wasm_offset(mut self, offset: u32) -> Self {
        self.wasm_offset = Some(offset);
        self
    }

    /// Set the source location
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = Some(location);
        self
    }

    /// Add a stack frame
    pub fn add_frame(&mut self, frame: StackFrame) {
        self.stack_trace.push(frame);
    }

    /// Format as a human-readable error message with source context
    pub fn format(&self) -> String {
        let mut output = format!("Error: {}", self.message);

        if let Some(loc) = &self.source_location {
            output.push_str(&format!("\n  at {}", loc.format()));
        } else if let Some(offset) = self.wasm_offset {
            output.push_str(&format!("\n  at WASM offset 0x{:08x}", offset));
        }

        for frame in &self.stack_trace {
            output.push_str(&format!("\n  at "));
            if let Some(loc) = &frame.source_location {
                output.push_str(&loc.format());
            } else if let Some(offset) = frame.wasm_offset {
                output.push_str(&format!("WASM offset 0x{:08x}", offset));
            }
            if let Some(func) = &frame.function_name {
                output.push_str(&format!(" [{}]", func));
            }
        }

        output
    }
}


/// A single frame in a stack trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// WASM instruction offset
    pub wasm_offset: Option<u32>,
    /// Function name if available
    pub function_name: Option<String>,
    /// Source location if available
    pub source_location: Option<SourceLocation>,
}

impl StackFrame {
    /// Create a new stack frame
    pub fn new() -> Self {
        Self {
            wasm_offset: None,
            function_name: None,
            source_location: None,
        }
    }

    /// Set the WASM offset
    pub fn with_wasm_offset(mut self, offset: u32) -> Self {
        self.wasm_offset = Some(offset);
        self
    }

    /// Set the function name
    pub fn with_function(mut self, name: impl Into<String>) -> Self {
        self.function_name = Some(name.into());
        self
    }

    /// Set the source location
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = Some(location);
        self
    }
}

impl Default for StackFrame {
    fn default() -> Self {
        Self::new()
    }
}


/// Utility to generate source maps from Rust source code.
///
/// This is used during the build process to create source maps
/// that map WASM offsets back to Rust source locations.
pub struct SourceMapGenerator;

impl SourceMapGenerator {
    /// Generate a source map from Rust source code and compiled WASM.
    ///
    /// This is a simplified implementation. In production, this would:
    /// 1. Parse the Rust source with syn
    /// 2. Extract span information for each function
    /// 3. Map to WASM offsets using DWARF debug info
    pub fn generate(rust_source: &str, wasm_bytes: &[u8]) -> Result<SourceMap> {
        // For now, try DWARF extraction if feature is enabled
        #[cfg(feature = "dwarf")]
        {
            if let Ok(map) = DwarfSourceMapExtractor::extract(wasm_bytes) {
                if !map.is_empty() {
                    return Ok(map);
                }
            }
        }

        // Fallback: generate basic source map from AST
        Self::generate_from_ast(rust_source)
    }

    /// Generate a basic source map from Rust AST
    fn generate_from_ast(rust_source: &str) -> Result<SourceMap> {
        use syn::visit::Visit;

        let file = syn::parse_file(rust_source)
            .context("Failed to parse Rust source")?;

        let mut generator = SourceMapGeneratorVisitor::default();
        generator.visit_file(&file);

        let mut source_map = SourceMap::new();
        source_map.add_source("lib.rs", rust_source);

        // Add mappings for each function
        // Note: Without DWARF, we can't map to actual WASM offsets
        // This is just a placeholder for the structure
        for (i, func_info) in generator.functions.iter().enumerate() {
            let location = SourceLocation::new(
                "lib.rs",
                func_info.line as u32,
                func_info.column as u32,
            ).with_function(&func_info.name);

            // Use function index as placeholder offset
            source_map.add_mapping(i as u32 * 1000, location);
        }

        Ok(source_map)
    }
}

#[derive(Default)]
struct SourceMapGeneratorVisitor {
    functions: Vec<FunctionSpanInfo>,
}

struct FunctionSpanInfo {
    name: String,
    line: usize,
    column: usize,
}

impl<'ast> Visit<'ast> for SourceMapGeneratorVisitor {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        // Note: In a full implementation, we would extract actual line/column
        // from the span. For now, we just record the function name.
        // proc_macro2::Span requires the "span-locations" feature for start()
        self.functions.push(FunctionSpanInfo {
            name: item.sig.ident.to_string(),
            line: 0,
            column: 0,
        });
        syn::visit::visit_item_fn(self, item);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_format() {
        let loc = SourceLocation::new("src/lib.rs", 42, 10)
            .with_function("my_function");

        assert_eq!(
            loc.format(),
            "src/lib.rs:42:10 (in my_function)"
        );
    }

    #[test]
    fn test_source_map_serialization() {
        let mut map = SourceMap::new();
        map.add_mapping(0x100, SourceLocation::new("src/lib.rs", 10, 5));
        map.add_mapping(0x200, SourceLocation::new("src/lib.rs", 20, 3));
        map.add_source("src/lib.rs", "fn main() {}");

        // Test JSON roundtrip
        let json = map.to_json().unwrap();
        let map2 = SourceMap::from_json(&json).unwrap();

        assert_eq!(map2.len(), 2);
        assert_eq!(map2.lookup(0x100).unwrap().line, 10);
        assert_eq!(map2.lookup(0x200).unwrap().line, 20);

        // Test binary roundtrip
        let bytes = map.to_bytes().unwrap();
        let map3 = SourceMap::from_bytes(&bytes).unwrap();

        assert_eq!(map3.len(), 2);
        assert_eq!(map3.lookup(0x100).unwrap().line, 10);
    }

    #[test]
    fn test_source_map_lookup_nearest() {
        let mut map = SourceMap::new();
        map.add_mapping(0x100, SourceLocation::new("src/lib.rs", 10, 5));
        map.add_mapping(0x200, SourceLocation::new("src/lib.rs", 20, 3));
        map.add_mapping(0x300, SourceLocation::new("src/lib.rs", 30, 1));

        // Exact match
        assert_eq!(map.lookup_nearest(0x200).unwrap().line, 20);

        // Nearest before
        assert_eq!(map.lookup_nearest(0x250).unwrap().line, 20);

        // Before first mapping
        assert!(map.lookup_nearest(0x50).is_none());
    }

    #[test]
    fn test_source_map_manager() {
        let mut manager = SourceMapManager::new();

        let json = r#"{
            "mappings": {
                "256": {"file": "src/lib.rs", "line": 10, "column": 5}
            },
            "sources": {},
            "version": 1
        }"#;

        let map = manager.load_from_json("module1", json).unwrap();
        assert_eq!(map.len(), 1);

        // Should be cached
        let map2 = manager.get("module1").unwrap();
        assert_eq!(map2.len(), 1);

        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_mapped_error_format() {
        let error = MappedError::new("Something went wrong")
            .with_wasm_offset(0x100)
            .with_source_location(SourceLocation::new("src/lib.rs", 42, 10));

        let formatted = error.format();
        assert!(formatted.contains("Error: Something went wrong"));
        assert!(formatted.contains("src/lib.rs:42:10"));
    }

    #[test]
    fn test_generate_from_ast() {
        let source = r#"
            fn helper() {}

            #[query]
            pub fn my_query() -> i32 {
                42
            }
        "#;

        let map = SourceMapGenerator::generate_from_ast(source).unwrap();

        // Should have at least one mapping for the function
        assert!(map.len() >= 1);
    }
}
