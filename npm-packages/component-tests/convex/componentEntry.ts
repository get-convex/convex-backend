import { Doc } from "../component/_generated/dataModel";
import { query, app, action, mutation } from "./_generated/server";

export const list = query(async (ctx): Promise<Doc<"messages">[]> => {
  const result = await ctx.runQuery(app.component.messages.listMessages, {});
  console.log(result);
  return result;
});

export const insert = mutation(
  async (ctx, { channel, text }: { channel: string; text: string }) => {
    await ctx.runMutation(app.component.messages.insertMessage, {
      channel,
      text,
    });
  },
);

export const hello = action(async (ctx) => {
  return await ctx.runAction(app.component.messages.hello, {});
});

export const url = action(async (ctx) => {
  return await ctx.runAction(app.component.messages.url, {});
});

export const envVarQuery = query(async (ctx) => {
  return await ctx.runQuery(app.component.messages.envVarQuery, {});
});
export const envVarAction = action(async (ctx) => {
  return await ctx.runAction(app.component.messages.envVarAction, {});
});
export const systemEnvVarQuery = query(async (ctx) => {
  return await ctx.runQuery(app.component.messages.systemEnvVarQuery, {});
});
export const systemEnvVarAction = action(async (ctx) => {
  return await ctx.runAction(app.component.messages.systemEnvVarAction, {});
});
