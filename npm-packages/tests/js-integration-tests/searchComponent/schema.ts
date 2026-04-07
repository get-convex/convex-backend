import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  foods: defineTable({
    description: v.string(),
    cuisine: v.string(),
    theLetterA: v.string(),
    bOrC: v.string(),
    embedding: v.array(v.float64()),
  })
    .vectorIndex("by_embedding", {
      vectorField: "embedding",
      dimensions: 1536,
      filterFields: ["cuisine"],
    })
    .searchIndex("by_description", {
      searchField: "description",
      filterFields: ["cuisine"],
    }),
});

export const ALL_TABLE_NAMES = ["foods"] as const;
