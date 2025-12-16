import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const send = mutation({
  args: {
    body: v.string(),
    author: v.string(),
  },
  handler: async (ctx, { body, author }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
});

export const sendImage = mutation({
  args: {
    storageId: v.id("_storage"),
    author: v.string(),
  },
  handler: async (ctx, { storageId, author }) => {
    const message = { body: storageId, author, format: "image" };
    await ctx.db.insert("messages", message);
  },
});
