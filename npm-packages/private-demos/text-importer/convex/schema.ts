import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  // Contains the data in the index so we can build and search arbitrarily sized indexes.
  documents2: defineTable({
    text: v.string(),
  }).searchIndex("by_text", {
    searchField: "text",
  }),
  // The source table from which we populate the search table. We use two separate tables so that we do not OCC
  // during the insertion process (e.g. try to query from and then insert into the same table).
  documents: defineTable({
    text: v.string(),
  }),
  // The (rough) size of documents2 as a single row.
  size: defineTable({
    size: v.int64(),
  }),
});
