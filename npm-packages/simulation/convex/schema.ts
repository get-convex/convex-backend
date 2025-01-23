import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  objects: defineTable(v.any()),

  elleRegisters: defineTable({}),
  elleRegisterValues: defineTable({
    registerId: v.id("elleRegisters"),
    offset: v.number(),
    value: v.number(),
  }).index("registerId", ["registerId", "offset"]),

  users: defineTable({
    name: v.string(),
    email: v.optional(v.string()),
  }).index("by_name", ["name"]),

  messages: defineTable({
    conversationId: v.id("conversations"),
    author: v.id("users"),
    body: v.string(),
  }).index("by_conversation", ["conversationId"]),
  conversationMembers: defineTable({
    conversationId: v.id("conversations"),
    userId: v.id("users"),
    hasUnreadMessages: v.boolean(),
    latestMessageTime: v.number(),
  })
    .index("by_conversation", ["conversationId"])
    .index("by_user_conversation", ["userId", "conversationId"])
    .index("by_latest_message_time", [
      "userId",
      "hasUnreadMessages",
      "latestMessageTime",
    ]),
  conversations: defineTable({
    emoji: v.optional(v.string()),
  }),
});
