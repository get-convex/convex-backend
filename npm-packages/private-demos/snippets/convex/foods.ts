import { v } from "convex/values";
import { internalQuery } from "./_generated/server";

// used for vector search

// @snippet start fetchResults
export const fetchResults = internalQuery({
  args: { ids: v.array(v.id("foods")) },
  handler: async (ctx, args) => {
    const results = [];
    for (const id of args.ids) {
      const doc = await ctx.db.get(id);
      if (doc === null) {
        continue;
      }
      results.push(doc);
    }
    return results;
  },
});
// @snippet end fetchResults
