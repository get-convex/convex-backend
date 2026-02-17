# RFC: FlowFields, ComputedFields & FlowFilters for Convex

## Summary

Add three new field types to Convex schemas that compute values dynamically rather than storing them:

- **FlowFields** — Cross-table aggregations (sum, count, avg, min, max) resolved via SQL at read time
- **ComputedFields** — Row-level expressions evaluated from stored + FlowField values
- **FlowFilters** — Runtime parameters that parameterize FlowField aggregations (inspired by Microsoft Dynamics Business Central)

These fields are **read-only**, **not stored**, and have **type-appropriate defaults** (0, "", false, [], {}).

---

## Proposed API

### Schema Declaration

```typescript
import { defineSchema, defineTable, v } from "convex/server";

export default defineSchema({
  customers: defineTable({
    name: v.string(),
  })
    // FlowFilters — runtime parameters, not stored
    .flowFilter("dateFilter", {
      type: v.object({ from: v.float64(), to: v.float64() }),
    })
    .flowFilter("dimensionFilter", {
      type: v.optional(v.string()),
    })

    // FlowFields — cross-table aggregations
    .flowField("orderCount", {
      returns: v.float64(),
      type: "count",
      source: "orders",
      key: "customerId",
    })
    .flowField("totalSpent", {
      returns: v.float64(),
      type: "sum",
      source: "orders",
      key: "customerId",
      field: "amount",
      filter: { status: "completed" },
    })
    .flowField("balanceLCY", {
      returns: v.float64(),
      type: "sum",
      source: "custLedgerEntries",
      key: "customerNo",
      field: "remainingAmtLCY",
      filter: {
        postingDate: { $field: "dateFilter" },        // references FlowFilter
        globalDimension1: { $field: "dimensionFilter" },
        open: true,
      },
    })

    // ComputedFields — row-level expressions
    .computed("tier", {
      returns: v.string(),
      expr: { $cond: { $gt: ["$totalSpent", 1000] }, then: "VIP", else: "STANDARD" },
    })
    .computed("displayName", {
      returns: v.string(),
      expr: { $concat: ["$name", " (", "$tier", ")"] },
    }),

  orders: defineTable({
    customerId: v.id("customers"),
    amount: v.float64(),
    status: v.union(
      v.literal("pending"),
      v.literal("completed"),
      v.literal("cancelled")
    ),
  }).index("by_customer", ["customerId"]),

  custLedgerEntries: defineTable({
    customerNo: v.string(),
    postingDate: v.float64(),
    globalDimension1: v.string(),
    remainingAmtLCY: v.float64(),
    open: v.boolean(),
  }).index("by_customer", ["customerNo", "postingDate"]),
});
```

### Generated Types (codegen output)

```typescript
// _generated/dataModel.d.ts (auto-generated)

// Insert/update type — excludes FlowFields, ComputedFields, FlowFilters
type CustomerInput = {
  name: string;
};

// Read type — includes everything
type CustomerDoc = {
  _id: Id<"customers">;
  _creationTime: number;
  name: string;
  orderCount: number;     // flowField (read-only)
  totalSpent: number;     // flowField (read-only)
  balanceLCY: number;     // flowField (read-only, parameterized)
  tier: string;           // computed (read-only)
  displayName: string;    // computed (read-only)
};

// FlowFilter input type
type CustomerFlowFilters = {
  dateFilter?: { from: number; to: number };
  dimensionFilter?: string;
};
```

### Query API

```typescript
// Standard read — static FlowFields computed, parameterized ones use defaults
const customer = await ctx.db.get(customerId);
// customer.orderCount = 5
// customer.totalSpent = 2400
// customer.balanceLCY = 0 (no flowFilters passed → default)
// customer.tier = "VIP"

// With FlowFilters — parameterized FlowFields get real values
const customer = await ctx.db
  .query("customers")
  .withIndex("by_name", (q) => q.eq("name", "Acme Corp"))
  .flowFilters({
    dateFilter: { from: 1704067200, to: 1735689600 },
    dimensionFilter: "SALES",
  })
  .unique();
// customer.balanceLCY = 54200.50 (filtered aggregation)
```

---

## Expression DSL

A minimal, serializable expression language that maps to SQL and works across all SDKs (TS, Python, Kotlin, Swift, Rust) since it's just JSON.

### Operators

