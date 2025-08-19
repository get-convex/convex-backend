import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// Math.random() works in schema
const _unused = Math.random();

// Date.now() works in schema
const _unused_date = Date.now();

export default defineSchema({
  messages: defineTable({
    channel: v.string(),
    text: v.string(),
  }).index("by_channel", ["channel"]),

  users: defineTable({
    name: v.string(),
  }),

  counters: defineTable({
    count: v.number(),
  }),

  maps: defineTable({
    map: v.any(),
  }),

  nodes: defineTable({
    name: v.string(),
  }),

  edges: defineTable({
    src: v.id("nodes"),
    dst: v.id("nodes"),
  }),

  sets: defineTable({
    set: v.any(),
  }),

  any: defineTable(v.any()),

  // This table serves as a serialization test to make sure backend can
  // serialize all the types in schemas.
  testTypes: defineTable({
    nullField: v.null(),
    numberField: v.number(),
    int64Field: v.int64(),
    booleanField: v.boolean(),
    stringField: v.string(),
    bytesField: v.bytes(),
    arrayField: v.array(v.boolean()),
    anyField: v.any(),
    literalBigint: v.literal(1n),
    literalNumber: v.literal(0.0),
    literalString: v.literal("hello world"),
    literalBoolean: v.literal(true),
    union: v.union(
      v.object({ a: v.array(v.number()), b: v.optional(v.string()) }),
      v.object({ c: v.any(), d: v.bytes() }),
    ),
    object: v.object({ a: v.array(v.number()), b: v.optional(v.string()) }),
  }),

  foods: defineTable({
    description: v.string(),
    cuisine: v.string(),
    embedding: v.array(v.float64()),
  })
    .vectorIndex("by_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["cuisine"],
    })
    .searchIndex("by_description", {
      searchField: "description",
      filterFields: ["cuisine"],
    }),
  // This table serves as a test to ensure virtual ids can be represented
  // within schemas as foreign keys.
  virtualForeignKeys: defineTable({
    foreignKeyField: v.id("_scheduled_functions"),
  }),

  stagedIndexes: defineTable({
    name: v.string(),
    embedding: v.array(v.float64()),
  })
    .index("by_name", { fields: ["name"], staged: true })
    .searchIndex("search_by_name", {
      searchField: "name",
      staged: true,
    })
    .vectorIndex("by_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      staged: true,
    }),
});

// Keep this in sync with the schema! It's important for cleaning up data between tests.
export const ALL_TABLE_NAMES = [
  "messages",
  "users",
  "maps",
  "nodes",
  "edges",
  "sets",
  "any",
  "testTypes",
  "foods",
  "stagedIndexes",
] as const;
