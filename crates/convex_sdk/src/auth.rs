//! Authentication and identity module
//!
//! This module provides access to the current user's identity for authentication
//! and authorization purposes in Convex functions.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::*;
//!
//! #[query]
//! pub async fn get_current_user(db: Database) -> Result<Option<Document>> {
//!     match get_identity()? {
//!         Some(identity) => {
//!             // User is authenticated
//!             let user_id = identity.id();
//!             db.get(user_id.into()).await
//!         }
//!         None => {
//!             // User is not authenticated
//!             Ok(None)
//!         }
//!     }
//! }
//! ```

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::{ConvexError, Result};

/// Represents a user's identity/authentication state
///
/// This struct contains information about the currently authenticated user,
/// if any. It can represent various authentication methods supported by Convex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// The unique identifier for this user (e.g., subject claim from JWT)
    pub id: String,

    /// The authentication provider (e.g., "clerk", "auth0", "custom")
    pub provider: String,

    /// Additional claims from the authentication token
    ///
    /// These may include email, name, profile picture URL, etc.
    /// depending on the authentication provider configuration.
    #[serde(flatten)]
    pub claims: serde_json::Map<String, serde_json::Value>,
}

impl Identity {
    /// Create a new identity with the given ID and provider
    pub fn new(id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            provider: provider.into(),
            claims: serde_json::Map::new(),
        }
    }

    /// Get the user's email if available
    pub fn email(&self) -> Option<&str> {
        self.claims
            .get("email")
            .and_then(|v| v.as_str())
    }

    /// Get the user's name if available
    pub fn name(&self) -> Option<&str> {
        self.claims
            .get("name")
            .and_then(|v| v.as_str())
    }

    /// Get a custom claim by name
    pub fn claim(&self, name: &str) -> Option<&serde_json::Value> {
        self.claims.get(name)
    }

    /// Check if the user has a specific role (if roles are configured)
    pub fn has_role(&self, role: &str) -> bool {
        match self.claims.get("roles") {
            Some(serde_json::Value::Array(roles)) => {
                roles.iter().any(|r| r.as_str() == Some(role))
            }
            _ => false,
        }
    }

    /// Check if this is an admin user
    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
    }
}

/// Get the current user's identity
///
/// Returns `Ok(Some(Identity))` if the user is authenticated,
/// `Ok(None)` if the user is not authenticated (anonymous),
/// or an error if the identity information is invalid.
///
/// # Example
///
/// ```ignore
/// use convex_sdk::*;
///
/// #[query]
/// pub async fn my_profile(db: Database) -> Result<Option<Document>> {
///     match get_identity()? {
///         Some(identity) => {
///             db.get(identity.id.into()).await
///         }
///         None => Ok(None),
///     }
/// }
/// ```
pub fn get_identity() -> Result<Option<Identity>> {
    call_get_user_identity()
}

/// Check if the current user is authenticated
///
/// Returns `true` if a user identity is available, `false` otherwise.
///
/// # Example
///
/// ```ignore
/// use convex_sdk::*;
///
/// #[query]
/// pub async fn protected_data(db: Database) -> Result<ConvexValue> {
///     if !is_authenticated() {
///         return Err(ConvexError::unauthorized("Authentication required"));
///     }
///
///     // Fetch protected data...
///     Ok(ConvexValue::Null)
/// }
/// ```
pub fn is_authenticated() -> bool {
    get_identity().map(|i| i.is_some()).unwrap_or(false)
}

/// Require authentication, returning an error if not authenticated
///
/// # Example
///
/// ```ignore
/// use convex_sdk::*;
///
/// #[query]
/// pub async fn admin_only(db: Database) -> Result<ConvexValue> {
///     let identity = require_auth()?;
///
///     if !identity.is_admin() {
///         return Err(ConvexError::forbidden("Admin access required"));
///     }
///
///     // Perform admin operation...
///     Ok(ConvexValue::Null)
/// }
/// ```
pub fn require_auth() -> Result<Identity> {
    match get_identity()? {
        Some(identity) => Ok(identity),
        None => Err(ConvexError::unauthorized("Authentication required")),
    }
}

// FFI function to call the host function
// This is provided by the host environment
extern "C" {
    /// Call the __convex_get_user_identity host function
    ///
    /// Returns a pointer to a JSON-encoded IdentityResult in WASM memory.
    /// The caller is responsible for reading and deallocating the memory.
    fn __convex_get_user_identity() -> i32;
}

/// Result from the get_user_identity host function
#[derive(Debug, Deserialize)]
struct IdentityResult {
    success: bool,
    identity: Option<Identity>,
    error: Option<String>,
}

/// Call the host function to get user identity
fn call_get_user_identity() -> Result<Option<Identity>> {
    // Call the host function
    let result_ptr = unsafe { __convex_get_user_identity() };

    if result_ptr < 0 {
        return Err(ConvexError::internal("Failed to get user identity"));
    }

    // Read the result from memory
    // The result is a JSON string at the memory location
    let result_json = unsafe {
        // Read length prefix (4 bytes, little-endian)
        // SAFETY: We use read_unaligned because the pointer may not be aligned to u32
        let len_ptr = result_ptr as *const u8;
        let len = core::ptr::read_unaligned(len_ptr as *const u32) as usize;

        // Read the JSON data
        let data_ptr = len_ptr.add(4);
        let slice = alloc::slice::from_raw_parts(data_ptr, len);
        let data = alloc::vec::Vec::from(slice);

        // Parse as string
        String::from_utf8(data).map_err(|_| {
            ConvexError::internal("Invalid UTF-8 in identity response")
        })?
    };

    // Parse the result
    let result: IdentityResult = serde_json::from_str(&result_json)
        .map_err(|e| ConvexError::internal(format!("Failed to parse identity: {}", e)))?;

    if !result.success {
        return Err(ConvexError::internal(
            result.error.unwrap_or_else(|| "Unknown identity error".into())
        ));
    }

    Ok(result.identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_new() {
        let identity = Identity::new("user123", "clerk");
        assert_eq!(identity.id, "user123");
        assert_eq!(identity.provider, "clerk");
        assert!(identity.claims.is_empty());
    }

    #[test]
    fn test_identity_with_claims() {
        let mut identity = Identity::new("user123", "auth0");
        identity.claims.insert("email".into(), "test@example.com".into());
        identity.claims.insert("name".into(), "Test User".into());

        assert_eq!(identity.email(), Some("test@example.com"));
        assert_eq!(identity.name(), Some("Test User"));
    }

    #[test]
    fn test_identity_roles() {
        let mut identity = Identity::new("admin123", "custom");
        identity.claims.insert("roles".into(), serde_json::json!(["user", "admin"]));

        assert!(identity.has_role("admin"));
        assert!(identity.has_role("user"));
        assert!(!identity.has_role("moderator"));
        assert!(identity.is_admin());
    }
}
