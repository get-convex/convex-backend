import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    channel: v.string(),
    text: v.string(),
  }).index("by_channel", ["channel"]),
});

// Keep this in sync with the schema! It's important for cleaning up data between tests.
export const ALL_TABLE_NAMES = ["messages"] as const;
