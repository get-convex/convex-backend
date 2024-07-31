import { query, app, action } from "./_generated/server";

export const throwSystemErrorFromQuery = query(async (ctx) => {
  await ctx.runQuery(app.errors.throwSystemError.fromQuery, {});
});

export const throwSystemErrorFromAction = action(async (ctx) => {
  await ctx.runAction(app.errors.throwSystemError.fromAction, {});
});
