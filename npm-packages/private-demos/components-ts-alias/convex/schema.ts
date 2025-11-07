import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import waitlistSchema from "@convex-dev/waitlist/schema";

export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
  }),

  // Only to check that we can use the ts alias when evaluating the schema.
  notes: defineTable(waitlistSchema.tables.messages.validator),
});
