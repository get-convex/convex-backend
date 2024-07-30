import { query, app, action } from "./_generated/server";

export const throwSystemErrorFromQuery = query(async (ctx) => {
  await ctx.runQuery(app.component.throwSystemError.fromQuery, {});
});

export const throwSystemErrorFromAction = action(async (ctx) => {
  await ctx.runAction(app.component.throwSystemError.fromAction, {});
});
