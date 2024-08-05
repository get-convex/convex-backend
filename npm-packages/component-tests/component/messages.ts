import { v } from "convex/values";
import { mutation, query, action, componentArg } from "./_generated/server";

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

export const hello = action(async (ctx) => {
  const name = componentArg(ctx, "name");
  console.log(`hi from ${name}`);
  return name;
});

export const url = action(async (ctx) => {
  return componentArg(ctx, "url");
});
