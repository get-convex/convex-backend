import { query, action, mutation } from "./_generated/server";
import { components } from "./_generated/api";
import { api } from "./_generated/api";

export const throwSystemErrorFromQuery = query({
  args: {},
  handler: async (ctx) => {
    await ctx.runQuery(components.errors.throwSystemError.fromQuery, {});
  },
});

export const throwSystemErrorFromAction = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runAction(components.errors.throwSystemError.fromAction, {});
  },
});

export const tryPaginateWithinComponent = query({
  args: {},
  handler: async (ctx) => {
    await ctx.runQuery(components.component.messages.tryToPaginate, {});
  },
});

export const tryInfiniteLoop = query({
  args: {},
  handler: async (ctx) => {
    await ctx.runQuery(api.errors.tryInfiniteLoop, {});
  },
});

export const insertDoc = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("table", { name: "emma", age: 27 });
  },
});
