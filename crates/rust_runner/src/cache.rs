//! Persistent module caching for Rust/WASM modules
//!
//! This module provides a persistent cache for compiled WASM modules,
//! storing them on disk to avoid recompilation across process restarts.
//!
//! The cache uses a content-addressed storage scheme where the cache key
//! is derived from the WASM binary's hash, ensuring cache integrity.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};
use wasmtime::Module;

/// A persistent cache for compiled WASM modules
///
/// The cache stores compiled modules on disk, keyed by the hash of the
/// original WASM binary. This allows modules to be reused across
/// process restarts without recompilation.
#[derive(Debug)]
pub struct PersistentModuleCache {
    /// Cache directory path
    cache_dir: PathBuf,
    /// In-memory cache of recently used modules
    memory_cache: Mutex<HashMap<String, Arc<Module>>>,
    /// Maximum number of modules to keep in memory
    max_memory_entries: usize,
    /// Maximum size of cached modules on disk (in bytes)
    max_disk_size: usize,
}

impl PersistentModuleCache {
    /// Create a new persistent module cache
    ///
    /// # Arguments
    /// * `cache_dir` - Directory to store cached modules
    /// * `max_memory_entries` - Maximum number of modules to keep in memory
    /// * `max_disk_size` - Maximum size of cached modules on disk (in bytes)
    ///
    /// # Returns
    /// A new `PersistentModuleCache` instance
    pub fn new(
        cache_dir: impl AsRef<Path>,
        max_memory_entries: usize,
        max_disk_size: usize,
    ) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        // Create subdirectories for organized storage
        std::fs::create_dir_all(cache_dir.join("modules"))?;
        std::fs::create_dir_all(cache_dir.join("metadata"))?;

        info!(
            "Initialized persistent module cache at {:?}",
            cache_dir
        );

        Ok(Self {
            cache_dir,
            memory_cache: Mutex::new(HashMap::new()),
            max_memory_entries,
            max_disk_size,
        })
    }

    /// Create a default cache in the system's temp directory
    pub fn default_temp() -> Result<Self> {
        let cache_dir = std::env::temp_dir().join("convex_rust_module_cache");
        Self::new(cache_dir, 100, 1024 * 1024 * 1024) // 100 entries, 1GB disk
    }

    /// Get a cached module by its WASM binary hash
    ///
    /// This method first checks the in-memory cache, then falls back
    /// to the disk cache if the module is not in memory.
    ///
    /// # Arguments
    /// * `wasm_hash` - The SHA-256 hash of the WASM binary
    /// * `engine` - The WASM engine to use for deserialization
    ///
    /// # Returns
    /// The cached module if found, or None if not in cache
    pub fn get(&self, wasm_hash: &str, engine: &wasmtime::Engine) -> Result<Option<Arc<Module>>> {
        // Check memory cache first
        {
            let memory = self.memory_cache.lock().unwrap();
            if let Some(module) = memory.get(wasm_hash) {
                debug!("Module {} found in memory cache", wasm_hash);
                return Ok(Some(module.clone()));
            }
        }

        // Check disk cache
        let module_path = self.module_path(wasm_hash);
        if module_path.exists() {
            debug!("Module {} found in disk cache", wasm_hash);

            // Read compiled module from disk
            let module_data = std::fs::read(&module_path)
                .with_context(|| format!("Failed to read cached module: {:?}", module_path))?;

            // Deserialize the module
            let module = unsafe { Module::deserialize(engine, &module_data) }
                .with_context(|| "Failed to deserialize cached module")?;

            let module = Arc::new(module);

            // Add to memory cache
            self.insert_into_memory_cache(wasm_hash.to_string(), module.clone());

            return Ok(Some(module));
        }

        Ok(None)
    }

    /// Insert a module into the cache
    ///
    /// # Arguments
    /// * `wasm_hash` - The SHA-256 hash of the WASM binary
    /// * `module` - The compiled module to cache
    /// * `engine` - The WASM engine (needed for serialization)
    ///
    /// # Returns
    /// Ok(()) on success, or an error if caching fails
    pub fn insert(
        &self,
        wasm_hash: String,
        module: Arc<Module>,
        engine: &wasmtime::Engine,
    ) -> Result<()> {
        // Serialize the module
        let serialized = module
            .serialize()
            .with_context(|| "Failed to serialize module")?;

        // Check disk size limit
        self.enforce_disk_size_limit()?;

        // Write to disk
        let module_path = self.module_path(&wasm_hash);
        let temp_path = module_path.with_extension("tmp");

        std::fs::write(&temp_path, serialized)
            .with_context(|| format!("Failed to write module to cache: {:?}", temp_path))?;

        std::fs::rename(&temp_path, &module_path)
            .with_context(|| format!("Failed to rename cached module: {:?}", module_path))?;

        // Add to memory cache
        self.insert_into_memory_cache(wasm_hash, module);

        debug!("Module cached successfully");
        Ok(())
    }

    /// Insert a module into the memory cache with LRU eviction
    fn insert_into_memory_cache(&self, wasm_hash: String, module: Arc<Module>) {
        let mut memory = self.memory_cache.lock().unwrap();

        // Evict oldest entries if at capacity
        while memory.len() >= self.max_memory_entries {
            // Simple eviction: remove first entry (not true LRU but sufficient)
            if let Some(key) = memory.keys().next().cloned() {
                memory.remove(&key);
            }
        }

        memory.insert(wasm_hash, module);
    }

    /// Compute the SHA-256 hash of a WASM binary
    pub fn compute_hash(wasm_binary: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(wasm_binary);
        format!("{:x}", hasher.finalize())
    }

    /// Get the cache path for a module
    fn module_path(&self, wasm_hash: &str) -> PathBuf {
        // Use first 2 chars as subdirectory for better filesystem performance
        let subdir = &wasm_hash[..2.min(wasm_hash.len())];
        self.cache_dir
            .join("modules")
            .join(subdir)
            .join(format!("{}.cwasm", wasm_hash))
    }

    /// Get the metadata path for a module
    fn metadata_path(&self, wasm_hash: &str) -> PathBuf {
        self.cache_dir
            .join("metadata")
            .join(format!("{}.json", wasm_hash))
    }

    /// Enforce the disk size limit by removing oldest cached modules
    fn enforce_disk_size_limit(&self) -> Result<()> {
        let modules_dir = self.cache_dir.join("modules");

        // Get all cached files with their modification times
        let mut files: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();
        let total_size: u64 = self.collect_cached_files(&modules_dir, &mut files)?;

        if total_size > self.max_disk_size as u64 {
            // Sort by modification time (oldest first)
            files.sort_by_key(|(_, time, _)| *time);

            // Remove oldest files until under limit
            let mut current_size = total_size;
            let target_size = (self.max_disk_size as f64 * 0.8) as u64; // Target 80% of max

            for (path, _, size) in files {
                if current_size <= target_size {
                    break;
                }

                if let Err(e) = std::fs::remove_file(&path) {
                    warn!("Failed to remove cached module {:?}: {}", path, e);
                } else {
                    current_size -= size;
                    debug!("Evicted cached module: {:?}", path);
                }
            }
        }

        Ok(())
    }

    /// Collect all cached files with their metadata
    fn collect_cached_files(
        &self,
        dir: &Path,
        files: &mut Vec<(PathBuf, std::time::SystemTime, u64)>,
    ) -> Result<u64> {
        let mut total_size: u64 = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                let size = metadata.len();
                let modified = metadata.modified()?;
                files.push((path, modified, size));
                total_size += size;
            } else if metadata.is_dir() {
                total_size += self.collect_cached_files(&path, files)?;
            }
        }

        Ok(total_size)
    }

    /// Clear the entire cache (both memory and disk)
    pub fn clear(&self) -> Result<()> {
        // Clear memory cache
        {
            let mut memory = self.memory_cache.lock().unwrap();
            memory.clear();
        }

        // Clear disk cache
        let modules_dir = self.cache_dir.join("modules");
        if modules_dir.exists() {
            std::fs::remove_dir_all(&modules_dir)?;
            std::fs::create_dir_all(&modules_dir)?;
        }

        let metadata_dir = self.cache_dir.join("metadata");
        if metadata_dir.exists() {
            std::fs::remove_dir_all(&metadata_dir)?;
            std::fs::create_dir_all(&metadata_dir)?;
        }

        info!("Module cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let memory_entries = self.memory_cache.lock().unwrap().len();

        let mut disk_entries = 0;
        let mut disk_size: u64 = 0;

        let modules_dir = self.cache_dir.join("modules");
        if modules_dir.exists() {
            self.collect_stats(&modules_dir, &mut disk_entries, &mut disk_size)?;
        }

        Ok(CacheStats {
            memory_entries,
            disk_entries,
            disk_size,
            max_memory_entries: self.max_memory_entries,
            max_disk_size: self.max_disk_size,
        })
    }

    fn collect_stats(&self, dir: &Path, entries: &mut usize, size: &mut u64) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                *entries += 1;
                *size += metadata.len();
            } else if metadata.is_dir() {
                self.collect_stats(&entry.path(), entries, size)?;
            }
        }

        Ok(())
    }
}

