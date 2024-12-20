import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import { tableResolverFactory } from "local-store/server/resolvers";
import { streamQueryForServerSchema } from "local-store/server/streamQuery";
import schema from "../schema";

export const sync = defineSchema({
  messages: defineTable({
    _id: v.string(),
    _creationTime: v.number(),
    conversationId: v.id("conversations"),
    author: v.string(),
    body: v.string(),
    color: v.optional(v.string()),
  })
    .index("by_creation_time", ["_creationTime"])
    .index("by_conversation", ["conversationId", "_creationTime"]),

  users: defineTable({
    _id: v.string(),
    name: v.string(),
  }).index("by_id", ["_id"]),

  // specific to current user
  conversations: defineTable({
    _id: v.string(),
    latestMessageTime: v.number(),
    emoji: v.optional(v.string()),
    users: v.array(v.id("users")),
    hasUnreadMessages: v.boolean(),
  }).index("by_priority", ["hasUnreadMessages", "latestMessageTime"]),
});

export const s = tableResolverFactory(sync, schema);
export const streamQuery = streamQueryForServerSchema(schema);
