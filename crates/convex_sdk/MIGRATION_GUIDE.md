# Migration Guide: TypeScript to Rust

A comprehensive guide for migrating Convex functions from TypeScript to Rust.

## Table of Contents

1. [Overview](#overview)
2. [Quick Comparison](#quick-comparison)
3. [Type Mappings](#type-mappings)
4. [Function Migration](#function-migration)
5. [Query Patterns](#query-patterns)
6. [Common Pitfalls](#common-pitfalls)
7. [Performance Considerations](#performance-considerations)

---

## Overview

### Why Migrate?

| Aspect | TypeScript | Rust |
|--------|-----------|------|
| **Type Safety** | Runtime checks | Compile-time guarantees |
| **Performance** | V8 interpretation | Native WASM execution |
| **Bundle Size** | Larger runtime | Minimal overhead |
| **Error Handling** | Exceptions | Explicit Results |
| **Memory Safety** | GC-managed | Ownership system |

### When to Use Rust

✅ **Good candidates:**
- Computation-heavy operations
- Critical business logic
- Large codebases needing type safety
- Performance-sensitive queries

❌ **Stay with TypeScript:**
- Simple CRUD operations
- Rapid prototyping
- Teams without Rust experience
- Heavy use of external npm packages

---

## Quick Comparison

### Function Definition

#### TypeScript
```typescript
// query.ts
import { query } from "./_generated/server";
import { v } from "convex/values";

export const getUser = query({
  args: { id: v.id("users") },
  returns: v.optional(v.any()),
  handler: async (ctx, { id }) => {
    return await ctx.db.get(id);
  },
});
```

#### Rust
```rust
// src/queries.rs
use convex_sdk::*;

#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}
```

### Mutation

#### TypeScript
```typescript
// mutation.ts
import { mutation } from "./_generated/server";
import { v } from "convex/values";

export const createUser = mutation({
  args: {
    name: v.string(),
    email: v.string(),
    age: v.number(),
  },
  returns: v.id("users"),
  handler: async (ctx, { name, email, age }) => {
    return await ctx.db.insert("users", { name, email, age });
  },
});
```

#### Rust
```rust
// src/mutations.rs
use convex_sdk::*;
use serde_json::json;

#[mutation]
pub async fn create_user(
    db: Database,
    name: String,
    email: String,
    age: i64,
) -> Result<DocumentId> {
    db.insert("users", json!({
        "name": name,
        "email": email,
        "age": age,
    })).await
}
```

### Action

#### TypeScript
```typescript
// action.ts
import { action } from "./_generated/server";
import { fetch } from "convex-helpers";

export const sendWebhook = action({
  args: { url: v.string(), payload: v.any() },
  handler: async (ctx, { url, payload }) => {
    const response = await fetch(url, {
      method: "POST",
      body: JSON.stringify(payload),
    });
    return response.status === 200;
  },
});
```

#### Rust
```rust
// src/actions.rs
use convex_sdk::*;

#[action]
pub async fn send_webhook(url: String, payload: String) -> Result<bool> {
    let response = fetch(
        &url,
        FetchOptions::new()
            .method("POST")
            .body(payload.into_bytes())
    ).await?;

    Ok(response.status == 200)
}
```

---

## Type Mappings

### Convex Value Types

| Convex Type | TypeScript | Rust |
|-------------|-----------|------|
| `v.null()` | `null` | `ConvexValue::Null` |
| `v.number()` | `number` | `i64` / `f64` |
| `v.boolean()` | `boolean` | `bool` |
| `v.string()` | `string` | `String` |
| `v.bytes()` | `ArrayBuffer` | `Vec<u8>` |
| `v.array()` | `T[]` | `Vec<T>` |
| `v.object()` | `{ [key]: T }` | `struct` / `HashMap` |
| `v.id(table)` | `Id<"table">` | `DocumentId` |

### TypeScript to Rust Examples

#### TypeScript
```typescript
// Schema
defineSchema({
  users: defineTable({
    name: v.string(),
    email: v.string(),
    age: v.optional(v.number()),
    tags: v.array(v.string()),
    metadata: v.object({
      joinedAt: v.number(),
      source: v.string(),
    }),
  }),
});
```

#### Rust
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<i64>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(rename = "joinedAt")]
    pub joined_at: i64,
    pub source: String,
}
```

### Optional Fields

#### TypeScript
```typescript
const schema = {
  name: v.string(),
  bio: v.optional(v.string()),
};

// Usage
const user = { name: "John" }; // bio is optional
```

#### Rust
```rust
#[derive(Serialize, Deserialize)]
pub struct User {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
}

// Usage
let user = User {
    name: "John".into(),
    bio: None,
};
```

### Unions and Enums

#### TypeScript
```typescript
type Status = "active" | "inactive" | "pending";

interface Task {
  status: Status;
}
```

#### Rust
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub status: Status,
}
```

---

## Function Migration

### Query Migration

#### TypeScript Pattern
```typescript
// queries/getUser.ts
import { query } from "./_generated/server";
import { v } from "convex/values";

export const getUser = query({
  args: { id: v.id("users") },
  handler: async (ctx, { id }) => {
    return await ctx.db.get(id);
  },
});

// queries/listUsers.ts
export const listUsers = query({
  args: { limit: v.optional(v.number()) },
  handler: async (ctx, { limit }) => {
    let q = ctx.db.query("users");
    if (limit) {
      q = q.take(limit);
    }
    return await q.collect();
  },
});

// queries/searchUsers.ts
export const searchUsers = query({
  args: {
    namePrefix: v.string(),
    minAge: v.optional(v.number()),
  },
  handler: async (ctx, { namePrefix, minAge }) => {
    let q = ctx.db
      .query("users")
      .withIndex("by_name", (q) => q.gte("name", namePrefix).lt("name", namePrefix + "\xFF"));

    const users = await q.collect();

    if (minAge) {
      return users.filter(u => u.age >= minAge);
    }
    return users;
  },
});
```

#### Rust Equivalent
```rust
// src/queries.rs
use convex_sdk::*;

/// Get a single user by ID
#[query]
pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    db.get(id.into()).await
}

/// List users with optional limit
#[query]
pub async fn list_users(db: Database, limit: Option<i64>) -> Result<Vec<Document>> {
    let mut query = db.query("users");

    if let Some(n) = limit {
        query = query.limit(n as usize);
    }

    query.collect().await
}

/// Search users by name prefix with optional age filter
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
```

### Mutation Migration

#### TypeScript Pattern
```typescript
// mutations/createUser.ts
import { mutation } from "./_generated/server";
import { v } from "convex/values";

export const createUser = mutation({
  args: {
    name: v.string(),
    email: v.string(),
    age: v.number(),
  },
  handler: async (ctx, args) => {
    // Validation
    if (args.name.length < 2) {
      throw new Error("Name must be at least 2 characters");
    }

    if (args.age < 0 || args.age > 150) {
      throw new Error("Invalid age");
    }

    // Check for existing email
    const existing = await ctx.db
      .query("users")
      .withIndex("by_email", q => q.eq("email", args.email))
      .unique();

    if (existing) {
      throw new Error("Email already exists");
    }

    // Insert
    return await ctx.db.insert("users", {
      name: args.name,
      email: args.email,
      age: args.age,
      createdAt: Date.now(),
    });
  },
});

// mutations/updateUser.ts
export const updateUser = mutation({
  args: {
    id: v.id("users"),
    updates: v.object({
      name: v.optional(v.string()),
      email: v.optional(v.string()),
    }),
  },
  handler: async (ctx, { id, updates }) => {
    await ctx.db.patch(id, updates);
  },
});
```

#### Rust Equivalent
```rust
// src/mutations.rs
use convex_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
    pub age: i64,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUserInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Create a new user with validation
#[mutation]
pub async fn create_user(
    db: Database,
    input: CreateUserInput,
) -> Result<DocumentId> {
    // Validation
    if input.name.len() < 2 {
        return Err(ConvexError::InvalidArgument(
            "Name must be at least 2 characters".into()
        ));
    }

    if input.age < 0 || input.age > 150 {
        return Err(ConvexError::InvalidArgument(
            "Invalid age".into()
        ));
    }

    // Check for existing email
    let existing = db.query("users")
        .filter("email", "eq", &input.email)?
        .limit(1)
        .collect()
        .await?;

    if !existing.is_empty() {
        return Err(ConvexError::InvalidArgument(
            "Email already exists".into()
        ));
    }

    // Insert
    db.insert("users", json!({
        "name": input.name,
        "email": input.email,
        "age": input.age,
        "createdAt": 0, // Use appropriate timestamp
    })).await
}

/// Update user fields
#[mutation]
pub async fn update_user(
    db: Database,
    id: String,
    updates: UpdateUserInput,
) -> Result<()> {
    // Convert to JSON, filtering out None values
    let value = serde_json::to_value(&updates)
        .map_err(ConvexError::Serialization)?;

    db.patch(id.into(), value).await
}
```

### Action Migration

#### TypeScript Pattern
```typescript
// actions/sendEmail.ts
import { action } from "./_generated/server";
import { v } from "convex/values";
import { fetch } from "convex-helpers";

export const sendEmail = action({
  args: {
    to: v.string(),
    subject: v.string(),
    body: v.string(),
  },
  handler: async (ctx, { to, subject, body }) => {
    const response = await fetch("https://api.sendgrid.com/v3/mail/send", {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${process.env.SENDGRID_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        personalizations: [{ to: [{ email: to }] }],
        from: { email: "noreply@example.com" },
        subject,
        content: [{ type: "text/plain", value: body }],
      }),
    });

    if (!response.ok) {
      throw new Error(`Email failed: ${response.statusText}`);
    }

    return { success: true };
  },
});

// actions/processPayment.ts
export const processPayment = action({
  args: {
    amount: v.number(),
    currency: v.string(),
    token: v.string(),
  },
  handler: async (ctx, { amount, currency, token }) => {
    // Call payment API
    const response = await fetch("https://api.stripe.com/v1/charges", {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${process.env.STRIPE_SECRET_KEY}`,
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        amount: (amount * 100).toString(),
        currency,
        source: token,
      }),
    });

    const result = await response.json();

    if (result.error) {
      throw new Error(result.error.message);
    }

    // Record in database via mutation
    await ctx.runMutation(api.payments.recordPayment, {
      stripeId: result.id,
      amount,
      currency,
    });

    return { paymentId: result.id };
  },
});
```

#### Rust Equivalent
```rust
// src/actions.rs
use convex_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct EmailInput {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct PaymentInput {
    pub amount: f64,
    pub currency: String,
    pub token: String,
}

/// Send an email via SendGrid
#[action]
pub async fn send_email(input: EmailInput) -> Result<serde_json::Value> {
    let response = fetch(
        "https://api.sendgrid.com/v3/mail/send",
        FetchOptions::new()
            .method("POST")
            .header("Authorization", "Bearer YOUR_API_KEY")
            .header("Content-Type", "application/json")
            .body(json!({
                "personalizations": [{ "to": [{ "email": input.to }] }],
                "from": { "email": "noreply@example.com" },
                "subject": input.subject,
                "content": [{ "type": "text/plain", "value": input.body }],
            }).to_string().into_bytes())
    ).await?;

    if response.status < 200 || response.status >= 300 {
        return Err(ConvexError::Unknown(
            format!("Email failed: {}", response.status)
        ));
    }

    Ok(json!({ "success": true }))
}

/// Process a payment via Stripe
#[action]
pub async fn process_payment(input: PaymentInput) -> Result<serde_json::Value> {
    // Build form data
    let form_data = format!(
        "amount={}&currency={}&source={}",
        (input.amount * 100.0) as i64,
        input.currency,
        input.token
    );

    let response = fetch(
        "https://api.stripe.com/v1/charges",
        FetchOptions::new()
            .method("POST")
            .header("Authorization", "Bearer YOUR_SECRET_KEY")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_data.into_bytes())
    ).await?;

    let body = String::from_utf8(response.body)
        .map_err(|e| ConvexError::Unknown(e.to_string()))?;

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(ConvexError::Serialization)?;

    if result.get("error").is_some() {
        return Err(ConvexError::Unknown(
            result["error"]["message"].as_str()
                .unwrap_or("Payment failed")
                .to_string()
        ));
    }

    // Note: Recording to database would require calling a mutation
    // This would be done via the Convex client in a real scenario

    Ok(json!({
        "paymentId": result["id"]
    }))
}
```

---

## Query Patterns

### Basic Queries

#### TypeScript
```typescript
// Get all
db.query("users").collect();

