import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  npmVersion: defineTable({
    value: v.string(),
  }),
  cursorRules: defineTable({
    hash: v.string(),
    version: v.string(),
    content: v.string(),
  }),
});
