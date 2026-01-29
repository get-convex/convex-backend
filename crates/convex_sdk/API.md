# Convex Rust SDK API Documentation

## Overview

The Convex Rust SDK provides a type-safe, async API for writing Convex backend functions that compile to WebAssembly (WASM). This document covers all public types, functions, and modules.

## Core Types

### ConvexValue
The fundamental value type for Convex documents, similar to JSON but with Convex-specific types.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConvexValue {
    Null,
    Int64(i64),
    Float64(f64),
    Boolean(bool),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<ConvexValue>),
    Object(HashMap<String, ConvexValue>),
}
```

#### Variants
| Variant | Description | Example |
|---------|-----------|--------|
| `Null` | Null value | `ConvexValue::Null` |
| `Int64` | 64-bit integer | `ConvexValue::Int64(42)` |
| `Float64` | 64-bit floating point | `ConvexValue::Float64(3.14)` |
| `Boolean` | Boolean | `ConvexValue::Boolean(true)` |
| `String` | UTF-8 string | `ConvexValue::String("hello".into())` |
| `Bytes` | Raw bytes | `ConvexValue::Bytes(vec![0u8, 1])` |
| `Array` | Array of values | `ConvexValue::Array(vec![...])` |
| `Object` | Key-value map | `ConvexValue::Object(map)` |

#### JSON Conversion
```rust
use serde_json::json;

// ConvexValue to JSON
let value = json!({
    "name": "John",
    age: 30
});

// JSON to ConvexValue
let parsed: ConvexValue = serde_json::from_value(json).unwrap();
```

---
docId
A type-safe document identifier in Convex.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(String);
```
docId::new(id: impl Into<String>) -> Self
```
Creates a new document ID from a string.
#### Example
```rust
use convex_sdk::DocumentId;

let id = DocumentId::new("k57e...");
let id: DocumentId = "k57e...".into();
```
as_str(&self) -> &str
Returns the ID as a string slice.
---
documents
A document with ID and value.
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
pub id: DocumentId,
    pub value: ConvexValue,
}
```
---
tError
Error type for all Convex operations.
```rust
#[derive(Debug, thiserror::Error)]
c enum ConvexError {
    #[error("database error: {0}")]
    Database(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("not found")]
    NotFound,
    non_error("permission denied")]
    PermissionDenied,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[n    #[error("unknown error: {0}")]
    Unknown(String),
}
```
type Result<T> = std::result::Result<T, ConvexError>;
```
nvenient type alias for results.
---
tabases
| Error | When It Occurs |
|-------|---------------|
| `Database` | Database operation fails |
| `Serialization` | JSON (de)serialization fails |
| `NotFound` | Document not found |
| `PermissionDenied` | Auth check fails |
| `InvalidArgument` | Bad input |
| `Unknown` | Unexpected error |

## Database Module (`convex_sdk::db`)
The database module provides CRUD operations and querying.
### Database
The main handle for database operations.
```rust
#[derive(Debug)]
pub struct Database {
    _handle: u32, // Opaque WASM handle
}
```
#### Methods
new(handle: u32) -> Self
Creates a database handle (internal use).
---
ery(&self, table: &str) -> QueryBuilder
Starts building a query on a table.
```
t async fn list_users(db: Database) -> Result<Vec<Document>>> {
    db.query("users").collect().await
}
```
---
t(&self, id: DocumentId) -> Result<Option<Document>>
trieves a document by ID.
```rust
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}
```
---
sert
insert(&self, table: &str, value: impl Serialize) -> Result<DocumentId>
serts a new document.
```rust
#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
) -> Result<DocumentId> {
    db.insert("users", json!({ "name": name })).await
}
```
tch
patch(&self, id: DocumentId, value: impl Serialize) -> Result<()>
dates an existing document.
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
elete
delete(&self, id: DocumentId) -> Result<()>
ets a document.
```rust
#[mutation]
pub async fn delete_user(db: Database, id: String) -> Result<()> {
    db.delete(id.into()).await
}
```
eryBuilder
Fluent API for building queries.
```rust
#[derive(Debug)]
pub struct QueryBuilder {
    table: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
}
```
er Methods
---
er Methods (chainable)
| Method | Description | Returns |
|--------|-------------|---------|
| `filter(field, op, value)` | Add WHERE clause | `Result<Self>` |
| `order(field, ascending)` | Add ORDER BY | `Self` |
| `limit(n)` | Limit results | `Self` |
| `collect()` | Execute query | `Result<Vec<Document>>` |
| `count()` | Count matches | `Result<usize>` |

#### Filter Operations
Supported filter operators:
- `"eq"` - Equal
- `"ne"` - Not equal
- `"lt"` - Less than
- `"lte"` - Less than or equal
- `"gt"` - Greater than
- `"gte"` - Greater than or equal
- `"in"` - In array
- `"startsWith"` - String prefix

#### Example
```rust
#[query]
pub async fn search_users(
    db: Database,
    min_age: i64,
    name_prefix: String,
) -> Result<Vec<Document>> {
    db.query("users")
        .filter("age", "gte", min_age)?
        .filter("name", "startsWith", name_prefix)?
        .order("created_at", false)
        .limit(10)
        .collect()
        .await
}
```

### FilterCondition
Internal filter representation.
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilterCondition {
    field: String,
    op: String,
    value: serde_json::Value,
}
```

