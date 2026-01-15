import { v } from "convex/values";
import { query, mutation } from "./_generated/server";

export const listMessages = query({
  args: {},
  handler: async (ctx) => {
    const messages = await ctx.db.query("messages").order("desc").take(10);
    return messages;
  },
});

export const sendMessage = mutation({
  args: {
    username: v.string(),
    message: v.string(),
  },
  handler: async (ctx, args) => {
    await ctx.db.insert("messages", {
      username: args.username,
      message: args.message,
    });
  },
});
