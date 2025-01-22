import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
  }),
  presence: defineTable({
    user: v.string(),
    room: v.string(),
    updated: v.number(),
    data: v.any(),
  })
    .index("by_room_updated", ["room", "updated"])
    .index("by_user_room", ["user", "room"]),
});
