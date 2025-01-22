import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// @snippet start schema
export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
  }),
  // highlight-start
  likes: defineTable({
    liker: v.string(),
    messageId: v.id("messages"),
  }),
  // highlight-end
});
// @snippet end schema