// Get one
db.get(id);

// Paginate
db.query("users").paginate({ cursor, numItems: 10 });
```

#### Rust
```rust
// Get all
db.query("users").collect().await?;

// Get one
db.get(id.into()).await?;

// Paginate (manual)
let page = db.query("users")
    .limit(10)
    .collect()
    .await?;
```

### Filtered Queries

#### TypeScript
```typescript
// Equality
db.query("users").withIndex("by_email", q => q.eq("email", email));

// Range
db.query("users").withIndex("by_age", q => q.gte("age", 18).lte("age", 65));

// Compound
db.query("users")
  .withIndex("by_status_created", q =>
    q.eq("status", "active").gt("_creationTime", yesterday)
  );
```

#### Rust
```rust
// Equality
db.query("users")
    .filter("email", "eq", email)?
    .collect().await?;

// Range
db.query("users")
    .filter("age", "gte", 18)?
    .filter("age", "lte", 65)?
    .collect().await?;

// Compound
db.query("users")
    .filter("status", "eq", "active")?
    .filter("createdAt", "gt", yesterday)?
    .collect().await?;
```

### Ordering

#### TypeScript
```typescript
db.query("users")
  .order("desc")
  .take(10);
```

#### Rust
```rust
db.query("users")
    .order("createdAt", false)  // false = descending
    .limit(10)
    .collect().await?;
