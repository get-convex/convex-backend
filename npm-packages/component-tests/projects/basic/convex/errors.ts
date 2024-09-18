import { query, action } from "./_generated/server";
import { components } from "./_generated/api";
import { api } from "./_generated/api";

export const throwSystemErrorFromQuery = query(async (ctx) => {
  await ctx.runQuery(components.errors.throwSystemError.fromQuery, {});
});

export const throwSystemErrorFromAction = action(async (ctx) => {
  await ctx.runAction(components.errors.throwSystemError.fromAction, {});
});

export const tryPaginateWithinComponent = query(async (ctx) => {
  await ctx.runQuery(components.component.messages.tryToPaginate, {});
});

export const tryInfiniteLoop = query(async (ctx) => {
  await ctx.runQuery(api.errors.tryInfiniteLoop, {});
});
