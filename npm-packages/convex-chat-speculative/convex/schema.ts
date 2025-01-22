import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    channel: v.string(),
    body: v.string(),
    author: v.string(),
  }).index("by_channel", ["channel"]),
});
