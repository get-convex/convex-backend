import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

export const sendMessage = mutation({
  args: { message: v.string() },
  handler: async (ctx, { message }) => {
    return await ctx.db.insert("messages", {
      channel: "general",
      text: message,
    });
  },
});

export const sendButFail = mutation({
  args: { message: v.string() },
  handler: async (ctx, { message }) => {
    await ctx.db.insert("messages", {
      channel: "general",
      text: message,
    });
    throw new Error("fail within component's mutation");
  },
});

export const allMessages = query({
  args: {},
  returns: v.array(v.string()),
  handler: async (ctx) => {
    const messages = await ctx.db.query("messages").collect();
    return messages.map((message) => message.text);
  },
});
