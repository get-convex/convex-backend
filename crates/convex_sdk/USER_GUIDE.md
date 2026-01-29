# Convex Rust SDK User Guide

A comprehensive guide to building Convex backend functions with Rust.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Your First Function](#your-first-function)
3. [Understanding Function Types](#understanding-function-types)
4. [Database Operations](#database-operations)
5. [HTTP Requests](#http-requests)
6. [File Storage](#file-storage)
7. [Error Handling](#error-handling)
8. [Testing](#testing)
9. [Deployment](#deployment)
10. [Best Practices](#best-practices)

---

## Getting Started

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- WASM target: `rustup target add wasm32-wasip1`
- Convex CLI: `npm install -g convex`

### Project Setup

1. **Create a new Rust library:**

```bash
cargo new --lib my-convex-functions
cd my-convex-functions
```

2. **Configure `Cargo.toml`:**

```toml
[package]
name = "my-convex-functions"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
convex_sdk = { path = "../path/to/convex_sdk" }
convex_sdk_macros = { path = "../path/to/convex_sdk_macros" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = 3
lto = true
```

3. **Create your first function in `src/lib.rs`:**

```rust
use convex_sdk::*;
use serde_json::json;

#[query]
pub async fn hello(_db: Database, name: String) -> Result<String> {
    Ok(format!("Hello, {}!", name))
}
```

4. **Build for WASM:**

```bash
cargo build --target wasm32-wasip1 --release
```

---

## Your First Function

Let's build a complete user management system.

### 1. Define Your Data Model

```rust
// src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub email: String,
    pub age: i64,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
    pub age: i64,
}
```

### 2. Create Queries

```rust
// src/queries.rs
use convex_sdk::*;
use serde_json::json;

/// Get a single user by ID
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}

/// List all users (with optional limit)
#[query]
pub async fn list_users(db: Database, limit: Option<i64>) -> Result<Vec<Document>> {
    let mut query = db.query("users");

    if let Some(n) = limit {
        query = query.limit(n as usize);
    }

    query.collect().await
}

/// Search users by name prefix
#[query]
pub async fn search_users(
    db: Database,
    name_prefix: String,
    min_age: Option<i64>,
) -> Result<Vec<Document>> {
    let mut builder = db.query("users")
        .filter("name", "startsWith", name_prefix)?;

    if let Some(age) = min_age {
        builder = builder.filter("age", "gte", age)?;
    }

    builder
        .order("name", true)
        .limit(50)
        .collect()
        .await
}

/// Count total users
#[query]
pub async fn count_users(db: Database) -> Result<usize> {
    db.query("users").count().await
}
```

### 3. Create Mutations

```rust
// src/mutations.rs
use convex_sdk::*;
use serde_json::json;
use crate::models::{CreateUserInput, User};

/// Create a new user
#[mutation]
pub async fn create_user(
    db: Database,
    input: CreateUserInput,
) -> Result<DocumentId> {
    // Validate input
    if input.name.is_empty() {
        return Err(ConvexError::InvalidArgument(
            "Name cannot be empty".into()
        ));
    }

    if input.age < 0 {
        return Err(ConvexError::InvalidArgument(
            "Age cannot be negative".into()
        ));
    }

    let user = User {
        name: input.name,
        email: input.email,
        age: input.age,
        verified: false,
    };

    db.insert("users", json!(user)).await
}

/// Update a user's name
#[mutation]
pub async fn update_user_name(
    db: Database,
    id: String,
    new_name: String,
) -> Result<()> {
    if new_name.is_empty() {
        return Err(ConvexError::InvalidArgument(
            "Name cannot be empty".into()
        ));
    }

    db.patch(id.into(), json!({ "name": new_name })).await
}

/// Verify a user
#[mutation]
pub async fn verify_user(db: Database, id: String) -> Result<()> {
    db.patch(id.into(), json!({ "verified": true })).await
}

/// Delete a user
#[mutation]
pub async fn delete_user(db: Database, id: String) -> Result<()> {
    db.delete(id.into()).await
}
```

### 4. Create Actions

```rust
// src/actions.rs
use convex_sdk::*;
use serde_json::json;

/// Send a welcome email to a new user
#[action]
pub async fn send_welcome_email(email: String, name: String) -> Result<()> {
    let response = fetch(
        "https://api.emailservice.com/send",
        FetchOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer your-api-key")
            .body(json!({
                "to": email,
                "subject": "Welcome to our platform!",
                "body": format!("Hi {}, welcome aboard!", name)
            }).to_string().into_bytes())
    ).await?;

    if response.status != 200 {
        return Err(ConvexError::Unknown(
            format!("Email service returned {}", response.status)
        ));
    }

    Ok(())
}

/// Process a payment (calls external API)
#[action]
pub async fn process_payment(
    user_id: String,
    amount: f64,
    currency: String,
) -> Result<String> {
    if amount <= 0.0 {
        return Err(ConvexError::InvalidArgument(
            "Amount must be positive".into()
        ));
    }

    let response = fetch(
        "https://api.paymentprovider.com/charges",
        FetchOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(json!({
                "user_id": user_id,
                "amount": amount,
                "currency": currency
            }).to_string().into_bytes())
    ).await?;

    if response.status == 200 {
        // Parse payment ID from response
        let body = String::from_utf8(response.body)
            .map_err(|e| ConvexError::Unknown(e.to_string()))?;
        Ok(body)
    } else {
        Err(ConvexError::Unknown("Payment failed".into()))
    }
}
```

### 5. Wire Everything Together

```rust
// src/lib.rs
pub mod models;
pub mod queries;
pub mod mutations;
pub mod actions;

// Re-export for Convex
pub use queries::*;
pub use mutations::*;
pub use actions::*;
```

---

## Understanding Function Types

### Queries

**Purpose:** Read data, cached and deterministic

```rust
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}
```

**Characteristics:**
- ✅ Can read from database
- ✅ Results are cached
- ✅ Must be deterministic
- ❌ Cannot write to database
- ❌ Cannot make HTTP requests

**When to use:** Any read-only operation that should be cached.

### Mutations

**Purpose:** Write data, transactional and deterministic

```rust
#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
) -> Result<DocumentId> {
    db.insert("users", json!({ "name": name })).await
}
```

**Characteristics:**
- ✅ Can read and write database
- ✅ Atomic and isolated
- ✅ Must be deterministic
- ❌ Cannot make HTTP requests

**When to use:** Any database modification that needs to be atomic.

### Actions

**Purpose:** Side effects, non-deterministic operations

```rust
#[action]
pub async fn send_webhook(url: String, payload: String) -> Result<()> {
    fetch(&url, FetchOptions::new()
        .method("POST")
        .body(payload.into_bytes())
    ).await?;
    Ok(())
}
```

**Characteristics:**
- ✅ Can make HTTP requests
- ✅ Can call other actions
- ✅ Non-deterministic (not cached)
- ❌ Cannot directly access database (use queries/mutations)

**When to use:** External API calls, webhooks, non-deterministic operations.

---

## Database Operations

### Basic CRUD

```rust
// CREATE
let id = db.insert("users", json!({
    "name": "John",
    "email": "john@example.com"
})).await?;

// READ
let doc = db.get(id).await?;

// UPDATE
 db.patch(id, json!({ "name": "Jane" })).await?;

// DELETE
db.delete(id).await?;
```

### Query Building

```rust
// Simple query
let users = db.query("users").collect().await?;

// With filters
let adults = db.query("users")
    .filter("age", "gte", 18)?
    .filter("verified", "eq", true)?
    .collect()
    .await?;

// With ordering and limit
let recent = db.query("posts")
    .order("created_at", false)  // descending
    .limit(10)
    .collect()
    .await?;

// Count only
let count = db.query("users").count().await?;
```

### Filter Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `eq` | Equal | `.filter("status", "eq", "active")` |
| `ne` | Not equal | `.filter("status", "ne", "deleted")` |
| `lt` | Less than | `.filter("age", "lt", 18)` |
| `lte` | Less than or equal | `.filter("score", "lte", 100)` |
| `gt` | Greater than | `.filter("age", "gt", 21)` |
| `gte` | Greater than or equal | `.filter("price", "gte", 0)` |
| `in` | In array | `.filter("tag", "in", vec!["a", "b"])` |
| `startsWith` | String prefix | `.filter("name", "startsWith", "A")` |

### Working with Documents

```rust
// Extract fields from document
if let Some(doc) = db.get(id).await? {
    // Access the document ID
    let doc_id = doc.id;

    // Access the value
    if let ConvexValue::Object(fields) = doc.value {
        if let Some(ConvexValue::String(name)) = fields.get("name") {
            println!("User name: {}", name);
        }
    }
}

// Deserialize to struct
#[derive(Deserialize)]
struct User {
    name: String,
    email: String,
}

if let Some(doc) = db.get(id).await? {
    let user: User = serde_json::from_value(
        serde_json::to_value(&doc.value)?
    )?;
}
```

---

## HTTP Requests

HTTP requests are only available in **actions**.

### Basic GET Request

```rust
#[action]
pub async fn fetch_data(url: String) -> Result<String> {
    let response = fetch(&url, FetchOptions::new()).await?;

    if response.status == 200 {
        String::from_utf8(response.body)
            .map_err(|e| ConvexError::Unknown(e.to_string()))
    } else {
        Err(ConvexError::Unknown(format!(
            "HTTP error: {}", response.status
        )))
    }
}
```

### POST with JSON Body

```rust
#[action]
pub async fn create_remote_resource(data: serde_json::Value) -> Result<String> {
    let response = fetch(
        "https://api.example.com/resources",
        FetchOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(data.to_string().into_bytes())
    ).await?;

    // Parse response
    let body = String::from_utf8(response.body)
        .map_err(|e| ConvexError::Unknown(e.to_string()))?;

    Ok(body)
}
```

### Handling Headers

```rust
#[action]
pub async fn call_authenticated_api(
    token: String,
) -> Result<String> {
    let response = fetch(
        "https://api.example.com/protected",
        FetchOptions::new()
            .header("Authorization", format!("Bearer {}", token))
            .header("X-Request-ID", "unique-id-123")
    ).await?;

    Ok(String::from_utf8_lossy(&response.body).to_string())
}
```

### Error Handling

```rust
#[action]
pub async fn robust_api_call(url: String) -> Result<String> {
    let response = fetch(&url, FetchOptions::new())
        .await
        .map_err(|e| ConvexError::Unknown(
            format!("Request failed: {}", e)
        ))?;

    match response.status {
        200..=299 => Ok(String::from_utf8_lossy(&response.body).to_string()),
        400 => Err(ConvexError::InvalidArgument("Bad request".into())),
        401 => Err(ConvexError::PermissionDenied),
        404 => Err(ConvexError::NotFound),
        _ => Err(ConvexError::Unknown(format!(
            "Unexpected status: {}", response.status
        ))),
    }
}
```

---

## File Storage

### Storing Files

```rust
#[mutation]
pub async fn upload_avatar(data: Vec<u8>) -> Result<StorageId> {
    // Validate file size
    if data.len() > 5_000_000 {
        return Err(ConvexError::InvalidArgument(
            "File too large (max 5MB)".into()
        ));
    }

    // Detect content type (simplified)
    let content_type = if data.starts_with(b"\x89PNG") {
        "image/png"
    } else if data.starts_with(b"\xff\xd8") {
        "image/jpeg"
    } else {
        return Err(ConvexError::InvalidArgument(
            "Only PNG and JPEG allowed".into()
        ));
    };

    store(content_type, data).await
}
```

### Retrieving Files

```rust
#[query]
pub async fn get_file(storage_id: String) -> Result<Option<Vec<u8>>> {
    let id = StorageId::new(storage_id);

    if !id.is_valid() {
        return Err(ConvexError::InvalidArgument(
            "Invalid storage ID".into()
        ));
    }

    match get(&id).await {
        Ok(file) => Ok(Some(file.data)),
        Err(ConvexError::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}
```

### File Metadata

```rust
#[query]
pub async fn get_file_info(storage_id: String) -> Result<serde_json::Value> {
    let file = get(&StorageId::new(storage_id)).await?;

    Ok(json!({
        "content_type": file.content_type(),
        "size_bytes": file.len(),
        "is_text": file.content_type().starts_with("text/"),
    }))
}
```

---

## Error Handling

### The Result Type

All SDK operations return `Result<T>`:

```rust
pub type Result<T> = std::result::Result<T, ConvexError>;
```

### Error Types

```rust
use convex_sdk::ConvexError;

match result {
    Ok(value) => value,
    Err(ConvexError::NotFound) => handle_not_found(),
    Err(ConvexError::PermissionDenied) => handle_auth_error(),
    Err(ConvexError::InvalidArgument(msg)) => handle_bad_input(msg),
    Err(ConvexError::Database(msg)) => handle_db_error(msg),
    Err(ConvexError::Serialization(e)) => handle_json_error(e),
    Err(ConvexError::Unknown(msg)) => handle_unknown(msg),
}
```

### Propagating Errors

```rust
// Using ? operator
#[query]
pub async fn get_user_safe(db: Database, id: String) -> Result<Document> {
    let doc = db.get(id.into()).await?
        .ok_or(ConvexError::NotFound)?;
    Ok(doc)
}

// Custom error messages
#[mutation]
pub async fn create_user_safe(
    db: Database,
    name: String,
) -> Result<DocumentId> {
    if name.is_empty() {
        return Err(ConvexError::InvalidArgument(
            "Name is required".into()
        ));
    }

    db.insert("users", json!({ "name": name }))
        .await
        .map_err(|e| ConvexError::Database(
            format!("Failed to create user: {}", e)
        ))
}
```

### Validation Patterns

```rust
fn validate_email(email: &str) -> Result<()> {
    if !email.contains('@') {
        return Err(ConvexError::InvalidArgument(
            "Invalid email format".into()
        ));
    }
    Ok(())
}

fn validate_age(age: i64) -> Result<()> {
    if age < 0 || age > 150 {
        return Err(ConvexError::InvalidArgument(
            "Age must be between 0 and 150".into()
        ));
    }
    Ok(())
}

#[mutation]
pub async fn create_validated_user(
    db: Database,
    name: String,
    email: String,
    age: i64,
) -> Result<DocumentId> {
    validate_email(&email)?;
    validate_age(age)?;

    // ... create user
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("invalid").is_err());
    }

    #[test]
    fn test_age_validation() {
        assert!(validate_age(25).is_ok());
        assert!(validate_age(-1).is_err());
        assert!(validate_age(200).is_err());
    }
}
```

### Integration Testing

Since functions run in WASM, test the logic separately:

```rust
// Extract business logic for testing
fn calculate_price(base: f64, quantity: i64, discount: f64) -> f64 {
    base * quantity as f64 * (1.0 - discount)
}

#[query]
pub async fn get_price(
    _db: Database,
    base: f64,
    quantity: i64,
) -> Result<f64> {
    Ok(calculate_price(base, quantity, 0.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_calculation() {
        assert_eq!(calculate_price(10.0, 5, 0.1), 45.0);
        assert_eq!(calculate_price(100.0, 1, 0.0), 100.0);
    }
}
```

### Mocking (Future)

The SDK will support mocking in future versions:

```rust
#[cfg(test)]
mod tests {
    use convex_sdk::testing::*;

    #[tokio::test]
    async fn test_with_mock_db() {
        let mock_db = MockDatabase::new()
            .with_document("k123", json!({ "name": "Test" }));

        let result = get_user(mock_db, "k123".into()).await;
        assert!(result.is_ok());
    }
}
```

---

## Deployment

### Build for Production

```bash
# Optimized release build
cargo build --target wasm32-wasip1 --release

# The output is in:
# target/wasm32-wasip1/release/my_convex_functions.wasm
```

### Deploy to Convex

```bash
# Using Convex CLI
convex dev

# Or deploy to production
convex deploy
```

### Configuration

Create `convex.json` in your project root:

```json
{
  "functions": "./target/wasm32-wasip1/release",
  "auth": {
    "providers": ["clerk", "auth0"]
  }
}
```

---

## Best Practices

### 1. Keep Functions Small

```rust
// Good: Single responsibility
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}

// Bad: Too many responsibilities
#[query]
pub async fn messy_function(
    db: Database,
    // ... 10 parameters
) -> Result<serde_json::Value> {
    // 100 lines of mixed logic
}
```

### 2. Use Strong Types

```rust
// Good: Type-safe
#[derive(Serialize, Deserialize)]
struct UserInput {
    name: String,
    email: String,
}

#[mutation]
pub async fn create_user(
    db: Database,
    input: UserInput,
) -> Result<DocumentId> {
    db.insert("users", json!(input)).await
}

// Bad: Untyped
#[mutation]
pub async fn create_user_untyped(
    db: Database,
    data: serde_json::Value,
) -> Result<DocumentId> {
    db.insert("users", data).await
}
```

### 3. Validate Early

```rust
#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
    email: String,
) -> Result<DocumentId> {
    // Validate before any DB operations
    if name.is_empty() {
        return Err(ConvexError::InvalidArgument(
            "Name required".into()
        ));
    }

    if !email.contains('@') {
        return Err(ConvexError::InvalidArgument(
            "Invalid email".into()
        ));
    }

    // Now do DB work
    db.insert("users", json!({
        "name": name,
        "email": email
    })).await
}
```

### 4. Handle Errors Gracefully

```rust
#[query]
pub async fn get_user_or_default(
    db: Database,
    id: String,
) -> Result<serde_json::Value> {
    match db.get(id.into()).await {
        Ok(Some(doc)) => Ok(doc.value),
        Ok(None) => Ok(json!({
            "name": "Anonymous",
            "is_default": true
        })),
        Err(e) => Err(e),
    }
}
```

### 5. Use Transactions for Related Operations

```rust
#[mutation]
pub async fn transfer_funds(
    db: Database,
    from: String,
    to: String,
    amount: i64,
) -> Result<()> {
    // Both operations happen atomically
    db.patch(from.into(), json!({
        "balance": json!({ "$inc": -amount })
    })).await?;

    db.patch(to.into(), json!({
        "balance": json!({ "$inc": amount })
    })).await?;

    Ok(())
}
```

### 6. Document Your Functions

```rust
/// Get a user by their unique identifier.
///
/// # Arguments
/// * `db` - Database handle
/// * `id` - The user's document ID
///
/// # Returns
/// * `Ok(Some(Document))` - User found
/// * `Ok(None)` - User not found
/// * `Err(ConvexError)` - Database error
///
/// # Example
/// ```
/// let user = get_user(db, "k57e...".into()).await?;
/// ```
#[query]
pub async fn get_user(
    db: Database,
    id: String,
) -> Result<Option<Document>> {
    db.get(id.into()).await
}
```

### 7. Optimize Queries

```rust
// Good: Specific query
let user = db.query("users")
    .filter("email", "eq", email)?
    .limit(1)
    .collect()
    .await?;

// Bad: Fetch all and filter
let all_users = db.query("users").collect().await?;
let user = all_users.into_iter()
    .find(|u| /* check email */);
```

---

## Troubleshooting

### Common Issues

#### "Cannot find macro `query`"

Make sure the `macros` feature is enabled:

```toml
[dependencies]
convex_sdk = { path = "...", features = ["macros"] }
```

#### "Function not exported"

Ensure you're using `pub`:

```rust
#[query]
pub async fn my_function(...)  // Must be pub
```

#### "WASM build fails"

Check your target:

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1
```

#### "HTTP request in query/mutation"

HTTP requests are only allowed in actions:

```rust
// Wrong
#[query]
pub async fn bad(db: Database) -> Result<()> {
    fetch("...", ...).await  // Error!
}

// Correct
#[action]
pub async fn good() -> Result<()> {
    fetch("...", ...).await  // OK
}
```

---

## Additional Resources

- [API Reference](./API.md) - Complete API documentation
- [Migration Guide](./MIGRATION_GUIDE.md) - TypeScript to Rust
- [Convex Documentation](https://docs.convex.dev)
- [Rust Book](https://doc.rust-lang.org/book/)
- [WASM Book](https://rustwasm.github.io/book/)

## Getting Help

- Discord: [Convex Community](https://convex.dev/community)
- GitHub Issues: Report bugs and feature requests
- Stack Overflow: Tag with `convex` and `rust`