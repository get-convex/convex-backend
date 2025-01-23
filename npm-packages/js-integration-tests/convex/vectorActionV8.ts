import { v } from "convex/values";
import { ActionCtx, action } from "./_generated/server";
import { api } from "./_generated/api";
import { Doc } from "./_generated/dataModel";

export const vectorSearchHandler = async (
  ctx: ActionCtx,
  args: { embedding: number[]; cuisine: string },
): Promise<Doc<"foods">[]> => {
  const result = await ctx.vectorSearch("foods", "by_embedding", {
    vector: args.embedding,
    limit: 1,
    filter: (q) => q.eq("cuisine", args.cuisine),
  });
  return await ctx.runQuery(api.foods.queryDocs, {
    ids: result.map((value) => value._id),
  });
};

export const vectorSearch = action({
  args: { embedding: v.array(v.float64()), cuisine: v.string() },
  // Avoid a method reference so that this action and the node action do not
  // register exactly the same function twice.
  handler: async (ctx, args) => vectorSearchHandler(ctx, args),
});
