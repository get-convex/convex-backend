import { v } from "convex/values";
import { mutation, query, action, componentArgs } from "./_generated/server";

export const listMessages = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").take(10);
  },
});

export const insertMessage = mutation({
  args: { channel: v.string(), text: v.string() },
  handler: async (ctx, { channel, text }) => {
    return await ctx.db.insert("messages", { channel, text });
  },
});

export const hello = action(async () => {
  console.log(`hi from ${componentArgs.name}`);
  return componentArgs.name;
});

export const url = action(async () => {
  return componentArgs.url;
});

export const envVarQuery = query(async () => {
  return process.env.NAME;
});

export const systemEnvVarQuery = query(async () => {
  return process.env.CONVEX_CLOUD_URL;
});

export const envVarAction = action(async () => {
  return process.env.NAME;
});

export const systemEnvVarAction = action(async () => {
  return process.env.CONVEX_CLOUD_URL;
});
