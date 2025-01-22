import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
    format: v.union(v.literal("text"), v.literal("giphy")),
    extras: v.optional(v.any()),
  })
    .index("by_author", ["author"])
    .searchIndex("search_body", {
      searchField: "body",
    }),
});
