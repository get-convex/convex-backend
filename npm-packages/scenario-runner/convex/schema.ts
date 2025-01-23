import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import { EMBEDDING_SIZE } from "../types";

export default defineSchema({
  messages: defineTable({
    channel: v.string(),
    timestamp: v.number(),
    body: v.string(),
    rand: v.number(),
    ballastArray: v.array(v.number()),
  }).index("by_channel_rand", ["channel", "rand"]),
  messages_with_search: defineTable({
    channel: v.string(),
    timestamp: v.number(),
    body: v.string(),
    rand: v.number(),
    ballastArray: v.array(v.number()),
  })
    .index("by_channel_rand", ["channel", "rand"])
    .index("by_rand", ["rand"])
    .searchIndex("search_body", {
      searchField: "body",
      filterFields: ["channel"],
    }),
  openclaurd: defineTable({
    user: v.string(),
    timestamp: v.number(),
    text: v.string(),
    rand: v.number(),
    embedding: v.array(v.number()),
  })
    .index("by_rand", ["rand"])
    .vectorIndex("embedding", {
      vectorField: "embedding",
      dimensions: EMBEDDING_SIZE,
      filterFields: ["user"],
    })
    .searchIndex("search_text", {
      searchField: "text",
      filterFields: ["user"],
    }),
});
