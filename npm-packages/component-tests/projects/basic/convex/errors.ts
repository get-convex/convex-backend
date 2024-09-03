import { query, components, action } from "./_generated/server";

export const throwSystemErrorFromQuery = query(async (ctx) => {
  await ctx.runQuery(components.errors.throwSystemError.fromQuery, {});
});

export const throwSystemErrorFromAction = action(async (ctx) => {
  await ctx.runAction(components.errors.throwSystemError.fromAction, {});
});

export const tryPaginateWithinComponent = query(async (ctx) => {
  await ctx.runQuery(components.component.messages.tryToPaginate, {});
});