/// Statistics about the module cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of modules in memory cache
    pub memory_entries: usize,
    /// Number of modules on disk
    pub disk_entries: usize,
    /// Total size of disk cache (in bytes)
    pub disk_size: u64,
    /// Maximum memory entries
    pub max_memory_entries: usize,
    /// Maximum disk size
    pub max_disk_size: usize,
}

impl CacheStats {
    /// Get memory cache utilization as a percentage
    pub fn memory_utilization(&self) -> f64 {
        if self.max_memory_entries == 0 {
            return 0.0;
        }
        (self.memory_entries as f64 / self.max_memory_entries as f64) * 100.0
    }

    /// Get disk cache utilization as a percentage
    pub fn disk_utilization(&self) -> f64 {
        if self.max_disk_size == 0 {
            return 0.0;
        }
        (self.disk_size as f64 / self.max_disk_size as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_hash() {
        let data1 = b"test wasm binary";
        let data2 = b"different binary";

        let hash1 = PersistentModuleCache::compute_hash(data1);
        let hash2 = PersistentModuleCache::compute_hash(data2);
        let hash1_copy = PersistentModuleCache::compute_hash(data1);

        assert_eq!(hash1.len(), 64); // SHA-256 hex string
        assert_ne!(hash1, hash2); // Different data = different hash
        assert_eq!(hash1, hash1_copy); // Same data = same hash
    }

    #[test]
    fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = PersistentModuleCache::new(temp_dir.path(), 10, 1024 * 1024);
        assert!(cache.is_ok());

        // Check that subdirectories were created
        assert!(temp_dir.path().join("modules").exists());
        assert!(temp_dir.path().join("metadata").exists());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache = PersistentModuleCache::new(temp_dir.path(), 10, 1024 * 1024).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.memory_entries, 0);
        assert_eq!(stats.disk_entries, 0);
        assert_eq!(stats.disk_size, 0);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache = PersistentModuleCache::new(temp_dir.path(), 10, 1024 * 1024).unwrap();

        // Create a file in the cache
        std::fs::write(temp_dir.path().join("modules/test.txt"), b"test").unwrap();

        // Clear cache
        cache.clear().unwrap();

        // Verify cleared
        let stats = cache.stats().unwrap();
        assert_eq!(stats.disk_entries, 0);
    }
}