| Category | Expression | SQL Equivalent |
|----------|-----------|----------------|
| **Arithmetic** | `{ $add: ["$price", "$tax"] }` | `price + tax` |
| | `{ $sub: ["$total", "$discount"] }` | `total - discount` |
| | `{ $mul: ["$qty", "$unitPrice"] }` | `qty * unitPrice` |
| | `{ $div: ["$total", "$count"] }` | `total / count` |
| **Comparison** | `{ $gt: ["$amount", 1000] }` | `amount > 1000` |
| | `{ $gte: [...] }`, `{ $lt: [...] }`, `{ $lte: [...] }`, `{ $eq: [...] }`, `{ $ne: [...] }` | Standard comparisons |
| **Conditional** | `{ $cond: <bool_expr>, then: <expr>, else: <expr> }` | `CASE WHEN ... THEN ... ELSE ... END` |
| **String** | `{ $concat: ["$first", " ", "$last"] }` | `CONCAT(...)` or `\|\|` |
| **Null handling** | `{ $ifNull: ["$nickname", "$name"] }` | `COALESCE(nickname, name)` |
| **References** | `"$fieldName"` | Column reference (stored, flow, or computed field) |
| | `{ $field: "flowFilterName" }` | Runtime parameter substitution |
| **Literals** | `1000`, `"VIP"`, `true`, `null` | SQL literals |

### Evaluation Order

1. Stored fields (already in document)
2. FlowFields (resolved via SQL aggregation)
3. ComputedFields (evaluated in dependency order, topologically sorted)

Circular dependencies detected at deploy time and rejected.

---

## Architecture: How It Works

### Core Insight

The persistence layer is SQL (SQLite/PostgreSQL/MySQL). FlowFields become SQL subqueries — no changes to the commit path, no cascading writes.

### FlowField Resolution (read-time SQL)

When a document with FlowFields is read, the persistence layer executes aggregation queries:

```sql
-- FlowField: customer.orderCount (type: count)
SELECT COUNT(*) FROM (
  SELECT key, MAX(ts) as max_ts
  FROM indexes
  WHERE index_id = :orders_by_customerId
    AND key >= :customer_id AND key < :customer_id_upper
    AND ts <= :read_ts
  GROUP BY key
) latest
JOIN indexes i ON i.key = latest.key AND i.ts = latest.max_ts
WHERE i.deleted = FALSE;

-- FlowField: customer.totalSpent (type: sum, filter: status = completed)
SELECT COALESCE(SUM(json_extract(C.json_value, '$.amount')), 0)
FROM (
  SELECT index_id, key, MAX(ts) as max_ts
  FROM indexes
  WHERE index_id = :orders_by_customerId
    AND key >= :customer_id AND key < :customer_id_upper
    AND ts <= :read_ts
  GROUP BY index_id, key
) A
JOIN indexes B ON A.index_id = B.index_id AND A.key = B.key AND A.max_ts = B.ts
LEFT JOIN documents C ON B.ts = C.ts AND B.table_id = C.table_id AND B.document_id = C.id
WHERE B.deleted = FALSE
  AND json_extract(C.json_value, '$.status') = 'completed';
```

### ComputedField Resolution (Rust evaluation)

After FlowFields are resolved, ComputedFields are evaluated in Rust using a simple expression evaluator. No V8/isolate needed — just a recursive match over the expression AST:

```rust
fn evaluate(expr: &Expr, row: &ConvexObject) -> ConvexValue {
    match expr {
        Expr::FieldRef(name) => row.get(name).clone(),
        Expr::Literal(val) => val.clone(),
        Expr::Add(a, b) => evaluate(a, row) + evaluate(b, row),
        Expr::Cond { test, then, else_ } => {
            if evaluate(test, row).is_truthy() {
                evaluate(then, row)
            } else {
                evaluate(else_, row)
            }
        }
        // ... other operators
    }
}
```

### FlowFilter Resolution

FlowFilters are passed through the query API and substituted into FlowField SQL queries as bind parameters:

```
db.query("customers").flowFilters({ dateFilter: { from: X, to: Y } })
  → FlowField SQL WHERE clause gets: AND posting_date BETWEEN :from AND :to
```

### Subscription/Reactivity

When a FlowField is resolved during a read, the aggregation query touches the source table's index. This read is recorded in the transaction's `ReadSet`:

