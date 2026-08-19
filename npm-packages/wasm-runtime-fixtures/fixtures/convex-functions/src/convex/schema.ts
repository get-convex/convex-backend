import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    channel: v.string(),
    author: v.string(),
    body: v.string(),
    tags: v.array(v.string()),
    edited: v.boolean(),
  }).index("by_channel", ["channel"]),

  summaries: defineTable({
    channel: v.string(),
    summary: v.string(),
  }).index("by_channel", ["channel"]),
});
