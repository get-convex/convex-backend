import { v } from "convex/values";
import { query, mutation } from "./_generated/server";

export const list = query({
  args: { channelId: v.id("channels") },
  handler: async (ctx, { channelId }) => {
    return await ctx.db
      .query("messages")
      .filter((q) => q.eq(q.field("channel"), channelId))
      .collect();
  },
});

export const send = mutation({
  args: { channel: v.id("channels"), body: v.string(), author: v.string() },
  handler: async (ctx, { channel, body, author }) => {
    const message = { channel, body, author };
    await ctx.db.insert("messages", message);
  },
});