```
Reading customer.totalSpent:
  1. Record read on customers table (the document itself)
  2. Execute aggregation on orders index
  3. Record derived read on orders table range (customerId = X)
```

When orders are written, the subscription manager checks `ReadSet` overlaps and invalidates subscribers whose FlowField source data changed.

**Key file:** `crates/database/src/reads.rs` — extend `ReadSet` with derived/flow-field reads.

---

## Code Change Map

### Phase 1: Schema Declaration (TS SDK + Rust Parsing)

#### TypeScript SDK

| File | Change |
|------|--------|
| `npm-packages/convex/src/server/schema.ts` | Add `.flowField()`, `.computed()`, `.flowFilter()` methods to `TableDefinition` class (follow `.index()` pattern at lines 338-368). Add new type parameters for tracking declared flow/computed fields. |
| `npm-packages/convex/src/values/validator.ts` | No changes needed — `returns` uses existing validators. |
| `npm-packages/convex/src/server/schema.ts` → `export()` | Extend `TableDefinition.export()` (line 570) to include `flowFields`, `computedFields`, `flowFilters` in serialized JSON. |
| `npm-packages/convex/src/server/schema.test.ts` | Add tests for new chaining methods, type inference, serialization. |

#### Rust Backend (Schema Parsing)

| File | Change |
|------|--------|
| `crates/common/src/schemas/mod.rs` | Add `flow_fields`, `computed_fields`, `flow_filters` to `TableDefinition` struct (line ~500). Add `FlowFieldSchema`, `ComputedFieldSchema`, `FlowFilterSchema` types. |
| `crates/common/src/schemas/json.rs` | Add `FlowFieldJson`, `ComputedFieldJson`, `FlowFilterJson` deserialization. Extend `TableDefinitionJson` → `TableDefinition` conversion. |
| `crates/common/src/schemas/validator.rs` | Add expression DSL validation — verify `$fieldRef` references exist, types match `returns`, no cycles. |
| `crates/common/src/schemas/tests.rs` | Add roundtrip tests, validation tests, cycle detection tests. |

#### Codegen

| File | Change |
|------|--------|
| `npm-packages/convex/src/cli/codegen_templates/dataModel.ts` | Split generated types into `Input` (writable fields only) and `Doc` (all fields including flow/computed). Generate `FlowFilters` type per table. |
| `npm-packages/convex/src/server/schema.ts` → type utilities | Add `ExtractFlowFields`, `ExtractComputedFields` type-level extractors for codegen. |

### Phase 2: Read-Time Resolution

#### Persistence Layer

| File | Change |
|------|--------|
| `crates/common/src/persistence.rs` | Add `aggregate()` method to `PersistenceReader` trait — executes SUM/COUNT/AVG/MIN/MAX queries against indexes. |
| `crates/sqlite/src/lib.rs` | Implement `aggregate()` for SQLite — generate SQL aggregation subqueries. |
| `crates/postgres/src/lib.rs` | Implement `aggregate()` for PostgreSQL. |
| `crates/mysql/src/lib.rs` | Implement `aggregate()` for MySQL. |

#### Query/Transaction Layer

| File | Change |
|------|--------|
| `crates/database/src/transaction.rs` | Add `resolve_flow_fields()` method — given a document + schema, execute FlowField aggregations and merge into document. |
| `crates/database/src/flow_fields.rs` | **New file.** Expression DSL evaluator in Rust. FlowField computation coordinator. Dependency graph resolver for ComputedFields. |

#### JS ↔ Rust Bridge

| File | Change |
|------|--------|
| `crates/isolate/src/environment/udf/async_syscall.rs` | At document return point (line ~1258), call `resolve_flow_fields()` before converting to JSON. Inject computed values into `ConvexObject`. |
| `crates/common/src/document.rs` | Add `with_computed_fields()` method to `DeveloperDocument` — merges FlowField/Computed values into the document's `ConvexObject`. |

### Phase 3: Subscription Support

| File | Change |
|------|--------|
| `crates/database/src/reads.rs` | Extend `ReadSet` to track FlowField source reads. When a FlowField is resolved, record the index range queried on the source table. |
| `crates/database/src/subscription.rs` | `overlaps_document()` / `writes_overlap_docs()` already check index ranges — FlowField reads registered as derived index reads will invalidate automatically. |

### Phase 4: FlowFilters

