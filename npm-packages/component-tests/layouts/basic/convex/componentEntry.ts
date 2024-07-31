import { query, app, action } from "./_generated/server";

export const hello = action(async (ctx) => {
  return await ctx.runAction(app.envVars.messages.hello, {});
});

export const url = action(async (ctx) => {
  return await ctx.runAction(app.envVars.messages.url, {});
});
export const envVarQuery = query(async (ctx) => {
  return await ctx.runQuery(app.envVars.messages.envVarQuery, {});
});
export const envVarAction = action(async (ctx) => {
  return await ctx.runAction(app.envVars.messages.envVarAction, {});
});
export const systemEnvVarQuery = query(async (ctx) => {
  return await ctx.runQuery(app.envVars.messages.systemEnvVarQuery, {});
});
export const systemEnvVarAction = action(async (ctx) => {
  return await ctx.runAction(app.envVars.messages.systemEnvVarAction, {});
});
