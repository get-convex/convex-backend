import { defineTable } from "convex/server";
import { defineSchema } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  table: defineTable({
    name: v.string(),
    age: v.number(),
  })
    .index("name", ["name"])
    .index("age", {
      fields: ["age"],
      staged: true,
    }),
});