```

---

## Common Pitfalls

### 1. Error Handling

#### TypeScript (throws)
```typescript
if (!user) {
  throw new Error("User not found");
}
```

#### Rust (returns)
```rust
// Wrong: Using panic!
if user.is_none() {
    panic!("User not found");  // Don't do this!
}

// Right: Return Result
if user.is_none() {
    return Err(ConvexError::NotFound);
}

// Better: Using ?
let user = db.get(id.into()).await?.ok_or(ConvexError::NotFound)?;
```

### 2. Async/Await

#### TypeScript
```typescript
const user = await db.get(id);
const posts = await db.query("posts").collect();
```

#### Rust
```rust
// Sequential (slower)
let user = db.get(id.into()).await?;
let posts = db.query("posts").collect().await?;

// Concurrent (faster when independent)
let (user, posts) = tokio::join!(
    db.get(id.into()),
    db.query("posts").collect()
);
let user = user?;
let posts = posts?;
```

### 3. JSON Handling

#### TypeScript
```typescript
const data = { name: "John", age: 30 };
await db.insert("users", data);
```

#### Rust
```rust
// Using json! macro
use serde_json::json;

db.insert("users", json!({
    "name": "John",
    "age": 30
})).await?;

// Using struct
#[derive(Serialize)]
struct User {
    name: String,
    age: i64,
}

