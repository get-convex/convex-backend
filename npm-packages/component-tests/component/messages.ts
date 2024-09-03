import { v } from "convex/values";
import { action, mutation, query } from "./_generated/server";

export const hello = action({
  args: {},
  handler: async () => {
    return "hello";
  },
});

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

export const tryToPaginate = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").paginate({
      cursor: null,
      numItems: 10,
    });
  },
});
