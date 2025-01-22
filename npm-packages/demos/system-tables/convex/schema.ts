import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  file_uploads: defineTable({
    author: v.string(),
    upload_id: v.id("_storage"),
  }),
  messages: defineTable({
    author: v.string(),
    body: v.string(),
    format: v.string(),
  }),
});
