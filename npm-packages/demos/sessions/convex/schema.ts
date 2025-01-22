import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import { vSessionId } from "convex-helpers/server/sessions";

export default defineSchema({
  messages: defineTable({
    author: v.id("users"),
    body: v.string(),
  }),
  users: defineTable({
    // Note: make sure not to leak this to clients. See this post for more info:
    // https://stack.convex.dev/track-sessions-without-cookies
    sessionId: vSessionId,
    name: v.string(),
  }).index("by_sessionId", ["sessionId"]),
});
