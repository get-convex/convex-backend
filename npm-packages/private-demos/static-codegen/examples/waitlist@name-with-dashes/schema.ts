import { defineTable, defineSchema } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  roomMember: defineTable({
    identifier: v.string(),
    active: v.boolean(),
  })
    .index("by_active", ["active"])
    .index("by_identifier", ["identifier"]),
  waitlistMember: defineTable({
    identifier: v.string(),
    position: v.float64(),
  })
    .index("by_identifier", ["identifier"])
    .index("by_position", ["position"]),
  messages: defineTable({
    text: v.string(),
  }),
});
