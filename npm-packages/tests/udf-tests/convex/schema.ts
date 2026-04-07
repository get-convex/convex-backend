import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  table: defineTable({}),
  accounts: defineTable({
    name: v.string(),
    balance: v.number(),
  }),
  boats: defineTable({}),
  boatVotes: defineTable({
    boat: v.id("boats"),
  }).index("by_boat", ["boat"]),
  completedScheduledJobs: defineTable({
    jobId: v.id("_scheduled_functions"),
  }),
  users: defineTable({
    identity: v.number(),
  }).index("by_identity", ["identity"]),
  test: defineTable({
    hello: v.optional(v.any()),
    counter: v.optional(v.any()),
    data: v.optional(v.any()),
  }).index("by_hello", ["hello"]),
  objects: defineTable(v.any()),
  ok: defineTable({}),
  messages: defineTable(v.any()).searchIndex("by_body", {
    searchField: "body",
    filterFields: ["filterField"],
  }),
  vectorTable: defineTable({
    vector: v.optional(v.array(v.number())),
    filterA: v.string(),
    filterB: v.boolean(),
    id: v.string(),
  }).vectorIndex("vector", {
    vectorField: "vector",
    dimensions: 4,
    filterFields: ["filterA", "filterB"],
  }),
});
