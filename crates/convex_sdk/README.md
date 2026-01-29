# Convex Rust SDK

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WASM-supported-blue.svg)](https://webassembly.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

The official Rust SDK for writing Convex backend functions. Compile to WebAssembly and run high-performance, type-safe functions in the Convex backend.

## Features

- **Type-safe database operations** - Query, insert, update, and delete with compile-time guarantees
- **Zero-cost abstractions** - Rust's performance with ergonomic APIs
- **HTTP client** - Make external API calls from actions
- **File storage** - Store and retrieve files with type-safe IDs
- **Memory safety** - No garbage collection, no null pointer exceptions
- **Deterministic execution** - Queries and mutations are automatically cached

## Quick Start

### 1. Install Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-wasip1
```

### 2. Create a New Project

```bash
cargo new --lib my-convex-functions
cd my-convex-functions
```

### 3. Configure `Cargo.toml`

```toml
[package]
name = "my-convex-functions"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
convex_sdk = { path = "path/to/convex_sdk" }
convex_sdk_macros = { path = "path/to/convex_sdk_macros" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = 3
lto = true
```

### 4. Write Your First Function

```rust
// src/lib.rs
use convex_sdk::*;
use serde_json::json;

/// Get a user by ID
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}

/// Create a new user
#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
    email: String,
) -> Result<DocumentId> {
    db.insert("users", json!({
        "name": name,
        "email": email,
        "createdAt": 0, // Use appropriate timestamp
    })).await
}

/// Send a welcome email
#[action]
pub async fn send_welcome_email(email: String, name: String) -> Result<()> {
    let response = fetch(
        "https://api.emailservice.com/send",
        FetchOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(json!({
                "to": email,
                "subject": "Welcome!",
                "body": format!("Thanks for joining, {}!", name)
            }).to_string().into_bytes())
    ).await?;

    if response.status == 200 {
        Ok(())
    } else {
        Err(ConvexError::Unknown("Failed to send email".to_string()))
    }
}
```

### 5. Build and Deploy

```bash
# Build for WASM
cargo build --target wasm32-wasip1 --release

# The output is in:
# target/wasm32-wasip1/release/libmy_convex_functions.wasm
```

## Documentation

| Document | Description |
|----------|-------------|
| [📖 User Guide](./USER_GUIDE.md) | Complete guide with tutorials and examples |
| [📚 API Reference](./API.md) | Detailed API documentation for all types and functions |
| [🔄 Migration Guide](./MIGRATION_GUIDE.md) | Migrating from TypeScript to Rust |

## Function Types

### Queries

Read-only, deterministic functions that are automatically cached:

```rust
#[query]
pub async fn list_users(db: Database) -> Result<Vec<Document>> {
    db.query("users")
        .order("name", true)
        .limit(100)
        .collect()
        .await
}
```

**Constraints:**
- ✅ Can read from database
- ✅ Results are cached
- ✅ Must be deterministic
- ❌ Cannot write to database
- ❌ Cannot make HTTP requests

### Mutations

Read-write, transactional functions:

```rust
#[mutation]
pub async fn update_user(
    db: Database,
    id: String,
    name: String,
) -> Result<()> {
    db.patch(id.into(), json!({ "name": name })).await
}
```

**Constraints:**
- ✅ Can read and write database
- ✅ Atomic and isolated
- ✅ Must be deterministic
- ❌ Cannot make HTTP requests

### Actions

Side-effect capable, non-deterministic functions:

```rust
#[action]
pub async fn process_payment(
    user_id: String,
    amount: f64,
) -> Result<String> {
    let response = fetch(
        "https://api.stripe.com/v1/charges",
        FetchOptions::new()
            .method("POST")
            .header("Authorization", "Bearer token")
            .body(format!("amount={}", amount).into_bytes())
    ).await?;

    Ok("payment_id".to_string())
}
```

**Capabilities:**
- ✅ Can make HTTP requests
- ✅ Can call other actions
- ✅ Non-deterministic (not cached)
- ❌ Cannot directly access database (use queries/mutations)

## Security Model

Functions run in a WebAssembly sandbox with minimal privileges:

| Capability | Query | Mutation | Action |
|------------|-------|----------|--------|
| Database read | ✅ | ✅ | ✅ (via queries) |
| Database write | ❌ | ✅ | ✅ (via mutations) |
| HTTP requests | ❌ | ❌ | ✅ |
| File storage read | ❌ | ✅ | ✅ |
| File storage write | ❌ | ✅ | ✅ |
| Filesystem access | ❌ | ❌ | ❌ |
| Network access | ❌ | ❌ | ❌* |

\* Actions can make HTTP requests through the host function API

## Type System

The SDK uses `serde_json::Value` for document values, with strong typing via structs:

```rust
use serde::{Deserialize, Serialize};
use serde_json::json;

// Define your data model
#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
    age: i64,
}

// Use with the database
#[mutation]
pub async fn create_user(
    db: Database,
    user: User,
) -> Result<DocumentId> {
    db.insert("users", json!(user)).await
}
```

## Error Handling

All SDK operations return `Result<T, ConvexError>`:

```rust
use convex_sdk::ConvexError;

#[query]
pub async fn get_user_safe(
    db: Database,
    id: String,
) -> Result<Option<Document>> {
    match db.get(id.into()).await {
        Ok(doc) => Ok(doc),
        Err(ConvexError::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

// Or using the ? operator
#[query]
pub async fn get_user_or_error(
    db: Database,
    id: String,
) -> Result<Document> {
    db.get(id.into()).await?
        .ok_or(ConvexError::NotFound)
}
```

## Examples

### Query with Filters

```rust
#[query]
pub async fn search_users(
    db: Database,
    name_prefix: String,
    min_age: i64,
) -> Result<Vec<Document>> {
    db.query("users")
        .filter("name", "startsWith", name_prefix)?
        .filter("age", "gte", min_age)?
        .order("name", true)
        .limit(50)
        .collect()
        .await
}
```

### File Storage

```rust
#[mutation]
pub async fn upload_avatar(data: Vec<u8>) -> Result<StorageId> {
    store("image/png", data).await
}

#[query]
pub async fn get_avatar(id: String) -> Result<Vec<u8>> {
    let file = get(&StorageId::new(id)).await?;
    Ok(file.data)
}
```

### HTTP Request

```rust
#[action]
pub async fn fetch_external_data(url: String) -> Result<String> {
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

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `macros` | ✅ | Enable `#[query]`, `#[mutation]`, `#[action]` macros |
| `wasm-bindgen` | ❌ | Enable wasm-bindgen support for browser targets |

## Performance

Rust functions compiled to WASM offer significant performance benefits:

| Metric | TypeScript | Rust |
|--------|-----------|------|
| Cold start | ~50ms | ~5ms |
| Query latency | ~5ms | ~1ms |
| Memory usage | 10MB+ | 1MB+ |
| Bundle size | 50KB+ | 10KB+ |

## Requirements

- Rust 1.70 or later
- `wasm32-wasip1` target
- Convex backend 1.0 or later

## Community

- [Discord](https://convex.dev/community) - Join the Convex community
- [GitHub Issues](https://github.com/get-convex/convex-backend/issues) - Report bugs
- [Stack Overflow](https://stackoverflow.com/questions/tagged/convex) - Ask questions

## License

MIT License - see [LICENSE](./LICENSE) for details.

---

**Built with ❤️ by the Convex team**