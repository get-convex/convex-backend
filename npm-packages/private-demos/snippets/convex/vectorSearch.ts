// @snippet start vectorSearchQuery
import { v } from "convex/values";
import { action } from "./_generated/server";

export const similarFoods = action({
  args: {
    descriptionQuery: v.string(),
  },
  handler: async (ctx, args) => {
    // 1. Generate an embedding from you favorite third party API:
    const embedding = await embed(args.descriptionQuery);
    // 2. Then search for similar foods!
    // highlight-start
    const results = await ctx.vectorSearch("foods", "by_embedding", {
      vector: embedding,
      limit: 16,
      filter: (q) => q.eq("cuisine", "French"),
    });
    // highlight-end
    // ...
  },
});
// @snippet end vectorSearchQuery

const embed = (...args: any[]): number[] => {
  return [];
};

import { query } from "./_generated/server";
// @snippet start fetchMovies
export const fetchMovies = query({
  args: {
    ids: v.array(v.id("movieEmbeddings")),
  },
  handler: async (ctx, args) => {
    const results = [];
    for (const id of args.ids) {
      const doc = await ctx.db
        .query("movies")
        .withIndex("by_embedding", (q) => q.eq("embeddingId", id))
        .unique();
      if (doc === null) {
        continue;
      }
      results.push(doc);
    }
    return results;
  },
});
// @snippet end fetchMovies

const filters = action({
  args: {},
  handler: async (ctx, args) => {
    await ctx.vectorSearch("foods", "by_embedding", {
      vector: [],
      // @snippet start filterSingleValue
      filter: (q) => q.eq("cuisine", "French"),
      // @snippet end filterSingleValue
    });

    await ctx.vectorSearch("foods", "by_embedding", {
      vector: [],
      // @snippet start filterMultipleValues
      filter: (q) =>
        q.or(q.eq("cuisine", "French"), q.eq("cuisine", "Indonesian")),
      // @snippet end filterMultipleValues
    });

    await ctx.vectorSearch("foods", "by_embedding", {
      vector: [],
      // @snippet start filterMultipleFields
      filter: (q) =>
        q.or(q.eq("cuisine", "French"), q.eq("mainIngredient", "butter")),
      // @snippet end filterMultipleFields
    });
  },
});
