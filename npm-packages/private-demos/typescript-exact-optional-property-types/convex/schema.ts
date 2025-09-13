import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
    optionalString: v.optional(v.string()),
    objectWithOptionalString: v.object({
      optionalString: v.optional(v.string()),
    }),
  }),
});