db.insert("users", User {
    name: "John".into(),
    age: 30,
}).await?;
```

### 4. String Types

#### TypeScript
```typescript
// Just strings
const id: string = "k57e...";
```

#### Rust
```rust
// Multiple string types
let s1: String = "owned".to_string();
let s2: &str = "borrowed";
let s3: DocumentId = "k57e...".into();

// Converting
fn takes_string(s: String) {}
fn takes_str(s: &str) {}

takes_string("hello".to_string());
takes_str("hello");
takes_str(&s1);  // &String -> &str
```

### 5. Option Handling

#### TypeScript
```typescript
const maybeValue: string | undefined = getValue();
const value = maybeValue || "default";
```

#### Rust
```rust
let maybe_value: Option<String> = get_value();

// Pattern matching
match maybe_value {
    Some(v) => v,
    None => "default".to_string(),
}

// unwrap_or
let value = maybe_value.unwrap_or_else(|| "default".to_string());

// ? operator (propagates None)
let value = maybe_value?;
```

---

## Performance Considerations

### Bundle Size

| Approach | Size |
|----------|------|
| TypeScript (minimal) | ~5KB |
| Rust (minimal) | ~50KB |
| Rust (with std) | ~100KB |

**Tips:**
- Use `no_std` where possible
- Enable LTO: `lto = true` in Cargo.toml
- Strip symbols: `strip = true` in Cargo.toml

### Runtime Performance

| Operation | TypeScript | Rust |
|-----------|-----------|------|
| Simple query | 1ms | 0.1ms |
| Heavy computation | 10ms | 1ms |
| JSON parsing | 0.5ms | 0.3ms |

### Memory Usage

| Metric | TypeScript | Rust |
|--------|-----------|------|
| Baseline | 10MB | 1MB |
| Per request | +100KB | +10KB |
| Peak | 100MB | 20MB |

### Optimization Tips

1. **Use release builds:**
```bash
cargo build --target wasm32-wasip1 --release
```

2. **Enable optimizations in Cargo.toml:**
```toml
[profile.release]
opt-level = 3
lto = true
strip = true
```

3. **Minimize allocations:**
```rust
// Bad: Multiple allocations
let s = "".to_string();
for part in parts {
    s += part;  // Allocates each time
}

// Good: Pre-allocate
let mut s = String::with_capacity(total_size);
for part in parts {
    s.push_str(part);
}
```

4. **Reuse buffers:**
```rust
// Use a buffer pool for repeated operations
let mut buffer = Vec::with_capacity(1024);
for item in items {
    buffer.clear();
    serialize(item, &mut buffer)?;
    process(&buffer)?;
}
```

---

## Migration Checklist

- [ ] Set up Rust toolchain (`rustup target add wasm32-wasip1`)
- [ ] Create Cargo.toml with proper dependencies
- [ ] Define data models as Rust structs
- [ ] Migrate queries (read-only functions)
- [ ] Migrate mutations (write functions)
- [ ] Migrate actions (HTTP/external calls)
- [ ] Add comprehensive error handling
- [ ] Write unit tests for business logic
- [ ] Build and test WASM output
- [ ] Deploy and verify functionality
- [ ] Update documentation

---

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Serde Documentation](https://serde.rs/)
- [Convex TypeScript Docs](https://docs.convex.dev)
- [API Reference](./API.md)
- [User Guide](./USER_GUIDE.md)