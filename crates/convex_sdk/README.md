# Convex Rust SDK

The official Rust SDK for writing Convex backend functions.

## Overview

This SDK allows you to write Convex queries, mutations, and actions in Rust,
compiling to WebAssembly for execution in the Convex backend.

## Quick Start

Add to your `Cargo.toml`:

```toml
[package]
name = "my-convex-functions"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
convex_sdk = { path = "path/to/convex_sdk" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# For the procedural macros
convex_sdk_macros = { path = "path/to/convex_sdk_macros" }
```

Write your functions:

```rust
use convex_sdk::*;
use serde_json::json;

#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}

#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
    email: String,
) -> Result<DocumentId> {
    db.insert("users", json!({
        "name": name,
        "email": email,
    })).await
}

#[action]
pub async fn send_welcome_email(email: String) -> Result<()> {
    let response = fetch(
        "https://api.emailservice.com/send",
        FetchOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(json!({
                "to": email,
                "subject": "Welcome!",
                "body": "Thanks for joining!"
            }).to_string().into_bytes())
    ).await?;

    if response.status == 200 {
        Ok(())
    } else {
        Err(ConvexError::Unknown("Failed to send email".to_string()))
    }
}
```

## Features

- **Type-safe database operations**: Query, insert, update, and delete documents
- **HTTP client**: Make external API calls from actions
- **File storage**: Store and retrieve files
- **Structured logging**: Integrated with Convex's logging system
- **Deterministic execution**: Queries and mutations are deterministic for caching

## Security

Functions written with this SDK run in a WebAssembly sandbox with minimal
privileges:

- ✅ Access to Convex database (according to function type)
- ✅ HTTP requests (actions only)
- ✅ File storage
- ❌ No filesystem access
- ❌ No network access (except through host functions)
- ❌ No environment variable access

## Function Types

### Queries

Read-only, deterministic functions that can be cached:

```rust
#[query]
pub async fn list_users(db: Database) -> Result<Vec<Document>> {
    db.query("users").collect().await
}
```

**Constraints:**
- Cannot write to database
- Cannot make HTTP requests
- Must be deterministic (same input → same output)

### Mutations

Read-write, transactional functions:

```rust
#[mutation]
pub async fn update_user(
    db: Database,
    id: String,
    name: String,
) -> Result<()> {
    let doc_id: DocumentId = id.parse()?;
    db.patch(doc_id, json!({ "name": name })).await
}
```

**Constraints:**
- Can read and write to database
- Cannot make HTTP requests
- Must be deterministic
- Atomic and isolated

### Actions

Side-effect capable, non-deterministic functions:

```rust
#[action]
pub async fn process_payment(
    user_id: String,
    amount: f64,
) -> Result<String> {
    // Call external payment API
    let response = fetch(/* ... */).await?;
    // Process result...
    Ok("payment_id".to_string())
}
```

**Capabilities:**
- Can make HTTP requests
- Can call other actions
- Non-deterministic (different result each call)
- Not cached

## Type System

The SDK uses `serde_json::Value` for document values:

```rust
use serde_json::json;

let value = json!({
    "name": "John Doe",
    "age": 30,
    "tags": ["premium", "verified"],
    "metadata": {
        "joined": "2024-01-01",
        "source": "web"
    }
});
```

## Error Handling

All SDK operations return `Result<T, ConvexError>`:

```rust
#[query]
pub async fn get_user_safe(db: Database, id: String) -> Result<Option<Document>> {
    match db.get(id.parse()?).await {
        Ok(doc) => Ok(doc),
        Err(ConvexError::NotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}
```

## Development

Build for WASM target:

```bash
cargo build --target wasm32-wasip1 --release
```

## License

MIT
