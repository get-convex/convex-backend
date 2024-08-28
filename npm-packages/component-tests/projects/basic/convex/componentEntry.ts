import { query, components, action } from "./_generated/server";

export const hello = action(async (ctx) => {
  return await ctx.runAction(components.envVars.messages.hello, {});
});

export const envVarQuery = query(async (ctx) => {
  return await ctx.runQuery(components.envVars.messages.envVarQuery, {});
});
export const envVarAction = action(async (ctx) => {
  return await ctx.runAction(components.envVars.messages.envVarAction, {});
});
export const systemEnvVarQuery = query(async (ctx) => {
  return await ctx.runQuery(components.envVars.messages.systemEnvVarQuery, {});
});
export const systemEnvVarAction = action(async (ctx) => {
  return await ctx.runAction(
    components.envVars.messages.systemEnvVarAction,
    {},
  );
});
