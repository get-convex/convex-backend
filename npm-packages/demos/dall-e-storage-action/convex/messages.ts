import { query, internalMutation, mutation } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
  args: {},
  handler: async (ctx) => {
    const messages = await ctx.db.query("messages").collect();
    for (const message of messages) {
      if (message.format === "dall-e") {
        message.body = await ctx.storage.getUrl(message.body);
      }
    }
    return messages;
  },
});

export const sendDallEMessage = internalMutation({
  args: {
    body: v.string(),
    author: v.string(),
    prompt: v.string(),
  },
  handler: async (ctx, { body, author, prompt }) => {
    const message = { body, author, format: "dall-e", prompt };
    await ctx.db.insert("messages", message);
  },
});

export const send = mutation({
  args: {
    body: v.string(),
    author: v.string(),
  },
  handler: async (ctx, { body, author }) => {
    const message = { body, author, format: "text" };
    await ctx.db.insert("messages", message);
  },
});
