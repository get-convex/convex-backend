import { v } from "convex/values";
import { action, mutation, query } from "./_generated/server";
import { components } from "./_generated/api";

export const populateFoods = action({
  args: {},
  handler: async (ctx) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    await ctx.runAction(components.searchComponent.foods.populate);
  },
});

export const cleanUp = mutation({
  args: {},
  handler: async (ctx) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    await ctx.runMutation(components.searchComponent.cleanUp.default);
  },
});

export const fullTextSearchQuery = query({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runQuery(
      components.searchComponent.textSearch.fullTextSearchQuery,
      args,
    );
  },
});

export const fullTextSearchMutation = mutation({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runMutation(
      components.searchComponent.textSearch.fullTextSearchMutation,
      args,
    );
  },
});

export const fullTextSearchMutationWithWrite = mutation({
  args: {
    query: v.string(),
    cuisine: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runMutation(
      components.searchComponent.textSearch.fullTextSearchMutationWithWrite,
      args,
    );
  },
});
