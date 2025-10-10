import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
  }),
  conversationParticipant: defineTable({
    user: v.string(),
    active: v.boolean(),
  })
    .index("by_active", ["active"])
    .index("by_user", ["user"]),
  waitlist: defineTable({
    user: v.string(),
    position: v.float64(),
  })
    .index("by_user", ["user"])
    .index("by_position", ["position"]),
});
