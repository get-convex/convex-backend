import { v } from "convex/values";
import { mutation, query, action } from "./_generated/server";

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
  console.log("hi from v8 action");
});
