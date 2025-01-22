import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  channels: defineTable({
    name: v.string(),
  }),
  messages: defineTable({
    author: v.string(),
    body: v.string(),
    channel: v.id("channels"),
  }),
});