### OrderSpec
Internal ordering specification.
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderSpec {
    field: String,
    ascending: bool,
}
```

## HTTP Module (`convex_sdk::http`)
HTTP client for actions only.

### fetch
```rust
pub async fn fetch(url: &str, options: FetchOptions) -> Result<HttpResponse>
```
Makes an HTTP request. **Only available in actions.**

#### Example
```rust
#[action]
pub async fn call_api(url: String) -> Result<String> {
    let response = fetch(
        &url,
        FetchOptions::new()
            .method("POST")
            .header("Authorization", "Bearer token")
            .header("Content-Type", "application/json")
            .body(json!({ "key": "value" }).to_string().into_bytes())
    ).await?;

    if response.status == 200 {
        String::from_utf8(response.body)
            .map_err(|e| ConvexError::Unknown(e.to_string()))
    } else {
        Err(ConvexError::Unknown(format!("HTTP {}", response.status)))
    }
}
```

### HttpResponse
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | `u16` | HTTP status code |
| `headers` | `Vec<(String, String)>` | Response headers |
| `body` | `Vec<u8>` | Raw response body |

### FetchOptions
Builder for HTTP request options.
```rust
#[derive(Debug, Default, Clone)]
pub struct FetchOptions {
    pub method: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
}
```

#### Builder Methods
| Method | Description | Example |
|--------|-------------|---------|
| `new()` | Create empty options | `FetchOptions::new()` |
| `method(m)` | Set HTTP method | `.method("POST")` |
| `header(k, v)` | Add header | `.header("X-API-Key", "secret")` |
| `body(b)` | Set body bytes | `.body(vec![...])` |

#### Complete Example
```rust
let options = FetchOptions::new()
    .method("PUT")
    .header("Content-Type", "application/json")
    .header("X-Request-ID", "12345")
    .body(r#"{"status":"active"}"#.as_bytes().to_vec());
```

## Storage Module (`convex_sdk::storage`)
File storage operations.

### StorageId
Validated identifier for stored files.
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageId(String);
```

#### Methods

##### `StorageId::new(id: impl Into<String>) -> Self`
Creates a storage ID.

##### `as_str(&self) -> &str`
Returns the ID as a string.

##### `is_valid(&self) -> bool`
Validates the ID format (alphanumeric, hyphens, underscores).

```rust
let id = StorageId::new("file_123-abc");
assert!(id.is_valid());
```

### StorageFile
Represents a stored file.
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFile {
    pub content_type: String,
    pub data: Vec<u8>,
}
```

#### Methods

##### `new(content_type: impl Into<String>, data: Vec<u8>) -> Self`
Creates a storage file.

##### `len(&self) -> usize`
Returns file size in bytes.

##### `is_empty(&self) -> bool`
Checks if file is empty.

##### `content_type(&self) -> &str`
Returns MIME type.

##### `data(&self) -> &[u8]`
Returns file data.

##### `data_as_string(&self) -> Option<String>`
Attempts to convert data to UTF-8 string.

### store
```rust
pub async fn store(content_type: &str, data: Vec<u8>) -> Result<StorageId>
```
Stores a file in Convex storage.

```rust
#[mutation]
pub async fn upload_image(data: Vec<u8>) -> Result<StorageId> {
    store("image/png", data).await
}
```

### get
```rust
pub async fn get(storage_id: &StorageId) -> Result<StorageFile>
```
Retrieves a file from storage.

```rust
#[query]
pub async fn download_file(id: String) -> Result<Vec<u8>> {
    let file = get(&StorageId::new(id)).await?;
    Ok(file.data)
}
```

## Macros (`convex_sdk_macros`)
Procedural macros for function types.

### `#[query]`
Marks a function as a Convex query.

```rust
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}
```

**Constraints:**
- Must accept `Database` as first parameter
- Must return `Result<T>`
- Cannot make HTTP requests
- Must be deterministic

### `#[mutation]`
Marks a function as a Convex mutation.

```rust
#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
) -> Result<DocumentId> {
    db.insert("users", json!({ "name": name })).await
}
```

**Constraints:**
- Must accept `Database` as first parameter
- Must return `Result<T>`
- Cannot make HTTP requests
- Must be deterministic

### `#[action]`
Marks a function as a Convex action.

```rust
#[action]
pub async fn send_email(email: String) -> Result<()> {
    fetch("https://api.example.com/send", FetchOptions::new()
        .method("POST")
        .body(json!({ "email": email }).to_string().into_bytes())
    ).await?;
    Ok(())
}
```

**Constraints:**
- Can make HTTP requests
- Can call other actions
- Cannot directly access database (use queries/mutations)
- Not cached

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `macros` | ✅ | Enable procedural macros |
| `wasm-bindgen` | ❌ | Browser WASM support |

### Cargo.toml
```toml
[dependencies]
convex_sdk = { path = "../convex_sdk", features = ["macros"] }
```

## Constants

### VERSION
```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```
SDK version string.

## Re-exports

The SDK re-exports commonly used crates:

```rust
pub use serde_json;  // JSON handling
pub use serde;       // Serialization derive macros
```

## Type Conversions

### From Rust Types
```rust
// String -> DocumentId
let id: DocumentId = "k57e...".into();

// &str -> DocumentId
let id: DocumentId = "k57e...".into();

// String -> StorageId
let storage: StorageId = "file_123".into();

// &str -> StorageId
let storage: StorageId = "file_123".into();
```

### To JSON
```rust
use serde_json::json;

let value = json!({
    "name": "John",
    "tags": ["premium", "verified"],
    "metadata": {
        "joined": "2024-01-01"
    }
});
```

## Error Handling Patterns

### Basic Match
```rust
match db.get(id.into()).await {
    Ok(Some(doc)) => process(doc),
    Ok(None) => handle_not_found(),
    Err(ConvexError::NotFound) => handle_not_found(),
    Err(e) => return Err(e),
}
```

### Using `?`
```rust
let doc = db.get(id.into()).await?.ok_or(ConvexError::NotFound)?;
```

### Custom Errors
```rust
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("user not found")]
    UserNotFound,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error(transparent)]
    Convex(#[from] ConvexError),
}
```

## Security Model

### Capability Matrix

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

### Determinism Requirements

**Queries and mutations must be deterministic:**

✅ **Deterministic:**
- Database reads
- Pure computation
- Seeded random numbers
- Constant values

❌ **Non-deterministic:**
- HTTP requests
- Random numbers (unseeded)
- Current time
- Filesystem access
- Environment variables

## WASM Host Functions

Internal host functions (not for direct use):

```rust
extern "C" {
    fn __convex_db_query(table_ptr: i32, table_len: i32) -> i32;
    fn __convex_db_get(id_ptr: i32, id_len: i32) -> i32;
    fn __convex_db_insert(table_ptr: i32, table_len: i32, value_ptr: i32, value_len: i32) -> i32;
    fn __convex_db_patch(id_ptr: i32, id_len: i32, value_ptr: i32, value_len: i32);
    fn __convex_db_delete(id_ptr: i32, id_len: i32);
    fn __convex_db_query_advanced(query_ptr: i32, query_len: i32) -> i32;
    fn __convex_db_count(table_ptr: i32, table_len: i32) -> i32;
    fn __convex_http_fetch(url_ptr: i32, url_len: i32, options_ptr: i32, options_len: i32) -> i32;
    fn __convex_storage_store(content_type_ptr: i32, content_type_len: i32, data_ptr: i32, data_len: i32) -> i32;
    fn __convex_storage_get(storage_id_ptr: i32, storage_id_len: i32) -> i32;
    fn __convex_alloc(size: i32) -> i32;
    fn __convex_free(ptr: i32);
}
```

These are called internally by the SDK and should not be used directly.

## Version Compatibility

| SDK Version | Convex Backend | Rust Edition |
|-------------|---------------|--------------|
| 0.1.x | ≥ 1.0 | 2021 |

## See Also

- [User Guide](./USER_GUIDE.md) - Getting started and tutorials
- [Migration Guide](./MIGRATION_GUIDE.md) - TypeScript to Rust migration
- [README](./README.md) - Overview and quick start