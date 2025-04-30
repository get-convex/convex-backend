import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  numbers: defineTable({
    number: v.number(),
    color: v.optional(v.string()),
    id: v.string(),
  }).index("number", ["number"]),
});
