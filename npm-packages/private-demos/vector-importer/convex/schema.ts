import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  embeddings: defineTable({
    input: v.string(),
    embedding: v.array(v.float64()),
  }).vectorIndex("by_embedding", {
    vectorField: "embedding",
    dimensions: 1536,
  }),
});
