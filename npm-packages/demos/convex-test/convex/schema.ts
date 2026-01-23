import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  posts: defineTable({
    title: v.string(),
    content: v.string(),
    author: v.string(),
  }),

  tasks: defineTable({
    author: v.optional(v.string()),
    text: v.optional(v.string()),
  }).index("by_author", ["author"]),

  messages: defineTable({
    body: v.string(),
    author: v.string(),
  }),
});
