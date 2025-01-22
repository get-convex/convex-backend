import { v } from "convex/values";
import { DatabaseReader, mutation, query } from "./_generated/server";
import { PaginationOptions, paginationOptsValidator } from "convex/server";
import { EXAMPLE_DATA } from "../foodData";

export const fullTextSearchQuery = query({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    return await runQuery(ctx.db, args.query, args.cuisine);
  },
});

export const fullTextSearchMutation = mutation({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    return await runQuery(ctx.db, args.query, args.cuisine);
  },
});

export const fullTextSearchMutationWithWrite = mutation({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    await ctx.db.insert("foods", EXAMPLE_DATA[0]);
    return await runQuery(ctx.db, args.query, args.cuisine);
  },
});

export const paginatedFullTextSearchQuery = query({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
    paginationOptions: paginationOptsValidator,
  },
  handler: async (ctx, args) => {
    return await runQueryPaginated(
      ctx.db,
      args.query,
      args.cuisine,
      args.paginationOptions,
    );
  },
});

export const paginatedFullTextSearchMutation = mutation({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
    paginationOptions: paginationOptsValidator,
  },
  handler: async (ctx, args) => {
    return await runQueryPaginated(
      ctx.db,
      args.query,
      args.cuisine,
      args.paginationOptions,
    );
  },
});

async function runQueryPaginated(
  db: DatabaseReader,
  query: string,
  cuisine: string | undefined,
  paginationOptions: PaginationOptions,
) {
  return await db
    .query("foods")
    .withSearchIndex("by_description", (q) => {
      const result = q.search("description", query);
      if (cuisine) {
        return result.eq("cuisine", cuisine);
      } else {
        return result;
      }
    })
    .paginate(paginationOptions);
}

async function runQuery(
  db: DatabaseReader,
  query: string,
  cuisine: string | undefined,
) {
  return await db
    .query("foods")
    .withSearchIndex("by_description", (q) => {
      const result = q.search("description", query);
      if (cuisine) {
        return result.eq("cuisine", cuisine);
      } else {
        return result;
      }
    })
    .collect();
}
