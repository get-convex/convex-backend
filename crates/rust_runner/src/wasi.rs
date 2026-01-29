//! WASI context setup for WASM execution
//!
//! This module provides secure WASI context configuration for executing
//! untrusted WASM modules. The configuration follows the principle of
//! least privilege - only granting capabilities that are explicitly needed.
//!
//! # Security Model
//!
//! WASM modules executed by the rust_runner are untrusted user code.
//! We minimize the WASI capabilities to reduce the attack surface:
//!
//! - ✅ **Stdio inheritance**: For structured logging (stdout/stderr)
//! - ❌ **No filesystem access**: No read/write to host filesystem
//! - ❌ **No network access**: HTTP goes through host functions
//! - ❌ **No environment inheritance**: Environment vars passed explicitly
//! - ❌ **No argument inheritance**: Arguments passed through function calls
//!
//! # Usage
//!
//! ```rust
//! use rust_runner::wasi::SecureWasiContext;
//!
//! let wasi_ctx = SecureWasiContext::new()
//!     .with_stdio()  // Enable logging
//!     .with_env_var("RUST_LOG", "debug")  // Optional: specific env var
//!     .build();
//! ```

use wasi_common::sync::WasiCtxBuilder;
use wasi_common::WasiCtx;

/// A secure WASI context builder that minimizes capabilities.
///
/// By default, this builder creates a context with NO capabilities.
/// You must explicitly enable each capability you need.
pub struct SecureWasiContext {
    builder: WasiCtxBuilder,
    allow_stdio: bool,
    allowed_env_vars: Vec<(String, String)>,
}

impl SecureWasiContext {
    /// Create a new secure WASI context builder with all capabilities disabled.
    ///
    /// This is the most restrictive configuration. Capabilities must be
    /// explicitly enabled using the builder methods.
    pub fn new() -> Self {
        Self {
            builder: WasiCtxBuilder::new(),
            allow_stdio: false,
            allowed_env_vars: Vec::new(),
        }
    }

    /// Enable stdio inheritance for logging purposes.
    ///
    /// When enabled, the WASM module can write to stdout and stderr,
    /// which is captured for structured logging. This is typically
    /// safe as long as log output is properly rate-limited and sized.
    ///
    /// # Security Note
    ///
    /// This allows the guest to produce arbitrary output. Ensure that
    /// log collection has appropriate size limits to prevent DoS.
    pub fn with_stdio(mut self) -> Self {
        self.allow_stdio = true;
        self
    }

    /// Add a specific environment variable to the WASI context.
    ///
    /// Unlike `inherit_env()`, this only adds explicitly specified
    /// variables, preventing accidental leakage of host environment.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ctx = SecureWasiContext::new()
    ///     .with_env_var("RUST_LOG", "info")
    ///     .build();
    /// ```
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.allowed_env_vars.push((key.into(), value.into()));
        self
    }

    /// Build the WASI context with the configured capabilities.
    ///
    /// This method applies all the security restrictions and returns
    /// a WasiCtx ready for use with wasmtime.
    pub fn build(mut self) -> WasiCtx {
        // Apply stdio settings
        if self.allow_stdio {
            self.builder.inherit_stdio();
        }

        // Explicitly deny capabilities we don't want
        // Note: These are denied by default, but we make it explicit
        // for documentation and defense in depth

        // Don't inherit environment - only use explicitly provided vars
        for (key, value) in self.allowed_env_vars {
            // Ignore errors for individual env vars - failure to set
            // one env var shouldn't break the entire context
            let _ = self.builder.env(&key, &value);
        }

        // Build the context
        self.builder.build()
    }
}

impl Default for SecureWasiContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a secure WASI context suitable for Convex function execution.
///
/// This creates a context with minimal capabilities:
/// - Stdio for logging
/// - No filesystem access
/// - No network access
/// - No environment variable inheritance
///
/// # Example
///
/// ```rust
/// use rust_runner::wasi::create_secure_wasi_context;
///
/// let ctx = create_secure_wasi_context();
/// ```
pub fn create_secure_wasi_context() -> WasiCtx {
    SecureWasiContext::new()
        .with_stdio()
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_context_creation() {
        let ctx = SecureWasiContext::new().build();
        // Context should be created successfully with no capabilities
        // Actual capability testing would require integration tests
    }

    #[test]
    fn test_context_with_stdio() {
        let ctx = SecureWasiContext::new()
            .with_stdio()
            .build();
        // Context with stdio should be created successfully
    }

    #[test]
    fn test_context_with_env_vars() {
        let ctx = SecureWasiContext::new()
            .with_env_var("TEST_KEY", "test_value")
            .build();
        // Context with explicit env vars should be created successfully
    }
}
