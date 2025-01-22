"use node";

import { action } from "./_generated/server";
import { v } from "convex/values";
import { SearchResult, embed } from "./foods";
import { internal } from "./_generated/api";

export const similarFoods = action({
  args: { query: v.string(), cuisines: v.optional(v.array(v.string())) },
  handler: async (ctx, args) => {
    const embedding = await embed(args.query);
    const cuisines = args.cuisines;
    let results;
    if (cuisines !== undefined) {
      results = await ctx.vectorSearch("foods", "by_embedding", {
        vector: embedding,
        limit: 16,
        filter: (q) =>
          q.or(...cuisines.map((cuisine) => q.eq("cuisine", cuisine))),
      });
    } else {
      results = await ctx.vectorSearch("foods", "by_embedding", {
        vector: embedding,
        limit: 16,
      });
    }
    const rows: SearchResult[] = await ctx.runQuery(
      internal.foods.fetchResults,
      { results },
    );
    return rows;
  },
});

export const similarMovies = action({
  args: { query: v.string(), genres: v.optional(v.array(v.string())) },
  handler: async (ctx, args) => {
    const embedding = await embed(args.query);
    const { genres } = args;
    let results;
    if (genres !== undefined) {
      results = await ctx.vectorSearch("movieEmbeddings", "by_embedding", {
        vector: embedding,
        limit: 16,
        filter: (q) => q.or(...genres.map((c) => q.eq("genre", c))),
      });
    } else {
      results = await ctx.vectorSearch("movieEmbeddings", "by_embedding", {
        vector: embedding,
        limit: 16,
      });
    }
    return results;
  },
});
