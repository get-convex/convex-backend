import { v } from "convex/values";
import { action } from "./_generated/server";
import { internal } from "./_generated/api";
import { Doc } from "./_generated/dataModel";

// @snippet start fetchResults
export const similarFoods = action({
  args: {
    descriptionQuery: v.string(),
  },
  handler: async (ctx, args) => {
    // 1. Generate an embedding from you favorite third party API:
    const embedding = await embed(args.descriptionQuery);
    // 2. Then search for similar foods!
    const results = await ctx.vectorSearch("foods", "by_embedding", {
      vector: embedding,
      limit: 16,
      filter: (q) => q.eq("cuisine", "French"),
    });
    // highlight-start
    // 3. Fetch the results
    const foods: Array<Doc<"foods">> = await ctx.runQuery(
      internal.foods.fetchResults,
      { ids: results.map((result) => result._id) },
    );
    return foods;
    // highlight-end
  },
});
// @snippet end fetchResults

const embed = (...args: any[]): number[] => {
  return [];
};
