import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export const messageValidator = v.object({
  author: v.string(),
  body: v.string(),
});

export default defineSchema({
  messages: defineTable(messageValidator),
  typeTestMessages: defineTable(
    messageValidator.extend({
      optionalString: v.optional(v.string()),
      objectWithOptionalString: v.object({
        optionalString: v.optional(v.string()),
      }),
    }),
  ),
});