| File | Change |
|------|--------|
| `crates/isolate/src/environment/udf/async_syscall.rs` | Parse `.flowFilters({...})` from query syscall args. Pass filter context to `resolve_flow_fields()`. |
| `crates/database/src/flow_fields.rs` | Accept `FlowFilterContext` in aggregation resolution. Substitute `{ $field: "name" }` references with runtime values. |
| `npm-packages/convex/src/server/schema.ts` | Add `.flowFilters()` to query builder type. Ensure type safety between declared FlowFilter types and query-time values. |

---

## Default Values

FlowFields and ComputedFields always return a value (never undefined). Defaults by validator type:

| Validator | Default |
|-----------|---------|
| `v.float64()` | `0` |
| `v.int64()` | `0n` |
| `v.string()` | `""` |
| `v.boolean()` | `false` |
| `v.array(...)` | `[]` |
| `v.object(...)` | `{}` with each field set to its default |
| `v.optional(...)` | `undefined` |
| `v.union(...)` | First variant's default |

For FlowFields with FlowFilters: if no FlowFilter value is provided at query time, the FlowField returns its type default.

---

## Validation Rules (Deploy-Time)

1. **Source table exists** — FlowField `source` must reference a declared table
2. **Key field exists** — FlowField `key` must exist on the source table's schema
3. **Aggregation field exists** — FlowField `field` (for sum/avg/min/max) must exist on source table
4. **Type compatibility** — `returns` validator must match the aggregation output type (count → float64, sum of float64 → float64, etc.)
5. **No circular dependencies** — Topological sort of ComputedField dependencies; reject cycles
6. **FlowFilter references valid** — `{ $field: "name" }` in filters must reference a declared FlowFilter
7. **Expression type checking** — Verify expression operand types match operator expectations

---

## Testing Strategy

### TypeScript Unit Tests

**File:** `npm-packages/convex/src/server/schema.test.ts`

- `.flowField()` chaining returns correct type
- `.computed()` chaining returns correct type
- `.flowFilter()` chaining returns correct type
- `export()` serializes flow/computed/filter definitions correctly
- Type-level tests: `CustomerDoc` includes flow/computed fields, `CustomerInput` excludes them

**Run:** `cd npm-packages/convex && npm test -- schema.test.ts`

### Rust Unit Tests

**File:** `crates/common/src/schemas/tests.rs`

- JSON roundtrip for FlowField/Computed/FlowFilter schemas
- Validation: reject invalid source table, invalid field, type mismatch
- Cycle detection in computed field dependencies
- Expression DSL parsing and type checking

**Run:** `cargo test -p common -- schema`

### Rust Integration Tests

**File:** `crates/database/src/flow_fields.rs` (new, with `#[cfg(test)]` module)

- Expression evaluator correctness (arithmetic, conditionals, string ops)
- FlowField aggregation against in-memory test data
- Subscription invalidation with cross-table FlowField dependencies

**Run:** `cargo test -p database -- flow_field`

### End-to-End Tests

**File:** `crates/isolate/src/tests/` or `crates/application/src/tests/`

- Push schema with FlowFields → read document → verify computed values
- Mutate source table → verify subscription invalidates → re-read → verify updated values
- FlowFilter parameterization → same document, different filter values, different results

---

## Open Questions

1. **Batch optimization** — When loading N documents with FlowFields, should we batch the aggregation queries (single `GROUP BY` query) or execute N individual queries?
2. **Caching** — Should FlowField results be cached within a transaction? (Likely yes, keyed by `(docId, fieldName, flowFilterHash)`)
3. **Indexing computed fields** — Should we allow `.index("by_tier", ["tier"])` on a ComputedField? This would require materialization.
4. **Migration** — When a FlowField definition changes (e.g., adding a filter), do existing reads just get the new computation, or do we need a backfill-like process?
5. **Limits** — Max FlowFields per table? Max expression depth? Max source table fan-out?

---

## Prior Art

- **Microsoft Dynamics Business Central** — FlowFields, FlowFilters, CalcFormula (primary inspiration)
- **PostgreSQL Materialized Views** — Similar concept at SQL level
- **Drizzle ORM `.computed()`** — Row-level computed columns
- **Prisma `@computed`** — Proposed but not implemented
- **MongoDB Aggregation Pipeline** — Expression DSL inspiration (`$cond`, `$sum`, etc.)
